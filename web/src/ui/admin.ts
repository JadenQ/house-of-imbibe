// ui/ — Admin 独立管理台（DOM overlay，非 Phaser）。CLAUDE.md: is_admin 才显示入口。
// 移动端横屏优先：flex 布局、大目标、@media 堆叠。
// tabs: 成员 / 地图（背景+可走网格+装饰放置/列表）/ 酒单。
// 新增网格/酒单样式由 injectAdminStyles() JS 注入 <style>（不碰 index.html）。
import { api, type AdminMenuItem, type Decoration, type MapInfo, type Member } from "../net/api";

/** HTML 转义用户名（防 XSS）。纯字符串替换，不依赖 DOM，可单测。 */
export function escapeHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

/** 注入 admin 新增样式（酒单行/网格/模式切换/图例）。幂等：只追加一次。 */
function injectAdminStyles(): void {
  if (document.getElementById("admin-extra-styles")) return;
  const style = document.createElement("style");
  style.id = "admin-extra-styles";
  style.textContent = `
    /* === 酒单 tab === */
    .admin-menu-new, .admin-menu-row {
      display: flex; flex-wrap: wrap; gap: 6px; align-items: center;
      padding: 8px 4px; border-bottom: 1px dashed #4a3826;
    }
    .admin-menu-row input, .admin-menu-new input { flex: 1; min-width: 60px; }
    .admin-menu-row input[name="description"], .admin-menu-new input[name="description"] { flex: 2; }
    .admin-menu-id { font-size: 10px; color: #9a8a70; flex: 0 0 auto; min-width: 44px; word-break: break-all; }
    .admin-menu-row label, .admin-menu-new label {
      display: flex; align-items: center; gap: 4px; font-size: 11px;
      color: #9a8a70; flex: 0 0 auto; white-space: nowrap;
    }
    .admin-menu-new .btn, .admin-menu-row .btn {
      width: auto; flex: 0 0 auto; margin: 0; padding: 8px 12px; font-size: 11px;
    }
    .admin-menu-rows { margin-top: 4px; }

    /* === 地图网格（可走笔刷 + 装饰点选） === */
    .admin-mode { display: flex; gap: 6px; margin: 8px 0; }
    .admin-mode .btn { width: auto; flex: 1; margin: 0; padding: 10px 8px; font-size: 12px; }
    .admin-legend {
      display: flex; gap: 14px; flex-wrap: wrap; font-size: 11px;
      color: #9a8a70; margin: 6px 0;
    }
    .admin-legend .lg { display: flex; align-items: center; gap: 4px; }
    .admin-legend .sw {
      width: 14px; height: 14px; display: inline-block; border: 1px solid #4a3826;
    }
    .admin-legend .sw.walk { background: #6b4a2e; }
    .admin-legend .sw.blocked { background: #2e2115; }
    .admin-legend .sw.sel {
      background: #6b4a2e; outline: 2px solid #d4a24e; outline-offset: -2px;
    }
    .admin-legend .sw.deco { background: #6b4a2e; color: #d4a24e; }
    .admin-grid {
      display: grid; gap: 1px; background: #4a3826; padding: 2px;
      border: 2px solid #4a3826; margin: 8px 0; width: fit-content;
      max-width: 100%; overflow: auto;
    }
    .admin-cell {
      width: 24px; height: 24px; cursor: pointer; image-rendering: pixelated;
      display: flex; align-items: center; justify-content: center; font-size: 11px;
      box-sizing: border-box;
    }
    .admin-cell.walk { background: #6b4a2e; }
    .admin-cell.walk:hover { background: #7d5836; }
    .admin-cell.blocked { background: #2e2115; }
    .admin-cell.blocked:hover { background: #3a2a1c; }
    .admin-cell.selected { outline: 2px solid #d4a24e; outline-offset: -2px; z-index: 1; }
    .admin-cell.deco-mark::after { content: "◆"; color: #d4a24e; }
    .admin-grid-hint { font-size: 11px; color: #9a8a70; margin: 4px 0 8px; }
    .admin-grid-save { width: auto; margin: 4px 0 12px; padding: 10px 16px; }
    .admin-deco-pos { font-size: 12px; color: var(--accent); margin: 4px 0; }

    /* === 移动端横屏优先 === */
    @media (max-width: 500px) {
      .admin-menu-new, .admin-menu-row { flex-direction: column; align-items: stretch; }
      .admin-menu-new input, .admin-menu-row input { min-width: 0; flex: 1 1 auto; }
      .admin-menu-new .btn, .admin-menu-row .btn { width: 100%; }
      .admin-mode .btn { padding: 12px 6px; }
      /* 网格格子保持 min 24px（移动端横屏 15×24=360 适配；超出滚动） */
      .admin-cell { width: 24px; height: 24px; }
    }
  `;
  document.head.appendChild(style);
}

