// ui/avatarBuilder.ts — 捏脸 UI（切片 2a modular）。
// 逐项选样式 + 选颜色，实时预览（canvas 用 characterSheet 渲染当前选择，含方向切换）。
// 保存 → PUT /api/avatar（modular）。DOM/CSS 实现，不进 Phaser，移动端横屏优先。
//
// 已知限制：后端 put_avatar（src/lib.rs）把 config 规范化为 {kind,skin,hair,shirt,pants}，
// 会丢弃样式字段。样式不落服务端 → 远端玩家看到默认样式。本文件用 localStorage 兜底：
// 本机保存完整 modular 配置，boot 时按颜色匹配回灌样式，使本机捏脸跨刷新仍可见。
import {
  characterSheet,
  DIRS,
  FRAME_W,
  FRAME_H,
  HAIR_STYLES,
  TOP_STYLES,
  BOTTOM_STYLES,
  SHOE_STYLES,
  type Dir,
} from "../game/character";
import { api, type ModularAvatar } from "../net/api";

// ── localStorage 兜底（后端不持久化样式字段）──
const LS_KEY = "hoi:modular-avatar";

/** 把完整 modular 配置（含样式）写入本机 localStorage。 */
export function saveModularLocal(cfg: ModularAvatar): void {
  try {
    localStorage.setItem(LS_KEY, JSON.stringify(cfg));
  } catch {
    /* 隐私模式 / 配额 → 静默 */
  }
}

/** 用本机保存的样式回灌服务端返回的（被规范化的）modular 配置。
 *  仅当四色完全匹配（同一用户上次保存）才回灌，避免多账号串样式。 */
export function loadModularLocal(serverCfg: ModularAvatar): ModularAvatar {
  try {
    const raw = localStorage.getItem(LS_KEY);
    if (!raw) return serverCfg;
    const saved = JSON.parse(raw) as Partial<ModularAvatar>;
    if (saved.kind !== "modular") return serverCfg;
    if (
      saved.skin === serverCfg.skin &&
      saved.hair === serverCfg.hair &&
      saved.shirt === serverCfg.shirt &&
      saved.pants === serverCfg.pants
    ) {
      return { ...serverCfg, ...saved };
    }
  } catch {
    /* 静默 */
  }
  return serverCfg;
}

// ── 调色板 ──
const PALETTE = {
  skin: ["#f0c8a0", "#e0b088", "#c89060", "#a06840", "#704830", "#ffe0c0"],
  hair: ["#503018", "#8a5028", "#d8a050", "#1a1008", "#909098", "#c84028", "#e8e0d0"],
  shirt: ["#3868b0", "#b03838", "#38a058", "#d4a24e", "#7048b0", "#e0e0e0", "#282830"],
  pants: ["#404048", "#283040", "#503028", "#284030", "#604830", "#1a1a22"],
  shoes: ["#201510", "#604030", "#888888", "#d83030", "#e0e0e0", "#3868b0"],
} as const;

const STYLE_LABELS: Record<string, string> = {
  short: "短发", long: "长发", bald: "光头", cap: "鸭舌帽",
  tshirt: "T恤", longsleeve: "长袖", vest: "马甲",
  pants: "长裤", shorts: "短裤", skirt: "裙子",
  boots: "靴子", sneakers: "球鞋", sandals: "凉鞋",
};

const DIR_LABELS: Record<Dir, string> = { s: "正", n: "背", w: "左", e: "右" };

const DEFAULT_CFG: ModularAvatar = {
  kind: "modular",
  skin: "#f0c8a0",
  hair: "#503018",
  shirt: "#3868b0",
  pants: "#404048",
  shoes: "#201510",
};

interface BuilderOpts {
  initial: ModularAvatar | null;
  onSave: (cfg: ModularAvatar) => void;
  onBack: () => void;
}

