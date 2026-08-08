// game-state/map.ts — 静态地图数据与可走性查询（无 phaser、无 DOM）。dev-plan §2.3。
// 装载 assets/maps/bar.json；服务端与前端共享同一份碰撞数据，禁止两份手写表漂移。

import barMap from "../../../assets/maps/bar.json";
import type { MapDef } from "../game/tiles";

export const BAR_MAP: MapDef = {
  rows: barMap.rows,
  solid: new Set<string>(barMap.solid),
  interact: barMap.interact,
  spawn: barMap.spawn,
};

/** 坐标 (tx,ty) 是否可走：越界或落在 solid 字符上则不可走。 */
export function isWalkable(tx: number, ty: number): boolean {
  if (ty < 0 || ty >= BAR_MAP.rows.length) return false;
  const row = BAR_MAP.rows[ty];
  if (tx < 0 || tx >= row.length) return false;
  return !BAR_MAP.solid.has(row[tx]);
}

/** 出生点（tile 坐标）。 */
export function spawn(): { tx: number; ty: number } {
  return BAR_MAP.spawn;
}
