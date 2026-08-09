// game/ — 角色 sprite sheet 运行时合成（纯 canvas，不 import phaser）。
// 契约：帧 16×16，行=方向 [s,n,w,e]，列=帧 [stand, stepA, stepB]。
// 切片 2a（modular 捏脸）：发型/上衣/下装/鞋子样式变体 + 颜色，程序化像素绘制。
// GBA 绿宝石画风、16×16、轮廓清晰。每种样式有可辨认的剪影差异。
// 向后兼容：样式字段缺失 → 默认 short/tshirt/pants/boots，渲染与扩展前逐像素一致。

import type { ModularAvatar, GeneratedAvatar, AvatarData, EquippedItem } from "../net/api";

export const FRAME_W = 16;
export const FRAME_H = 16;
export const DIRS = ["s", "n", "w", "e"] as const;
export type Dir = (typeof DIRS)[number];

// ── 样式类型 + 选项列表（供 UI + 测试）──
export type HairStyle = "short" | "long" | "bald" | "cap";
export type TopStyle = "tshirt" | "longsleeve" | "vest";
export type BottomStyle = "pants" | "shorts" | "skirt";
export type ShoeStyle = "boots" | "sneakers" | "sandals";

export const HAIR_STYLES: readonly HairStyle[] = ["short", "long", "bald", "cap"];
export const TOP_STYLES: readonly TopStyle[] = ["tshirt", "longsleeve", "vest"];
export const BOTTOM_STYLES: readonly BottomStyle[] = ["pants", "shorts", "skirt"];
export const SHOE_STYLES: readonly ShoeStyle[] = ["boots", "sneakers", "sandals"];

export const DEFAULT_HAIR_STYLE: HairStyle = "short";
export const DEFAULT_TOP_STYLE: TopStyle = "tshirt";
export const DEFAULT_BOTTOM_STYLE: BottomStyle = "pants";
export const DEFAULT_SHOE_STYLE: ShoeStyle = "boots";
/** 默认鞋色 = 原轮廓色（向后兼容：扩展前鞋用此色）。 */
export const DEFAULT_SHOE_COLOR = "rgba(20,12,8,0.9)";

/** 在已知选项里挑；未知/缺失 → fallback。 */
function pick<T extends string>(v: string | undefined, opts: readonly T[], fallback: T): T {
  return v && (opts as readonly string[]).includes(v) ? (v as T) : fallback;
}

export interface ResolvedStyles {
  hairStyle: HairStyle;
  topStyle: TopStyle;
  bottomStyle: BottomStyle;
  shoeStyle: ShoeStyle;
  shoes: string;
}

/**
 * 把可选样式字符串解析为已校验的联合 + 默认值。纯函数，无 DOM/canvas。
 * 向后兼容：字段缺失或未知值 → 默认样式（short/tshirt/pants/boots）。
 */
export function resolveStyles(cfg: {
  hairStyle?: string;
  topStyle?: string;
  bottomStyle?: string;
  shoeStyle?: string;
  shoes?: string;
}): ResolvedStyles {
  return {
    hairStyle: pick(cfg.hairStyle, HAIR_STYLES, DEFAULT_HAIR_STYLE),
    topStyle: pick(cfg.topStyle, TOP_STYLES, DEFAULT_TOP_STYLE),
    bottomStyle: pick(cfg.bottomStyle, BOTTOM_STYLES, DEFAULT_BOTTOM_STYLE),
    shoeStyle: pick(cfg.shoeStyle, SHOE_STYLES, DEFAULT_SHOE_STYLE),
    shoes: cfg.shoes ?? DEFAULT_SHOE_COLOR,
  };
}

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

interface PaintColors {
  skin: string;
  skinDark: string;
  hair: string;
  hairDark: string;
  shirt: string;
  shirtDark: string;
  pants: string;
  pantsDark: string;
  shoes: string;
}

const EYE = "#201510";
const SNEAKER_WHITE = "#f0e8d8";
const ACC_PLACEHOLDER = "#c0a060"; // 占位配件色（asset_key null）

/** accessory asset_key → 已加载的 HTMLImageElement（复用 loadAssetImage 缓存模式）。 */
type AccessoryImages = Map<string, HTMLImageElement>;

