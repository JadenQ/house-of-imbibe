// ui/ — 形象创建：上传照片（生成）/ 捏脸（modular）/ 使用默认。
// 异步 UX（非阻塞 library，2026-08-09 定稿，issue #0010）：提交生成后立即用默认/当前
// 形象入场游玩，后台轮询 job，完成时弹 toast 通知刷新——不在模态里干等 5–9 分钟。
//
// 生成中心面板（2026-08-10）：toast 上的「查看全部生成」入口打开 DOM overlay，
// 列出当前用户所有 job（kind/status/时间/done预览/failed错误），3s 自动刷新直到无 pending/running。
import {
  api,
  type AvatarData,
  type ModularAvatar,
  type AvatarJob,
  type GeneratedAvatar,
} from "../net/api";
import { renderAvatarBuilder, saveModularLocal } from "./avatarBuilder";

const POLL_INTERVAL = 3000; // 3s 轮询

/** 默认模块化配色（fallback） */
const DEFAULT_COLORS: ModularAvatar = {
  kind: "modular",
  skin: "#f0c8a0",
  hair: "#503018",
  shirt: "#3868b0",
  pants: "#404048",
  shoes: "#201510",
};

export function showAvatarCreate(initial: AvatarData | null): Promise<AvatarData> {
  return new Promise((resolve) => {
    showChoose();

    function showChoose() {
      const ui = document.getElementById("ui")!;
      ui.innerHTML = `
        <div class="overlay"><div class="panel avatar-panel">
          <h1>捏 个 人</h1>
          <h2>上传照片生成，或自选捏脸</h2>
          <div class="av-upload" id="av-upload">
            <input type="file" id="av-photo" accept="image/jpeg,image/png,image/webp,image/gif" />
            <div class="av-upload-hint" id="av-upload-hint">点击或拖拽照片到这里</div>
            <div class="av-upload-file" id="av-file-name"></div>
          </div>
          <div class="err" id="av-err"></div>
          <button class="btn" id="av-go" disabled>生 成 形 象</button>
          <button class="btn ghost" id="av-build">捏 脸（自 选 样 式）</button>
          <button class="btn ghost" id="av-text">文 字 描 述</button>
          <button class="btn ghost" id="av-skip">使 用 默 认 形 象</button>
        </div></div>`;

      const upload = document.getElementById("av-upload")!;
      const fileInput = document.getElementById("av-photo") as HTMLInputElement;
      const fileName = document.getElementById("av-file-name")!;
      const goBtn = document.getElementById("av-go") as HTMLButtonElement;
      const buildBtn = document.getElementById("av-build") as HTMLButtonElement;
      const textBtn = document.getElementById("av-text") as HTMLButtonElement;
      const skipBtn = document.getElementById("av-skip") as HTMLButtonElement;
      const errEl = document.getElementById("av-err")!;
      let pickedFile: File | null = null;

      upload.onclick = () => fileInput.click();
      fileInput.onchange = () => {
        if (fileInput.files?.[0]) onFile(fileInput.files[0]);
      };
      upload.addEventListener("dragenter", (e) => { e.preventDefault(); upload.classList.add("dragover"); });
      upload.addEventListener("dragover", (e) => { e.preventDefault(); upload.classList.add("dragover"); });
      upload.addEventListener("dragleave", () => upload.classList.remove("dragover"));
      upload.addEventListener("drop", (e) => {
        e.preventDefault();
        upload.classList.remove("dragover");
        if (e.dataTransfer?.files[0]) onFile(e.dataTransfer.files[0]);
      });

      function onFile(f: File) {
        pickedFile = f;
        fileName.textContent = `${f.name}  (${Math.round(f.size / 1024)} KB)`;
        goBtn.disabled = false;
        errEl.textContent = "";
        document.getElementById("av-upload-hint")!.textContent = "已选择照片（点击更换）";
      }

      // 生成：非阻塞——提交后立即用默认/当前形象入场，后台轮询；完成时 toast 通知刷新。
      goBtn.onclick = async () => {
        if (!pickedFile) return;
        goBtn.disabled = true;
        skipBtn.disabled = true;
        buildBtn.disabled = true;
        errEl.textContent = "";
        try {
          const { job_id } = await api.generateAvatar(pickedFile);
          ui.innerHTML = "";
          resolve(initial ?? DEFAULT_COLORS); // 立即入场（默认/当前形象）
          pollAvatarInBackground(job_id); // 后台轮询，完成时 toast
        } catch (e) {
          errEl.textContent = (e as Error).message;
          goBtn.disabled = false;
          skipBtn.disabled = false;
          buildBtn.disabled = false;
        }
      };

      // 捏脸（modular）入口
      buildBtn.onclick = () => {
        renderAvatarBuilder({
          initial: initial?.kind === "modular" ? initial : null,
          onSave: (cfg) => {
            ui.innerHTML = "";
            resolve(cfg);
          },
          onBack: () => showChoose(),
        });
      };

      // 文字描述（2c）入口
      textBtn.onclick = () => showTextEntry();

      // 跳过：使用默认配色
      skipBtn.onclick = async () => {
        const fallback = initial ?? DEFAULT_COLORS;
        if (fallback.kind === "modular") {
          try {
            await api.saveAvatar(fallback);
            saveModularLocal(fallback);
          } catch {
            /* 静默 */
          }
        }
        ui.innerHTML = "";
        resolve(fallback);
      };
    }

    // 文字描述（2c）：写描述 → 非阻塞生成（复用 pollAvatarInBackground）
    function showTextEntry() {
      const ui = document.getElementById("ui")!;
      ui.innerHTML = `
        <div class="overlay"><div class="panel avatar-panel">
          <h1>捏 个 人</h1>
          <h2>用文字描述你的形象</h2>
          <textarea id="av-desc" rows="4" maxlength="2000" placeholder="例：穿绿铠甲的骑士，短发，手持长剑…（≤2000 字）" style="width:100%;box-sizing:border-box;background:#14100e;color:#e8dcc8;border:2px solid #4a3826;font:inherit;font-size:12px;padding:6px;outline:none;resize:vertical"></textarea>
          <div class="err" id="av-err"></div>
          <button class="btn" id="av-go-text">生 成 形 象</button>
          <button class="btn ghost" id="av-back">返 回</button>
        </div></div>`;
      const descEl = document.getElementById("av-desc") as HTMLTextAreaElement;
      const errEl = document.getElementById("av-err")!;
      const goBtn = document.getElementById("av-go-text") as HTMLButtonElement;
      document.getElementById("av-back")!.onclick = () => showChoose();
      goBtn.onclick = async () => {
        const d = descEl.value.trim();
        if (!d) {
          errEl.textContent = "写点描述吧";
          return;
        }
        goBtn.disabled = true;
        errEl.textContent = "";
        try {
          const { job_id } = await api.generateAvatarText(d);
          ui.innerHTML = "";
          resolve(initial ?? DEFAULT_COLORS); // 立即入场（默认/当前形象）
          pollAvatarInBackground(job_id); // 后台轮询，完成时 toast
        } catch (e) {
          errEl.textContent = (e as Error).message;
          goBtn.disabled = false;
        }
      };
    }
  });
}

