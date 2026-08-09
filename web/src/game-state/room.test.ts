// game-state/room.test.ts — 纯状态机单测（vitest）。无 phaser、无 DOM。时钟注入。
// dev-plan §三 切片1 C 组：net + game-state 有单测（含插值时间快进）。

import { describe, expect, it } from "vitest";
import { applyServerMsg, avatarHashOf, initialRoomState, interpolate } from "./room";
import type { Decoration, PlayerSnap } from "../protocol/types";

const AV = { kind: "modular", skin: "#000000", hair: "#000000", shirt: "#000000", pants: "#000000" } as const;

const snap = (id: number, x: number, y: number, name = "x"): PlayerSnap => ({
  id,
  x,
  y,
  dir: "s",
  name,
  avatar: { ...AV },
  avatar_hash: "",
  target_tx: Math.floor(x / 16),
  target_ty: Math.floor(y / 16),
});

const dec = (id: string, tx: number, ty: number, asset_key: string | null = null, z = 0): Decoration => ({
  id,
  scene: "bar",
  tile_x: tx,
  tile_y: ty,
  asset_id: null,
  asset_key,
  z_layer: z,
  placed_by: 1,
});

describe("applyServerMsg", () => {
  it("welcome sets selfId + scene", () => {
    const s = applyServerMsg(initialRoomState(0), {
      v: 1, type: "welcome", self_id: 42, scene: "bar", tick_hz: 10, server_time: 0,
    });
    expect(s.selfId).toBe(42);
    expect(s.scene).toBe("bar");
  });

  it("snapshot_full replaces players", () => {
    let s = initialRoomState(1);
    s = applyServerMsg(s, {
      v: 1, type: "snapshot_full", tick: 0, t: 1000,
      players: [snap(2, 10, 10), snap(3, 20, 20)], decorations: [], npcs: [],
    });
    expect(s.players.size).toBe(2);
    expect(s.players.get(2)?.name).toBe("x");
    expect(s.players.get(2)?.samples[0]).toEqual({ t: 1000, x: 10, y: 10, dir: "s" });
  });

  it("snapshot_delta upsert appends a sample; remove deletes", () => {
    let s = initialRoomState(1);
    s = applyServerMsg(s, {
      v: 1, type: "snapshot_full", tick: 0, t: 1000, players: [snap(2, 0, 0)], decorations: [], npcs: [],
    });
    s = applyServerMsg(s, {
      v: 1, type: "snapshot_delta", tick: 1, t: 1100, upsert: [snap(2, 50, 50)], remove: [9],
    });
    expect(s.players.get(2)?.samples.length).toBe(2);
    expect(s.players.get(2)?.samples[1].x).toBe(50);
    expect(s.players.has(9)).toBe(false);
  });

  it("chat appends and caps at 50", () => {
    let s = initialRoomState(1);
    for (let i = 0; i < 55; i++) {
      s = applyServerMsg(s, { v: 1, type: "chat", from: 2, name: "a", text: `m${i}`, ts: i });
    }
    expect(s.chat.length).toBe(50);
    expect(s.chat[0].text).toBe("m5"); // 前 5 条被淘汰
  });

  it("chat_backlog sets the list", () => {
    let s = initialRoomState(1);
    s = applyServerMsg(s, {
      v: 1, type: "chat_backlog", items: [{ from: 2, name: "a", text: "hi", ts: 1 }],
    });
    expect(s.chat.length).toBe(1);
    expect(s.chat[0].text).toBe("hi");
  });

  it("pong sets clockOffset = m.t - localMs (injected clock)", () => {
    let s = initialRoomState(1);
    s = applyServerMsg(s, { v: 1, type: "pong", t: 5000 }, 1000); // localMs=1000
    expect(s.clockOffset).toBe(4000);
  });

  it("unknown/error leaves state unchanged", () => {
    const s0 = initialRoomState(1);
    const s1 = applyServerMsg(s0, { v: 1, type: "error", code: "x", msg: "y" });
    expect(s1).toBe(s0);
  });
});

