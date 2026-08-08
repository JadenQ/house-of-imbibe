# AI 生成像素/马赛克画风素材工具与模型 调研（2025-2026）

> **调研日期**：2026-07-27
> **状态**：**部分过期** —— PixelLab 段落的 MCP 结论与定价已在 2026-08-01 核实并更正，
> 接口事实以 [`../reference/pixellab-api.md`](../reference/pixellab-api.md) 为准。
> Retro Diffusion 段落**尚未核实**（★ 推荐结论建立在未实测的基础上）。

## 一、专用像素艺术生成 SaaS

### 1. Retro Diffusion（首选主力生成工具）
- **网址**: https://retrodiffusion.ai/
- **API 文档**: https://www.retrodiffusion.ai/app/guide/api  |  示例仓库: https://github.com/Retro-Diffusion/api-examples
- **官方 MCP 服务器**: https://github.com/Retro-Diffusion/retro-diffusion-mcp （Endpoint: `https://mcp.retrodiffusion.ai/mcp`，Streamable HTTP）
- **用途**：由资深像素画师 Astropulse 训练的**专门**像素艺术模型，输出**真正的**网格对齐像素、限定调色板、无抗锯齿。支持三大模型系列（RD Fast / RD Plus / RD Pro / FLUX 大模型）+ 动画模型 + Tileset（Wang 平铺）模型 + 90+ 风格预设。
- **能力覆盖**：
  - 文生图 / 图生图（`input_image + strength`）
  - **参考图保持角色一致性**（RD Pro 特有）
  - 生成 **GIF / sprite sheet 动画**（walking、idle 等标准游戏引擎排列）
  - 生成 **Wang-style tileset**
  - 内置 background_removal、"repair pixel art" 编辑工具（把糊掉的像素图重新对齐网格）
  - **免费成本估算** `check_cost:true`，可让 Claude 在花钱前先算价
- **定价**：预付余额，无订阅，**信用永不过期**；单张约 $0.01 起步；RD Pro 单图约 $0.058，RD Pro 一张 256×256 = $0.25。有新账号赠送信用。Aseprite 扩展一次性 $65（本地跑，但用小模型，无动画）。
- **上手难度**：⭐（极低）——REST API 只有 `POST /v1/inferences`，一个 header `X-RD-Token`。
- **AI 友好度**：★★★★★——**唯一官方发布 MCP Server 的像素艺术平台**。一行 `claude mcp add --transport http retro-diffusion https://mcp.retrodiffusion.ai/mcp --header "Authorization: Bearer rdpk-..."` 即可让 Claude Code / Cursor / Windsurf / VS Code / Claude Desktop 直接调用 17 个 typed tool（生成、成本估算、异步任务、canvas edit、自定义风格）。还提供 `llms.txt` 完整 API 摘要。
- **也在 Replicate 上架**：https://replicate.com/retro-diffusion/rd-animation （方便非 MCP 场景用）

### 2. PixelLab.ai（首选辅助工具，负责角色 sprite 全流程）
- **网址**: https://www.pixellab.ai/  |  **API**: https://api.pixellab.ai/v2/docs  |  **产品页**: https://www.pixellab.ai/pixellab-api
- **用途**：面向**独立游戏开发者的一站式 sprite pipeline**——生成静图 + **角色 4/8 方向旋转** + skeleton 骨骼动画 + 文生动画 + **完整 top-down / sidescroller tileset** + **等距 tile** + **UI 元素**（按钮、血条）+ 真正的 inpainting + character 持久化管理（可导出 ZIP 含所有方向、动画、关键点）。
- **模型系列**：Pixflux（≤400×400）、Pixen（≤512×512）、Bitforge（参考风格迁移，≤200×200）
- **定价**（2026 现价）：
  - 免费试用 40 次快速生成（无需信用卡）
  - Pixel Apprentice $12/月（≤320×320）
  - Pixel Artisan $24/月（≤400×400，实验工具，优先队列）
  - Pixel Architect $50/月（20 并发，团队）
    <!-- ⚠️ 2026-08-01：上述四条订阅档位**未能核实** —— pricing 页面为客户端渲染，
         curl 抓不到价格文本。需登录官网人工确认。 -->
  - **纯 API 按次计费**（✅ 2026-08-01 核实，官方 estimated USD）：
    4 方向角色 48×48 **$0.0105**；8 方向 48×48 **$0.0133**；
    角色动画 **按方向计费** —— template 64×64 **$0.0323/方向**，v3 4帧 64×64 **$0.0129/方向**；
    image-to-pixelart 64×64 $0.006；完整表见 [`../reference/pixellab-api.md`](../reference/pixellab-api.md) §七
    <!-- ❌ 原文"4-frame 动画 64×64 ≈ $0.016"不准确，且漏掉了最关键的
         "每个方向单独计费"——4 方向动画实际是 4 倍价格 -->
  - 所有付费计划均包含**商用授权**
