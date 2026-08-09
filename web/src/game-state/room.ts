// game-state/room.ts — 纯状态机：applyServerMsg（消费 ServerMsg）+ interpolate（产出 RenderView）。
// 无 phaser、无 DOM、无 Date.now()（时钟由 caller 注入）。dev-plan §2.3 + §三 切片1。
//
// 设计决策（契约未写明，在此固化）：
// - RoomState.players 包含所有玩家（含 self）。self 的权威采样用于本地预测纠正；
//   渲染时 interpolate 把 self 从 RenderView.players 中滤掉，scene 用 localX/Y/localDir 渲染 self。
// - 本地预测位置 localX/Y/localDir 由 scene 推进，并向 players.get(selfId) 最新采样收敛（非硬 snap）。

import { TILE } from "../game/tiles";
import { spawn } from "./map";
import type { Decoration, PlayerSnap, ServerMsg } from "../protocol/types";
import type {
  BubbleView,
  ChatLine,
  DecorationView,
  PlayerSample,
  PlayerState,
  PlayerView,
  RenderView,
  RoomState,
} from "./types";

const WALK_CYCLE = [0, 1, 0, 2] as const;
const MAX_SAMPLES = 20;
const MAX_CHAT = 50;
const BUBBLE_TTL_MS = 4000;

/** djb2 风格稳定哈希，用作合成纹理缓存键（dev-plan §三 切片2：缓存键 = AvatarSnapshot 的稳定哈希）。 */
function hashStr(s: string): string {
  let h = 5381;
  for (let i = 0; i < s.length; i++) {
    h = ((h << 5) + h + s.charCodeAt(i)) >>> 0;
  }
  return h.toString(36);
}

/** AvatarSnapshot 的稳定哈希；同一 avatar 必出同一 hash，avatar 变则 hash 变。 */
export function avatarHashOf(a: unknown): string {
  return hashStr(JSON.stringify(a));
}

/** 初始 RoomState：空玩家、空装饰、空聊天、本地预测位置 = 出生 tile 中心（+8 像素）。 */
export function initialRoomState(selfId: number): RoomState {
  const sp = spawn();
  return {
    selfId,
    scene: "bar",
    players: new Map(),
    decorations: new Map(),
    chat: [],
    clockOffset: 0,
    localX: sp.tx * TILE + 8,
    localY: sp.ty * TILE + 8,
    localDir: "s",
  };
}

/** PlayerSnap → PlayerState（单采样；delta 对已有玩家会追加采样而非重建）。 */
function snapToState(p: PlayerSnap, t: number): PlayerState {
  return {
    id: p.id,
    name: p.name,
    avatar: p.avatar,
    avatarHash: avatarHashOf(p.avatar),
    samples: [{ t, x: p.x, y: p.y, dir: p.dir }],
  };
}

/** 消费一条 ServerMsg，返回新 RoomState（浅拷贝 + 替换变更字段；不可变）。
 *  localMs = 发 ping 时的本地时间，仅 pong 用（注入时钟，不直调 Date.now）。 */
export function applyServerMsg(
  state: RoomState,
  m: ServerMsg,
  localMs: number = 0,
): RoomState {
  switch (m.type) {
    case "welcome":
      return { ...state, selfId: m.self_id, scene: m.scene };

    case "snapshot_full": {
      const players = new Map<number, PlayerState>();
      for (const p of m.players) {
        players.set(p.id, snapToState(p, m.t));
      }
      const decorations = new Map<string, Decoration>();
      for (const d of m.decorations) {
        decorations.set(d.id, d);
      }
      return { ...state, players, decorations };
    }

    case "snapshot_delta": {
      const players = new Map(state.players);
      for (const p of m.upsert) {
        const existing = players.get(p.id);
        if (existing) {
          const samples = [
            ...existing.samples,
            { t: m.t, x: p.x, y: p.y, dir: p.dir },
          ];
          while (samples.length > MAX_SAMPLES) samples.shift();
          players.set(p.id, {
            ...existing,
            name: p.name,
            avatar: p.avatar,
            avatarHash: avatarHashOf(p.avatar),
            samples,
          });
        } else {
          players.set(p.id, snapToState(p, m.t));
        }
      }
      for (const id of m.remove) {
        players.delete(id);
      }
      return { ...state, players };
    }

    case "chat": {
      const chat: ChatLine[] = [
        ...state.chat,
        { from: m.from, name: m.name, text: m.text, ts: m.ts },
      ];
      while (chat.length > MAX_CHAT) chat.shift();
      return { ...state, chat };
    }

    case "chat_backlog":
      // ChatItem 与 ChatLine 结构同形；截最近 MAX_CHAT 条。
      return { ...state, chat: m.items.slice(-MAX_CHAT) };

    case "pong":
      // serverMs - localMs；localhost 近零延迟。
      return { ...state, clockOffset: m.t - localMs };

    case "decoration_added": {
      const decorations = new Map(state.decorations);
      decorations.set(m.decoration.id, m.decoration);
      return { ...state, decorations };
    }

    case "decoration_removed": {
      const decorations = new Map(state.decorations);
      decorations.delete(m.id);
      return { ...state, decorations };
    }

    default:
      // error / kicked / dialogue / scene_changed / 未实现：状态不变。
      return state;
  }
}