/** 按 storage key 经 /api/assets/{key} 取一张资产图（唯一拼 URL 的地方在公共 URL 形状）。 */
function loadAssetImage(key: string): Promise<HTMLImageElement> {
  const img = new Image();
  img.crossOrigin = "anonymous";
  return new Promise((resolve, reject) => {
    img.onload = () => resolve(img);
    img.onerror = () => reject(new Error(`Failed to load asset ${key}`));
    img.src = `/api/assets/${key}`;
  });
}

/** 预装所有 equipped 配件 PNG（asset_key 非空项）；失败静默跳过（drawAccImg 画占位色块）。 */
async function loadAccessoryImages(
  equipped: EquippedItem[] | undefined,
): Promise<AccessoryImages> {
  const map: AccessoryImages = new Map();
  if (!equipped) return map;
  await Promise.all(
    equipped
      .filter((item) => item.asset_key)
      .map(async (item) => {
        try {
          map.set(item.asset_key!, await loadAssetImage(item.asset_key!));
        } catch {
          /* ignore: drawAccImg 画占位 */
        }
      }),
  );
  return map;
}

/** 画配件图：有 PNG → drawImage 缩放到 w×h；无 → 占位色块。 */
function drawAccImg(
  ctx: Ctx,
  img: HTMLImageElement | undefined,
  x: number,
  y: number,
  w: number,
  h: number,
) {
  if (img) {
    ctx.drawImage(img, 0, 0, img.width, img.height, x, y, w, h);
  } else {
    px(ctx, x, y, w, h, ACC_PLACEHOLDER);
  }
}

/** back slot：在身体后方画（z 在身体下，先于身体绘制）。
 *  dir n（朝北/背朝观察者）时全显；dir s（朝南/面对观察者）时部分被身体挡。
 *  简单 anchor offset：dir n 在背后中央偏上，dir s 在背后偏下被身体覆盖。 */
function drawBackAccessory(
  ctx: Ctx,
  ox: number,
  oy: number,
  dir: Dir,
  item: EquippedItem,
  imgs: AccessoryImages | undefined,
) {
  const img = item.asset_key ? imgs?.get(item.asset_key) : undefined;
  if (dir === "n") {
    // 背朝观察者：背包全显，在身体后方中央
    drawAccImg(ctx, img, ox + 4, oy + 4, 8, 6);
  } else if (dir === "s") {
    // 面对观察者：背包在身后，被身体大部分挡住，只露上方边缘
    drawAccImg(ctx, img, ox + 5, oy + 6, 6, 3);
  } else {
    // 侧面：背包在角色背后一侧
    const faceRight = dir === "e";
    drawAccImg(ctx, img, faceRight ? ox + 3 : ox + 9, oy + 4, 4, 6);
  }
}

/** hand slot：在身体侧手位置画（z 在身体上，后于身体绘制）。
 *  dir e 在右侧、dir w 在左侧、dir s/n 在右手侧。 */
function drawHandAccessory(
  ctx: Ctx,
  ox: number,
  oy: number,
  dir: Dir,
  item: EquippedItem,
  imgs: AccessoryImages | undefined,
) {
  const img = item.asset_key ? imgs?.get(item.asset_key) : undefined;
  // 手在 y9-y12 区间，x 随方向变化
  if (dir === "w") {
    drawAccImg(ctx, img, ox + 1, oy + 9, 3, 3);
  } else {
    // s/n/e 都在右手侧
    drawAccImg(ctx, img, ox + 12, oy + 9, 3, 3);
  }
}

/** 在 (ox,oy) 处画一帧人物。dir 方向，step: 0 站立 1/2 迈步。
 *  默认样式（short/tshirt/pants/boots）与扩展前逐像素一致。
 *  equipped + accImgs 可选：有则画 back/hand accessory overlay（modular 专用）。 */