// ── 后台轮询 + toast（非阻塞，不挡游玩）─────────────────────────────────────

/**
 * 后台轮询生成 job，完成/失败时弹一个固定 toast（非阻塞，不挡游玩）。
 * toast 上附「查看全部生成」链接 → 打开生成中心面板（showGenerationCenter）。
 * 完成后 worker 已把生成形象存进 avatars.config_json；用户点「刷新」应用。
 * status 对齐后端 DB 值：pending/running/done/failed。
 */
function pollAvatarInBackground(jobId: string): void {
  ensureGenCenterStyles();

  const toast = document.createElement("div");
  toast.className = "av-toast";

  const toastText = document.createElement("div");
  toastText.className = "av-toast-text";
  toastText.textContent = "形象生成中…（可继续游玩）";

  const toastLink = document.createElement("a");
  toastLink.className = "av-toast-link";
  toastLink.textContent = "查看全部生成";
  toastLink.href = "#";
  toastLink.onclick = (e: Event) => {
    e.preventDefault();
    showGenerationCenter();
  };

  toast.appendChild(toastText);
  toast.appendChild(toastLink);
  document.body.appendChild(toast);

  const start = Date.now();
  const tick = async (): Promise<void> => {
    let finished = false;
    try {
      const st = await api.pollAvatarJob(jobId);
      if (st.status === "done") {
        toastText.textContent = "✓ 形象生成完成";
        const btn = document.createElement("button");
        btn.textContent = "刷新应用";
        btn.className = "av-toast-btn";
        btn.onclick = () => location.reload();
        toast.appendChild(btn);
        finished = true;
      } else if (st.status === "failed") {
        toastText.textContent = "形象生成失败：" + (st.error ?? "未知");
        finished = true;
      } else {
        // pending / running
        toastText.textContent = `形象生成中…（${Math.round((Date.now() - start) / 1000)}s）`;
      }
    } catch {
      /* 网络瞬断，继续轮询 */
    }
    if (!finished) setTimeout(() => void tick(), POLL_INTERVAL);
  };
  void tick();
}

