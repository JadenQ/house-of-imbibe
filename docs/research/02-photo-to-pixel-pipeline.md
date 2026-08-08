# 用户照片 → 像素卡通头像转换管线（调研）

> **调研日期**：2026-07-27
> **状态**：**部分过期** —— PixelLab 与 Retro Diffusion 的定价/能力描述已在 2026-08-01
> 部分核实，见 [`../reference/pixellab-api.md`](../reference/pixellab-api.md)。
> SD/ControlNet/后处理部分**未核实**但结论仍适用。

## 一、图像 → 卡通/风格化（生成侧）

### 1. Stable Diffusion + ControlNet + IP-Adapter/InstantID（可控性最强）
- **Stable Diffusion 1.5 / SDXL** — 底座；SD 1.5 生态最成熟，SDXL 质量高但显存/成本翻倍
- **ControlNet** 关键分支：
  - `lineart` / `lineart_anime` — 保线条，最适合卡通化
  - `canny` — 边缘保留，快
  - `openpose` — 保姿态（生成全身立绘时必装）
  - `depth` — 层次感
  - 项目地址：https://github.com/lllyasviel/ControlNet-v1-1-nightly
- **InstantID**（人脸身份保持，单张参考图，无需 LoRA 训练）
  - https://github.com/InstantID/InstantID
  - Replicate: `zsxkib/instant-id`（`https://replicate.com/zsxkib/instant-id`），冷启动后 ~10-15s，单张约 $0.005-0.015
  - 优点：一张脸就能锁身份，跟 ControlNet 叠加可控风格；缺点：需较高 CFG 才够卡通
- **IP-Adapter / IP-Adapter-FaceID / IP-Adapter-FaceID-Plus-v2**
  - https://github.com/tencent-ailab/IP-Adapter
  - 比 InstantID 轻量，Face-ID 变体保脸更强；FaceID-Portrait 版对亚洲脸有偏差，需搭配 anime LoRA
- **PhotoMaker v2**（腾讯 ARC）
  - https://github.com/TencentARC/PhotoMaker
  - Replicate: `tencentarc/photomaker`（`https://replicate.com/tencentarc/photomaker`）
  - 优点：多张参考图更准；缺点：SDXL 底座，慢

**风格 LoRA / Checkpoint（关键，决定"GBA 味"是否到位）：**
- **Pixel Art XL** LoRA — https://civitai.com/models/120096/pixel-art-xl（SDXL 生态最主流）
- **All-In-One Pixel Model** — https://civitai.com/models/1866
- **PixelArtRedmond** — https://huggingface.co/artificialguybr/PixelArtRedmond
- **Pokemon Trainer Sprite** 类 LoRA（Civitai 搜 "pokemon trainer" / "gba sprite"），风格贴近绿宝石但质量参差
- Replicate 直接可跑：`fofr/sdxl-pixel-art`（`https://replicate.com/fofr/sdxl-pixel-art`）— 内嵌 pixel-art LoRA + 后处理量化

### 2. 传统卡通化 GAN（无需 Prompt，轻量，风格固定）
- **AnimeGANv3** — https://github.com/TachibanaYoshino/AnimeGANv3；ONNX 可导出，10MB 级，浏览器/服务端秒级；风格偏日式动画不是像素
- **White-box Cartoonization** (CVPR 2020) — https://github.com/SystemErrorWang/White-box-Cartoonization；温和卡通感，人脸辨识度保留最好
- **Cartoonizer API**（第三方托管）：DeepAI Toonify、Vance AI、Fotor — 便宜但风格锁死、无 GBA 味
- 缺点：**不像 GBA**；需再串一道像素化

### 3. 端到端 SaaS
- **PixelLab** — https://www.pixellab.ai/ ；主打 photo-to-sprite / character generator，支持 skeleton animation；有 API，**按次计费**（非"$10-30/月起"），风格接近 JRPG，**最省事**
  - ✅ 2026-08-01 核实的照片入口真实链路：`POST /v2/portrait-character-pro`
    （半身照 → 朝南全身 sprite，`result_size` 仅支持 16/32/48/64/128/160，约 30–80 秒）
    → `POST /v2/create-character-v3`（朝南图 → 8 方向）
    → `POST /v2/animate-character`（加行走动画，**按方向计费**）
  - ⚠️ 全链路**异步**，端到端 6–11 分钟，单角色约 $0.19–0.30
  - ❌ **`/v2/image-to-pixelart` 不是这条链路的入口** —— 它只做降采样像素化，
    产出不满足 `create-character-v3` 对"朝南站姿 sprite"的要求
  - 详见 [`../reference/pixellab-api.md`](../reference/pixellab-api.md) §五