- **上手难度**：⭐⭐——REST API `POST https://api.pixellab.ai/v2/...`，Bearer token；有官方 Python client `pip install pixellab`
- **AI 友好度**：★★★★★——提供 `https://api.pixellab.ai/v2/llms.txt`；**有官方 MCP server**（`https://api.pixellab.ai/mcp`，实测在线，**65 个工具**）
  <!-- ❌ 2026-08-01 核实：原文写"未见官方 MCP"是错的。官方 MCP 存在、在线、65 个工具。
       但注意：MCP 只适合开发期，不能用于面向终端用户的网页（浏览器无 MCP 客户端 + token 暴露）。
       见 ../reference/pixellab-api.md §九 -->
- **杀手锏**：**"Create character with 4/8 directions"** + **"Animate character"** 两个端点 = 恰好对应你项目里"上传照片→ AI 生成 GBA 风格 4 方向行走 sprite"的核心需求，其他工具都要拼凑。
  <!-- ⚠️ 2026-08-01 补充：这两个端点确实存在且对口，但都是**异步**的（返回 job id 需轮询），
       端到端 5–9 分钟；且动画**按方向逐个计费**。"照片入口"另需先过 portrait-character-pro。
       image-to-pixelart **不能**作为照片→sprite 的入口。 -->
- ⚠️ **JS SDK 勿用** —— `pixellab-code/pixellab-js` 最后更新 2025-07，README 仍指向已废弃的 **v1** API。Python SDK（2026-05 更新）才是当前的。

### 3. Scenario.com（原 Scenario.gg）
- **网址**: https://www.scenario.com/  |  **API**: https://docs.scenario.com/  |  **定价**: https://www.scenario.com/pricing
- **用途**：面向大团队的**通用 AI 游戏素材工厂**——训练**自定义 LoRA**（10-30 张图起）、Multi-LoRA 融合、text-to-image / img-to-img / **ControlNet**（对 tileset 极重要）、text-to-3D、text-to-video、Workflow 节点编辑器、Apps 预制工作流。风格更泛，需自训 LoRA 才能得到稳定像素画风。
- **定价**：Free 50 credits/天（无卡） → Starter $15/月 1,500 credits → Pro $45/月 5,000 credits（含训练自定义模型）→ Max $75/月 → 企业版；Credits 不滚存。年付 33% off。
- **上手难度**：⭐⭐⭐——功能极多，API 是异步 job 模式，需自训模型才能得到 GBA 风格
- **AI 友好度**：★★★☆☆——纯 HTTP REST，无官方 MCP；因功能面广，让 Claude 自动 orchestrate 不如 Retro Diffusion / PixelLab 那样即插即用
- **优点**：真正的 ControlNet 支持 = 想做**地图 tileset 拼接一致性**时很有用
- **不建议做主力**：单人开发者、无美术团队场景下 overkill，且需要自训 LoRA 才有 GBA 味

### 4. Rosebud AI / PixelVibe
- **网址**: https://rosebud.ai/ai-game-assets  |  https://lab.rosebud.ai/
- **用途**：PixelVibe 是网页端生成器（Pixel Icons / Pixel Pixie Portraits / Pixel Characters Full Body 三个模型专攻像素）+ Rosebud 是"聊天式"AI 游戏工厂（会同时生成代码+资源），browser-based 一键分享。
- **定价**：免费 10 次/天；Standard 与 Pro 订阅（具体价格官网需登录）。
- **API/MCP**：**无公开 API**，只能在网页界面用。
- **AI 友好度**：★☆☆☆☆（不可编程集成）
- **适用**：早期灵感/原型探索、免费资产包（他们提供多个免费素材包如 Red & Blue 185 tiles、Fairy Inspired 161 items、67 全身角色等，**可直接下载**当占位资源）。**不能作为主力**。

