// scene/ — Phaser 场景：只读 game-state 的 RenderView，通过 Transport 发意图。
// 不在 net/protocol/game-state 里 import phaser（分层约束；本文件在 scene/，可用 phaser）。
// 无 avatar kind 分支（禁令 #3）：prepareCharacterSheet 装载层统一 modular/generated。

import Phaser from "phaser";
import { BAR_MAP, TILE, renderMap } from "../game/tiles";
import { prepareCharacterSheet, DIRS, FRAME_W, FRAME_H, type Dir } from "../game/character";
import type { AvatarData } from "../net/api";
import type { Transport } from "../net/transport";
import { parseMsg, msg } from "../protocol/types";
import { initialRoomState, applyServerMsg, interpolate } from "../game-state/room";
import type { RoomState, RenderView } from "../game-state/types";
import type { ChatPanel } from "../ui/chat";

const SPEED = 42; // px/s，240×160 世界里的步行速度
const WALK_CYCLE = [0, 1, 0, 2]; // stand, stepA, stand, stepB
const STEP_MS = 130;
const MOVE_SEND_INTERVAL_MS = 100;
const RECONCILE_THRESHOLD_PX = 12;
const RECONCILE_LERP = 0.15;
const PING_INTERVAL_MS = 5000;
const BUBBLE_OFFSET_Y = 18;

export interface BarSceneInit {
  avatar: AvatarData;
  transport: Transport;
  selfId: number;
  chatPanel: ChatPanel;
}

export class BarScene extends Phaser.Scene {
  private opts!: BarSceneInit;
  private avatarData!: AvatarData;
  private player!: Phaser.GameObjects.Sprite;
  private keys!: Record<string, Phaser.Input.Keyboard.Key>;
  private facing: Dir = "s";
  private walkTimer = 0;
  private walkIdx = 0;
  private hintShown = false;

  private roomState!: RoomState;
  private remotes: Map<number, Phaser.GameObjects.Sprite> = new Map();
  private sheetCache: Map<string, string> = new Map(); // avatarHash -> texKey
  private preparing: Set<string> = new Set();
  private bubbleTexts: Map<number, Phaser.GameObjects.Text> = new Map();
  private lastSentTile = { tx: -1, ty: -1 };
  private lastChatLen = 0;
  private pendingPing = 0;
  private pingTimer: ReturnType<typeof setInterval> | null = null;
  private lastSendAt = 0;

  constructor() {
    super("bar");
  }

  init(data: BarSceneInit) {
    this.opts = data;
    this.avatarData = data.avatar;
  }

  async create() {
    // 地图背景：整图单纹理
    const mapCanvas = renderMap(BAR_MAP);
    this.textures.addCanvas("bar-map", mapCanvas);
    this.add.image(0, 0, "bar-map").setOrigin(0, 0);

    // 本地玩家 sheet（装载层，不分支 kind）
    const sheet = await prepareCharacterSheet(this.avatarData);
    this.registerSheet("hero", sheet);

    this.roomState = initialRoomState(this.opts.selfId);
    this.player = this.add.sprite(this.roomState.localX, this.roomState.localY, "hero", "d0f0");
    this.player.setDepth(this.player.y);

    this.keys = this.input.keyboard!.addKeys("W,A,S,D,UP,DOWN,LEFT,RIGHT,E,SPACE") as Record<
      string,
      Phaser.Input.Keyboard.Key
    >;

    // 服务端消息 → 纯状态机
    this.opts.transport.onMessage((raw) => {
      const m = parseMsg(raw);
      if (!m) return;
      if (m.type === "pong") {
        this.roomState = applyServerMsg(this.roomState, m, this.pendingPing);
      } else {
        this.roomState = applyServerMsg(this.roomState, m);
      }
    });

    // 时钟同步 ping（服务端 pong.t = 服务器 ms）
    const sendPing = () => {
      this.pendingPing = performance.now();
      this.opts.transport.send(JSON.stringify(msg.ping(this.pendingPing)));
    };
    sendPing();
    this.pingTimer = setInterval(sendPing, PING_INTERVAL_MS);
  }

  /** 装一张 canvas sheet 为纹理并注册 3 列×4 行帧（与本地 hero 同布局）。 */
  private registerSheet(texKey: string, sheet: HTMLCanvasElement) {
    const tex = this.textures.addCanvas(texKey, sheet)!;
    DIRS.forEach((_, row) => {
      for (let f = 0; f < 3; f++) {
        tex.add(`d${row}f${f}`, 0, f * FRAME_W, row * FRAME_H, FRAME_W, FRAME_H);
      }
    });
  }

