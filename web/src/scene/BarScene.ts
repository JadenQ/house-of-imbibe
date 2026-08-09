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
import { dominantDir } from "../game-state/joystick";
import type { RoomState, RenderView } from "../game-state/types";
import type { ChatPanel } from "../ui/chat";
import type { TouchControls } from "../ui/touch";

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
  /** 触控层（仅触控设备存在；桌面为 undefined，走键盘回退）。 */
  touch?: TouchControls;
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
  private touch?: TouchControls;
  private decorationSprites: Map<string, Phaser.GameObjects.Image> = new Map();
  private decTexCache: Map<string, string> = new Map(); // asset_key -> texKey
  private decTexLoading: Set<string> = new Set();

  constructor() {
    super("bar");
  }

  init(data: BarSceneInit) {
    this.opts = data;
    this.avatarData = data.avatar;
    this.touch = data.touch;
  }

  async create() {
    // 地图背景：整图单纹理（depth=-10 确保在 z_layer<0 装饰之下）
    const mapCanvas = renderMap(BAR_MAP);
    this.textures.addCanvas("bar-map", mapCanvas);
    this.add.image(0, 0, "bar-map").setOrigin(0, 0).setDepth(-10);

    // 占位装饰纹理（半透明色块 + 边框，表明「占位装饰」）
    const phCanvas = document.createElement("canvas");
    phCanvas.width = TILE;
    phCanvas.height = TILE;
    const pctx = phCanvas.getContext("2d")!;
    pctx.fillStyle = "rgba(255, 215, 0, 0.25)";
    pctx.fillRect(0, 0, TILE, TILE);
    pctx.strokeStyle = "rgba(255, 215, 0, 0.7)";
    pctx.lineWidth = 1;
    pctx.strokeRect(0.5, 0.5, TILE - 1, TILE - 1);
    this.textures.addCanvas("deco-placeholder", phCanvas);

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
    // 键盘为二值 {-1,0,1}²（归一化到满速）；触控摇杆为模拟量 [-1,1]²（模长即速度）。
    // 摇杆 active 时覆盖键盘；桌面无触控层 → 走原 WASD 路径不变（回退方案）。
    const k = this.keys;
    let vx = 0;
    let vy = 0;
    if (k.A.isDown || k.LEFT.isDown) vx = -1;
    else if (k.D.isDown || k.RIGHT.isDown) vx = 1;
    if (k.W.isDown || k.UP.isDown) vy = -1;
    else if (k.S.isDown || k.DOWN.isDown) vy = 1;

    // 触控模拟量覆盖键盘（同一条本地预测路径：facing/碰撞/发送/纠正/插值不变）
    let analog = false;
    const ti = this.touch?.getInput();
    if (ti && ti.active && (ti.x !== 0 || ti.y !== 0)) {
      vx = ti.x;
      vy = ti.y;
      analog = true;
    }

    const moving = vx !== 0 || vy !== 0;
    if (moving) {
      if (analog) {
        // 模拟量用主轴方向（微倾不误判；键盘仍走 vy-priority 原逻辑不变）
        const d = dominantDir(vx, vy);
        if (d) this.facing = d;
      } else {
        if (vy < 0) this.facing = "n";
        else if (vy > 0) this.facing = "s";
        else if (vx < 0) this.facing = "w";
        else if (vx > 0) this.facing = "e";
      }

      const dist = (SPEED * delta) / 1000;
      // 模拟量向量已在 [-1,1] 且模长 ≤1 → 直接乘 dist（模长即速度）；
      // 键盘二值需归一化到单位向量（对角线不加速）。
      let mx: number;
      let my: number;
      if (analog) {
        mx = vx * dist;
        my = vy * dist;
      } else {
        const len = Math.hypot(vx, vy);
        mx = (vx / len) * dist;
        my = (vy / len) * dist;
      }
      const nx = this.player.x + mx;
      const ny = this.player.y + my;
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

    // ── 装饰渲染（来自 RenderView.decorations；asset_key 非空→按 /api/assets/{key} 取 PNG，null→占位）──
    this.renderDecorations(view);

    // ── 聊天气泡（每人最新一条，仍在 4s TTL 内即显示）──
    this.renderBubbles(view);

    // ── 聊天侧栏（条数变化时刷新）──
    if (this.roomState.chat.length !== this.lastChatLen) {
      this.opts.chatPanel.setChat(this.roomState.chat);
      this.lastChatLen = this.roomState.chat.length;
    }

    // ── 交互：面向吧台/酒架时提示，按 E / 动作键开酒单（不变）──
    const ch = this.facingTile();
    const interactId = ch ? BAR_MAP.interact[ch] : undefined;
    if (interactId && !this.hintShown) {
      this.hintShown = true;
      window.dispatchEvent(new CustomEvent("hoi:hint", { detail: "按 E 查看酒单" }));
    } else if (!interactId && this.hintShown) {
      this.hintShown = false;
      window.dispatchEvent(new CustomEvent("hoi:hint", { detail: null }));
    }
    // 动作键 = E：consumeInteract 每帧消费边沿（无论是否面向可交互物），
    // 避免按下未命中后走过去触发陈旧 interact；与 JustDown 同为边沿语义。
    const wantInteract =
      Phaser.Input.Keyboard.JustDown(this.keys.E) ||
      Phaser.Input.Keyboard.JustDown(this.keys.SPACE) ||
      (this.touch?.consumeInteract() ?? false);
    if (interactId && wantInteract) {
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

  /** 异步装载装饰纹理（按 asset_key 缓存；Image 元素 → textures.addImage）。 */
  private loadDecTex(assetKey: string): void {
    if (this.decTexCache.has(assetKey) || this.decTexLoading.has(assetKey)) return;
    this.decTexLoading.add(assetKey);
    const texKey = `dec_${assetKey.replace(/[^a-zA-Z0-9_]/g, "_")}`;
    const img = new Image();
    img.onload = () => {
      if (!this.textures.exists(texKey)) {
        this.textures.addImage(texKey, img);
      }
      this.decTexCache.set(assetKey, texKey);
      this.decTexLoading.delete(assetKey);
    };
    img.onerror = () => {
      this.decTexLoading.delete(assetKey);
    };
    img.src = `/api/assets/${assetKey}`;
  }

  /** 渲染装饰对象：asset_key 非空→异步装 PNG 纹理 + drawImage；null→占位色块。
   *  depth：z_layer<0 设 y-1（玩家下方），z_layer>=0 设 y+1000（玩家上方）。 */
  private renderDecorations(view: RenderView) {
    const seen = new Set<string>();
    for (const dec of view.decorations) {
      seen.add(dec.id);
      let spr = this.decorationSprites.get(dec.id);
      if (dec.asset_key) {
        // 异步装载纹理（按 asset_key 缓存，复用 sheetCache 模式）
        if (!this.decTexCache.has(dec.asset_key) && !this.decTexLoading.has(dec.asset_key)) {
          this.loadDecTex(dec.asset_key);
        }
        const texKey = this.decTexCache.get(dec.asset_key);
        if (texKey && this.textures.exists(texKey)) {
          if (!spr) {
            spr = this.add.image(dec.x, dec.y, texKey).setOrigin(0, 0);
            this.decorationSprites.set(dec.id, spr);
          } else if (spr.texture.key !== texKey) {
            spr.setTexture(texKey);
          }
          spr.setPosition(dec.x, dec.y);
          spr.setOrigin(0, 0);
          spr.setVisible(true);
        } else {
          // 纹理未就绪：创建/复用占位 sprite（隐），等纹理就绪后替换
          if (!spr) {
            spr = this.add.image(dec.x, dec.y, "deco-placeholder").setOrigin(0, 0).setVisible(false);
            this.decorationSprites.set(dec.id, spr);
          }
        }
      } else {
        // asset_key 为 null → 占位标记（半透明色块 + 边框）
        if (!spr) {
          spr = this.add.image(dec.x, dec.y, "deco-placeholder").setOrigin(0, 0);
          this.decorationSprites.set(dec.id, spr);
        }
        spr.setPosition(dec.x, dec.y);
        spr.setOrigin(0, 0);
        spr.setVisible(true);
      }
      // z_layer<0 在玩家下方（地板/地毯）；>=0 在玩家上方（墙上挂画/头顶吊灯）
      spr.setDepth(dec.z_layer < 0 ? dec.y - 1 : dec.y + 1000);
    }
    // 清除离开视野的装饰 sprite
    for (const [id, spr] of this.decorationSprites) {
      if (!seen.has(id)) {
        spr.destroy();
        this.decorationSprites.delete(id);
      }
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