/** 渲染捏脸面板到 #ui。回调式（便于嵌入 avatarCreate 子视图导航）。 */
export function renderAvatarBuilder(opts: BuilderOpts): void {
  const ui = document.getElementById("ui")!;
  ensureBuilderStyles();

  const init = opts.initial ?? DEFAULT_CFG;
  const cfg: ModularAvatar = { ...DEFAULT_CFG, ...init };

  ui.innerHTML = `
    <div class="overlay"><div class="panel builder-panel">
      <div class="bdr-head">
        <h1>捏 脸</h1>
        <button class="btn ghost bdr-back" id="bdr-back">← 返回</button>
      </div>

      <div class="bdr-body">
        <div class="bdr-preview-col">
          <canvas class="bdr-canvas" id="bdr-canvas" width="64" height="64"></canvas>
          <div class="bdr-dirs" id="bdr-dirs"></div>
          <div class="bdr-walk">
            <label class="bdr-check"><input type="checkbox" id="bdr-walk-toggle" checked /> 行走预览</label>
          </div>
        </div>

        <div class="bdr-opts" id="bdr-opts"></div>
      </div>

      <div class="err" id="bdr-err"></div>
      <button class="btn" id="bdr-save">保 存 形 象</button>
    </div></div>`;

  const canvas = document.getElementById("bdr-canvas") as HTMLCanvasElement;
  const pctx = canvas.getContext("2d")!;
  pctx.imageSmoothingEnabled = false;

  const optsEl = document.getElementById("bdr-opts")!;
  const dirsEl = document.getElementById("bdr-dirs")!;
  const errEl = document.getElementById("bdr-err")!;
  const saveBtn = document.getElementById("bdr-save") as HTMLButtonElement;
  const backBtn = document.getElementById("bdr-back") as HTMLButtonElement;
  const walkToggle = document.getElementById("bdr-walk-toggle") as HTMLInputElement;

  let dir: Dir = "s";
  let walkFrame = 0;
  let walking = true;
  let timer: ReturnType<typeof setInterval> | null = null;

  // ── 方向按钮 ──
  dirsEl.innerHTML = DIRS.map(
    (d) => `<button class="bdr-dir${d === dir ? " sel" : ""}" data-dir="${d}">${DIR_LABELS[d]}</button>`,
  ).join("");
  dirsEl.addEventListener("click", (e) => {
    const t = (e.target as HTMLElement).closest("[data-dir]") as HTMLElement | null;
    if (!t) return;
    dir = t.dataset.dir as Dir;
    dirsEl.querySelectorAll(".bdr-dir").forEach((b) => b.classList.toggle("sel", b === t));
    renderPreview();
  });

  // ── 样式 + 颜色选项行 ──
  const styleRows: { key: keyof Pick<ModularAvatar, "hairStyle" | "topStyle" | "bottomStyle" | "shoeStyle">; label: string; options: readonly string[] }[] = [
    { key: "hairStyle", label: "发型", options: HAIR_STYLES },
    { key: "topStyle", label: "上衣", options: TOP_STYLES },
    { key: "bottomStyle", label: "下装", options: BOTTOM_STYLES },
    { key: "shoeStyle", label: "鞋子", options: SHOE_STYLES },
  ];

  optsEl.innerHTML = styleRows
    .map(
      (r) => `
      <div class="bdr-row">
        <span class="bdr-row-label">${r.label}</span>
        <div class="bdr-pills">
          ${r.options
            .map(
              (o) =>
                `<button class="bdr-pill${cfg[r.key] === o ? " sel" : ""}" data-style="${r.key}" data-val="${o}">${STYLE_LABELS[o] ?? o}</button>`,
            )
            .join("")}
        </div>
      </div>`,
    )
    .join("");

  const colorRows: { key: keyof Pick<ModularAvatar, "skin" | "hair" | "shirt" | "pants" | "shoes">; label: string; colors: readonly string[] }[] = [
    { key: "skin", label: "肤色", colors: PALETTE.skin },
    { key: "hair", label: "发色", colors: PALETTE.hair },
    { key: "shirt", label: "衣色", colors: PALETTE.shirt },
    { key: "pants", label: "裤色", colors: PALETTE.pants },
    { key: "shoes", label: "鞋色", colors: PALETTE.shoes },
  ];

  optsEl.innerHTML += `<div class="bdr-sep"></div>` + colorRows
    .map(
      (r) => `
      <div class="bdr-row">
        <span class="bdr-row-label">${r.label}</span>
        <div class="bdr-swatches">
          ${r.colors
            .map(
              (c) =>
                `<button class="swatch bdr-sw${cfg[r.key] === c ? " sel" : ""}" data-color="${r.key}" data-val="${c}" style="background:${c}"></button>`,
            )
            .join("")}
        </div>
      </div>`,
    )
    .join("");

  // ── 样式选择 ──
  optsEl.addEventListener("click", (e) => {
    const pill = (e.target as HTMLElement).closest("[data-style]") as HTMLElement | null;
    if (pill) {
      const k = pill.dataset.style as "hairStyle" | "topStyle" | "bottomStyle" | "shoeStyle";
      cfg[k] = pill.dataset.val!;
      pill.parentElement!.querySelectorAll(".bdr-pill").forEach((b) => b.classList.toggle("sel", b === pill));
      renderPreview();
      return;
    }
    const sw = (e.target as HTMLElement).closest("[data-color]") as HTMLElement | null;
    if (sw) {
      const k = sw.dataset.color as "skin" | "hair" | "shirt" | "pants" | "shoes";
      cfg[k] = sw.dataset.val!;
      sw.parentElement!.querySelectorAll(".bdr-sw").forEach((b) => b.classList.toggle("sel", b === sw));
      renderPreview();
    }
  });

  // ── 预览渲染 ──
  function renderPreview() {
    const sheet = characterSheet(cfg);
    pctx.clearRect(0, 0, canvas.width, canvas.height);
    const row = DIRS.indexOf(dir);
    const sx = walkFrame * FRAME_W;
    const sy = row * FRAME_H;
    const scale = Math.floor(canvas.width / FRAME_W); // 4
    pctx.drawImage(sheet, sx, sy, FRAME_W, FRAME_H, 0, 0, FRAME_W * scale, FRAME_H * scale);
  }

  // ── 行走动画循环 ──
  function startWalk() {
    stopWalk();
    timer = setInterval(() => {
      if (walking) {
        walkFrame = (walkFrame + 1) % 3;
        renderPreview();
      }
    }, 220);
  }
  function stopWalk() {
    if (timer) {
      clearInterval(timer);
      timer = null;
    }
  }
  walkToggle.addEventListener("change", () => {
    walking = walkToggle.checked;
    if (!walking) {
      walkFrame = 0;
      renderPreview();
    }
  });

  // ── 保存 / 返回 ──
  saveBtn.onclick = async () => {
    saveBtn.disabled = true;
    errEl.textContent = "";
    try {
      await api.saveAvatar(cfg);
      saveModularLocal(cfg);
      opts.onSave(cfg);
    } catch (e) {
      errEl.textContent = (e as Error).message;
      saveBtn.disabled = false;
    }
  };
  backBtn.onclick = () => {
    stopWalk();
    opts.onBack();
  };

  renderPreview();
  startWalk();
}