/** 全屏 DOM 管理台。tabs: 成员(active) / 地图 / 酒单。
 *  onClose: 关闭后回调（回到游戏，不 reload）。 */
export function showAdminConsole(onClose: () => void): void {
  injectAdminStyles();
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
          <button class="admin-tab" data-tab="menu">酒 单</button>
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
      else if (tab === "map") void renderMapTab(content);
      else if (tab === "menu") void renderMenu(content);
    });
  });

  // 默认加载成员 tab
  void renderMembers(content);
}

/** 在 content 顶部显示一条内联提示（已有则更新文本）。复用为错误/成功提示。 */
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

/** 地图 tab：背景图预览 + 重新生成 + 可走笔刷网格 + 装饰点选放置 + 已放装饰列表。
 *  walkable 从 admin map 加载（null=全可走）；改完「保存可走网格」调 setWalkable。
 *  装饰：点格子选位置 → 填表单 → 放置（placeDecoration）；列表可移除。 */
async function renderMapTab(content: HTMLElement): Promise<void> {
  content.innerHTML = `<div class="admin-loading">加载中…</div>`;
  let mapInfo: MapInfo;
  try {
    mapInfo = await api.admin.getMap("bar");
  } catch {
    content.innerHTML = `<div class="admin-error">加载失败</div>`;
    return;
  }
  // 装饰列表失败不阻断网格编辑
  let decos: Decoration[] = [];
  try {
    decos = await api.admin.listDecorations("bar");
  } catch {
    /* 装饰加载失败时继续，列表显示空 */
  }

  const W = mapInfo.width;
  const H = mapInfo.height;
  // 初始化网格：null=全可走(0)；越界/缺格按 0 处理（防脏数据）
  const grid: number[][] = [];
  for (let y = 0; y < H; y++) {
    const row: number[] = [];
    const src = mapInfo.walkable?.[y];
    for (let x = 0; x < W; x++) row.push(src && src[x] === 1 ? 1 : 0);
    grid.push(row);
  }

  // 内存态：模式 + 选中的放置位置
  let mode: "walk" | "deco" = "walk";
  let selected: { x: number; y: number } | null = null;

  const bgPreview = mapInfo.bg_key
    ? `<img class="admin-map-thumb" src="/api/assets/${encodeURIComponent(mapInfo.bg_key)}" alt="背景图" style="max-width:240px;image-rendering:pixelated;border:1px solid #555">`
    : `<span class="admin-empty">无背景图，用静态 tile 渲染</span>`;

  content.innerHTML = `
    <div class="admin-map">
      <h2 class="admin-section-title">地图背景（${escapeHtml(mapInfo.scene)} ${W}×${H}）</h2>
      <div class="admin-map-bg-preview" style="margin:8px 0">${bgPreview}</div>

      <h2 class="admin-section-title">可走 / 装饰网格</h2>
      <div class="admin-mode" id="map-mode">
        <button class="btn admin-mode-btn" data-mode="walk">可走笔刷</button>
        <button class="btn ghost admin-mode-btn" data-mode="deco">放置装饰</button>
      </div>
      <div class="admin-legend">
        <span class="lg"><span class="sw walk"></span>可走</span>
        <span class="lg"><span class="sw blocked"></span>阻挡</span>
        <span class="lg"><span class="sw sel"></span>选中位置</span>
        <span class="lg"><span class="sw deco">◆</span>已放装饰</span>
      </div>
      <div class="admin-grid" id="map-grid" style="grid-template-columns:repeat(${W},24px)"></div>
      <div class="admin-grid-hint" id="map-hint">点击格子切换可走/阻挡。「保存可走网格」后生效。未保存的修改切换 tab 会丢失。</div>
      <button class="btn admin-grid-save" id="map-save-walkable">保存可走网格</button>

      <h2 class="admin-section-title">放置装饰</h2>
      <div class="admin-deco-pos" id="deco-selected">未选位置</div>
      <form class="admin-form" id="deco-place-form">
        <label>资产ID（可选）<input name="asset_id" type="text" placeholder="占位留空"></label>
        <label>Z层<input name="z_layer" type="number" inputmode="numeric" value="0"></label>
        <button type="submit" class="btn" id="deco-place-btn" disabled>放置装饰</button>
      </form>

      <h2 class="admin-section-title">已放装饰</h2>
      <div id="deco-list"></div>

      <h2 class="admin-section-title">重新生成背景</h2>
      <form class="admin-form" id="map-regenerate-form">
        <label>描述文字<input name="prompt" type="text" placeholder="cozy tavern interior, wooden bar counter, warm lighting..." required></label>
        <button type="submit" class="btn">重新生成背景</button>
      </form>
    </div>`;

  // 构建网格格子（row-major: y 外 x 内，children[y*W+x]）
  const gridEl = document.getElementById("map-grid")!;
  for (let y = 0; y < H; y++) {
    for (let x = 0; x < W; x++) {
      const cell = document.createElement("div");
      cell.className = "admin-cell " + (grid[y][x] ? "blocked" : "walk");
      cell.dataset.x = String(x);
      cell.dataset.y = String(y);
      gridEl.appendChild(cell);
    }
  }
  markDecos(gridEl, decos, W, H);

  // 网格点击（事件委托）：可走模式=切换，装饰模式=选位置
  gridEl.addEventListener("click", (e) => {
    const cell = (e.target as HTMLElement).closest(".admin-cell") as HTMLElement | null;
    if (!cell) return;
    const x = parseInt(cell.dataset.x!, 10);
    const y = parseInt(cell.dataset.y!, 10);
    if (mode === "walk") {
      grid[y][x] = grid[y][x] ? 0 : 1;
      cell.classList.toggle("walk", grid[y][x] === 0);
      cell.classList.toggle("blocked", grid[y][x] === 1);
    } else {
      gridEl.querySelectorAll(".admin-cell.selected").forEach((c) => c.classList.remove("selected"));
      cell.classList.add("selected");
      selected = { x, y };
      updateDecoSelected();
    }
  });

  // 模式切换
  const hintEl = content.querySelector("#map-hint")!;
  content.querySelectorAll("#map-mode .admin-mode-btn").forEach((btn) => {
    btn.addEventListener("click", () => {
      mode = (btn as HTMLElement).dataset.mode as "walk" | "deco";
      content.querySelectorAll("#map-mode .admin-mode-btn").forEach((b) => {
        const m = (b as HTMLElement).dataset.mode;
        b.classList.toggle("ghost", m !== mode);
      });
      hintEl.textContent =
        mode === "walk"
          ? "点击格子切换可走/阻挡。「保存可走网格」后生效。未保存的修改切换 tab 会丢失。"
          : "点击格子选放置位置，再填下方表单放置装饰。";
      // 切回可走模式时清掉选中
      if (mode === "walk") {
        selected = null;
        gridEl.querySelectorAll(".admin-cell.selected").forEach((c) => c.classList.remove("selected"));
        updateDecoSelected();
      }
    });
  });

  const decoSelectedEl = content.querySelector("#deco-selected")!;
  const decoPlaceBtn = content.querySelector("#deco-place-btn") as HTMLButtonElement;
  function updateDecoSelected(): void {
    decoSelectedEl.textContent = selected ? `已选位置：(${selected.x}, ${selected.y})` : "未选位置";
    decoPlaceBtn.disabled = !selected;
  }

  // 保存可走网格
  content.querySelector("#map-save-walkable")!.addEventListener("click", async () => {
    const btn = content.querySelector("#map-save-walkable") as HTMLButtonElement;
    btn.disabled = true;
    try {
      await api.admin.setWalkable(mapInfo.scene, grid);
      showAdminError(content, "可走网格已保存");
    } catch (e) {
      showAdminError(content, (e as Error).message || "保存失败");
    } finally {
      btn.disabled = false;
    }
  });

  // 放置装饰表单
  const decoForm = content.querySelector("#deco-place-form") as HTMLFormElement;
  decoForm.addEventListener("submit", async (e) => {
    e.preventDefault();
    if (!selected) return;
    const fd = new FormData(decoForm);
    const asset_id_raw = (fd.get("asset_id") as string).trim();
    const asset_id = asset_id_raw || undefined;
    const z_raw = fd.get("z_layer") as string;
    const z_layer = z_raw ? parseInt(z_raw, 10) : undefined;
    decoPlaceBtn.disabled = true;
    try {
      await api.admin.placeDecoration({
        scene: mapInfo.scene,
        tile_x: selected.x,
        tile_y: selected.y,
        asset_id,
        z_layer,
      });
      decos = await safeListDecorations(mapInfo.scene);
      renderDecoList();
      markDecos(gridEl, decos, W, H);
      showAdminError(content, `装饰已放置于 (${selected.x}, ${selected.y})`);
    } catch (e) {
      showAdminError(content, (e as Error).message || "放置失败");
    } finally {
      decoPlaceBtn.disabled = false;
    }
  });

  // 已放装饰列表 + 移除
  function renderDecoList(): void {
    const el = content.querySelector("#deco-list") as HTMLElement;
    if (decos.length === 0) {
      el.innerHTML = `<div class="admin-empty">暂无装饰</div>`;
      return;
    }
    const rows = decos
      .map((d) => {
        const aid = d.asset_id ? escapeHtml(d.asset_id) : "—";
        return `
        <tr>
          <td class="admin-name">${escapeHtml(d.id)}</td>
          <td>(${d.tile_x}, ${d.tile_y})</td>
          <td>${aid}</td>
          <td>${d.z_layer}</td>
          <td class="admin-actions">
            <button class="btn ghost admin-action" data-action="remove-deco" data-id="${escapeHtml(d.id)}">移除</button>
          </td>
        </tr>`;
      })
      .join("");
    el.innerHTML = `
      <table class="admin-table">
        <thead>
          <tr><th>ID</th><th>坐标</th><th>资产ID</th><th>Z层</th><th>操作</th></tr>
        </thead>
        <tbody>${rows}</tbody>
      </table>`;
    el.querySelectorAll('.admin-action[data-action="remove-deco"]').forEach((btn) => {
      btn.addEventListener("click", async () => {
        const el2 = btn as HTMLButtonElement;
        const id = el2.dataset.id!;
        el2.disabled = true;
        try {
          await api.admin.removeDecoration(id);
          decos = await safeListDecorations(mapInfo.scene);
          renderDecoList();
          markDecos(gridEl, decos, W, H);
          showAdminError(content, "装饰已移除");
        } catch (e) {
          showAdminError(content, (e as Error).message || "移除失败");
          el2.disabled = false;
        }
      });
    });
  }
  renderDecoList();

  // 重新生成背景表单
  const regenForm = content.querySelector("#map-regenerate-form") as HTMLFormElement | null;
  if (regenForm) {
    regenForm.addEventListener("submit", async (e) => {
      e.preventDefault();
      const fd = new FormData(regenForm);
      const prompt = (fd.get("prompt") as string).trim();
      if (!prompt) return;
      const submitBtn = regenForm.querySelector('button[type="submit"]') as HTMLButtonElement;
      submitBtn.disabled = true;
      try {
        const { job_id } = await api.admin.regenerateMap(prompt, "bar");
        showAdminError(content, `已提交生成任务（job: ${job_id}），生成中…`);
        void pollMapJob(content, job_id);
      } catch (err) {
        showAdminError(content, (err as Error).message || "提交失败");
        submitBtn.disabled = false;
      }
    });
  }
}

