// game/ — 纯 canvas 纹理生成，不 import phaser。
// 短期手工像素 tile；工期3 由 PixelLab 生成的 tileset 替换（同尺寸契约：16×16）。

export const TILE = 16;

function makeCanvas(w: number, h: number): [HTMLCanvasElement, CanvasRenderingContext2D] {
  const c = document.createElement("canvas");
  c.width = w;
  c.height = h;
  const ctx = c.getContext("2d")!;
  ctx.imageSmoothingEnabled = false;
  return [c, ctx];
}

function px(ctx: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, color: string) {
  ctx.fillStyle = color;
  ctx.fillRect(x, y, w, h);
}

/** 木地板 */
export function floorTile(): HTMLCanvasElement {
  const [c, ctx] = makeCanvas(TILE, TILE);
  px(ctx, 0, 0, TILE, TILE, "#6b4a2e");
  px(ctx, 0, 0, TILE, 1, "#7d5836");
  px(ctx, 0, 8, TILE, 1, "#57391f");
  px(ctx, 5, 0, 1, 8, "#57391f");
  px(ctx, 11, 8, 1, 8, "#57391f");
  return c;
}

/** 木墙板 */
export function wallTile(): HTMLCanvasElement {
  const [c, ctx] = makeCanvas(TILE, TILE);
  px(ctx, 0, 0, TILE, TILE, "#3a2a1c");
  px(ctx, 0, 0, TILE, 3, "#241a10");
  px(ctx, 0, 3, TILE, 1, "#4a3826");
  px(ctx, 4, 4, 1, 12, "#2e2115");
  px(ctx, 12, 4, 1, 12, "#2e2115");
  return c;
}

/** 吧台（上面） */
export function barTopTile(): HTMLCanvasElement {
  const [c, ctx] = makeCanvas(TILE, TILE);
  px(ctx, 0, 0, TILE, TILE, "#8a6a3e");
  px(ctx, 0, 0, TILE, 2, "#a8824e");
  px(ctx, 0, 14, TILE, 2, "#5c4020");
  px(ctx, 0, 6, TILE, 1, "#77552e");
  return c;
}

/** 吧台（正面，带竖纹与黄铜条） */
export function barFrontTile(): HTMLCanvasElement {
  const [c, ctx] = makeCanvas(TILE, TILE);
  px(ctx, 0, 0, TILE, TILE, "#5c4020");
  px(ctx, 0, 0, TILE, 2, "#d4a24e");
  px(ctx, 0, 2, TILE, 1, "#3a2a14");
  for (let x = 2; x < TILE; x += 4) px(ctx, x, 4, 1, 12, "#4a3018");
  return c;
}

/** 酒架（带瓶子）— 可交互：打开酒单 */
export function shelfTile(variant: number): HTMLCanvasElement {
  const [c, ctx] = makeCanvas(TILE, TILE);
  px(ctx, 0, 0, TILE, TILE, "#2e2115");
  // 上下两条木板
  px(ctx, 0, 4, TILE, 2, "#5c4020");
  px(ctx, 0, 11, TILE, 2, "#5c4020");
  // 瓶子：两排，颜色轮换
  const bottles = ["#3e7d4e", "#8a2e2e", "#b08430", "#3e5f8a", "#6b3e7d"];
  for (let i = 0; i < 4; i++) {
    const col = bottles[(i + variant) % bottles.length];
    const x = 1 + i * 4;
    px(ctx, x, 0, 2, 4, col); // 上排瓶身
    px(ctx, x, 0, 1, 1, "#d8d0c0"); // 瓶口高光
    const col2 = bottles[(i + variant + 2) % bottles.length];
    px(ctx, x, 7, 2, 4, col2);
    px(ctx, x, 7, 1, 1, "#d8d0c0");
  }
  return c;
}