  private solidAt(px: number, py: number): boolean {
    const tx = Math.floor(px / TILE);
    const ty = Math.floor(py / TILE);
    const row = BAR_MAP.rows[ty];
    if (!row) return true;
    return BAR_MAP.solid.has(row[tx] ?? "#");
  }

  /** 以角色脚底小盒子做碰撞（头顶允许伸到吧台前景里，GBA 惯例） */
  private canStand(x: number, y: number): boolean {
    const hw = 5;
    const feet = [y + 4, y + 7];
    for (const fy of feet) {
      if (this.solidAt(x - hw, fy) || this.solidAt(x + hw, fy)) return false;
    }
    return true;
  }

  private facingTile(): string | null {
    const d: Record<Dir, [number, number]> = { s: [0, 1], n: [0, -1], w: [-1, 0], e: [1, 0] };
    const [dx, dy] = d[this.facing];
    const tx = Math.floor(this.player.x / TILE) + dx;
    const ty = Math.floor(this.player.y / TILE) + dy;
    return BAR_MAP.rows[ty]?.[tx] ?? null;
  }

  update(_time: number, delta: number) {
    // ── 本地预测：输入 → 移动 + 碰撞（与单机版一致）──
    const k = this.keys;
    let vx = 0;
    let vy = 0;
    if (k.A.isDown || k.LEFT.isDown) vx = -1;
    else if (k.D.isDown || k.RIGHT.isDown) vx = 1;
    if (k.W.isDown || k.UP.isDown) vy = -1;
    else if (k.S.isDown || k.DOWN.isDown) vy = 1;

    const moving = vx !== 0 || vy !== 0;
    if (moving) {
      if (vy < 0) this.facing = "n";
      else if (vy > 0) this.facing = "s";
      else if (vx < 0) this.facing = "w";
      else if (vx > 0) this.facing = "e";

      const dist = (SPEED * delta) / 1000;
      const len = Math.hypot(vx, vy);
      const nx = this.player.x + (vx / len) * dist;
      const ny = this.player.y + (vy / len) * dist;
      if (this.canStand(nx, this.player.y)) this.player.x = nx;
      if (this.canStand(this.player.x, ny)) this.player.y = ny;

      this.walkTimer += delta;
      if (this.walkTimer >= STEP_MS) {
        this.walkTimer = 0;
        this.walkIdx = (this.walkIdx + 1) % WALK_CYCLE.length;
      }
    } else {
      this.walkIdx = 0;
      this.walkTimer = 0;
    }
    this.roomState.localX = this.player.x;
    this.roomState.localY = this.player.y;
    this.roomState.localDir = this.facing;
    this.player.setFrame(`d${DIRS.indexOf(this.facing)}f${WALK_CYCLE[this.walkIdx]}`);
    this.player.setDepth(this.player.y);

    // ── 发移动意图（tile 变化 / 移动中 100ms 节流）──
    const tx = Math.floor(this.player.x / TILE);
    const ty = Math.floor(this.player.y / TILE);
    const now = performance.now();
    if (
      tx !== this.lastSentTile.tx ||
      ty !== this.lastSentTile.ty ||
      (moving && now - this.lastSendAt > MOVE_SEND_INTERVAL_MS)
    ) {
      this.opts.transport.send(JSON.stringify(msg.move(tx, ty)));
      this.lastSentTile = { tx, ty };
      this.lastSendAt = now;
    }

    // ── 服务端纠正：偏差大则平滑收敛（非硬 snap，避免抽搐）──
    const self = this.roomState.players.get(this.opts.selfId);
    if (self && self.samples.length) {
      const last = self.samples[self.samples.length - 1];
      if (Math.hypot(this.player.x - last.x, this.player.y - last.y) > RECONCILE_THRESHOLD_PX) {
        this.player.x += (last.x - this.player.x) * RECONCILE_LERP;
        this.player.y += (last.y - this.player.y) * RECONCILE_LERP;
        this.roomState.localX = this.player.x;
        this.roomState.localY = this.player.y;
      }
    }

    // ── 远端玩家渲染（来自 RenderView；sheet 按 hash 缓存，异步装载）──
    const view = interpolate(this.roomState, performance.now(), 120);
    const seen = new Set<number>();
    for (const pv of view.players) {
      seen.add(pv.id);
      let spr = this.remotes.get(pv.id);
      if (!spr) {
        // 占位（不可见，等 sheet 装好）；用 hero 纹理占 frame 名空间
        spr = this.add.sprite(pv.x, pv.y, "hero", "d0f0").setVisible(false);
        this.remotes.set(pv.id, spr);
      }
      // 异步装载该 avatar 的 sheet（modular 近同步，generated 需下载）
      if (!this.sheetCache.has(pv.avatarHash) && !this.preparing.has(pv.avatarHash)) {
        this.preparing.add(pv.avatarHash);
        prepareCharacterSheet(pv.avatar)
          .then((sheet2) => {
            const texKey = `av_${pv.avatarHash}`;
            if (!this.sheetCache.has(texKey)) {
              this.registerSheet(texKey, sheet2);
              this.sheetCache.set(texKey, texKey);
            }
            this.preparing.delete(pv.avatarHash);
          })
          .catch(() => this.preparing.delete(pv.avatarHash));
      }
      const texKey = this.sheetCache.get(pv.avatarHash);
      if (texKey) {
        if (spr.texture.key !== texKey) spr.setTexture(texKey);
        spr.setVisible(true);
        spr.setFrame(`d${DIRS.indexOf(pv.dir as Dir)}f${pv.frame}`);
        spr.x = pv.x;
        spr.y = pv.y;
        spr.setDepth(pv.y);
      }
    }
    // 清除离开视野的远端 sprite
    for (const [id, spr] of this.remotes) {
      if (!seen.has(id)) {
        spr.destroy();
        this.remotes.delete(id);
        this.bubbleTexts.get(id)?.destroy();
        this.bubbleTexts.delete(id);
      }
    }

    // ── 聊天气泡（每人最新一条，仍在 4s TTL 内即显示）──
    this.renderBubbles(view);

    // ── 聊天侧栏（条数变化时刷新）──
    if (this.roomState.chat.length !== this.lastChatLen) {
      this.opts.chatPanel.setChat(this.roomState.chat);
      this.lastChatLen = this.roomState.chat.length;
    }

    // ── 交互：面向吧台/酒架时提示，按 E 开酒单（不变）──
    const ch = this.facingTile();
    const interactId = ch ? BAR_MAP.interact[ch] : undefined;
    if (interactId && !this.hintShown) {
      this.hintShown = true;
      window.dispatchEvent(new CustomEvent("hoi:hint", { detail: "按 E 查看酒单" }));
    } else if (!interactId && this.hintShown) {
      this.hintShown = false;
      window.dispatchEvent(new CustomEvent("hoi:hint", { detail: null }));
    }
    if (
      interactId &&
      (Phaser.Input.Keyboard.JustDown(this.keys.E) || Phaser.Input.Keyboard.JustDown(this.keys.SPACE))
    ) {
      window.dispatchEvent(new CustomEvent("hoi:interact", { detail: interactId }));
    }
  }

