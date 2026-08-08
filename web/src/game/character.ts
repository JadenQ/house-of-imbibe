// game/ — 角色 sprite sheet 运行时合成（纯 canvas，不 import phaser）。
// 契约（docs 定稿前的临时版本）：帧 16×16，行=方向 [s,n,w,e]，列=帧 [stand, stepA, stepB]。
// 正式契约将按 sprite-contract.md 预留 8 方向槽，MVP 只渲染 4 主方向。

import type { ModularAvatar, GeneratedAvatar, AvatarData } from "../net/api";

export const FRAME_W = 16;
export const FRAME_H = 16;
export const DIRS = ["s", "n", "w", "e"] as const;
export type Dir = (typeof DIRS)[number];

type Ctx = CanvasRenderingContext2D;

function px(ctx: Ctx, x: number, y: number, w: number, h: number, color: string) {
  ctx.fillStyle = color;
  ctx.fillRect(x, y, w, h);
}

function shade(hex: string, f: number): string {
  const n = parseInt(hex.slice(1), 16);
  const r = Math.min(255, Math.round(((n >> 16) & 255) * f));
  const g = Math.min(255, Math.round(((n >> 8) & 255) * f));
  const b = Math.min(255, Math.round((n & 255) * f));
  return `rgb(${r},${g},${b})`;
}

/** 在 (ox,oy) 处画一帧人物。dir 方向，step: 0 站立 1/2 迈步 */
function drawFrame(ctx: Ctx, ox: number, oy: number, dir: Dir, step: number, cfg: ModularAvatar) {
  const skin = cfg.skin;
  const skinDark = shade(cfg.skin, 0.82);
  const hair = cfg.hair;
  const hairDark = shade(cfg.hair, 0.75);
  const shirt = cfg.shirt;
  const shirtDark = shade(cfg.shirt, 0.75);
  const pants = cfg.pants;
  const outline = "rgba(20,12,8,0.9)";

  // 腿：迈步帧左右交替
  const legOffA = step === 1 ? 1 : 0;
  const legOffB = step === 2 ? 1 : 0;
  px(ctx, ox + 5 - legOffA, oy + 12, 2, 3, pants);
  px(ctx, ox + 9 + legOffB, oy + 12, 2, 3, pants);
  px(ctx, ox + 5 - legOffA, oy + 15, 2, 1, outline); // 鞋
  px(ctx, ox + 9 + legOffB, oy + 15, 2, 1, outline);

  // 身体（衬衫）
  px(ctx, ox + 4, oy + 8, 8, 5, shirt);
  px(ctx, ox + 4, oy + 12, 8, 1, shirtDark);
  // 手臂：迈步时前后摆
  const armA = step === 1 ? 1 : 0;
  const armB = step === 2 ? 1 : 0;
  px(ctx, ox + 3, oy + 8 + armA, 1, 4, shirtDark);
  px(ctx, ox + 12, oy + 8 + armB, 1, 4, shirtDark);

  // 头
  px(ctx, ox + 4, oy + 1, 8, 7, skin);
  px(ctx, ox + 4, oy + 7, 8, 1, skinDark);

  // 头发与五官按方向
  if (dir === "s") {
    px(ctx, ox + 4, oy + 1, 8, 2, hair);
    px(ctx, ox + 4, oy + 3, 1, 2, hair);
    px(ctx, ox + 11, oy + 3, 1, 2, hair);
    px(ctx, ox + 6, oy + 5, 1, 1, "#201510"); // 眼
    px(ctx, ox + 9, oy + 5, 1, 1, "#201510");
  } else if (dir === "n") {
    px(ctx, ox + 4, oy + 1, 8, 6, hair); // 背面全是头发
    px(ctx, ox + 4, oy + 6, 8, 1, hairDark);
  } else {
    // w / e 侧面
    const faceRight = dir === "e";
    px(ctx, ox + 4, oy + 1, 8, 2, hair);
    const backX = faceRight ? ox + 4 : ox + 10;
    px(ctx, backX, oy + 3, 2, 4, hair);
    px(ctx, faceRight ? ox + 9 : ox + 6, oy + 5, 1, 1, "#201510"); // 单眼
    px(ctx, faceRight ? ox + 11 : ox + 4, oy + 6, 1, 1, skinDark); // 鼻
  }
}

/** 生成整张 sheet：3 列 × 4 行（48×64），返回 canvas 供 textures.addCanvas 使用 */
export function characterSheet(cfg: ModularAvatar): HTMLCanvasElement {
  const c = document.createElement("canvas");
  c.width = FRAME_W * 3;
  c.height = FRAME_H * DIRS.length;
  const ctx = c.getContext("2d")!;
  ctx.imageSmoothingEnabled = false;
  DIRS.forEach((dir, row) => {
    for (let f = 0; f < 3; f++) drawFrame(ctx, f * FRAME_W, row * FRAME_H, dir, f, cfg);
  });
  return c;
}

/** 从 4 方向 URL 加载生成形象，合成 sprite sheet（3 列 × 4 行，每帧 16×16） */
export async function loadGeneratedSheet(avatar: GeneratedAvatar): Promise<HTMLCanvasElement> {
  const dirMap: Record<string, number> = { south: 0, north: 1, west: 2, east: 3 };

  // 下载各方向 PNG
  const images: (HTMLImageElement | null)[] = [null, null, null, null];
  await Promise.all(
    avatar.rotations.map(async (r) => {
      const row = dirMap[r.direction];
      if (row === undefined) return;
      const img = new Image();
      img.crossOrigin = "anonymous";
      await new Promise<void>((resolve, reject) => {
        img.onload = () => resolve();
        img.onerror = () => reject(new Error(`Failed to load ${r.direction} sprite`));
        img.src = r.url;
      });
      images[row] = img;
    }),
  );

  // 合成 sprite sheet: 3 列（stand/stepA/stepB）× 4 行（方向）
  // MVP: 每个方向只有 1 帧静态图，3 列用同一帧
  const c = document.createElement("canvas");
  c.width = FRAME_W * 3;
  c.height = FRAME_H * DIRS.length;
  const ctx = c.getContext("2d")!;
  ctx.imageSmoothingEnabled = false;

  for (let row = 0; row < 4; row++) {
    const img = images[row];
    if (!img) continue;
    for (let col = 0; col < 3; col++) {
      // 将 92×92 缩放到 16×16（nearest-neighbor）
      ctx.drawImage(img, 0, 0, img.width, img.height, col * FRAME_W, row * FRAME_H, FRAME_W, FRAME_H);
    }
  }

  return c;
}

/** 统一加载层：无论模块化还是生成，都返回 canvas sprite sheet */
export async function prepareCharacterSheet(data: AvatarData): Promise<HTMLCanvasElement> {
  if (data.kind === "modular") {
    return characterSheet(data);
  } else {
    return loadGeneratedSheet(data);
  }
}

/** 方向行号（供动画定义使用） */
export function dirRow(dir: Dir): number {
  return DIRS.indexOf(dir);
}