/** 桌子 + 蜡烛 */
export function tableTile(): HTMLCanvasElement {
  const [c, ctx] = makeCanvas(TILE, TILE);
  px(ctx, 0, 0, TILE, TILE, "#6b4a2e"); // 地板底
  px(ctx, 0, 8, TILE, 1, "#57391f");
  px(ctx, 2, 2, 12, 12, "#4a3018");
  px(ctx, 3, 3, 10, 10, "#5c4020");
  px(ctx, 7, 6, 2, 4, "#d8d0c0"); // 蜡烛
  px(ctx, 7, 5, 2, 1, "#f0d060"); // 火苗
  return c;
}

/** 高脚凳 */
export function stoolTile(): HTMLCanvasElement {
  const [c, ctx] = makeCanvas(TILE, TILE);
  px(ctx, 0, 0, TILE, TILE, "#6b4a2e");
  px(ctx, 0, 8, TILE, 1, "#57391f");
  px(ctx, 4, 4, 8, 3, "#8a2e2e");
  px(ctx, 5, 3, 6, 1, "#a84444");
  px(ctx, 7, 7, 2, 6, "#3a2a14");
  px(ctx, 5, 13, 6, 1, "#3a2a14");
  return c;
}

/** 地毯 */
export function rugTile(variant: number): HTMLCanvasElement {
  const [c, ctx] = makeCanvas(TILE, TILE);
  px(ctx, 0, 0, TILE, TILE, "#5a2430");
  px(ctx, 0, 0, TILE, 1, "#7d3a44");
  px(ctx, 0, 15, TILE, 1, "#7d3a44");
  if (variant % 2 === 0) px(ctx, 7, 7, 2, 2, "#d4a24e");
  return c;
}

/** 门垫/入口 */
export function doorTile(): HTMLCanvasElement {
  const [c, ctx] = makeCanvas(TILE, TILE);
  px(ctx, 0, 0, TILE, TILE, "#241a10");
  px(ctx, 1, 1, 14, 14, "#8a6a3e");
  px(ctx, 2, 2, 12, 12, "#6b4a2e");
  return c;
}

// ---------- 地图数据 ----------
// 15×10，图例：# 墙  . 地板  B 吧台面  b 吧台正面  S 酒架  T 桌子  s 凳子  r 地毯  D 门
export const MAP_ROWS = [
  "###############",
  "#SSSSSSSS....#",
  "#.............#",
  "#BBBBBBB..T.T.#",
  "#bbbbbbb.s....#",
  "#..s.....T.T..#",
  "#..rrrr.......#",
  "#.Trrrr..T....#",
  "#..rrrr.....s.#",
  "#######D#######",
];

export interface MapDef {
  rows: string[];
  /** 不可走字符集合 */
  solid: Set<string>;
  /** 可交互字符 → 交互 id */
  interact: Record<string, string>;
  spawn: { tx: number; ty: number };
}

export const BAR_MAP: MapDef = {
  rows: MAP_ROWS,
  solid: new Set(["#", "B", "b", "S", "T"]),
  interact: { B: "menu", b: "menu", S: "menu" },
  spawn: { tx: 7, ty: 6 },
};

/** 把整张地图合成到一张 canvas（240×160），场景里作为单张背景图 */
export function renderMap(map: MapDef): HTMLCanvasElement {
  const w = map.rows[0].length * TILE;
  const h = map.rows.length * TILE;
  const [c, ctx] = makeCanvas(w, h);
  map.rows.forEach((row, ty) => {
    [...row].forEach((ch, tx) => {
      let tile: HTMLCanvasElement;
      switch (ch) {
        case "#": tile = wallTile(); break;
        case "B": tile = barTopTile(); break;
        case "b": tile = barFrontTile(); break;
        case "S": tile = shelfTile(tx); break;
        case "T": tile = tableTile(); break;
        case "s": tile = stoolTile(); break;
        case "r": tile = rugTile(tx + ty); break;
        case "D": tile = doorTile(); break;
        default: tile = floorTile();
      }
      ctx.drawImage(tile, tx * TILE, ty * TILE);
    });
  });
  return c;
}
