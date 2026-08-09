// ui/ — 形象创建：上传照片（生成）/ 捏脸（modular）/ 使用默认。
// 异步 UX（非阻塞 library，2026-08-09 定稿，issue #0010）：提交生成后立即用默认/当前
// 形象入场游玩，后台轮询 job，完成时弹 toast 通知刷新——不在模态里干等 5–9 分钟。
import { api, type AvatarData, type ModularAvatar } from "../net/api";
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

/**
 * 后台轮询生成 job，完成/失败时弹一个固定 toast（非阻塞，不挡游玩）。
 * 完成后 worker 已把生成形象存进 avatars.config_json；用户点「刷新」应用。
 * status 对齐后端 DB 值：pending/running/done/failed。
 */
function pollAvatarInBackground(jobId: string): void {
  const toast = document.createElement("div");
  toast.style.cssText =
    "position:fixed;right:12px;bottom:12px;z-index:20;max-width:260px;" +
    "background:rgba(20,16,14,.92);border:2px solid #d4a24e;color:#e8dcc8;" +
    "font:11px 'Courier New',ui-monospace,monospace;padding:8px 10px;border-radius:4px;";
  toast.textContent = "形象生成中…（可继续游玩）";
  document.body.appendChild(toast);

  const start = Date.now();
  const tick = async (): Promise<void> => {
    let finished = false;
    try {
      const st = await api.pollAvatarJob(jobId);
      if (st.status === "done") {
        toast.textContent = "✓ 形象生成完成";
        const btn = document.createElement("button");
        btn.textContent = "刷新应用";
        btn.style.cssText =
          "margin-left:8px;border:1px solid #d4a24e;background:transparent;color:#d4a24e;" +
          "font:inherit;padding:1px 6px;cursor:pointer;";
        btn.onclick = () => location.reload();
        toast.appendChild(btn);
        finished = true;
      } else if (st.status === "failed") {
        toast.textContent = "形象生成失败：" + (st.error ?? "未知");
        finished = true;
      } else {
        // pending / running
        toast.textContent = `形象生成中…（${Math.round((Date.now() - start) / 1000)}s）`;
      }
    } catch {
      /* 网络瞬断，继续轮询 */
    }
    if (!finished) setTimeout(() => void tick(), POLL_INTERVAL);
  };
  void tick();
}