function drawFrame(
  ctx: Ctx,
  ox: number,
  oy: number,
  dir: Dir,
  step: number,
  cfg: ModularAvatar,
  equipped?: EquippedItem[],
  accImgs?: AccessoryImages,
) {
  const s = resolveStyles(cfg);
  const colors: PaintColors = {
    skin: cfg.skin,
    skinDark: shade(cfg.skin, 0.82),
    hair: cfg.hair,
    hairDark: shade(cfg.hair, 0.75),
    shirt: cfg.shirt,
    shirtDark: shade(cfg.shirt, 0.75),
    pants: cfg.pants,
    pantsDark: shade(cfg.pants, 0.75),
    shoes: s.shoes,
  };

  // 迈步偏移：腿/臂左右交替
  const legOffA = step === 1 ? 1 : 0;
  const legOffB = step === 2 ? 1 : 0;
  const armA = step === 1 ? 1 : 0;
  const armB = step === 2 ? 1 : 0;

  // ── 背饰（back slot，z 在身体下，先画）──
  if (equipped) {
    for (const item of equipped) {
      if (item.slot === "back") drawBackAccessory(ctx, ox, oy, dir, item, accImgs);
    }
  }

  // ── 腿 / 下装 ──
  drawLegs(ctx, ox, oy, legOffA, legOffB, s.bottomStyle, colors);

  // ── 鞋 ──
  drawShoes(ctx, ox, oy, legOffA, legOffB, s.shoeStyle, colors);

  // ── 身体 / 上衣 ──
  drawBody(ctx, ox, oy, armA, armB, s.topStyle, colors);

  // ── 头 ──
  px(ctx, ox + 4, oy + 1, 8, 7, colors.skin);
  px(ctx, ox + 4, oy + 7, 8, 1, colors.skinDark);

  // ── 头发 / 帽 ──
  drawHair(ctx, ox, oy, dir, s.hairStyle, colors);

  // ── 五官 ──
  drawFace(ctx, ox, oy, dir, colors.skinDark);

  // ── 手持物（hand slot，z 在身体上，后画）──
  if (equipped) {
    for (const item of equipped) {
      if (item.slot === "hand") drawHandAccessory(ctx, ox, oy, dir, item, accImgs);
    }
  }
}

/** 下装：pants(默认全腿) / shorts(短+裸小腿) / skirt(喇叭裙)。 */
function drawLegs(
  ctx: Ctx, ox: number, oy: number, legOffA: number, legOffB: number,
  style: BottomStyle, c: PaintColors,
) {
  if (style === "skirt") {
    // 喇叭裙：腰收窄贴身，裙摆外扩；不分行腿
    px(ctx, ox + 4, oy + 12, 8, 1, c.pants);
    px(ctx, ox + 3, oy + 13, 10, 2, c.pants);
    px(ctx, ox + 3, oy + 15, 10, 1, c.pantsDark); // 裙摆暗边
    return;
  }
  if (style === "shorts") {
    // 短裤：裤腿只到 y13，露出 y14 小腿（skin）
    px(ctx, ox + 5 - legOffA, oy + 12, 2, 2, c.pants);
    px(ctx, ox + 9 + legOffB, oy + 12, 2, 2, c.pants);
    px(ctx, ox + 5 - legOffA, oy + 14, 2, 1, c.skin);
    px(ctx, ox + 9 + legOffB, oy + 14, 2, 1, c.skin);
    return;
  }
  // pants（默认）：3 像素长裤腿
  px(ctx, ox + 5 - legOffA, oy + 12, 2, 3, c.pants);
  px(ctx, ox + 9 + legOffB, oy + 12, 2, 3, c.pants);
}

/** 鞋：boots(默认实色) / sneakers(白底+深跟) / sandals(赤脚+深带)。 */
function drawShoes(
  ctx: Ctx, ox: number, oy: number, legOffA: number, legOffB: number,
  style: ShoeStyle, c: PaintColors,
) {
  const lx = ox + 5 - legOffA; // 左脚 x
  const rx = ox + 9 + legOffB; // 右脚 x
  if (style === "sneakers") {
    px(ctx, lx, oy + 15, 2, 1, SNEAKER_WHITE);
    px(ctx, lx, oy + 15, 1, 1, c.shoes); // 外侧深跟标
    px(ctx, rx, oy + 15, 2, 1, SNEAKER_WHITE);
    px(ctx, rx + 1, oy + 15, 1, 1, c.shoes);
    return;
  }
  if (style === "sandals") {
    px(ctx, lx, oy + 15, 2, 1, c.skin); // 赤脚
    px(ctx, lx, oy + 15, 1, 1, c.shoes); // 外侧鞋带
    px(ctx, rx, oy + 15, 2, 1, c.skin);
    px(ctx, rx + 1, oy + 15, 1, 1, c.shoes);
    return;
  }
  // boots（默认）
  px(ctx, lx, oy + 15, 2, 1, c.shoes);
  px(ctx, rx, oy + 15, 2, 1, c.shoes);
}

