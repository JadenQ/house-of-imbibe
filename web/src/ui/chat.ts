// ui/ — 聊天侧栏：DOM overlay，最近 50 条 + 输入框 + 折叠/展开。无 phaser。
// 用户输入必须 HTML 转义（XSS 防护）。
// 默认展开；点击标题栏可折叠省屏（移动端横屏友好）。

import type { ChatLine } from "../game-state/types";

export interface ChatPanel {
  setChat(lines: ChatLine[]): void;
}

function escapeHtml(s: string): string {
  return s.replace(
    /[&<>"']/g,
    (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]!,
  );
}

/** 创建并挂载聊天面板。返回 setChat 接口 + onSend 回调注册。
 *  默认展开；点击标题栏可折叠省屏（折叠后仅显示标题栏，展开恢复列表+输入框）。 */
export function createChatPanel(root: HTMLElement): {
  el: HTMLElement;
  panel: ChatPanel;
  onSend: (cb: (text: string) => void) => void;
} {
  const el = document.createElement("div");
  el.id = "chat-panel";
  el.innerHTML = `
    <div id="chat-header">
      <span id="chat-title">聊天</span>
      <button id="chat-toggle" type="button" aria-label="折叠/展开聊天">▾</button>
    </div>
    <div id="chat-body">
      <div id="chat-list"></div>
      <input id="chat-input" placeholder="说点什么…" maxlength="200" autocomplete="off" />
    </div>`;
  el.style.cssText =
    "position:absolute;left:6px;top:6px;width:208px;max-height:48%;display:flex;flex-direction:column;" +
    "background:rgba(20,16,14,.85);border:2px solid #4a3826;z-index:8;font-size:11px;color:#c8b898;" +
    "font-family:'Courier New',ui-monospace,monospace";

  const header = el.querySelector<HTMLElement>("#chat-header")!;
  header.style.cssText =
    "display:flex;justify-content:space-between;align-items:center;padding:3px 6px;" +
    "background:#2a1f17;border-bottom:2px solid #4a3826;cursor:pointer;user-select:none";

  const title = el.querySelector<HTMLElement>("#chat-title")!;
  title.style.cssText = "font-weight:bold;color:#e8dcc8";

  const toggle = el.querySelector<HTMLButtonElement>("#chat-toggle")!;
  toggle.style.cssText =
    "background:none;border:0;color:#c8b898;font:inherit;font-size:13px;cursor:pointer;padding:0 2px;line-height:1";

  const body = el.querySelector<HTMLElement>("#chat-body")!;
  body.style.cssText = "flex:1;display:flex;flex-direction:column;min-height:0;overflow:hidden";

  const list = el.querySelector<HTMLElement>("#chat-list")!;
  list.style.cssText = "flex:1;overflow-y:auto;padding:4px 6px;min-height:40px";

  const input = el.querySelector<HTMLInputElement>("#chat-input")!;
  input.style.cssText =
    "border:0;border-top:2px solid #4a3826;background:#14100e;color:#e8dcc8;font:inherit;font-size:11px;padding:4px;outline:none";

  root.appendChild(el);

  // 折叠/展开（默认展开；点击标题栏切换）
  let collapsed = false;
  const toggleCollapse = () => {
    collapsed = !collapsed;
    body.style.display = collapsed ? "none" : "flex";
    toggle.textContent = collapsed ? "▸" : "▾";
    if (!collapsed) list.scrollTop = list.scrollHeight; // 展开时滚到最新
  };
  header.addEventListener("click", () => toggleCollapse());

  let sendCb: ((text: string) => void) | null = null;
  input.addEventListener("keydown", (e) => {
    // 方向键放行给场景移动（不 stopPropagation）；preventDefault 防光标在输入框里移动。
    if (e.key.startsWith("Arrow")) {
      e.preventDefault();
      return;
    }
    // Esc 失焦，回到游戏操作。
    if (e.key === "Escape") {
      input.blur();
      e.preventDefault();
      e.stopPropagation();
      return;
    }
    e.stopPropagation(); // 其余键（字母/数字/符号）输入聊天框，防止被场景捕获成走字
    if (e.key === "Enter") {
      const t = input.value.trim();
      if (t) {
        sendCb?.(t);
        input.value = "";
      }
      e.preventDefault();
    }
  });

  return {
    el,
    panel: {
      setChat(lines: ChatLine[]) {
        // 全量重绘最近 50 条（DOM 节点少，开销可忽略）
        list.innerHTML = lines
          .slice(-50)
          .map(
            (l) =>
              `<div><b>${escapeHtml(l.name)}</b>: ${escapeHtml(l.text)}</div>`,
          )
          .join("");
        list.scrollTop = list.scrollHeight;
      },
    },
    onSend(cb) {
      sendCb = cb;
    },
  };
}
