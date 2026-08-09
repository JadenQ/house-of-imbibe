// ui/touch.ts — DOM/CSS 触控层（【绝不进 Phaser 渲染层】，纯 DOM overlay 覆在 canvas 上）。
// 左侧虚拟摇杆（模拟量→方向向量，含死区）+ 右侧动作键（= E/interact）。
// pointer events；只在触控设备显示（matchMedia('(pointer:coarse)') 或首次 touch pointerdown）。
// 横屏布局优先；不破坏整数缩放与 imageSmoothingEnabled=false（纯 DOM，不触 canvas）。
// CLAUDE.md 设计第一原则：移动端横屏优先级高于桌面。桌面键盘为回退方案（BarScene 原样保留）。

import { computeJoystick } from "../game-state/joystick";

export interface TouchInput {
  /** 模拟量 x ∈ [-1,1]，右 = +；闲置时 0。屏幕坐标系（与 canvas/world 一致：右 = +x） */
  x: number;
  /** 模拟量 y ∈ [-1,1]，下 = +；闲置时 0。屏幕坐标系（下 = +y，与 canvas/world 一致） */
  y: number;
  /** true 当指针正按在摇杆上 */
  active: boolean;
}

export interface TouchControls {
  /** 每帧读取当前摇杆模拟量输入。 */
  getInput(): TouchInput;
  /** 消费一次 interact 请求（边沿触发：每次按下仅返回一次 true）。 */
  consumeInteract(): boolean;
  /** 销毁 DOM + 监听器。 */
  destroy(): void;
}

// 尺寸按横屏拇指可达性定（px，视口坐标）。摇杆 base 直径 96，knob 40；动作键 76。
const STICK_RADIUS = 46; // knob 行程 + 数学归一化半径
const STICK_BASE = 96;
const STICK_KNOB = 40;
const BTN_SIZE = 76;

const COLOR_PANEL = "rgba(20,16,14,0.35)";
const COLOR_KNOB = "rgba(212,162,78,0.55)";
const COLOR_KNOB_BORDER = "#d4a24e";
const COLOR_BTN = "rgba(212,162,78,0.25)";
const COLOR_BTN_PRESS = "rgba(212,162,78,0.55)";
const FONT = "'Courier New',ui-monospace,monospace";

