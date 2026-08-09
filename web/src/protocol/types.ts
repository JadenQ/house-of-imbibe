// protocol/ —— WS 消息类型，手工镜像 src/realtime/protocol.rs。
// net/ protocol/ game-state/ 禁止 import phaser（dev-plan §2.3）。

export interface EquippedItem {
  slot: "back" | "hand";
  asset_id: string | null;
  asset_key: string | null;
}

export type AvatarSnapshot =
  | {
      kind: "modular";
      skin: string;
      hair: string;
      shirt: string;
      pants: string;
      /** 切片 2a 捏脸样式（可选；服务端当前可能不持久化，缺失→默认） */
      shoes?: string;
      hairStyle?: string;
      topStyle?: string;
      bottomStyle?: string;
      shoeStyle?: string;
      /** 已装备配件（back/hand slot）；缺失 → 空（向后兼容） */
      equipped?: EquippedItem[];
    }
  | {
      kind: "generated";
      character_id: string;
      frames: Record<string, string[]>; // 每方向帧 key 数组（1=静站，3=行走）
      /** 已装备配件（D4 允许 generated 配件）；缺失 → 空（向后兼容） */
      equipped?: EquippedItem[];
    };

export interface PlayerSnap {
  id: number;
  x: number;
  y: number;
  dir: string;
  name: string;
  avatar: AvatarSnapshot;
  avatar_hash: string;
  target_tx: number;
  target_ty: number;
}

export interface ChatItem {
  from: number;
  name: string;
  text: string;
  ts: number;
}

/** 装饰对象 JSON 契约（广播 + 快照 + API 返回一致）。
 *  asset_id 可 null = 占位装饰（无关联资产）。
 *  asset_key = LEFT JOIN assets.storage_key；null = 无资产 / 资产不存在。
 *  前端拼 /api/assets/{asset_key} 取 PNG 的唯一来源。 */
export interface Decoration {
  id: string;
  scene: string;
  tile_x: number;
  tile_y: number;
  asset_id: string | null;
  asset_key: string | null;
  z_layer: number;
  placed_by: number;
}

export type ClientMsg =
  | { v: number; type: "move"; tx: number; ty: number }
  | { v: number; type: "chat"; text: string }
  | { v: number; type: "interact"; target: string }
  | { v: number; type: "dialogue_advance"; npc: string; choice?: string }
  | { v: number; type: "ping"; t: number };

export type ServerMsg =
  | {
      v: number;
      type: "welcome";
      self_id: number;
      scene: string;
      tick_hz: number;
      server_time: number;
    }
  | {
      v: number;
      type: "snapshot_full";
      tick: number;
      t: number;
      players: PlayerSnap[];
      decorations: Decoration[];
      npcs: unknown[];
    }
  | {
      v: number;
      type: "snapshot_delta";
      tick: number;
      t: number;
      upsert: PlayerSnap[];
      remove: number[];
    }
  | {
      v: number;
      type: "chat";
      from: number;
      name: string;
      text: string;
      ts: number;
    }
  | { v: number; type: "chat_backlog"; items: ChatItem[] }
  | { v: number; type: "dialogue"; npc: string; node: string; menu?: unknown }
  | { v: number; type: "decoration_added"; decoration: Decoration }
  | { v: number; type: "decoration_removed"; id: string }
  | { v: number; type: "scene_changed"; scene: string; spawn: [number, number] }
  | { v: number; type: "kicked"; reason: string }
  | { v: number; type: "error"; code: string; msg: string }
  | { v: number; type: "pong"; t: number };

const KNOWN_SERVER_TYPES = new Set<string>([
  "welcome",
  "snapshot_full",
  "snapshot_delta",
  "chat",
  "chat_backlog",
  "dialogue",
  "decoration_added",
  "decoration_removed",
  "scene_changed",
  "kicked",
  "error",
  "pong",
]);

/** 解析一帧 WS 文本；未知/非法 -> null（静默忽略，dev-plan §2.5）。 */
export function parseMsg(raw: string): ServerMsg | null {
  try {
    const m = JSON.parse(raw) as { type?: unknown };
    if (!m || typeof m.type !== "string") return null;
    return KNOWN_SERVER_TYPES.has(m.type) ? (m as ServerMsg) : null;
  } catch {
    return null;
  }
}

/** 构造客户端消息的小工厂（统一带 v:1）。 */
export const msg = {
  move: (tx: number, ty: number): ClientMsg => ({ v: 1, type: "move", tx, ty }),
  chat: (text: string): ClientMsg => ({ v: 1, type: "chat", text }),
  ping: (t: number): ClientMsg => ({ v: 1, type: "ping", t }),
};