describe("interpolate (injected clock, fast-forward)", () => {
  it("linearly interpolates x between two bracketing samples", () => {
    let s = initialRoomState(1);
    s = applyServerMsg(s, {
      v: 1, type: "snapshot_full", tick: 0, t: 1000, players: [snap(2, 0, 0)], decorations: [], npcs: [],
    });
    s = applyServerMsg(s, {
      v: 1, type: "snapshot_delta", tick: 1, t: 1100, upsert: [snap(2, 100, 0)], remove: [],
    });
    // clockOffset=0；nowMs=1050, delay=0 → tRender=1050 → (0@1000) 与 (100@1100) 之间 → x≈50
    const view = interpolate(s, 1050, 0);
    expect(view.players.length).toBe(1);
    expect(view.players[0].x).toBeCloseTo(50, 0);
    expect(view.players[0].y).toBeCloseTo(0, 0);
  });

  it("excludes self from RenderView.players", () => {
    let s = initialRoomState(7);
    s = applyServerMsg(s, {
      v: 1, type: "snapshot_full", tick: 0, t: 1000,
      players: [snap(7, 0, 0), snap(8, 10, 10)], decorations: [], npcs: [],
    });
    const view = interpolate(s, 1000, 0);
    expect(view.players.map((p) => p.id)).toEqual([8]);
  });
});

describe("avatarHashOf", () => {
  it("is stable per-avatar and differs when the avatar changes", () => {
    const a = { ...AV };
    expect(avatarHashOf(a)).toBe(avatarHashOf(a));
    expect(avatarHashOf({ ...a, skin: "#ffffff" })).not.toBe(avatarHashOf(a));
  });
});

describe("decorations", () => {
  it("snapshot_full populates decorations map", () => {
    let s = initialRoomState(1);
    s = applyServerMsg(s, {
      v: 1, type: "snapshot_full", tick: 0, t: 1000,
      players: [], decorations: [dec("d1", 5, 3, "k/a.png", -1), dec("d2", 2, 7)], npcs: [],
    });
    expect(s.decorations.size).toBe(2);
    expect(s.decorations.get("d1")?.asset_key).toBe("k/a.png");
    expect(s.decorations.get("d1")?.z_layer).toBe(-1);
    expect(s.decorations.get("d2")?.asset_key).toBeNull();
  });

  it("decoration_added upserts into map", () => {
    let s = initialRoomState(1);
    s = applyServerMsg(s, {
      v: 1, type: "snapshot_full", tick: 0, t: 1000,
      players: [], decorations: [dec("d1", 5, 3)], npcs: [],
    });
    s = applyServerMsg(s, {
      v: 1, type: "decoration_added",
      decoration: dec("d2", 1, 1, "k/b.png", 1),
    });
    expect(s.decorations.size).toBe(2);
    expect(s.decorations.get("d2")?.asset_key).toBe("k/b.png");
  });

  it("decoration_removed deletes from map", () => {
    let s = initialRoomState(1);
    s = applyServerMsg(s, {
      v: 1, type: "snapshot_full", tick: 0, t: 1000,
      players: [], decorations: [dec("d1", 5, 3)], npcs: [],
    });
    s = applyServerMsg(s, { v: 1, type: "decoration_removed", id: "d1" });
    expect(s.decorations.size).toBe(0);
  });

  it("interpolate outputs DecorationView with tile→pixel coords", () => {
    let s = initialRoomState(1);
    s = applyServerMsg(s, {
      v: 1, type: "snapshot_full", tick: 0, t: 1000,
      players: [], decorations: [dec("d1", 5, 3, "k/a.png", -1)], npcs: [],
    });
    const view = interpolate(s, 1000, 0);
    expect(view.decorations.length).toBe(1);
    expect(view.decorations[0]).toEqual({
      id: "d1", x: 80, y: 48, asset_key: "k/a.png", z_layer: -1,
    });
  });
});