/** 在两帧采样间线性插值出 (x,y)；dir 取较晚那一帧。两帧时间相同则取较晚帧（避免除零）。 */
function interpSample(
  a: PlayerSample,
  b: PlayerSample,
  t: number,
): { x: number; y: number; dir: string } {
  if (b.t <= a.t) {
    return { x: b.x, y: b.y, dir: b.dir };
  }
  const f = (t - a.t) / (b.t - a.t);
  return {
    x: a.x + (b.x - a.x) * f,
    y: a.y + (b.y - a.y) * f,
    dir: b.dir,
  };
}

/** 找出 t 落在哪两帧之间；越界 clamp 到端点帧，单帧返回 [s,s]，空返回 null。 */
function findBracket(
  samples: PlayerSample[],
  t: number,
): [PlayerSample, PlayerSample] | null {
  if (samples.length === 0) return null;
  if (samples.length === 1) return [samples[0], samples[0]];
  for (let i = 0; i < samples.length - 1; i++) {
    const a = samples[i];
    const b = samples[i + 1];
    if (a.t <= t && t <= b.t) return [a, b];
  }
  if (t < samples[0].t) return [samples[0], samples[0]];
  return [samples[samples.length - 1], samples[samples.length - 1]];
}

/** 由当前 RoomState + 注入的本地时钟产出渲染视图。
 *  serverNow = nowMs + clockOffset；tRender = serverNow - delayMs（100–200ms 缓冲）。
 *  远端玩家在 tRender 处两帧间线性插值；self 不出现在 RenderView.players（scene 用 localX/Y 渲染）。
 *  聊天气泡 = 最近 BUBBLE_TTL_MS 内的发言。 */
export function interpolate(
  state: RoomState,
  nowMs: number,
  delayMs: number = 120,
): RenderView {
  const serverNow = nowMs + state.clockOffset;
  const tRender = serverNow - delayMs;

  const players: PlayerView[] = [];
  for (const [id, p] of state.players) {
    if (id === state.selfId) continue; // self 由 scene 用 localX/Y/localDir 渲染
    const bracket = findBracket(p.samples, tRender);
    if (!bracket) continue;
    const [a, b] = bracket;
    const pos = interpSample(a, b, tRender);
    const moving = a.x !== b.x || a.y !== b.y;
    const frame = moving
      ? WALK_CYCLE[Math.floor(nowMs / 130) % WALK_CYCLE.length]
      : 0;
    players.push({
      id: p.id,
      x: pos.x,
      y: pos.y,
      dir: pos.dir,
      frame,
      avatar: p.avatar,
      avatarHash: p.avatarHash,
    });
  }

  const bubbles: BubbleView[] = [];
  for (const line of state.chat) {
    const age = serverNow - line.ts;
    if (age >= 0 && age <= BUBBLE_TTL_MS) {
      bubbles.push({ playerId: line.from, text: line.text });
    }
  }

  const decorations: DecorationView[] = [];
  for (const [, d] of state.decorations) {
    decorations.push({
      id: d.id,
      x: d.tile_x * TILE,
      y: d.tile_y * TILE,
      asset_key: d.asset_key,
      z_layer: d.z_layer,
    });
  }

  return { players, bubbles, decorations };
}
