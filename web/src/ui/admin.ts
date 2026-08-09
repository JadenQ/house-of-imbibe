// ui/ — Admin 独立管理台（DOM overlay，非 Phaser）。CLAUDE.md: is_admin 才显示入口。
// 移动端横屏优先：flex 布局、大目标、@media 堆叠。
import { api, type Member } from "../net/api";

/** HTML 转义用户名（防 XSS）。纯字符串替换，不依赖 DOM，可单测。 */
export function escapeHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

/** 全屏 DOM 管理台。tabs: 成员(active) / 地图(占位) / 装饰(占位)。
 *  onClose: 关闭后回调（回到游戏，不 reload）。 */
export function showAdminConsole(onClose: () => void): void {
  const ui = document.getElementById("ui")!;
  ui.innerHTML = `
    <div class="overlay" id="admin-overlay">
      <div class="panel admin-panel">
        <div class="admin-header">
          <h1>管 理 台</h1>
          <button class="btn ghost admin-close-btn" id="admin-close">关 闭</button>
        </div>
        <div class="admin-tabs">
          <button class="admin-tab active" data-tab="members">成 员</button>
          <button class="admin-tab" data-tab="map">地 图</button>
          <button class="admin-tab" data-tab="decorations">装 饰</button>
        </div>
        <div class="admin-content" id="admin-content"></div>
      </div>
    </div>`;

  const content = document.getElementById("admin-content")!;

  const close = (): void => {
    ui.innerHTML = "";
    onClose();
  };

  document.getElementById("admin-close")!.onclick = close;
  document.getElementById("admin-overlay")!.addEventListener("click", (e) => {
    if ((e.target as HTMLElement).id === "admin-overlay") close();
  });

  // Esc 关闭
  const onKey = (e: KeyboardEvent): void => {
    if (e.key === "Escape") close();
    window.removeEventListener("keydown", onKey);
  };
  window.addEventListener("keydown", onKey);

  // tab 切换
  ui.querySelectorAll(".admin-tab").forEach((btn) => {
    btn.addEventListener("click", () => {
      ui.querySelectorAll(".admin-tab").forEach((b) => b.classList.remove("active"));
      btn.classList.add("active");
      const tab = (btn as HTMLElement).dataset.tab;
      if (tab === "members") void renderMembers(content);
      else if (tab === "map") renderPlaceholder(content, "敬请期待，D1 待建");
      else renderPlaceholder(content, "敬请期待");
    });
  });

  // 默认加载成员 tab
  void renderMembers(content);
}

function renderPlaceholder(content: HTMLElement, msg: string): void {
  content.innerHTML = `<div class="admin-placeholder">${msg}</div>`;
}

async function renderMembers(content: HTMLElement): Promise<void> {
  content.innerHTML = `<div class="admin-loading">加载中…</div>`;
  let members: Member[];
  try {
    members = await api.admin.listMembers();
  } catch {
    content.innerHTML = `<div class="admin-error">加载失败</div>`;
    return;
  }

  const rows = members
    .map((m) => {
      const role = m.is_admin ? "admin" : "member";
      const status = m.banned ? "封禁" : "正常";
      const roleBtn = m.is_admin
        ? `<button class="btn ghost admin-action" data-action="demote" data-id="${m.id}">降级</button>`
        : `<button class="btn ghost admin-action" data-action="promote" data-id="${m.id}">升级</button>`;
      const banBtn = m.banned
        ? `<button class="btn ghost admin-action" data-action="unban" data-id="${m.id}">解封</button>`
        : `<button class="btn ghost admin-action" data-action="ban" data-id="${m.id}">封禁</button>`;
      return `
      <tr>
        <td>${m.id}</td>
        <td class="admin-name">${escapeHtml(m.username)}</td>
        <td>${role}</td>
        <td class="${m.banned ? "admin-banned" : ""}">${status}</td>
        <td class="admin-actions">${roleBtn}${banBtn}</td>
      </tr>`;
    })
    .join("");

  content.innerHTML = `
    <table class="admin-table">
      <thead>
        <tr><th>ID</th><th>名字</th><th>角色</th><th>状态</th><th>操作</th></tr>
      </thead>
      <tbody>${rows}</tbody>
    </table>`;

  // 绑定操作按钮
  content.querySelectorAll(".admin-action").forEach((btn) => {
    btn.addEventListener("click", async () => {
      const el = btn as HTMLButtonElement;
      const id = parseInt(el.dataset.id!, 10);
      const action = el.dataset.action!;
      (el as HTMLButtonElement).disabled = true;
      try {
        if (action === "promote") await api.admin.promote(id);
        else if (action === "demote") await api.admin.demote(id);
        else if (action === "ban") await api.admin.ban(id);
        else if (action === "unban") await api.admin.unban(id);
        await renderMembers(content);
      } catch (e) {
        const msg = (e as Error).message || "操作失败";
        const errDiv = content.querySelector(".admin-error-inline") as HTMLElement | null;
        if (errDiv) errDiv.textContent = msg;
        else {
          const notice = document.createElement("div");
          notice.className = "admin-error-inline";
          notice.textContent = msg;
          content.prepend(notice);
        }
        (el as HTMLButtonElement).disabled = false;
      }
    });
  });
}