/** 上衣：tshirt(默认窄袖) / longsleeve(整袖+手) / vest(窄身宽袖)。 */
function drawBody(
  ctx: Ctx, ox: number, oy: number, armA: number, armB: number,
  style: TopStyle, c: PaintColors,
) {
  if (style === "vest") {
    // 窄马甲身（x5-10）+ 宽袖（2 像素，衬衣暗色）
    px(ctx, ox + 5, oy + 8, 6, 5, c.shirt);
    px(ctx, ox + 5, oy + 12, 6, 1, c.shirtDark);
    px(ctx, ox + 3, oy + 8 + armA, 2, 4, c.shirtDark);
    px(ctx, ox + 11, oy + 8 + armB, 2, 4, c.shirtDark);
    return;
  }
  if (style === "longsleeve") {
    px(ctx, ox + 4, oy + 8, 8, 5, c.shirt);
    px(ctx, ox + 4, oy + 12, 8, 1, c.shirtDark);
    // 整袖 = 衬衣色 + 露 skin 手腕
    px(ctx, ox + 3, oy + 8 + armA, 1, 4, c.shirt);
    px(ctx, ox + 12, oy + 8 + armB, 1, 4, c.shirt);
    px(ctx, ox + 3, oy + 12 + armA, 1, 1, c.skin);
    px(ctx, ox + 12, oy + 12 + armB, 1, 1, c.skin);
    return;
  }
  // tshirt（默认）
  px(ctx, ox + 4, oy + 8, 8, 5, c.shirt);
  px(ctx, ox + 4, oy + 12, 8, 1, c.shirtDark);
  px(ctx, ox + 3, oy + 8 + armA, 1, 4, c.shirtDark);
  px(ctx, ox + 12, oy + 8 + armB, 1, 4, c.shirtDark);
}

/** 头发/帽：short(默认) / long(披肩) / bald(无发) / cap(鸭舌帽)。 */
function drawHair(
  ctx: Ctx, ox: number, oy: number, dir: Dir, style: HairStyle, c: PaintColors,
) {
  if (style === "bald") return; // 无发

  if (style === "cap") {
    if (dir === "s") {
      px(ctx, ox + 4, oy + 1, 8, 2, c.hair); // 帽顶
      px(ctx, ox + 3, oy + 3, 10, 1, c.hairDark); // 帽檐（外扩 1px）
    } else if (dir === "n") {
      px(ctx, ox + 4, oy + 1, 8, 3, c.hair); // 帽后
      px(ctx, ox + 4, oy + 4, 8, 1, c.hairDark); // 檐边
    } else {
      const faceRight = dir === "e";
      px(ctx, ox + 4, oy + 1, 8, 2, c.hair); // 帽顶
      px(ctx, faceRight ? ox + 11 : ox + 3, oy + 3, 2, 1, c.hairDark); // 前伸檐
    }
    return;
  }

  if (style === "long") {
    if (dir === "s") {
      px(ctx, ox + 4, oy + 1, 8, 2, c.hair); // 顶
      px(ctx, ox + 4, oy + 1, 1, 8, c.hair); // 左披肩发到 y8
      px(ctx, ox + 11, oy + 1, 1, 8, c.hair); // 右披肩发
    } else if (dir === "n") {
      px(ctx, ox + 4, oy + 1, 8, 6, c.hair); // 满背
      px(ctx, ox + 4, oy + 6, 8, 1, c.hairDark);
      px(ctx, ox + 4, oy + 1, 1, 8, c.hair); // 披肩
      px(ctx, ox + 11, oy + 1, 1, 8, c.hair);
    } else {
      const faceRight = dir === "e";
      px(ctx, ox + 4, oy + 1, 8, 2, c.hair); // 顶
      const backX = faceRight ? ox + 4 : ox + 10;
      px(ctx, backX, oy + 3, 2, 6, c.hair); // 后脑长发到 y8
    }
    return;
  }

  // short（默认）— 与扩展前逐像素一致
  if (dir === "s") {
    px(ctx, ox + 4, oy + 1, 8, 2, c.hair);
    px(ctx, ox + 4, oy + 3, 1, 2, c.hair);
    px(ctx, ox + 11, oy + 3, 1, 2, c.hair);
  } else if (dir === "n") {
    px(ctx, ox + 4, oy + 1, 8, 6, c.hair); // 背面全是头发
    px(ctx, ox + 4, oy + 6, 8, 1, c.hairDark);
  } else {
    const faceRight = dir === "e";
    px(ctx, ox + 4, oy + 1, 8, 2, c.hair);
    const backX = faceRight ? ox + 4 : ox + 10;
    px(ctx, backX, oy + 3, 2, 4, c.hair);
  }
}