### 5. Layer.ai
- 面向企业团队的**训练+版本化**素材平台，闭源、商业价格高、单人开发者不划算，跳过。

---

## 二、Stable Diffusion / SDXL 生态（Civitai LoRA + ComfyUI 本地方案）

### 关键 LoRA（Civitai）
| 模型 | 链接 | 用途/触发词 | 备注 |
|---|---|---|---|
| **Pixel Art XL v1.1**（Nerijs / Astropulse） | https://civitai.com/models/120096/pixel-art-xl | SDXL LoRA，最经典；无需 trigger word | 与 PixelDetector 8× 最近邻降采样搭配得像素完美 |
| **Pixel art SDXL RW** | https://civitai.com/models/114334/pixel-art-sdxl-rw | trigger `pixelart` | 早期主力 |
| **Super_PixelArt_Sprite_XL_M_V1** | https://civitai.com/models/579244/superpixelartspritexlmv1 | trigger `pixelart, sprite, fighting, 16bit`；在 1024² 里输出 128² 精灵 | 专攻 16bit 战斗风 sprite |
| **Pixo pixel art style Lora**（Illustrious） | https://civitai.com/models/1821405/pixo-pixel-art-style-lora | 8-bit ~ retro fantasy | 2025.7 更新，characters / monsters / items 都强 |
| **16-bit Pixel Art SDXL LoRA for Retro Game-Art** | https://civitai.com/models/2661436/16-bit-pixel-art-sdxl-lora-for-retro-game-art | trigger `p1x3l16`, strength 0.85 | **最贴近 GBA/SNES 时代**：限定调色板、chunky 像素、干净 dithering |
| **Flux.2 Klein 4-View Spritesheet LoRA** | https://huggingface.co/fal/flux-2-klein-4b-spritesheet-lora | 单图 → 4 视角 sprite sheet | 2026 新出，物体 4 视角好，角色一致性仍有限 |

### 后处理神器
- **Astropulse/pixeldetector**: https://github.com/Astropulse/pixeldetector （8× 最近邻降采样 + 自动调色板降色，把"像素画风"图变成真正网格对齐像素图）
- **ComfyUI 节点版**: https://github.com/dimtoneff/ComfyUI-PixelArt-Detector

### ComfyUI 参考工作流
- 完整 mac 教程（含 workflow JSON）: https://www.kokutech.com/blog/gamedev/pixel-art-generation-with-comfyui
- Sprite sheet 生成方法论: https://apatero.com/blog/generate-clean-spritesheets-comfyui-guide-2025
- 综合工作流: https://inzaniak.github.io/blog/articles/the-pixel-art-comfyui-workflow-guide.html