/** 装饰列表读取（失败返回空，不阻断）。 */
async function safeListDecorations(scene: string): Promise<Decoration[]> {
  try {
    return await api.admin.listDecorations(scene);
  } catch {
    return [];
  }
}

/** 在网格上标记已放装饰位置（◆）。清旧标再标新。 */
function markDecos(gridEl: Element, decos: Decoration[], W: number, H: number): void {
  gridEl.querySelectorAll(".admin-cell.deco-mark").forEach((c) => c.classList.remove("deco-mark"));
  for (const d of decos) {
    if (d.tile_x < 0 || d.tile_x >= W || d.tile_y < 0 || d.tile_y >= H) continue;
    const cell = gridEl.children[d.tile_y * W + d.tile_x] as HTMLElement | undefined;
    if (cell) cell.classList.add("deco-mark");
  }
}

/** 非阻塞轮询 map_bg job 状态（复用 generation_jobs 表）。完成→刷新，失败→提示。 */
async function pollMapJob(content: HTMLElement, jobId: string): Promise<void> {
  for (let i = 0; i < 120; i++) {
    await new Promise((r) => setTimeout(r, 2000));
    try {
      const status = await api.pollAvatarJob(jobId);
      if (status.status === "done") {
        showAdminError(content, "背景图生成完成！");
        await renderMapTab(content);
        return;
      }
      if (status.status === "failed") {
        showAdminError(content, `生成失败：${status.error ?? "未知错误"}`);
        return;
      }
    } catch {
      // 忽略轮询错误，继续重试
    }
  }
  showAdminError(content, "生成超时，请稍后刷新查看");
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

/** 酒单 tab：顶部新增表单 + 每行内联编辑(section/name/desc/price/排序/visible) + 删除。
 *  服务端 PUT 会用 unwrap_or_default() 清空缺失字段，故 description 必须随保存回传（防丢）。 */
async function renderMenu(content: HTMLElement): Promise<void> {
  content.innerHTML = `<div class="admin-loading">加载中…</div>`;
  let items: AdminMenuItem[];
  try {
    items = await api.admin.listMenu();
  } catch {
    content.innerHTML = `<div class="admin-error">加载失败</div>`;
    return;
  }

  // 按 section → sort_order → id 排序，与后端一致
  items.sort(
    (a, b) =>
      a.section.localeCompare(b.section) ||
      a.sort_order - b.sort_order ||
      a.id.localeCompare(b.id),
  );

  const rows = items
    .map((it) => {
      return `
      <form class="admin-menu-row" data-id="${escapeHtml(it.id)}">
        <span class="admin-menu-id">${escapeHtml(it.id)}</span>
        <input name="section" type="text" value="${escapeHtml(it.section)}" placeholder="分类" required>
        <input name="name" type="text" value="${escapeHtml(it.name)}" placeholder="名称" required>
        <input name="description" type="text" value="${escapeHtml(it.description)}" placeholder="描述">
        <input name="price" type="number" inputmode="numeric" step="1" value="${it.price}" placeholder="价格">
        <input name="sort_order" type="number" inputmode="numeric" step="1" value="${it.sort_order}" placeholder="排序">
        <label><input type="checkbox" name="visible" ${it.visible ? "checked" : ""}>显示</label>
        <button type="button" class="btn ghost" data-action="save">保存</button>
        <button type="button" class="btn ghost" data-action="delete">删除</button>
      </form>`;
    })
    .join("");

  content.innerHTML = `
    <div class="admin-menu">
      <h2 class="admin-section-title">新增酒单项</h2>
      <form class="admin-menu-new" id="menu-new-form">
        <input name="section" type="text" placeholder="分类" required>
        <input name="name" type="text" placeholder="名称" required>
        <input name="description" type="text" placeholder="描述">
        <input name="price" type="number" inputmode="numeric" step="1" placeholder="价格" value="0">
        <input name="sort_order" type="number" inputmode="numeric" step="1" placeholder="排序" value="0">
        <label><input type="checkbox" name="visible" checked>显示</label>
        <button type="submit" class="btn">新增</button>
      </form>
      <h2 class="admin-section-title">酒单列表（${items.length}）</h2>
      <div class="admin-menu-rows">${rows || '<div class="admin-empty">暂无酒单项</div>'}</div>
    </div>`;

  // 新增表单
  const newForm = content.querySelector("#menu-new-form") as HTMLFormElement;
  newForm.addEventListener("submit", async (e) => {
    e.preventDefault();
    const fd = new FormData(newForm);
    const section = (fd.get("section") as string).trim();
    const name = (fd.get("name") as string).trim();
    if (!section || !name) {
      showAdminError(content, "分类和名称必填");
      return;
    }
    const description = (fd.get("description") as string).trim();
    const price = parseInt(fd.get("price") as string, 10);
    const sort_order = parseInt(fd.get("sort_order") as string, 10);
    const visible = fd.get("visible") ? 1 : 0;
    const submitBtn = newForm.querySelector('button[type="submit"]') as HTMLButtonElement;
    submitBtn.disabled = true;
    try {
      await api.admin.createMenu({
        section,
        name,
        description: description || undefined,
        price: isNaN(price) ? undefined : price,
        sort_order: isNaN(sort_order) ? undefined : sort_order,
        visible,
      });
      await renderMenu(content);
    } catch (e) {
      showAdminError(content, (e as Error).message || "新增失败");
      submitBtn.disabled = false;
    }
  });

  // 每行 保存/删除
  content.querySelectorAll(".admin-menu-row").forEach((rowEl) => {
    const id = (rowEl as HTMLElement).dataset.id!;
    const saveBtn = rowEl.querySelector('[data-action="save"]') as HTMLButtonElement;
    const delBtn = rowEl.querySelector('[data-action="delete"]') as HTMLButtonElement;

    saveBtn.addEventListener("click", async () => {
      const fd = new FormData(rowEl as HTMLFormElement);
      const section = (fd.get("section") as string).trim();
      const name = (fd.get("name") as string).trim();
      if (!section || !name) {
        showAdminError(content, "分类和名称必填");
        return;
      }
      const description = (fd.get("description") as string).trim();
      const price = parseInt(fd.get("price") as string, 10);
      const sort_order = parseInt(fd.get("sort_order") as string, 10);
      const visible = fd.get("visible") ? 1 : 0;
      saveBtn.disabled = true;
      try {
        await api.admin.updateMenu(id, {
          section,
          name,
          // 必须回传 description：服务端缺失走默认""，会清空已有描述
          description: description || undefined,
          price: isNaN(price) ? undefined : price,
          sort_order: isNaN(sort_order) ? undefined : sort_order,
          visible,
        });
        showAdminError(content, "已保存");
        await renderMenu(content);
      } catch (e) {
        showAdminError(content, (e as Error).message || "保存失败");
        saveBtn.disabled = false;
      }
    });

    delBtn.addEventListener("click", async () => {
      delBtn.disabled = true;
      try {
        await api.admin.deleteMenu(id);
        await renderMenu(content);
      } catch (e) {
        showAdminError(content, (e as Error).message || "删除失败");
        delBtn.disabled = false;
      }
    });
  });
}
