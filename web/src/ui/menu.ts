// ui/ — 酒单架子：DOM overlay，数据来自 GET /api/menu
import { api, type MenuPayload } from "../net/api";

let open = false;
let closedAt = 0;

export function isMenuOpen(): boolean {
  return open;
}

/** 关闭后的短窗口内屏蔽交互键，避免同一次按 E 触发"关→开"抖动 */
export function menuBlocksInteract(): boolean {
  return open || Date.now() - closedAt < 400;
}

export async function showMenu(): Promise<void> {
  if (open) return;
  open = true;
  let menu: MenuPayload;
  try {
    menu = await api.menu();
  } catch {
    open = false;
    return;
  }

  const sections = menu.sections
    .map(
      (s) => `
      <div class="menu-section">${s.title}</div>
      ${s.items
        .map(
          (i) => `
        <div class="menu-item">
          <span class="price">${i.price != null ? "$" + i.price : ""}</span>
          <div class="name">${i.name}</div>
          <div class="desc">${i.desc}</div>
        </div>`,
        )
        .join("")}`,
    )
    .join("");

  const ui = document.getElementById("ui")!;
  ui.innerHTML = `
    <div class="overlay" id="menu-overlay"><div class="panel" style="min-width:300px">
      <h1>酒 单</h1>
      <h2>HOUSE OF IMBIBE · est. 2026</h2>
      ${sections}
      <button class="btn ghost" id="menu-close">放 回 架 子 (E)</button>
    </div></div>`;

  const close = () => {
    ui.innerHTML = "";
    open = false;
    closedAt = Date.now();
    window.removeEventListener("keydown", onKey);
  };
  const onKey = (e: KeyboardEvent) => {
    if (e.key === "e" || e.key === "E" || e.key === "Escape" || e.key === " ") close();
  };
  window.addEventListener("keydown", onKey);
  document.getElementById("menu-close")!.onclick = close;
  document.getElementById("menu-overlay")!.addEventListener("click", (e) => {
    if ((e.target as HTMLElement).id === "menu-overlay") close();
  });
}