/** 五官：s 双眼 / n 无 / w·e 单眼+鼻。 */
function drawFace(ctx: Ctx, ox: number, oy: number, dir: Dir, skinDark: string) {
  if (dir === "s") {
    px(ctx, ox + 6, oy + 5, 1, 1, EYE);
    px(ctx, ox + 9, oy + 5, 1, 1, EYE);
  } else if (dir === "n") {
    // 背面无五官
  } else {
    const faceRight = dir === "e";
    px(ctx, faceRight ? ox + 9 : ox + 6, oy + 5, 1, 1, EYE); // 单眼
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

/** 从 4 方向帧 key 加载生成形象，合成 sprite sheet（3 列 × 4 行，每帧 16×16） */
export async function loadGeneratedSheet(avatar: GeneratedAvatar): Promise<HTMLCanvasElement> {
  const dirMap: Record<string, number> = { south: 0, north: 1, west: 2, east: 3 };

  // 每方向加载最多 3 帧（stand/stepA/stepB）；不足 3 → 用最后一帧补齐（1 帧=静站）
  const rows: (HTMLImageElement[] | null)[] = [null, null, null, null];
  await Promise.all(
    Object.entries(avatar.frames).map(async ([dir, keys]) => {
      const row = dirMap[dir];
      if (row === undefined || keys.length === 0) return;
      rows[row] = await Promise.all(keys.slice(0, 3).map(loadAssetImage));
    }),
  );

  // 合成 sprite sheet: 3 列（stand/stepA/stepB）× 4 行（方向）
  const c = document.createElement("canvas");
  c.width = FRAME_W * 3;
  c.height = FRAME_H * DIRS.length;
  const ctx = c.getContext("2d")!;
  ctx.imageSmoothingEnabled = false;

  for (let row = 0; row < 4; row++) {
    const imgs = rows[row];
    if (!imgs || imgs.length === 0) continue;
    for (let col = 0; col < 3; col++) {
      // 不足 3 帧用最后一帧补齐；多于 3 取前 3。缩放到 16×16（nearest-neighbor）
      const img = imgs[Math.min(col, imgs.length - 1)];
      ctx.drawImage(img, 0, 0, img.width, img.height, col * FRAME_W, row * FRAME_H, FRAME_W, FRAME_H);
    }
  }

  return c;
}

/** async modular sheet：预装 accessory PNG 后合成 sheet（accessory overlay 经 drawFrame 画入帧）。 */
async function prepareModularSheet(cfg: ModularAvatar): Promise<HTMLCanvasElement> {
  const accImgs = await loadAccessoryImages(cfg.equipped);
  const c = document.createElement("canvas");
  c.width = FRAME_W * 3;
  c.height = FRAME_H * DIRS.length;
  const ctx = c.getContext("2d")!;
  ctx.imageSmoothingEnabled = false;
  DIRS.forEach((dir, row) => {
    for (let f = 0; f < 3; f++) {
      drawFrame(ctx, f * FRAME_W, row * FRAME_H, dir, f, cfg, cfg.equipped, accImgs);
    }
  });
  return c;
}

/** 统一加载层：无论模块化还是生成，都返回 canvas sprite sheet。
 *  modular：经 prepareModularSheet 合成（含 accessory overlay）。
 *  generated：loadGeneratedSheet，accessory overlay 渲染 DEFERRED（见下方 TODO）。 */
export async function prepareCharacterSheet(data: AvatarData): Promise<HTMLCanvasElement> {
  if (data.kind === "modular") {
    return prepareModularSheet(data);
  } else {
    // TODO: generated avatar accessory overlay rendering is DEFERRED (issue #0010).
    // equipped is persisted in config_json (via /api/avatar/equip) but
    // loadGeneratedSheet does not draw accessory overlays yet — to be
    // implemented in a later slice.
    return loadGeneratedSheet(data);
  }
}

/** 方向行号（供动画定义使用） */
export function dirRow(dir: Dir): number {
  return DIRS.indexOf(dir);
}
