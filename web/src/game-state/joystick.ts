// game-state/joystick.ts — 纯摇杆数学：偏移 → 模拟量向量（含死区 + 圆裁剪）。
// 无 phaser、无 DOM（dev-plan §2.3：game-state 为纯函数）。可在 node/vitest 单测。
// CLAUDE.md 设计第一原则：移动端横屏优先；本模块是触控层的可测纯逻辑核心。

export interface Vec2 {
  x: number;
  y: number;
}

export interface JoystickResult extends Vec2 {
  /** 死区 + 裁剪后的模长 ∈ [0,1] */
  magnitude: number;
}

/** 默认圆形死区半径（归一化后）。低于此值视为松手，消除摇杆漂移。 */
export const DEFAULT_DEAD_ZONE = 0.25;

/** 圆形死区：归一化模长 < deadZone → 归零（硬门，不重缩放）。
 *  死区边界处的速度跳变 = deadZone·SPEED，在 SPEED=42 时 ≈ 10.5 px/s，肉眼不可见。 */
export function applyDeadZone(x: number, y: number, deadZone: number): Vec2 {
  const mag = Math.hypot(x, y);
  if (mag < deadZone) return { x: 0, y: 0 };
  return { x, y };
}

/** 把向量裁到单位圆内（模长 ≤ 1）。超出半径的手指位移会被收敛到满档。 */
export function clampToUnit(x: number, y: number): Vec2 {
  const mag = Math.hypot(x, y);
  if (mag <= 1) return { x, y };
  return { x: x / mag, y: y / mag };
}

/** 摇杆全流程：原始指针偏移 (dx,dy 像素) 在 `radius` 范围内 → 模拟量向量。
 *  1. 按 radius 归一化到 [-1,1]
 *  2. 死区 → 低于阈值归零
 *  3. 裁到单位圆（模长 ≤ 1）
 *  返回 { x, y, magnitude }，magnitude ∈ [0,1]。radius ≤ 0 视为无输入。 */
export function computeJoystick(
  dx: number,
  dy: number,
  radius: number,
  deadZone: number = DEFAULT_DEAD_ZONE,
): JoystickResult {
  if (radius <= 0) return { x: 0, y: 0, magnitude: 0 };
  const nx = dx / radius;
  const ny = dy / radius;
  const dead = applyDeadZone(nx, ny, deadZone);
  if (dead.x === 0 && dead.y === 0) return { x: 0, y: 0, magnitude: 0 };
  const clamped = clampToUnit(dead.x, dead.y);
  return { x: clamped.x, y: clamped.y, magnitude: Math.hypot(clamped.x, clamped.y) };
}

/** 主轴方向（用于 4 方向 facing）：取 |分量| 更大的轴。
 *  平局时 y 轴优先（与键盘 vy-priority facing 一致）。零向量 → null。
 *  场景里：键盘走原有 vy-priority 分支不变；仅触控模拟量走此函数，避免微倾错判方向。 */
export function dominantDir(x: number, y: number): "n" | "s" | "e" | "w" | null {
  if (x === 0 && y === 0) return null;
  if (Math.abs(y) >= Math.abs(x)) {
    return y < 0 ? "n" : "s";
  }
  return x < 0 ? "w" : "e";
}
