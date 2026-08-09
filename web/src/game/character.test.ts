// game/character.test.ts — 切片 2a modular 捏脸纯逻辑单测（vitest）。无 phaser。
// 覆盖：resolveStyles（style-id→绘制参数纯函数）+ characterSheet 对各样式组合的尺寸契约 48×64。
// characterSheet 用 document.createElement("canvas")，在 node 环境用 stub mock（无 jsdom 依赖）。

import { describe, expect, it, vi, beforeAll, afterAll } from "vitest";
import {
  characterSheet,
  resolveStyles,
  FRAME_W,
  FRAME_H,
  DIRS,
  HAIR_STYLES,
  TOP_STYLES,
  BOTTOM_STYLES,
  SHOE_STYLES,
  DEFAULT_HAIR_STYLE,
  DEFAULT_TOP_STYLE,
  DEFAULT_BOTTOM_STYLE,
  DEFAULT_SHOE_STYLE,
  DEFAULT_SHOE_COLOR,
} from "./character";
import type { ModularAvatar } from "../net/api";

const baseCfg: ModularAvatar = {
  kind: "modular",
  skin: "#f0c8a0",
  hair: "#503018",
  shirt: "#3868b0",
  pants: "#404048",
};

describe("resolveStyles", () => {
  it("defaults all styles when fields absent (backward compat)", () => {
    const r = resolveStyles(baseCfg);
    expect(r.hairStyle).toBe(DEFAULT_HAIR_STYLE);
    expect(r.topStyle).toBe(DEFAULT_TOP_STYLE);
    expect(r.bottomStyle).toBe(DEFAULT_BOTTOM_STYLE);
    expect(r.shoeStyle).toBe(DEFAULT_SHOE_STYLE);
    expect(r.shoes).toBe(DEFAULT_SHOE_COLOR);
  });

  it("passes through valid style values", () => {
    const r = resolveStyles({
      ...baseCfg,
      hairStyle: "long",
      topStyle: "vest",
      bottomStyle: "skirt",
      shoeStyle: "sandals",
      shoes: "#abc123",
    });
    expect(r.hairStyle).toBe("long");
    expect(r.topStyle).toBe("vest");
    expect(r.bottomStyle).toBe("skirt");
    expect(r.shoeStyle).toBe("sandals");
    expect(r.shoes).toBe("#abc123");
  });

  it("falls back to default for unknown values", () => {
    const r = resolveStyles({
      ...baseCfg,
      hairStyle: "mohawk",
      topStyle: "hoodie",
      bottomStyle: "jeans",
      shoeStyle: "flippers",
    });
    expect(r.hairStyle).toBe(DEFAULT_HAIR_STYLE);
    expect(r.topStyle).toBe(DEFAULT_TOP_STYLE);
    expect(r.bottomStyle).toBe(DEFAULT_BOTTOM_STYLE);
    expect(r.shoeStyle).toBe(DEFAULT_SHOE_STYLE);
  });

  it("treats empty string as absent (default)", () => {
    const r = resolveStyles({ ...baseCfg, hairStyle: "" });
    expect(r.hairStyle).toBe(DEFAULT_HAIR_STYLE);
  });

  it("does not mutate the input config (pure)", () => {
    const cfg = { ...baseCfg };
    resolveStyles(cfg);
    expect(cfg).toEqual(baseCfg);
  });

  it("every option list contains its default", () => {
    expect(HAIR_STYLES).toContain(DEFAULT_HAIR_STYLE);
    expect(TOP_STYLES).toContain(DEFAULT_TOP_STYLE);
    expect(BOTTOM_STYLES).toContain(DEFAULT_BOTTOM_STYLE);
    expect(SHOE_STYLES).toContain(DEFAULT_SHOE_STYLE);
  });
});

// ── characterSheet 需要 document.createElement("canvas")；用最小 stub mock（无 jsdom）──
const fakeCtx = {
  imageSmoothingEnabled: false,
  fillRect: () => {},
  fillStyle: "",
};
const fakeCanvas = () => ({ width: 0, height: 0, getContext: () => fakeCtx });

describe("characterSheet dimensions", () => {
  beforeAll(() => {
    vi.stubGlobal("document", { createElement: () => fakeCanvas() });
  });
  afterAll(() => {
    vi.unstubAllGlobals();
  });

  it("produces 48×64 for default config", () => {
    const c = characterSheet(baseCfg);
    expect(c.width).toBe(FRAME_W * 3);
    expect(c.height).toBe(FRAME_H * DIRS.length);
  });

  it("produces 48×64 across every style combination (4×3×3×3)", () => {
    expect(HAIR_STYLES.length).toBe(4);
    expect(TOP_STYLES.length).toBe(3);
    expect(BOTTOM_STYLES.length).toBe(3);
    expect(SHOE_STYLES.length).toBe(3);
    for (const h of HAIR_STYLES) {
      for (const t of TOP_STYLES) {
        for (const b of BOTTOM_STYLES) {
          for (const sh of SHOE_STYLES) {
            const c = characterSheet({
              ...baseCfg,
              hairStyle: h,
              topStyle: t,
              bottomStyle: b,
              shoeStyle: sh,
            });
            expect(c.width).toBe(48);
            expect(c.height).toBe(64);
          }
        }
      }
    }
  });

  it("produces 48×64 with explicit shoes color + sneakers", () => {
    const c = characterSheet({ ...baseCfg, shoes: "#d83030", shoeStyle: "sneakers" });
    expect(c.width).toBe(48);
    expect(c.height).toBe(64);
  });

  it("produces 48×64 for bald + cap (no/extra hair pixels)", () => {
    const c = characterSheet({ ...baseCfg, hairStyle: "bald" });
    expect(c.width).toBe(48);
    const c2 = characterSheet({ ...baseCfg, hairStyle: "cap" });
    expect(c2.width).toBe(48);
    expect(c2.height).toBe(64);
  });
});