// ── 生成中心面板（DOM overlay）──────────────────────────────────────────────

/** 状态徽章信息：label + CSS class。status 对齐后端 DB 枚举。 */
const STATUS_INFO: Record<string, { label: string; cls: string }> = {
  pending: { label: "等待", cls: "gc-status-pending" },
  running: { label: "生成中", cls: "gc-status-running" },
  done: { label: "完成", cls: "gc-status-done" },
  failed: { label: "失败", cls: "gc-status-failed" },
  unknown: { label: "未知", cls: "gc-status-unknown" },
};

/** 4 方向预览顺序 + 中文标签。 */
const PREVIEW_DIRS: Array<{ key: "south" | "north" | "west" | "east"; label: string }> = [
  { key: "south", label: "前" },
  { key: "north", label: "后" },
  { key: "west", label: "左" },
  { key: "east", label: "右" },
];

/**
 * 生成中心面板：DOM overlay，列出当前用户所有形象生成 job。
 *
 * 每行显示 kind(图片/文字)、status 徽章(pending灰/running黄/done绿/failed红)、
 * 创建时间/耗时；done 的 job 显示 4 方向预览缩略图 + 应用按钮(location.reload)；
 * failed 显示错误（经 pollAvatarJob 取 error）。面板每 3s 自动刷新 jobs 列表，
 * 直到无 pending/running。
 *
 * 预览说明：后端每次成功生成都会覆盖 avatars.config_json（ON CONFLICT upsert），
 * 故当前 avatar 的 frames 只对应最近一个 done job。面板对最近的 done job 显示
 * 当前 avatar 的 4 方向预览，更早的 done job 标注「已被后续生成覆盖」。
 *
 * 当前由 pollAvatarInBackground toast 上的「查看全部生成」链接触发。
 * 待 main 接入 HUD 入口（showGenerationCenter 已导出，可直接绑 HUD 按钮）。
 */
