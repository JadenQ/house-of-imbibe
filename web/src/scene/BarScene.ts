// scene/ — Phaser 场景：只读地图/形象数据，通过 CustomEvent 与 DOM UI 通信。
import Phaser from "phaser";
import { BAR_MAP, TILE, renderMap } from "../game/tiles";
import { prepareCharacterSheet, DIRS, FRAME_W, FRAME_H, type Dir } from "../game/character";
import type { AvatarData } from "../net/api";

const SPEED = 42; // px/s，240×160 世界里的步行速度
const WALK_CYCLE = [0, 1, 0, 2]; // stand, stepA, stand, stepB
const STEP_MS = 130;

export class BarScene extends Phaser.Scene {
  private avatarData!: AvatarData;
  private player!: Phaser.GameObjects.Sprite;
  private keys!: Record<string, Phaser.Input.Keyboard.Key>;
  private facing: Dir = "s";
  private walkTimer = 0;
  private walkIdx = 0;
  private hintShown = false;

  constructor() {
    super("bar");
  }

  init(data: { avatar: AvatarData }) {
    this.avatarData = data.avatar;
  }

  async create() {
    // 地图背景：整图单纹理
    const mapCanvas = renderMap(BAR_MAP);
    this.textures.addCanvas("bar-map", mapCanvas);
    this.add.image(0, 0, "bar-map").setOrigin(0, 0);

    // 角色 sheet：统一加载层，不分支 kind
    const sheet = await prepareCharacterSheet(this.avatarData);
    const tex = this.textures.addCanvas("hero", sheet)!;
    DIRS.forEach((_, row) => {
      for (let f = 0; f < 3; f++) {
        tex.add(`d${row}f${f}`, 0, f * FRAME_W, row * FRAME_H, FRAME_W, FRAME_H);
      }
    });

    const { tx, ty } = BAR_MAP.spawn;
    this.player = this.add.sprite(tx * TILE + TILE / 2, ty * TILE + TILE / 2, "hero", "d0f0");

    this.keys = this.input.keyboard!.addKeys("W,A,S,D,UP,DOWN,LEFT,RIGHT,E,SPACE") as Record<
      string,
      Phaser.Input.Keyboard.Key
    >;
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
      // 分轴移动，贴墙滑行
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

    const row = DIRS.indexOf(this.facing);
    this.player.setFrame(`d${row}f${WALK_CYCLE[this.walkIdx]}`);

    // 交互：面向吧台/酒架时提示，按 E 打开
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
}
