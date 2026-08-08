// ui/ — 聊天侧栏：DOM overlay，最近 50 条 + 输入框。无 phaser。
// 用户输入必须 HTML 转义（XSS 防护）。

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

/** 创建并挂载聊天面板。返回 setChat 接口 + onSend 回调注册。 */
export function createChatPanel(root: HTMLElement): {
  el: HTMLElement;
  panel: ChatPanel;
  onSend: (cb: (text: string) => void) => void;
} {
  const el = document.createElement("div");
  el.id = "chat-panel";
  el.innerHTML = `<div id="chat-list"></div><input id="chat-input" placeholder="说点什么…" maxlength="200" autocomplete="off" />`;
  el.style.cssText =
    "position:absolute;left:6px;top:6px;width:208px;max-height:48%;display:flex;flex-direction:column;" +
    "background:rgba(20,16,14,.85);border:2px solid #4a3826;z-index:8;font-size:11px;color:#c8b898;" +
    "font-family:'Courier New',ui-monospace,monospace";
  const list = el.querySelector<HTMLElement>("#chat-list")!;
  list.style.cssText = "flex:1;overflow-y:auto;padding:4px 6px;min-height:40px";
  const input = el.querySelector<HTMLInputElement>("#chat-input")!;
  input.style.cssText =
    "border:0;border-top:2px solid #4a3826;background:#14100e;color:#e8dcc8;font:inherit;font-size:11px;padding:4px;outline:none";
  root.appendChild(el);

  let sendCb: ((text: string) => void) | null = null;
  input.addEventListener("keydown", (e) => {
    e.stopPropagation(); // 阻止 WASD/方向键被场景捕获成走字
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