export function showGenerationCenter(): void {
  ensureGenCenterStyles();

  // 若已打开，先移除旧的
  const existing = document.getElementById("gen-center");
  if (existing) existing.remove();

  const overlay = document.createElement("div");
  overlay.id = "gen-center";
  overlay.className = "gc-overlay";
  overlay.innerHTML = `
    <div class="gc-panel">
      <div class="gc-head">
        <h1>生成中心</h1>
        <button class="btn ghost gc-close" id="gc-close">✕ 关闭</button>
      </div>
      <div class="gc-body" id="gc-body">
        <div class="gc-loading">加载中…</div>
      </div>
      <div class="gc-foot">
        <span class="gc-hint" id="gc-hint"></span>
      </div>
    </div>`;
  document.body.appendChild(overlay);

  const body = overlay.querySelector<HTMLElement>("#gc-body")!;
  const hint = overlay.querySelector<HTMLElement>("#gc-hint")!;
  const closeBtn = overlay.querySelector<HTMLButtonElement>("#gc-close")!;

  let interval: ReturnType<typeof setInterval> | null = null;
  let closed = false;

  const cleanup = (): void => {
    closed = true;
    if (interval) {
      clearInterval(interval);
      interval = null;
    }
    overlay.remove();
  };

  closeBtn.onclick = cleanup;
  overlay.addEventListener("click", (e: MouseEvent) => {
    if (e.target === overlay) cleanup();
  });
  const escHandler = (e: KeyboardEvent): void => {
    if (e.key === "Escape") {
      cleanup();
      document.removeEventListener("keydown", escHandler);
    }
  };
  document.addEventListener("keydown", escHandler);

  /**
   * 刷新：并行拉取 jobs 列表 + 当前 avatar（用于 done 预览），重渲染面板。
   * 无 pending/running 时停止自动刷新（面板仍保持打开，用户手动关闭）。
   */
  const refresh = async (): Promise<void> => {
    if (closed) return;
    try {
      const [jobs, me] = await Promise.all([api.listAvatarJobs(), api.me()]);
      const av = me.avatar;
      const currentAvatar: GeneratedAvatar | null =
        av && av.kind === "generated" ? av : null;
      renderJobs(jobs, currentAvatar);
    } catch (e) {
      if (!closed) {
        body.innerHTML = `<div class="gc-empty">加载失败：${escapeHtml((e as Error).message)}</div>`;
      }
    }
  };

  /** 渲染 job 列表。jobs 按 created_at DESC（后端排序）。 */
  function renderJobs(jobs: AvatarJob[], currentAvatar: GeneratedAvatar | null): void {
    if (jobs.length === 0) {
      body.innerHTML = `<div class="gc-empty">还没有生成记录</div>`;
      hint.textContent = "";
      return;
    }

    const hasActive = jobs.some(
      (j) => j.status === "pending" || j.status === "running",
    );
    const now = Date.now();
    let doneCount = 0;

    body.innerHTML = jobs
      .map((job) => {
        // kind: params_json.mode === "text" → 文字，否则 图片（photo 模式无 mode 字段）
        const mode = job.params_json?.mode;
        const kindLabel = typeof mode === "string" && mode === "text" ? "文字" : "图片";
        const statusInfo = STATUS_INFO[job.status] ?? STATUS_INFO.unknown;
        const timeStr = formatTime(new Date(job.created_at * 1000));

        // pending/running 显示耗时（实时秒数）；done/failed 只显示创建时间
        let elapsed = "";
        if (job.status === "pending" || job.status === "running") {
          const secs = Math.max(0, Math.round((now - job.created_at * 1000) / 1000));
          elapsed = ` · ${secs}s`;
        }

        let detailHtml = "";
        if (job.status === "done") {
          // 最近 done job（DESC 第一个）= 当前 avatar；更早的 done 已被覆盖
          if (doneCount === 0 && currentAvatar) {
            detailHtml = renderPreview(currentAvatar);
          } else if (doneCount > 0) {
            detailHtml = `<span class="gc-overwritten">已被后续生成覆盖</span>`;
          }
          detailHtml += `<button class="btn gc-apply">应用</button>`;
          doneCount++;
        } else if (job.status === "failed") {
          detailHtml =
            `<span class="gc-failed-msg" data-job-id="${escapeAttr(job.id)}">载入错误…</span>`;
        }

        return `
          <div class="gc-row">
            <div class="gc-row-main">
              <span class="gc-kind">${kindLabel}</span>
              <span class="gc-status ${statusInfo.cls}">${statusInfo.label}</span>
              <span class="gc-time">${timeStr}${elapsed}</span>
            </div>
            ${detailHtml ? `<div class="gc-row-detail">${detailHtml}</div>` : ""}
          </div>`;
      })
      .join("");

    // 应用按钮：reload 后 boot 流程从 /api/me 取当前 avatar 入场
    body.querySelectorAll<HTMLButtonElement>(".gc-apply").forEach((btn) => {
      btn.onclick = () => location.reload();
    });

    // failed job：经 pollAvatarJob 取 error 文本（poll 返回 {status,error}）
    body.querySelectorAll<HTMLElement>(".gc-failed-msg").forEach((el) => {
      const jid = el.dataset.jobId!;
      void api.pollAvatarJob(jid).then(
        (st) => { el.textContent = st.error ?? "未知错误"; },
        () => { el.textContent = "无法获取错误信息"; },
      );
    });

    hint.textContent = hasActive ? "自动刷新中（3s）…" : "无进行中的任务";

    // 无 pending/running → 停止自动刷新（面板仍打开）
    if (!hasActive && interval) {
      clearInterval(interval);
      interval = null;
    }
  }

  // 首次加载 + 启动 3s 自动刷新
  void refresh();
  interval = setInterval(() => void refresh(), POLL_INTERVAL);
}