// ── 注入构建器样式（只注入一次）──
function ensureBuilderStyles() {
  if (document.getElementById("builder-styles")) return;
  const style = document.createElement("style");
  style.id = "builder-styles";
  style.textContent = `
  .builder-panel { min-width: 300px; max-width: 480px; padding: 16px 18px; }
  .bdr-head { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
  .bdr-head h1 { margin: 0; }
  .bdr-head .bdr-back { margin-top: 0; width: auto; padding: 6px 12px; }
  .bdr-body { display: flex; gap: 16px; margin: 14px 0; }
  .bdr-preview-col { display: flex; flex-direction: column; align-items: center; gap: 8px; flex-shrink: 0; }
  .bdr-canvas { width: 64px; height: 64px; image-rendering: pixelated; background: #14100e;
    border: 2px solid #4a3826; }
  .bdr-dirs { display: flex; gap: 4px; }
  .bdr-dir { width: 30px; height: 26px; background: #14100e; color: #c8b898;
    border: 2px solid #4a3826; font: inherit; font-size: 11px; cursor: pointer; }
  .bdr-dir.sel { border-color: var(--accent); color: var(--accent); }
  .bdr-walk { font-size: 11px; color: #9a8a70; }
  .bdr-check { display: flex; align-items: center; gap: 4px; cursor: pointer; }
  .bdr-opts { flex: 1; min-width: 0; }
  .bdr-row { display: flex; align-items: center; gap: 8px; margin: 6px 0; }
  .bdr-row-label { font-size: 11px; color: #c8b898; width: 32px; flex-shrink: 0; }
  .bdr-pills, .bdr-swatches { display: flex; gap: 4px; flex-wrap: wrap; }
  .bdr-pill { padding: 5px 8px; background: #14100e; color: #c8b898;
    border: 2px solid #4a3826; font: inherit; font-size: 11px; cursor: pointer; }
  .bdr-pill.sel { border-color: var(--accent); color: var(--accent); }
  .bdr-sw { width: 22px; height: 22px; }
  .bdr-sep { height: 1px; background: #4a3826; margin: 10px 0; }
  @media (max-width: 420px) {
    .builder-panel { max-width: 96vw; padding: 12px; }
    .bdr-body { flex-direction: column; align-items: center; }
    .bdr-opts { width: 100%; }
  }`;
  document.head.appendChild(style);
}
