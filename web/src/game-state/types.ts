// game-state/types.ts —— 纯状态形状（无 phaser、无 DOM）。dev-plan §2.3。
// 实现见 room.ts / map.ts。RenderView 是 scene 唯一读取的视图。

import type { AvatarSnapshot, Decoration } from "../protocol/types";

export interface PlayerView {
  id: number;
  x: number;
  y: number;
  dir: string;
  frame: number; // 0 stand, 1 stepA, 2 stepB
  avatar: AvatarSnapshot;
  avatarHash: string;
}

export interface BubbleView {
  playerId: number;
  text: string;
}

/** 装饰渲染视图（scene 只读）。x/y = tile 坐标 × TILE；asset_key 非空则按 /api/assets/{key} 取图。 */
export interface DecorationView {
  id: string;
  x: number;
  y: number;
  asset_key: string | null;
  z_layer: number;
}

export interface RenderView {
  players: PlayerView[];
  bubbles: BubbleView[];
  decorations: DecorationView[];
}

/** 远端玩家的权威位置采样（服务端 t + 像素坐标）。 */
export interface PlayerSample {
  t: number;
  x: number;
  y: number;
  dir: string;
}

export interface PlayerState {
  id: number;
  name: string;
  samples: PlayerSample[];
  avatar: AvatarSnapshot;
  avatarHash: string;
}

export interface ChatLine {
  from: number;
  name: string;
  text: string;
  ts: number;
}

export interface RoomState {
  selfId: number;
  scene: string;
  /** 所有玩家（含 self）。self 的权威采样用于本地预测纠正；
   *  interpolate 把 self 从 RenderView.players 滤掉，scene 用 localX/Y 渲染 self。 */
  players: Map<number, PlayerState>;
  /** 装饰对象层（内存缓存；DB 为真相源，snapshot_full 从 DB 查）。id → Decoration。 */
  decorations: Map<string, Decoration>;
  /** 全局聊天最近 50 条（禁令 #2：绝不落库）。 */
  chat: ChatLine[];
  /** 客户端<->服务端时钟偏移（serverMs - localMs），由 ping/pong 估计。 */
  clockOffset: number;
  /** 本地预测位置（本地玩家渲染权威）。 */
  localX: number;
  localY: number;
  localDir: string;
}
