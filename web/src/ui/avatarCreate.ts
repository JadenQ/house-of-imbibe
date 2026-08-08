// ui/ — 形象创建：上传照片 → PixelLab 4方向生成
import { api, type AvatarData, type ModularAvatar } from "../net/api";

const POLL_INTERVAL = 3000; // 3s 轮询

/** 默认模块化配色（fallback） */
const DEFAULT_COLORS: ModularAvatar = {
  kind: "modular",
  skin: "#f0c8a0",
  hair: "#503018",
  shirt: "#3868b0",
  pants: "#404048",
};

export function showAvatarCreate(initial: AvatarData | null): Promise<AvatarData> {
  return new Promise((resolve) => {
    const ui = document.getElementById("ui")!;

    ui.innerHTML = `
      <div class="overlay"><div class="panel avatar-panel">
        <h1>捏 个 人</h1>
        <h2>上传一张照片，生成你的像素形象</h2>

        <div class="av-upload" id="av-upload">
          <input type="file" id="av-photo" accept="image/jpeg,image/png,image/webp,image/gif" />
          <div class="av-upload-hint" id="av-upload-hint">点击或拖拽照片到这里</div>
          <div class="av-upload-file" id="av-file-name"></div>
        </div>

        <div class="av-progress" id="av-progress" style="display:none">
          <div class="av-progress-bar"><div class="av-progress-fill" id="av-fill"></div></div>
          <div class="av-progress-text" id="av-progress-text">生成中…</div>
        </div>

        <div class="av-preview" id="av-preview" style="display:none">
          <div class="av-preview-label">你的像素形象</div>
          <div class="av-preview-dirs" id="av-preview-dirs"></div>
        </div>

        <div class="err" id="av-err"></div>

        <button class="btn" id="av-go" disabled>生 成 形 象</button>
        <button class="btn ghost" id="av-skip">使 用 默 认 形 象</button>
      </div></div>`;

    const upload = document.getElementById("av-upload")!;
    const fileInput = document.getElementById("av-photo") as HTMLInputElement;
    const fileName = document.getElementById("av-file-name")!;
    const goBtn = document.getElementById("av-go") as HTMLButtonElement;
    const skipBtn = document.getElementById("av-skip") as HTMLButtonElement;
    const errEl = document.getElementById("av-err")!;
    const progressEl = document.getElementById("av-progress")!;
    const progressText = document.getElementById("av-progress-text")!;
    const fillEl = document.getElementById("av-fill")!;
    const previewEl = document.getElementById("av-preview")!;
    const previewDirs = document.getElementById("av-preview-dirs")!;

    let pickedFile: File | null = null;

    // 文件选择
    upload.onclick = () => fileInput.click();
    fileInput.onchange = () => {
      if (fileInput.files?.[0]) onFile(fileInput.files[0]);
    };

    // 拖拽
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

    // 生成
    goBtn.onclick = async () => {
      if (!pickedFile) return;
      goBtn.disabled = true;
      skipBtn.disabled = true;
      errEl.textContent = "";
      progressEl.style.display = "block";
      previewEl.style.display = "none";
      progressText.textContent = "正在上传…";

      try {
        const { job_id } = await api.generateAvatar(pickedFile);
        progressText.textContent = "生成中…（约 2 分钟）";
        fillEl.style.width = "30%";

        const startTime = Date.now();
        const poll = async (): Promise<void> => {
          const status = await api.pollAvatarJob(job_id);
          const elapsed = Math.round((Date.now() - startTime) / 1000);

          if (status.status === "completed") {
            fillEl.style.width = "100%";
            progressText.textContent = `完成！用时 ${elapsed} 秒`;

            // 重新获取用户信息以拿到生成的 avatar
            const me = await api.me();
            if (me.avatar && me.avatar.kind === "generated") {
              showPreview(me.avatar);
              // 自动保存完成，确认后 resolve
              showConfirm(me.avatar);
            } else {
              errEl.textContent = "生成完成但未找到形象数据";
              goBtn.disabled = false;
              skipBtn.disabled = false;
            }
            return;
          }

          if (status.status === "failed") {
            progressEl.style.display = "none";
            errEl.textContent = status.error ?? "生成失败，请重试";
            goBtn.disabled = false;
            skipBtn.disabled = false;
            return;
          }

          // 仍在处理中
          fillEl.style.width = `${Math.min(90, 30 + elapsed / 2)}%`;
          progressText.textContent = `生成中…（已等待 ${elapsed} 秒）`;
          await new Promise((r) => setTimeout(r, POLL_INTERVAL));
          await poll();
        };

        await poll();
      } catch (e) {
        progressEl.style.display = "none";
        errEl.textContent = (e as Error).message;
        goBtn.disabled = false;
        skipBtn.disabled = false;
      }
    };

    function showPreview(avatar: AvatarData) {
      if (avatar.kind !== "generated") return;
      previewEl.style.display = "block";
      const dirNames: Record<string, string> = { south: "正面", north: "背面", west: "左", east: "右" };
      previewDirs.innerHTML = avatar.rotations
        .map(
          (r) =>
            `<div class="av-dir-cell">
              <img src="${r.url}" alt="${r.direction}" />
              <div class="av-dir-label">${dirNames[r.direction] ?? r.direction}</div>
            </div>`,
        )
        .join("");
    }

    function showConfirm(avatar: AvatarData) {
      goBtn.textContent = "就 这 样 了";
      goBtn.disabled = false;
      goBtn.onclick = () => {
        ui.innerHTML = "";
        resolve(avatar);
      };
    }

    // 跳过：使用默认配色
    skipBtn.onclick = async () => {
      const fallback = initial ?? DEFAULT_COLORS;
      if (fallback.kind === "modular") {
        try {
          await api.saveAvatar(fallback);
        } catch {
          /* 静默 */
        }
      }
      ui.innerHTML = "";
      resolve(fallback);
    };
  });
}
