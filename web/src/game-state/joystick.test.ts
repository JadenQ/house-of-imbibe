// game-state/joystick.test.ts — 纯摇杆数学单测（vitest）。无 phaser、无 DOM。
// dev-plan §三 切片1 C 组：game-state 有单测。CLAUDE.md 设计第一原则触控可测核心。

import { describe, expect, it } from "vitest";
import {
  applyDeadZone,
  clampToUnit,
  computeJoystick,
  dominantDir,
  DEFAULT_DEAD_ZONE,
} from "./joystick";

describe("applyDeadZone", () => {
  it("zeros magnitude below dead zone", () => {
    expect(applyDeadZone(0.1, 0.1, 0.25)).toEqual({ x: 0, y: 0 });
    expect(applyDeadZone(0.2, 0, 0.25)).toEqual({ x: 0, y: 0 });
  });

  it("passes through magnitude at/above dead zone unchanged", () => {
    expect(applyDeadZone(0.3, 0, 0.25)).toEqual({ x: 0.3, y: 0 });
    expect(applyDeadZone(1, 1, 0.25)).toEqual({ x: 1, y: 1 });
  });

  it("treats exact boundary as below (<, not <=)", () => {
    // deadZone=0.25, mag exactly 0.25 → 0.25 < 0.25 is false → NOT zeroed
    expect(applyDeadZone(0.25, 0, 0.25)).toEqual({ x: 0.25, y: 0 });
  });
});

describe("clampToUnit", () => {
  it("passes through vectors already within the unit circle", () => {
    expect(clampToUnit(0.5, 0.5)).toEqual({ x: 0.5, y: 0.5 });
    expect(clampToUnit(0, 0)).toEqual({ x: 0, y: 0 });
    // (-0.6,-0.6) magnitude ≈0.849 < 1 → unchanged
    expect(clampToUnit(-0.6, -0.6)).toEqual({ x: -0.6, y: -0.6 });
  });

  it("clamps diagonal (-1,-1) (magnitude √2) to unit circle", () => {
    const r = clampToUnit(-1, -1);
    expect(Math.hypot(r.x, r.y)).toBeCloseTo(1, 10);
    expect(r.x).toBeCloseTo(-1 / Math.SQRT2, 10);
    expect(r.y).toBeCloseTo(-1 / Math.SQRT2, 10);
  });

  it("clamps outside the unit circle to magnitude 1", () => {
    const r = clampToUnit(2, 0);
    expect(r.x).toBeCloseTo(1, 10);
    expect(r.y).toBeCloseTo(0, 10);
    expect(Math.hypot(r.x, r.y)).toBeCloseTo(1, 10);
  });

  it("clamps (3,4) → (0.6,0.8)", () => {
    const r = clampToUnit(3, 4);
    expect(r.x).toBeCloseTo(0.6, 10);
    expect(r.y).toBeCloseTo(0.8, 10);
  });
});

describe("computeJoystick", () => {
  const R = 48;

  it("center → zero", () => {
    expect(computeJoystick(0, 0, R)).toEqual({ x: 0, y: 0, magnitude: 0 });
  });

  it("dead-zone area → zero (no drift)", () => {
    // R=48, deadZone 0.25 → 12px radius dead zone
    expect(computeJoystick(8, 0, R)).toEqual({ x: 0, y: 0, magnitude: 0 });
    expect(computeJoystick(8, 8, R)).toEqual({ x: 0, y: 0, magnitude: 0 });
  });

  it("full right edge → (1, 0, 1)", () => {
    const r = computeJoystick(R, 0, R);
    expect(r.x).toBeCloseTo(1, 10);
    expect(r.y).toBeCloseTo(0, 10);
    expect(r.magnitude).toBeCloseTo(1, 10);
  });

  it("full up → (0, -1, 1) (screen-down = +y)", () => {
    const r = computeJoystick(0, -R, R);
    expect(r.x).toBeCloseTo(0, 10);
    expect(r.y).toBeCloseTo(-1, 10);
    expect(r.magnitude).toBeCloseTo(1, 10);
  });

  it("clamps finger beyond radius to magnitude 1 (diagonal)", () => {
    // (R, R) normalized = (1,1) magnitude √2 → clamped to unit circle
    const r = computeJoystick(R, R, R);
    expect(r.magnitude).toBeCloseTo(1, 10);
    // 45° → x==y
    expect(r.x).toBeCloseTo(r.y, 10);
    expect(r.x).toBeCloseTo(1 / Math.SQRT2, 5);
  });

  it("preserves sub-1 analog magnitude inside the ring", () => {
    // 24px right of center, R=48 → nx=0.5, past dead zone
    const r = computeJoystick(24, 0, R);
    expect(r.x).toBeCloseTo(0.5, 10);
    expect(r.y).toBeCloseTo(0, 10);
    expect(r.magnitude).toBeCloseTo(0.5, 10);
  });

  it("respects a custom dead zone", () => {
    // deadZone 0.5 → 24px; 12px is below custom dead zone
    expect(computeJoystick(12, 0, R, 0.5)).toEqual({ x: 0, y: 0, magnitude: 0 });
    // 30px → nx≈0.625, past 0.5 dead zone
    const r = computeJoystick(30, 0, R, 0.5);
    expect(r.x).toBeCloseTo(0.625, 3);
    expect(r.magnitude).toBeCloseTo(0.625, 3);
  });

  it("radius <= 0 guards to zero", () => {
    expect(computeJoystick(10, 10, 0)).toEqual({ x: 0, y: 0, magnitude: 0 });
    expect(computeJoystick(10, 10, -5)).toEqual({ x: 0, y: 0, magnitude: 0 });
  });

  it("default dead zone constant is 0.25", () => {
    expect(DEFAULT_DEAD_ZONE).toBe(0.25);
  });
});

describe("dominantDir", () => {
  it("returns the 4 cardinal directions", () => {
    expect(dominantDir(0, -1)).toBe("n");
    expect(dominantDir(0, 1)).toBe("s");
    expect(dominantDir(1, 0)).toBe("e");
    expect(dominantDir(-1, 0)).toBe("w");
  });

  it("y-axis wins ties (matches keyboard vy-priority)", () => {
    expect(dominantDir(1, -1)).toBe("n");
    expect(dominantDir(-1, 1)).toBe("s");
  });

  it("picks the larger component", () => {
    expect(dominantDir(0.9, -0.1)).toBe("e");
    expect(dominantDir(-0.2, 0.8)).toBe("s");
  });

  it("zero vector → null", () => {
    expect(dominantDir(0, 0)).toBeNull();
  });
});