  /** 渲染聊天气泡：view.bubbles 已被 interpolate 过滤到 4s TTL 内；每人取最新。 */
  private renderBubbles(view: RenderView) {
    const latest = new Map<number, string>();
    for (const b of view.bubbles) latest.set(b.playerId, b.text);

    const placeFor = (pid: number): { x: number; y: number } | null => {
      if (pid === this.opts.selfId) return { x: this.player.x, y: this.player.y };
      const spr = this.remotes.get(pid);
      return spr ? { x: spr.x, y: spr.y } : null;
    };

    const active = new Set<number>();
    for (const [pid, text] of latest) {
      const pos = placeFor(pid);
      if (!pos) continue;
      active.add(pid);
      let t = this.bubbleTexts.get(pid);
      if (!t) {
        t = this.add
          .text(0, 0, "", {
            fontSize: "10px",
            color: "#f0e0c0",
            backgroundColor: "#14100e",
            padding: { x: 3, y: 2 },
          })
          .setOrigin(0.5, 1)
          .setDepth(9999);
        this.bubbleTexts.set(pid, t);
      }
      t.setText(text);
      t.setPosition(pos.x, pos.y - BUBBLE_OFFSET_Y);
      t.setVisible(true);
    }
    for (const [pid, t] of this.bubbleTexts) {
      if (!active.has(pid)) t.setVisible(false);
    }
  }

  shutdown() {
    if (this.pingTimer) {
      clearInterval(this.pingTimer);
      this.pingTimer = null;
    }
    this.opts.transport.close();
  }
}