/** 创建并挂载触控层到 `root`（建议 #app 全屏容器，使控制键落在屏幕拇指区而非仅游戏区）。 */
export function createTouchControls(root: HTMLElement): TouchControls {
  const layer = document.createElement("div");
  layer.id = "touch-layer";
  // pointer-events:none 容器 → 触摸穿透到 canvas/聊天；仅控制键自身 pointer-events:auto 捕获。
  // display:none 直到检测到触控设备才显示（桌面永不显示）。
  layer.style.cssText =
    "position:fixed;inset:0;pointer-events:none;z-index:5;display:none;" +
    "touch-action:none;user-select:none;-webkit-user-select:none;";

  // ── 左虚拟摇杆 ──
  const base = document.createElement("div");
  base.style.cssText =
    `position:absolute;left:24px;bottom:24px;width:${STICK_BASE}px;height:${STICK_BASE}px;` +
    `border-radius:50%;background:${COLOR_PANEL};border:2px solid #4a3826;` +
    `pointer-events:auto;touch-action:none;box-sizing:border-box;`;
  const knob = document.createElement("div");
  knob.style.cssText =
    `position:absolute;left:50%;top:50%;width:${STICK_KNOB}px;height:${STICK_KNOB}px;` +
    `margin-left:${-STICK_KNOB / 2}px;margin-top:${-STICK_KNOB / 2}px;border-radius:50%;` +
    `background:${COLOR_KNOB};border:2px solid ${COLOR_KNOB_BORDER};pointer-events:none;` +
    `box-sizing:border-box;`;
  base.appendChild(knob);

  // ── 右动作键（= E/interact）──
  const btn = document.createElement("div");
  btn.style.cssText =
    `position:absolute;right:24px;bottom:24px;width:${BTN_SIZE}px;height:${BTN_SIZE}px;` +
    `border-radius:50%;background:${COLOR_BTN};border:2px solid ${COLOR_KNOB_BORDER};` +
    `color:${COLOR_KNOB_BORDER};font-family:${FONT};font-size:24px;font-weight:bold;` +
    `display:flex;align-items:center;justify-content:center;` +
    `pointer-events:auto;touch-action:none;user-select:none;box-sizing:border-box;`;
  btn.textContent = "E";

  layer.appendChild(base);
  layer.appendChild(btn);
  root.appendChild(layer);

  let input: TouchInput = { x: 0, y: 0, active: false };
  let interactRequested = false;

  const centerOf = (el: HTMLElement) => {
    const r = el.getBoundingClientRect();
    return { cx: r.left + r.width / 2, cy: r.top + r.height / 2 };
  };

  /** 把 knob 视觉位移裁到 STICK_RADIUS 内。 */
  const moveKnob = (dx: number, dy: number) => {
    const mag = Math.hypot(dx, dy);
    let kx = dx;
    let ky = dy;
    if (mag > STICK_RADIUS) {
      kx = (dx / mag) * STICK_RADIUS;
      ky = (dy / mag) * STICK_RADIUS;
    }
    knob.style.transform = `translate(${kx}px, ${ky}px)`;
  };
  const resetKnob = () => {
    knob.style.transform = "translate(0px, 0px)";
  };

  // 摇杆指针处理（capture 保证手指移出 base 仍收到 move/up）
  let stickPointer: number | null = null;
  base.addEventListener("pointerdown", (e) => {
    e.preventDefault();
    stickPointer = e.pointerId;
    try {
      base.setPointerCapture(e.pointerId);
    } catch {
      /* setPointerCapture 在部分浏览器对 div 偶发抛错，忽略即可 */
    }
    const { cx, cy } = centerOf(base);
    const dx = e.clientX - cx;
    const dy = e.clientY - cy;
    const r = computeJoystick(dx, dy, STICK_RADIUS);
    input = { x: r.x, y: r.y, active: true };
    moveKnob(dx, dy);
  });
  base.addEventListener("pointermove", (e) => {
    if (e.pointerId !== stickPointer) return;
    const { cx, cy } = centerOf(base);
    const dx = e.clientX - cx;
    const dy = e.clientY - cy;
    const r = computeJoystick(dx, dy, STICK_RADIUS);
    input = { x: r.x, y: r.y, active: true };
    moveKnob(dx, dy);
  });
  const endStick = (e: PointerEvent) => {
    if (e.pointerId !== stickPointer) return;
    stickPointer = null;
    input = { x: 0, y: 0, active: false };
    resetKnob();
  };
  base.addEventListener("pointerup", endStick);
  base.addEventListener("pointercancel", endStick);

  // 动作键：按下置 interact 边沿；BarScene 每帧 consume（无论是否面向可交互物，
  // 都消费，避免按下未命中后走过去再触发陈旧 interact）。与 E 的 JustDown 同为边沿语义。
  btn.addEventListener("pointerdown", (e) => {
    e.preventDefault();
    btn.style.background = COLOR_BTN_PRESS;
    interactRequested = true;
  });
  const releaseBtn = () => {
    btn.style.background = COLOR_BTN;
  };
  btn.addEventListener("pointerup", releaseBtn);
  btn.addEventListener("pointercancel", releaseBtn);
  btn.addEventListener("pointerleave", releaseBtn);

  // ── 仅触控设备显示 ──
  // 1) pointer:coarse（触控主设备）→ 立即显示
  // 2) 否则监听首次 touch 类型 pointerdown → 显示后移除监听（覆盖带触屏的 fine 设备）
  const activate = () => {
    if (layer.style.display !== "block") layer.style.display = "block";
  };
  const coarse = window.matchMedia?.("(pointer: coarse)");
  if (coarse?.matches) {
    activate();
  } else {
    const onFirstTouchPointer = (e: PointerEvent) => {
      if (e.pointerType === "touch") {
        activate();
        window.removeEventListener("pointerdown", onFirstTouchPointer);
      }
    };
    window.addEventListener("pointerdown", onFirstTouchPointer, { passive: true });
  }

  return {
    getInput: () => input,
    consumeInteract: () => {
      if (interactRequested) {
        interactRequested = false;
        return true;
      }
      return false;
    },
    destroy: () => {
      layer.remove();
    },
  };
}
