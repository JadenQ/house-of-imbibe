// ui/ — Admin 独立管理台（DOM overlay，非 Phaser）。CLAUDE.md: is_admin 才显示入口。
// 移动端横屏优先：flex 布局、大目标、@media 堆叠。
import { api, type Decoration, type Member } from "../net/api";

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
      else if (tab === "decorations") void renderDecorations(content);
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
        showAdminError(content, msg);
        (el as HTMLButtonElement).disabled = false;
      }
    });
  });
}

/** 在 content 顶部显示一条内联错误（已有则更新文本）。 */
function showAdminError(content: HTMLElement, msg: string): void {
  const existing = content.querySelector(".admin-error-inline") as HTMLElement | null;
  if (existing) {
    existing.textContent = msg;
  } else {
    const notice = document.createElement("div");
    notice.className = "admin-error-inline";
    notice.textContent = msg;
    content.prepend(notice);
  }
}

/** 装饰 tab：列装饰 + 每行移除按钮 + 放置表单。移动端横屏优先（大目标）。 */
async function renderDecorations(content: HTMLElement): Promise<void> {
  content.innerHTML = `<div class="admin-loading">加载中…</div>`;
  let decorations: Decoration[];
  try {
    decorations = await api.admin.listDecorations("bar");
  } catch {
    content.innerHTML = `<div class="admin-error">加载失败</div>`;
    return;
  }

  const rows = decorations
    .map((d) => {
      const aid = d.asset_id ? escapeHtml(d.asset_id) : "—";
      return `
      <tr>
        <td class="admin-name">${escapeHtml(d.id)}</td>
        <td>(${d.tile_x}, ${d.tile_y})</td>
        <td>${aid}</td>
        <td>${d.z_layer}</td>
        <td class="admin-actions">
          <button class="btn ghost admin-action" data-action="remove-decoration" data-id="${escapeHtml(d.id)}">移除</button>
        </td>
      </tr>`;
    })
    .join("");

  content.innerHTML = `
    <div class="admin-decorations">
      <h2 class="admin-section-title">装饰列表</h2>
      <table class="admin-table">
        <thead>
          <tr><th>ID</th><th>坐标</th><th>资产ID</th><th>Z层</th><th>操作</th></tr>
        </thead>
        <tbody>${rows || '<tr><td colspan="5" class="admin-empty">暂无装饰</td></tr>'}</tbody>
      </table>
      <h2 class="admin-section-title">放置装饰</h2>
      <form class="admin-form" id="decoration-form">
        <label>场景<input name="scene" type="text" value="bar" required></label>
        <label>X<input name="tile_x" type="number" value="0" required></label>
        <label>Y<input name="tile_y" type="number" value="0" required></label>
        <label>资产ID（可选）<input name="asset_id" type="text" placeholder="占位留空"></label>
        <label>Z层<input name="z_layer" type="number" value="0"></label>
        <button type="submit" class="btn">放置</button>
      </form>
    </div>`;

  // 绑定移除按钮
  content
    .querySelectorAll('.admin-action[data-action="remove-decoration"]')
    .forEach((btn) => {
      btn.addEventListener("click", async () => {
        const el = btn as HTMLButtonElement;
        const id = el.dataset.id!;
        el.disabled = true;
        try {
          await api.admin.removeDecoration(id);
          await renderDecorations(content);
        } catch (e) {
          showAdminError(content, (e as Error).message || "移除失败");
          el.disabled = false;
        }
      });
    });

  // 绑定放置表单
  const form = content.querySelector("#decoration-form") as HTMLFormElement | null;
  if (form) {
    form.addEventListener("submit", async (e) => {
      e.preventDefault();
      const fd = new FormData(form);
      const scene = (fd.get("scene") as string).trim();
      const tile_x = parseInt(fd.get("tile_x") as string, 10);
      const tile_y = parseInt(fd.get("tile_y") as string, 10);
      const asset_id_raw = (fd.get("asset_id") as string).trim();
      const asset_id = asset_id_raw || undefined;
      const z_layer_raw = fd.get("z_layer") as string;
      const z_layer = z_layer_raw ? parseInt(z_layer_raw, 10) : undefined;
      const submitBtn = form.querySelector('button[type="submit"]') as HTMLButtonElement;
      submitBtn.disabled = true;
      try {
        await api.admin.placeDecoration({ scene, tile_x, tile_y, asset_id, z_layer });
        await renderDecorations(content);
      } catch (e) {
        showAdminError(content, (e as Error).message || "放置失败");
        submitBtn.disabled = false;
      }
    });
  }
}