- **Scenario.gg** — https://www.scenario.gg/ ；游戏美术专用 SD，可训练私有 pixel style，$20/月起 ⚠️ 未核实
- **Retro Diffusion** — https://www.retrodiffusion.ai/ ；专攻像素艺术，有 API，$8/月 1000 张，**GBA 风格最接近** ⚠️ 未核实（价格与画质论断均未实测；注意 research/01 中同一工具写的是"$6/月起"和"预付余额无订阅"，三处口径不一致）
- **PixelVibe / Layer.ai** — 概念图为主，不够 GBA

## 二、像素化后处理（关键收尾）

### 算法/工具
- **Pillow** `Image.resize(size, NEAREST)` + `Image.quantize(colors=N, method=Image.MEDIANCUT)` — 20 行搞定基础像素化 + 调色板量化
- **PixelPot / Pixelator** — https://github.com/giventofly/pixelit（JS，浏览器端） / https://pixelicious.xyz/
- **Aseprite CLI** — https://www.aseprite.org/docs/cli/ ；`--sheet` 批处理；商用 $19.99；命令行可脚本化
- **rgbquant.js** — https://github.com/leeoniya/RgbQuant.js ；浏览器端调色板量化（可喂入自定义 GBA 15-bit 调色板）
- **hqx / xBR** 反向不适用；像素化用简单 nearest-neighbor 即可

### 保脸辨识度 + GBA 调色板的方案
GBA 硬件：15-bit RGB（32768 色），单帧 palette 通常 16 色/子调色板 × 16。想要"绿宝石味"：
1. 生成 128×128 或 96×96 目标分辨率 → 再 nearest 缩到 32×32 或 64×64
2. 固定调色板量化：把绿宝石解包的调色板（可从 tSPR / Pokéemerald 项目 palette 目录取）传给 `PIL.Image.quantize(palette=Palette)`
3. 人脸辨识度：**先在高分辨率做 InstantID/IP-Adapter 锁身份，再降采样 + 调色板量化**；不要在低分辨率直接跑 SD（脸会糊）
4. 头身比：JRPG 头像多为 Q 版 2 头身，Prompt 里加 "chibi, 2 heads tall, pokemon emerald style, gba sprite"

参考调色板资源：
- Lospec 调色板库 — https://lospec.com/palette-list（搜 "gameboy advance" / "pokemon"）
- Pokéemerald 反编译 — https://github.com/pret/pokeemerald（`graphics/` 下有大量 .pal 原生调色板）

## 三、浏览器端 vs 服务端

| 维度 | 浏览器端 (WebGPU/TFJS/ONNX Runtime Web) | 服务端 (Replicate / 自托管 GPU) |
|---|---|---|
| 首屏加载 | 需下载 100MB-1GB 模型 | 无 |
| 首图延迟 | 30-120s（含加载） | 5-30s |
| 后续图 | 秒级 | 5-30s |
| SD 全流程 | **勉强**（WebSD/Web Stable Diffusion 有 demo，但 InstantID 生态不成熟） | 成熟 |
| AnimeGAN/White-box | **可行**，ONNX Runtime Web 已有生产案例 | 可行 |
| 像素化后处理 | **应该在浏览器做**（Pillow 等价的 JS：Canvas + rgbquant.js） | 也行 |
| 成本 | 零边际成本 | 按次 $0.005-0.02 |

**结论**：SD 类生成放服务端；像素化/调色板量化放浏览器。

## 四、成本估算（单张头像）

| 方案 | 单张成本 | 生成时间 |
|---|---|---|
| Replicate `zsxkib/instant-id` + SDXL Pixel LoRA | $0.008-0.015 | 8-15s |
| Replicate `fofr/sdxl-pixel-art` (无脸锁定) | $0.003-0.006 | 4-8s |
| Replicate `tencentarc/photomaker` | $0.02-0.04 | 15-25s |
| PixelLab API | **$0.19-0.30**（完整 4 方向行走 sprite）<br>$0.006（仅 image-to-pixelart） | **6-11 分钟**（异步） |
| Retro Diffusion API | ~$0.008 | 5-10s ⚠️ 未核实 |
| Fal.ai (`fal-ai/fast-lightning-sdxl` + IP-Adapter) — https://fal.ai/models | $0.003-0.01 | **2-5s**（Lightning） |
| 自托管 A10G on Modal/RunPod | $0.001-0.003 | 3-8s |

Fal.ai 值得单独提：https://fal.ai/ 上有 `fal-ai/ip-adapter-face-id`、`fal-ai/pixart-sigma`、`fal-ai/flux-lora`；延迟比 Replicate 明显低，SDK 简单，AI 友好度极高。

> ⚠️ **2026-08-01 注**：本表标题是"单张头像"，但 PixelLab 一行原写的 `$0.02-0.05 / 10-20s`
> 既不匹配"单张头像"（那应是 `image-to-pixelart`，$0.006 且同步）
> 也不匹配"可用的游戏 sprite"（那是 $0.19-0.30 且 6-11 分钟异步）。已按两种口径分别列出。
> **本项目需要的是后者** —— 头像不能拿来当行走 sprite。其余各行未核实。