### SDXL/ComfyUI 综合评估
- **优点**：零成本（自己有 GPU）、可无限微调；LoRA + ControlNet + Character LoRA 训练能做到**真正的角色一致性**（在项目里代表玩家角色的 4/8 方向 sprite）。
- **缺点**：mac 上跑 SDXL 慢；单人开发者维护 ComfyUI + workflow + LoRA + 降采样 pipeline **极耗时**——正好与"vibe coding 一人开发"目标冲突。
- **AI 友好度**：★★★☆☆——ComfyUI 有 REST API 和 [ComfyUI-MCP](https://github.com/joenorton/comfyui-mcp) 等社区 MCP 服务器，但节点编排让 Claude 自动化比调 SaaS 麻烦得多。
- **API 化路径**：不想自己部署可走 **fal.ai** 或 **Replicate**：
  - fal-ai/flux-lora + fal-ai/lora/image-to-image（https://fal.ai/models/fal-ai/flux-lora ）——按次计费，直接加载 Civitai/HF LoRA URL，API 极简
  - Replicate `retro-diffusion/rd-animation` 等（https://replicate.com/retro-diffusion ）

---

## 三、GBA / 16-bit JRPG 专用能力对比矩阵

| 需求 | Retro Diffusion | PixelLab | Scenario | SDXL+LoRA | Rosebud |
|---|---|---|---|---|---|
| 用户上传照片 → 像素头像 | ✅（img2img + rd_pro） | ✅（image-to-pixelart 端点） | ✅ | ✅ | ✅（PixelVibe） |
| **4/8 方向角色 sprite** | 部分（动画模型自带方向） | ⭐**最强**（专用端点） | 需自训 LoRA | 需 Character LoRA + IP-Adapter | ❌ |
| Walk cycle 动画 | ✅（RD Animation） | ✅（skeleton + text） | 需 workflow | 复杂 | ❌ |
| **Tileset / 地图** | ✅（Wang tileset） | ✅（top-down + sidescroll） | ✅（+ControlNet） | ⭐ ControlNet 强 | 有免费包 |
| GBA 调色板/画风保真 | ⭐**最真**（专训模型） | 良好 | 需自训 | `p1x3l16` LoRA 接近 | 一般 |
| 免费额度 | 新账号赠送信用 | 40 次试用 | 50/天 | 免费（自部署） | 10/天 |
| **官方 MCP** | ⭐**有** | ⭐**有**（65 工具，✅ 实测） | 无 | 社区 | 无 |
| HTTP API | ✅ | ✅ | ✅ | 走 fal/Replicate | ❌ |
| **同步还是异步** | ⚠️ 未核实 | ✅ **异步**（job id + 轮询，5–9 分钟） | 异步 job | 异步 | — |

> ❌ **2026-08-01 更正**：原表 PixelLab 行的「官方 MCP：无」是错的。
> ⚠️ **同时提醒**：本表 Retro Diffusion 一列**全部未经实测**，包括"唯一官方发 MCP"
> 这一核心论断 —— 在 PixelLab 也有官方 MCP 的事实下，该论断已不成立。

---

## 四、最终首选推荐

### 主生成工具：**Retro Diffusion**（通过官方 MCP 接入 Claude Code）

> ⚠️ **2026-08-01 提醒**：以下 5 条理由**均未实测核实**，且理由 1 已被证伪
> （PixelLab 也有官方 MCP）。这段结论在做决策前需要重新验证。

理由：
1. ~~**唯一一个官方发 MCP Server 的像素艺术平台**~~ —— ❌ 不成立，PixelLab 官方 MCP 实测在线且有 65 个工具
2. **画质是所有工具里最接近真 pixel art 的**（专训模型，非通用 SD 加 LoRA 模糊感）——GBA 味最正。⚠️ 未实测
3. **无订阅、预付余额、信用不过期、失败自动退款** —— 单人独立开发的现金流最友好。⚠️ 未核实
4. 覆盖你项目的三大素材需求：**用户头像（img2img + reference）+ 角色 sprite 动画（RD Animation）+ 地图 tileset（RD Tile）**。⚠️ 未实测
5. 有 `check_cost:true` 免费预估价，Claude 可先估价再花钱，避免烧钱。⚠️ 未核实
   （对照：PixelLab 的 MCP 在 pro 动画模式下也有"先不带 confirm_cost 调一次看价"的等价机制 ✅ 已核实）

### 辅助工具 1：**PixelLab.ai**（负责需要"标准游戏引擎 sprite sheet"的部分）
- 用户上传照片后需要生成 **4 或 8 方向行走 sprite**，Retro Diffusion 目前只在动画模型里给固定方向，**PixelLab 的 `create-character-with-4/8-directions` + `animate-character` 是最直接的现成端点**，可 `GET /v2/characters/{id}/zip` 导出含关键点。✅ 端点已核实
- 定价按次调 API 便宜（4 方向角色 $0.0105 起，动画 $0.0129/方向 起），不用订阅。✅ 已核实
- ⚠️ **但要注意**：异步分钟级、动画按方向计费、照片入口需先过 `portrait-character-pro`。
  用户侧接入必须自建后端代理，**不能用 MCP**。详见 [`../reference/pixellab-api.md`](../reference/pixellab-api.md)
- 通过 `https://api.pixellab.ai/v2/llms.txt` 塞进 Claude context 就能自动化，或直接装官方 MCP。

### 辅助工具 2：**Rosebud PixelVibe 免费素材包 + Astropulse/pixeldetector 后处理**
- 项目 MVP 阶段直接**下载 Rosebud 提供的免费 tileset/character 资产包**（Red & Blue 185 tiles、Fairy 161 items 等）当占位/兜底，先跑通地图和多用户交互。
- 任何来自 Retro Diffusion 或 PixelLab 的输出若感觉不够"像素对齐"，用 `Astropulse/pixeldetector` 一键 8× 降采样 + 自动调色板 —— 这一步能把画风统一到真正的 GBA 网格。

### 兜底方案（预算耗尽或深度定制时）：**fal.ai + Pixel Art XL + `p1x3l16` LoRA**
- 走 `fal-ai/flux-lora` 端点，加载 Civitai 上的 `pixel-art-xl` 或 `16-bit Pixel Art SDXL LoRA`（trigger `p1x3l16`），成本进一步下探；如需地图一致性可用 fal 的 controlnet 端点。
- Claude Code 通过 fal 的 HTTP API 直调完全 vibe 得动。

**不推荐做主力**：Scenario（overkill，自训 LoRA 门槛高）、Rosebud 生成器（无 API 不可自动化）、纯自部署 ComfyUI（维护成本与"一人 vibe coding"目标冲突）。

---

## 五、参考来源

Sources:
- [Retro Diffusion MCP Server](https://github.com/Retro-Diffusion/retro-diffusion-mcp)
- [Retro Diffusion API 示例](https://github.com/Retro-Diffusion/api-examples)
- [Retro Diffusion 官网](https://retrodiffusion.ai/)
- [Retro Diffusion on Replicate](https://replicate.com/retro-diffusion/rd-animation)
- [PixelLab.ai 官网](https://www.pixellab.ai/)
- [PixelLab API 定价](https://www.pixellab.ai/pixellab-api)
- [PixelLab API 文档](https://api.pixellab.ai/v2/docs)
- [Retro Diffusion vs PixelLab 2026 对比](https://gamedevaihub.com/retro-diffusion-vs-pixellab/)
- [Scenario.com 定价](https://www.scenario.com/pricing)
- [Scenario Generation API](https://apis.io/apis/scenario-gg/scenario-generation-api/)
- [Rosebud PixelVibe](https://rosebud.ai/ai-game-assets)
- [Pixel Art XL LoRA (Civitai)](https://civitai.com/models/120096/pixel-art-xl)
- [Super PixelArt Sprite XL M V1](https://civitai.com/models/579244/superpixelartspritexlmv1)
- [Pixo pixel art style Lora](https://civitai.com/models/1821405/pixo-pixel-art-style-lora)
- [16-bit Pixel Art SDXL LoRA](https://civitai.com/models/2661436/16-bit-pixel-art-sdxl-lora-for-retro-game-art)
- [Pixel art SDXL RW](https://civitai.com/models/114334/pixel-art-sdxl-rw)
- [Astropulse pixeldetector](https://github.com/Astropulse/pixeldetector)
- [ComfyUI PixelArt Detector](https://github.com/dimtoneff/ComfyUI-PixelArt-Detector)
- [ComfyUI + Pixel Art XL 教程](https://www.kokutech.com/blog/gamedev/pixel-art-generation-with-comfyui)
- [ComfyUI 干净 sprite sheet 生成指南](https://apatero.com/blog/generate-clean-spritesheets-comfyui-guide-2025)
- [Flux.2 Klein Spritesheet LoRA](https://huggingface.co/fal/flux-2-klein-4b-spritesheet-lora)
- [fal-ai/flux-lora](https://fal.ai/models/fal-ai/flux-lora/stream/api)
- [Replicate LoRAs 指南](https://replicate.com/docs/guides/extend/working-with-loras)
- [PixelLab OpenAPI spec](https://api.pixellab.ai/v2/openapi.json) ⭐ 2026-08-01 核实依据
- [PixelLab MCP 仓库](https://github.com/pixellab-code/pixellab-mcp) ⭐ 2026-08-01 核实依据

## 修订记录

- **2026-08-01**：核实 PixelLab v2 OpenAPI + MCP 实测 + 官方定价页。更正：
  「未见官方 MCP」→ 有，65 个工具；补充精确按次价格与「每方向计费」事实；
  标注 JS SDK 已过期（指向 v1）；补充异步语义；订阅档位标为未核实；
  Retro Diffusion 全段落标为未实测，其「唯一官方 MCP」论断已证伪。
  接口细节移入 [`../reference/pixellab-api.md`](../reference/pixellab-api.md)。