// ── 辅助函数 ──────────────────────────────────────────────────────────────────

/** 渲染 generated avatar 的 4 方向预览缩略图（每方向取第一帧 = 静站）。 */
function renderPreview(av: GeneratedAvatar): string {
  const thumbs = PREVIEW_DIRS.map((d) => {
    const frameKey = av.frames[d.key]?.[0];
    const img = frameKey
      ? `<img class="gc-thumb" src="/api/assets/${encodeURIComponent(frameKey)}" alt="${d.label}方向" />`
      : `<span class="gc-thumb-placeholder">—</span>`;
    return `<div class="gc-dir-wrap"><span class="gc-dir-label">${d.label}</span>${img}</div>`;
  }).join("");
  return `<div class="gc-preview">${thumbs}</div>`;
}

/** 时间格式化：MM-DD HH:mm（created_at 是 Unix 秒）。 */
function formatTime(d: Date): string {
  const pad = (n: number): string => n.toString().padStart(2, "0");
  return `${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

/** HTML 文本转义（防注入，用于 error 消息等动态文本）。 */
function escapeHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

/** HTML 属性值转义（用于 data-* 属性中的 job id）。 */
function escapeAttr(s: string): string {
  return s.replace(/&/g, "&amp;").replace(/"/g, "&quot;");
}

// ── 注入样式（只注入一次）─────────────────────────────────────────────────────

/** 注入 toast + 生成中心面板样式（参考 avatarBuilder.ts ensureBuilderStyles 模式）。 */
function ensureGenCenterStyles(): void {
  if (document.getElementById("gen-center-styles")) return;
  const style = document.createElement("style");
  style.id = "gen-center-styles";
  style.textContent = `
  /* ── toast（pollAvatarInBackground）── */
  .av-toast {
    position: fixed; right: 12px; bottom: 12px; z-index: 30;
    max-width: 280px; background: rgba(20,16,14,.94);
    border: 2px solid #d4a24e; color: #e8dcc8;
    font: 11px 'Courier New',ui-monospace,monospace;
    padding: 8px 10px; border-radius: 4px;
    display: flex; flex-direction: column; gap: 6px;
  }
  .av-toast-text { line-height: 1.4; }
  .av-toast-link {
    color: #d4a24e; text-decoration: underline; cursor: pointer;
    font-size: 10px; align-self: flex-start;
  }
  .av-toast-link:hover { color: #e8c068; }
  .av-toast-btn {
    margin-top: 2px; border: 1px solid #d4a24e; background: transparent;
    color: #d4a24e; font: inherit; padding: 3px 8px; cursor: pointer;
    border-radius: 2px; align-self: flex-start;
  }
  .av-toast-btn:hover { background: rgba(212,162,78,.15); }

  /* ── 生成中心面板（showGenerationCenter）── */
  .gc-overlay {
    position: fixed; inset: 0; z-index: 40;
    background: rgba(10,7,5,.7);
    display: flex; align-items: center; justify-content: center;
    padding: 12px;
  }
  .gc-panel {
    width: 100%; max-width: 480px; max-height: 85vh;
    background: #1a140f; border: 2px solid #4a3826; border-radius: 4px;
    display: flex; flex-direction: column; overflow: hidden;
  }
  .gc-head {
    display: flex; align-items: center; justify-content: space-between;
    padding: 10px 14px; border-bottom: 1px solid #4a3826; gap: 12px;
  }
  .gc-head h1 { margin: 0; font-size: 14px; color: #d4a24e; }
  .gc-close { width: auto; padding: 4px 10px; margin-top: 0; font-size: 11px; }
  .gc-body {
    flex: 1; overflow-y: auto; padding: 8px 10px;
    -webkit-overflow-scrolling: touch;
  }
  .gc-row {
    padding: 8px; margin-bottom: 6px;
    background: #14100e; border: 1px solid #2a2218; border-radius: 3px;
  }
  .gc-row-main {
    display: flex; align-items: center; gap: 8px; flex-wrap: wrap;
  }
  .gc-kind {
    font-size: 10px; padding: 2px 6px; border-radius: 2px;
    background: #2a2218; color: #c8b898;
  }
  .gc-status {
    font-size: 10px; padding: 2px 6px; border-radius: 2px; font-weight: bold;
  }
  .gc-status-pending { background: #2a2218; color: #9a8a70; }
  .gc-status-running { background: #3a3018; color: #d4a24e; }
  .gc-status-done { background: #1a3020; color: #48a058; }
  .gc-status-failed { background: #3a1818; color: #c84028; }
  .gc-status-unknown { background: #2a2218; color: #9a8a70; }
  .gc-time { font-size: 10px; color: #7a6a50; margin-left: auto; }
  .gc-row-detail {
    margin-top: 6px; display: flex; align-items: center; gap: 10px; flex-wrap: wrap;
  }
  .gc-preview { display: flex; gap: 6px; flex-wrap: wrap; }
  .gc-dir-wrap {
    display: flex; flex-direction: column; align-items: center; gap: 2px;
  }
  .gc-thumb {
    width: 32px; height: 32px; image-rendering: pixelated;
    border: 1px solid #4a3826; background: #0a0705; display: block;
  }
  .gc-thumb-placeholder {
    display: inline-flex; align-items: center; justify-content: center;
    width: 32px; height: 32px; color: #5a4a30; font-size: 10px;
    border: 1px solid #4a3826; box-sizing: border-box;
  }
  .gc-dir-label { font-size: 9px; color: #7a6a50; }
  .gc-apply {
    width: auto; padding: 4px 12px; margin-top: 0; font-size: 11px;
  }
  .gc-overwritten { font-size: 10px; color: #7a6a50; }
  .gc-failed-msg {
    font-size: 10px; color: #c84028; word-break: break-all;
  }
  .gc-loading, .gc-empty {
    text-align: center; padding: 24px 8px; color: #7a6a50; font-size: 12px;
  }
  .gc-foot { padding: 6px 14px; border-top: 1px solid #4a3826; }
  .gc-hint { font-size: 10px; color: #5a4a30; }
  @media (max-width: 420px) {
    .gc-panel { max-width: 96vw; max-height: 80vh; }
    .gc-thumb, .gc-thumb-placeholder { width: 28px; height: 28px; }
  }`;
  document.head.appendChild(style);
}

// 待 main 接入 HUD 入口：showGenerationCenter 已导出，
// 可在 main.ts startGame 里绑定 HUD 按钮（如 hud-edit 旁加「生成中心」）调用。