## 五、AI 友好度对比（Claude Code / Cursor 视角）

| 工具 | AI 友好度 | 说明 |
|---|---|---|
| Replicate JS/Python SDK | ★★★★★ | REST + `replicate.run()` 一行；文档全 |
| Fal.ai SDK | ★★★★★ | `fal.subscribe(model, {input})` 一行；有 TS 类型 |
| PixelLab / Retro Diffusion | ★★★★ | REST 简单，但社区样例少 |
| ComfyUI workflow JSON | ★★★ | Claude 能生成 workflow，但调参需人肉 |
| 本地 SD WebUI / diffusers | ★★ | 环境地狱 |
| ONNX Runtime Web | ★★★ | 需自己找模型；Claude 能写胶水代码 |

## 六、首选推荐（一个人 vibe coding，Web 优先，30s 内出图）

**推荐管线：Fal.ai（服务端生成）+ Canvas/rgbquant.js（前端像素化）**

具体三步：

1. **前端上传** → 直接 `POST` 到 Next.js API route（Vercel/Cloudflare Workers）
2. **服务端调 Fal.ai** — 首选 `fal-ai/ip-adapter-face-id` 或 `fal-ai/photomaker`：
   ```
   fal.subscribe('fal-ai/ip-adapter-face-id', {
     input: {
       face_image_url,
       prompt: 'pokemon emerald trainer sprite, gba pixel art, chibi 2 heads, front view, transparent background, <lora:pixel-art-xl:1>',
       num_inference_steps: 25,
       guidance_scale: 5,
       image_size: 'square_hd'
     }
   })
   ```
   - 延迟 3-6s，成本 ~$0.008/张
   - 备选：Replicate `zsxkib/instant-id` + `pixel-art-xl` LoRA
3. **前端后处理**（拿到 1024×1024 后）：
   - Canvas `drawImage` 缩到 64×64，`imageSmoothingEnabled=false`
   - `rgbquant.js` 传入从 Pokéemerald 提取的 15-bit 调色板做量化
   - 可选：`potrace` / 描边加强 → 再放大到 256×256 nearest 显示

**为什么选它**
- 一人开发、无美术：**InstantID/IP-Adapter 保脸**是不可让的，风格 LoRA 决定 GBA 味，两者叠加是当前最优解
- **Fal.ai** 而不是 Replicate：延迟低一半、SDK 更适合 Cursor/Claude 生成的 Next.js 代码、支持 webhook 便于异步
- **前端做像素化**：零成本、可实时预览调色板/分辨率参数、用户体验好（生成后还能拖滑块微调）
- **调色板量化不放 SD**：SD 直接生成低分辨率像素画会失真，"大图 + 后期降采样 + 固定调色板"是行业标准做法
- 想更省钱/更可控时，同一套代码换成自托管 ComfyUI（Modal serverless GPU，https://modal.com/），无缝迁移

**兜底方案**（如果想连 API 都不集成）：直接用 **PixelLab** SaaS 的 API，一天上线；美感稍逊 InstantID+LoRA 组合但零工程量。

> ❌ **2026-08-01 更正**：原文写「PixelLab SaaS 的**嵌入式 widget**」—— **不存在这个东西**。
> PixelLab 只有 Web UI（人工使用）、REST API、Python SDK、MCP 四种入口，
> 没有可嵌入你页面的 widget。所谓"零工程量"不成立：用户侧接入至少需要
> 后端代理 + 任务表 + 轮询 worker + SSE 通知。见 [`../reference/pixellab-api.md`](../reference/pixellab-api.md) §十一。

> 💡 **两条路线的真实分工**（2026-08-01）：
> - **头像/近景**（要保脸、要快）→ Fal.ai + InstantID + Pixel LoRA + 前端量化。秒级，$0.008
> - **行走 sprite/远景**（要多方向 + 动画）→ PixelLab REST v2。分钟级，$0.06–0.30
>
> 这两者**不可互相替代**：Fal.ai 那条链出不来方向一致的 8 方向行走帧，
> PixelLab 那条链慢且贵不适合做即时头像预览。

## 修订记录

- **2026-08-01**：核实 PixelLab v2 API。更正：删除不存在的"嵌入式 widget"；
  修正成本/耗时（$0.02-0.05 / 10-20s → 按两种口径分列，实为 $0.006 同步 或
  $0.19-0.30 / 6-11 分钟异步）；补充照片入口的真实三步链路；
  标注 `image-to-pixelart` **不能**作为照片→sprite 入口；
  指出 Retro Diffusion 定价在三份文档中口径不一致且均未核实。