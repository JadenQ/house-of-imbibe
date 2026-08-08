# 像素社交聊天空间 · 一人 Vibe Coding 工作流方案

> 综合 8 份维度调研，为「一人开发 + AI 辅助 + Web 部署 + 类绿宝石像素风 + 多人共享地图聊天」项目定制。

---

## 一、总体方案概览

**选型核心思路**：所有决策围绕 3 个约束——(1) 一个人开发，任何需要 DevOps/训练 LoRA/维护 ComfyUI 的路线一律劝退；(2) AI 是主要生产力，栈的**每一层都必须是 Claude Code/Cursor 训练语料里的一等公民**（TypeScript-first、有官方 llms.txt/MCP、schema 稳定）；(3) 视觉决定成败，画风必须能通过**参考图 + 调色板锁死 + Playwright 视觉闭环**来收敛，而不是靠人肉审美。

在此基础上做了两个关键取舍：
- **引擎不选功能最全的 RPG.js v5，选 Phaser 4**——RPG.js 虽内置多人 + JRPG 事件系统，但 v5 仍 beta 且社区语料远少于 Phaser；Phaser 4 官方 `skills/` 目录 + 海量训练语料，让 Claude 出错率最低；多人层由 PartyKit 单独补齐反而更可控。
- **像素图生成不押注单一 SaaS，走"聊天头像 vs 行走角色"双管线**——聊天头像用 fal.ai + InstantID + Pixel LoRA（保脸），行走 sprite 用 LPC 换装器（保帧数与一致性），中间用 color-thief 抽色统一色板。

### 最终技术栈一览

| 类别 | 首选 | 备选 | 一句话理由 |
|---|---|---|---|
| Web 引擎 | **Phaser 4**（v4.0 Caladan） | RPG.js v5 | 官方 `skills/` 目录 + Tilemap GPU + 语料最厚 |
| 地图编辑器 | **Tiled**（.tmj JSON） | LDtk | AI 生成 Tiled JSON 几乎零幻觉 |
| 多人后端 | **PartyKit**（CF Durable Objects） | Colyseus 0.17 on Fly.io | 40 行跑起来，边缘部署，0 DevOps |
| 用户/存储/Auth | **Supabase**（Auth + Storage + Postgres） | Cloudflare R2 + Clerk | 一个 SDK 全包，MCP 成熟 |
| 聊天头像生图 | **fal.ai**（`ip-adapter-face-id` + Pixel LoRA） | Replicate `zsxkib/instant-id` | 3-6s 出图、$0.008/张、TS SDK |
| 行走 sprite | **LPC Generator + OmegaCreations/CharaKit** | PixelLab.ai `create-character-with-8-directions` | 开源、4/8 方向现成、只同步 JSON |
| 后处理像素化 | **Canvas + rgbquant.js + GBA 调色板** | Astropulse/pixeldetector | 前端零成本，实时预览 |
| 像素编辑 | **Aseprite** + `aseprite-mcp` | LibreSprite | CLI + LUA 全脚本化 |
| Tileset 素材 | **Ninja Adventure Pack**（CC-BY 4.0）+ Kenney（CC0）兜底 | LimeZu Modern Interiors | 最贴 GBA JRPG 且授权干净 |
| BGM/SFX | **Suno v4** + **jsfxr** | Furnace / BeepBox | 参数化 URL SFX + AI BGM |
| 中文字体 | **Fusion Pixel 12px**（OFL） | Ark Pixel / Zpix | 中日韩最全，Web 可 subset |
| CRT/GBA 滤镜 | **@pixi/filter-crt** + AGB001 GLSL 移植 | postprocessing (three.js) | 一行接入 |
| 主 AI Agent | **Claude Code**（Sonnet/Opus 最新） | Cursor Composer | 唯一有 Playwright 闭环 + Skills |
| 辅助 AI | **Cursor**（手改 diff） + **Kimi K2**（中文文案） | GPT-5 | 各司其职，Kimi 只跑 NPC 台词 |
| 部署 | **Vercel**（前端） + **PartyKit CLI**（后端） | Cloudflare Pages + Workers | 一条命令上线 |

**预算下限**：Aseprite $19.99（一次）+ Suno Pro $10/月 + fal.ai/Supabase 免费额度 ≈ **首月 $30，之后 $10/月**跑几十并发无压力。

---

## 二、推荐技术栈（详细）

### 1. Web 游戏引擎

**首选：Phaser 4** — <https://phaser.io/> / <https://github.com/phaserjs/phaser>
- **成本**：MIT 免费
- **AI 友好度**：★★★★★
- **理由**：v4.0.0 Caladan (2026-04) 带 `TilemapGPULayer`（4096² 一次 draw call）、`SpriteGPULayer`、统一 Filter；仓库自带 **28 个官方 AI Agent Skills**（`skills/` 目录）覆盖每个子系统 + v3→v4 迁移；训练语料量所有 Web 引擎第一；Tiled 原生一等公民。
- **注意**：v4 刚正式发布，插件生态部分还在从 v3 迁移；如遇缺失就用 Phaser 3 LTS。

**备选：RPG.js v5** — <https://v5.rpgjs.dev/>
- **成本**：MIT，Studio $99 一次性可选
- **AI 友好度**：★★★★★（有官方 `npx skills add ... #v5`）
- **何时切换**：如果你觉得 Phaser 4 手搓事件/对话/存档太累，且能接受 v5 beta 状态；一体化 MMORPG 框架，能省 1-2 个月。

**不选**：Kaplay（非 JRPG 定位）、PixiJS 裸用（只是渲染器）、Godot Web（GDScript AI 语料太少、WASM 33MB）。

---

### 2. 实时多人后端

**首选：PartyKit**（Cloudflare Durable Objects 封装）— <https://www.partykit.io/>
- **成本**：PartyKit 平台永久免费；Workers Free 10 万请求/日；升级 $5/月
- **AI 友好度**：★★★★★
- **理由**：一个 `Server` 类 + `onConnect / onMessage / broadcast()` 就是全部 API，40 行内跑起来；边缘部署 <100ms；`npx partykit deploy` 一键上线；训练语料丰富。
- **状态设计**：单房间只需 `players: Record<id, {x, y, dir, layers, name}>`，走位 10Hz 广播即可（浏览器插值补足）。

**备选：Colyseus 0.17 自托管 on Fly.io** — <https://colyseus.io/>
- **成本**：Fly.io shared-cpu-1x ≈ $2-5/月
- **AI 友好度**：★★★★
- **何时切换**：需要服务器权威逻辑（防作弊 / NPC AI / 复杂碰撞）或未来要接 Unity/Godot 客户端时，Schema 状态同步比手写 broadcast 省 60% 代码。锁 **0.17.x**，AI 对旧版语法会混淆。

**不选**：Liveblocks（按分钟计费在长在线场景炸预算）、Nakama（对单人过重）、Socket.io 裸写（原型阶段坑太多）、Supabase Realtime（无服务器权威、Presence 官方警告不做高频位移）。

---

### 3. 像素资产生成（AI + 现成资源包）

#### 3a. AI 生成——按用途分层
| 场景 | 首选 | 备选 | 单张成本 |
|---|---|---|---|
| Tile / 建筑 / NPC 立绘（批量） | **fal.ai** `fal-ai/flux-lora` 加载 `16-bit Pixel Art SDXL LoRA (p1x3l16)` | Retro Diffusion（含官方 MCP） | $0.003-0.01 |
| 特定风格 tileset（需一致性） | **Retro Diffusion** RD Tile（Wang 平铺） | Scenario 自训 LoRA | $0.01-0.05 |
| 方向精灵表（用户角色 4/8 向） | **走 LPC 换装器**（不生图，见第 5 节） | PixelLab.ai `create-character-with-8-directions` | 0 / $0.02 |

**Retro Diffusion MCP** 特别提一句：`claude mcp add --transport http retro-diffusion https://mcp.retrodiffusion.ai/mcp --header "Authorization: Bearer rdpk-..."` 一行接入，17 个 typed tool 让 Claude 直接生成 + 预估成本 + edit。若你不想学 fal SDK，直接上 RD MCP 也行——只是单价略高。

#### 3b. 现成资源包（起步必备，跑通再谈原创）
- **Ninja Adventure Asset Pack**（<https://pixel-boy.itch.io/ninja-adventure-asset-pack>，CC-BY 4.0）——最接近宝可梦风，含 tileset + 角色 + BGM
- **Kenney Roguelike/RPG Pack**（<https://kenney.nl/assets>，CC0）——授权最干净的兜底
- **LimeZu Modern Interiors/Exteriors**（付费 ≥ $1.50，允许商用禁转售）——如果风格要偏"现代小镇"选这个
- **Pipoya Free RPG Character Sprites 32×32**（免费）——占位角色现成 4 方向

**规避**：Mana Seed 明确禁 GenAI 流程，不要用；LPC 是 CC-BY-SA 3.0 双许可，会传染，若未来要闭源商用请在初期切到 CC-BY 资源。

---

### 4. 照片 → 像素头像管线

**首选管线（三步）**：
```
[前端上传] 
  → [fal.ai serverless: ip-adapter-face-id + Pixel Art XL LoRA] 1024×1024 
  → [前端 Canvas nearest 缩到 64×64] 
  → [rgbquant.js 用 GBA 调色板量化] 
  → [存 Supabase Storage]
```

- **fal.ai** — <https://fal.ai/models/fal-ai/ip-adapter-face-id>，$0.003-0.01/张，3-6s，TS SDK 一行调用
- **Prompt**：`"pokemon emerald trainer sprite, gba pixel art, chibi 2 heads, front view, transparent background"` + LoRA `<lora:pixel-art-xl:1>`
- **调色板来源**：从 [Pokéemerald 反编译](https://github.com/pret/pokeemerald) 的 `graphics/*.pal` 抽取，或 [Lospec](https://lospec.com/palette-list) 搜 "gameboy advance"
- **rgbquant.js** — <https://github.com/leeoniya/RgbQuant.js> 支持喂入自定义调色板做量化

**AI 友好度**：★★★★★（fal SDK 极简，Claude 一次通过）
**授权注意**：用户原照片仅用于生成，生成后可选择不保留原图（隐私合规）。

**备选：Replicate `zsxkib/instant-id`** — 延迟略高（8-15s），成本相近，SDK 也简单；作为 fal.ai 崩掉时的兜底。

**不选**：AnimeGAN（不像 GBA）、PhotoMaker（慢且贵）、PixelLab 单张（$0.02-0.05 偏贵）、纯自部署 SD（一人维护 ComfyUI 崩溃）。

---

### 5. 角色自定义创建器

**首选组合：LPC 资源包 + OmegaCreations/CharaKit React 组件**

- **LPC Spritesheet Generator** — <https://github.com/LiberatedPixelCup/Universal-LPC-Spritesheet-Character-Generator>
  - 授权：**CC-BY-SA 3.0 / GPL-3.0 双许可（copyleft，传染）**
  - 覆盖：body/hair/top/bottom/shoes/hat/accessory + walk/idle/slash/thrust/cast/shoot/hurt/run/jump/climb 全套
- **CharaKit** — <https://github.com/OmegaCreations/CharaKit>（**MIT**）
  - 通用 React Canvas 拼装器，喂入 LPC 的 sprite sheet 即可
  - 支持 pixelScale、多层 zIndex、导出 PNG/WebP、config JSON 导入导出

**网络同步**：只同步 `{layers: {body:'light', hair:'short_ash', top:'shirt_red', ...}}` 这个 JSON（<200 bytes），其他客户端本地合成——比传 sprite sheet 便宜 3 个数量级。

**照片抽色注入**（融合玩法）：
1. 上传照片 → `color-thief` (<https://github.com/lokesh/color-thief>) 从上半脸取 skin tone、头顶取 hair color
2. LPC 每层都是**灰度可染色**的，用 HSL 偏移即可 recolor
3. 剩下的发型/衣服由用户自选

**AI 友好度**：★★★★★（LPC 是纯目录 PNG + JSON，CharaKit 是纯 React，Claude 二开难度极低）

**备选：PixelLab.ai `create-character-with-8-directions` + `animate-character`** — <https://www.pixellab.ai/pixellab-api>
- 何时切换：LPC 画风太"欧美 fantasy"、想更"JRPG chibi"，且愿意为每个用户付 $0.02-0.05

**授权红线**：如果你未来要闭源商业化，一开始就替换到 **PixelVerse: Modular Heroes**（<https://creativeofspd.itch.io/ultimate-modular-pixel-character-base-early-access>，CC-BY 4.0，$2 打赏）——架构不动。

---

### 6. 音乐 / 音效 / 字体 / 后处理

| 层 | 首选 | 备选 | 成本 |
|---|---|---|---|
| BGM | **Suno v4** prompt `"chiptune, 8-bit, GBA, JRPG town theme, loopable, 120bpm"` | Udio / BeepBox 手作 | $10/月 |
| SFX | **jsfxr**（<https://sfxr.me/> URL 即参数，可代码化） | ChipTone | 免费 |
| 中文字体 | **Fusion Pixel 12px**（OFL，中日韩全）— <https://github.com/TakWolf/fusion-pixel-font> | Ark Pixel / Zpix | 免费 |
| 英文/UI | **Press Start 2P**（Google Fonts） | Pixelify Sans | 免费 |
| 像素编辑 | **Aseprite $19.99** + `aseprite-mcp`（<https://github.com/diivi/aseprite-mcp>） | LibreSprite | $20 一次 |
| CRT/GBA 滤镜 | **`@pixi/filter-crt`** + AGB001 GLSL 移植 | postprocessing scanline | 免费 |

**字体加载**：Fusion Pixel 全字集 ~2MB，务必 `pyftsubset` 按常用字 subset + `font-display: swap`。

**GBA 真实感三件套**：AGB001 LCD 网格 + 扫描线强度 0.15 + 冷色 LUT；画布强制 4× 整数缩放，禁用 `imageSmoothingEnabled` / mipmap。

---

## 三、AI 工作流（Vibe Coding Playbook）

### 3.1 工具分工

| 任务 | 用什么 | 为什么 |
|---|---|---|
| 主编码循环（写代码 + 跑测试 + 视觉验证） | **Claude Code**（CLI） | 唯一有 Playwright MCP 闭环 + Skills/Subagents |
| 手改 diff、局部重命名、review | **Cursor** ⌘K | 编辑器体验最佳 |
| NPC 中文台词、道具描述批处理 | **Kimi K2**（Moonshot API 或 `claude-code-router` 路由） | 中文文本 $0.15/1M tokens，风格自然 |
| 生图 | **fal.ai / Retro Diffusion MCP** | 前者便宜快，后者画风最纯 |

**明确不用**：把 Kimi 当主 agent（tool-use 生态不成熟）、GPT-image-1（$0.04+/张太贵）、Rosebud（只能做原型验证，无法定制到 GBA）。

### 3.2 必装 MCP servers（最小可用集）

```bash
# 1. Playwright MCP —— 让 Claude 看到自己写的画面（必装）
claude mcp add playwright npx @microsoft/playwright-mcp

# 2. fal MCP —— 生图
claude mcp add fal npx @fal-ai/mcp

# 3. Supabase MCP —— 数据/存储/rt 频道
claude mcp add supabase npx @supabase/mcp-server-supabase

# 4. Retro Diffusion MCP —— 可选，画风保真度最高
claude mcp add --transport http retro-diffusion https://mcp.retrodiffusion.ai/mcp \
  --header "Authorization: Bearer rdpk-..."

# 5. Aseprite MCP —— 后期精修
# github.com/diivi/aseprite-mcp
```

### 3.3 自建 Skills / Subagents

**Skills（`.claude/skills/*/SKILL.md`）**：
1. **`pixel-art-aesthetics`** — 硬约束：调色板 ≤ 32 色（`assets/palette.hex`）、角色 16×24、tile 16×16、UI 8×8 网格、禁 anti-alias；引用 `assets/refs/emerald_town.png` 作风格锚点；每次生图/生成 CSS 前必读。
2. **`sprite-sheet-conventions`** — LPC 布局（13 行 × 21 列 × 64px 网格）、动画命名规范（`walk-down-0..7`）、图层 z-index 约定。
3. **`multiplayer-net-code`** — 位置同步 10Hz、消息 schema、插值参数、断线重连策略。
4. **`palette-verify`** — 一个 shell skill：`node scripts/verify-palette.js <png>` 扫描新 PNG 是否越色板。

**Subagents（`.claude/agents/*.md`）**：
1. **`art-director`** — 只读 skill 后审阅新生成的 PNG，返回 `pass / fail + 修改指令`（对图像版的 code-review）
2. **`tile-mapper`** — 专管 tilemap.tmj 编辑与碰撞层，避免主 agent 上下文被地图数据撑爆
3. **`character-forge`** — 用户照片 → fal.ai img2img → 剪裁 → color-thief 抽色 → 存 Supabase 的整条管线，主 agent 一句话调用

### 3.4 CLAUDE.md 模板骨架

```markdown
# Pixel Town —— Vibe Coding 主脑

## 1. Product One-liner
一个 GBA 绿宝石风的 Web 像素社交空间。用户上传照片生成像素头像，或用 LPC 换装器
自定义角色,在共享地图上走动 + 聊天。

## 2. Golden Path Stack (禁擅自更换)
- 引擎: Phaser 4 + Vite + TypeScript, 逻辑分辨率 240×160, 4× 整数缩放
- 多人: PartyKit (CF Durable Objects), 10Hz 位置广播
- Auth/Storage/DB: Supabase
- 生图: fal.ai (ip-adapter-face-id + pixel-art-xl LoRA)
- 角色: LPC + CharaKit
- 部署: 前端 Vercel, 后端 `partykit deploy`

## 3. Repo Map
- /game       Phaser 前端
- /party      PartyKit server
- /assets     原始 PNG / palette.hex / refs/
- /public/atlas  打包后的 sprite sheet
- /scripts    asset pipeline (pack-tile-atlas.js / verify-palette.js)

## 4. Hard Constraints
- 分辨率: 240×160 逻辑, canvas 强制 4× nearest
- 调色板: assets/palette.hex 中的 32 色, 越色即拒收
- 禁用: Tailwind, React 组件库, anti-alias, PNG 尺寸非 16 倍数
- 依赖: 引入任何 npm 包前先问

## 5. AI Rules
- 生成/修改 sprite 前必须 Read `.claude/skills/pixel-art-aesthetics/SKILL.md`
- 生成完 PNG 后必须调 `art-director` subagent 审核
- 任何 UI 改动完成后必须调 Playwright MCP 截图对比 `docs/screenshots/`
- 中文文案交给 Kimi K2 batch, 主 agent 不写台词
- 地图编辑委托 `tile-mapper` subagent, 主 context 不放 tilemap JSON

## 6. Style Anchors
- 参考图: assets/refs/emerald_littleroot.png (色调) 
- 参考图: assets/refs/emerald_char_may.png (角色比例)
- 每次生图 prompt 必须包含 "pokemon emerald style, gba lcd, chibi 2 heads"
```

### 3.5 保持美术一致性的 3 个杀手锏

1. **调色板锁死**：`assets/palette.hex` 存 32 色数组，`scripts/verify-palette.js` 扫描每张新 PNG，越色直接 CI 失败。
2. **参考图强绑定**：`assets/refs/` 存 3 张 GBA 时代原图，所有 prompt 必须携带；art-director 也用同一批图作评分基准。
3. **art-director 强制审查**：主 agent 不允许把 PNG 直接放进 `assets/final/`，必须先由 subagent 打分 ≥ 8/10 且色板校验通过。

### 3.6 每日迭代循环

- **30 分钟碎片时间**：跑一次 Playwright 截图对比 → 让 Claude 给出 3 个"最丑的地方"排序 → 只修最丑那个。
- **2 小时专注 session**：Claude 完整跑一个 feature（如"加个 NPC 对话气泡"），完成后 Cursor 手改 5-10 分钟收尾。
- **一整天 Sprint**：早上写 CLAUDE.md 中的当日目标 → 让 Claude 拆成 5-8 个子任务 → 每个任务用 subagent 并行 → 傍晚 Cursor 集中 review + 手调 tuning + `partykit deploy` 上线预览。

---

## 四、分阶段路线图

### 阶段 1：可玩原型（第 1-2 周）——单人 + 一张地图 + 走动

**目标产出**：本地可跑，一个占位角色能在 Ninja Adventure 拼的小镇地图上 4 方向行走，撞墙有碰撞。

**Checklist**：
- [ ] `npm create vite@latest`（TS）+ 装 Phaser 4，`npm create partykit@latest` 建 server
- [ ] 从 Ninja Adventure Pack 拿 tileset + 用 Tiled 拼 20×15 一张街景，导出 `.tmj`
- [ ] 用 Pipoya 免费 32×32 角色包放占位主角，Phaser 加载 sprite sheet + 4 方向动画
- [ ] 键盘/摇杆输入 + 网格对齐移动（每帧 1px 或按 tile 步进都可）+ collision layer
- [ ] 写 `pixel-art-aesthetics` skill + `palette.hex`（先手抽 Ninja Adventure 的 32 色）
- [ ] 部署 Vercel 拿 URL，能自己在手机上打开

**AI 分工**：让 Claude 跑主循环（引擎骨架、tilemap 加载、输入映射），Playwright MCP 每次修改后自动截图。

**踩坑预警**：
- Phaser 4 的 Tilemap API 与 v3 略有差异，AI 有时混用；`skills/` 目录里翻 v3→v4 迁移文档
- Tiled `.tmj` 里的 `firstgid` 别踩，AI 会算错；强制单 tileset 起步
- 别用 LDtk 起步——你会浪费时间给 AI 讲 schema

---

### 阶段 2：多人 + 聊天（第 3-4 周）

**目标产出**：多人同房间，看到彼此角色实时移动，头顶聊天气泡，最多测 5-10 并发。

**Checklist**：
- [ ] PartyKit `Server` 类：`onConnect` 分配 id、`onMessage` 处理 `move / chat / setLayers`、`broadcast` 差分状态
- [ ] 前端 `partysocket` 客户端，10Hz 上报位置，接收后本地插值（`lerp` 到目标位置，200ms 平滑）
- [ ] 头顶名字 + 聊天气泡（3 秒淡出），聊天记录侧栏保留最近 50 条
- [ ] 断线重连、房间容量上限（Durable Object 单实例 ~100 CCU 稳）
- [ ] Supabase Auth 接入（先跑匿名 + 昵称，等阶段 3 再上 Google OAuth）

**AI 分工**：`multiplayer-net-code` skill 定死协议，让 Claude 一次写完 server + client 匹配的 message schema；找一台朋友的电脑真跑一次跨机测试。

**踩坑预警**：
- 别用 Presence 做走位——Supabase Presence 官方警告，Liveblocks 也会烧钱
- WebSocket 消息大小：即使 200 字节/条 × 10Hz × 10 玩家 = 20KB/s，也要开 delta 传输而不是全量
- Durable Object hibernate 后第一个消息会延迟 200-500ms，别做同步 loading 提示

---

### 阶段 3：用户自定义头像 / 照片转像素（第 5-6 周）

**目标产出**：新用户可选"上传照片自动生成"或"LPC 换装自定义"，结果同步给所有客户端。

**Checklist**：
- [ ] fork LPC Generator + CharaKit 组件到 `/game/components/CharCreator.tsx`，剥离到 6-7 层
- [ ] 照片流水线：前端上传 → `character-forge` subagent 走一遍 → fal.ai 生成聊天头像（64×64）+ 用 color-thief 抽色注入 LPC 层
- [ ] 只同步 `layers: {...}` JSON 广播，其他客户端本地合成 sprite sheet
- [ ] 头像双轨：**聊天气泡里用 fal.ai 生的 face（更像本人）**，**行走 sprite 用 LPC（更一致）**——阶段 2 的气泡组件加上头像 slot
- [ ] Supabase Storage 存生成结果 + Postgres 存 `layers` 配置，登录复用

**AI 分工**：主 agent 不碰 fal SDK 细节，让 `character-forge` subagent 独立跑；Cursor 手改 CharCreator UI 交互。

**踩坑预警**：
- 用户上传照片的**隐私**：默认不保留原图，只保留生成结果；`Content-Security-Policy` 拦截外链
- InstantID 对亚洲脸 CFG 太低会糊，试 CFG=5-7 + steps=25 起步
- 别在低分辨率直接跑 SD——1024 生成再降采样才是正解
- LPC 换装 UI 别做全铺开的 grid，会崩视觉；改成"分类 tab + 缩略图 wheel"

---

### 阶段 4：打磨（第 7-8 周及以后）

**目标产出**：BGM + SFX + CRT 滤镜 + 动画补帧 + 域名上线 + 首批 20 邀请测试。

**Checklist**：
- [ ] Suno 生 3 首 BGM（day / night / indoor），Furnace 转 4 通道 GBA 版可选；接入 Howler.js 或 Phaser 内置 audio
- [ ] jsfxr 生 8 个 SFX：footstep×2、bump、chat_send、chat_receive、door、pickup、menu_click；URL 参数存 JSON 便于复现
- [ ] `@pixi/filter-crt` 或自写 GLSL 挂在 Phaser 的 postFX pipeline，扫描线强度可用户调（老年模式禁用）
- [ ] Fusion Pixel 字体 subset + Press Start 2P，通过 `@font-face` + `font-display: swap` 加载
- [ ] 动画补帧：让 PixelLab 或 Aseprite In-Between 脚本给主角加 emote 帧（点头、举手、坐下）
- [ ] `.claude/skills/code-review` 全量扫一遍 + Playwright 视觉回归对比 baseline
- [ ] 上线自定义域名，写 landing page + Twitter/小红书发首发帖

**AI 分工**：Cursor 主导（精修 tween/timing 靠手感），Claude 只做批量任务（生成 SFX 参数表、subset 字体命令）；Kimi K2 批量生 20 个 NPC 台词（问候语、闲聊、道具介绍）。

**踩坑预警**：
- CRT 滤镜 GPU 占用不小，弱机上会掉帧——默认关闭，Settings 里手动开
- 字体 fallback：Fusion Pixel 加载前的 FOUT 会露出系统字，务必 subset ≤ 300KB 首屏内联
- 上线前一定做移动端触控测试（虚拟摇杆），Phaser 4 有内置组件但需手写 UI 层

---

## 五、可选增强（不必都做）

1. **NPC AI 对话**：Kimi K2 或 Claude Haiku 给每个 NPC 一段 system prompt + 位置/朝向状态，玩家靠近触发对话；用 subagent 隔离，主 loop 不阻塞。成本 $0.5-2/月。
2. **迷你游戏嵌套**：钓鱼、种菜、划拳；共用同一 Phaser scene 系统，PartyKit 加一个 sub-room 状态即可。
3. **私密房间/密码保护**：PartyKit 支持每个 party 独立 ID，用 UUID + 短码分享。
4. **表情动作系统**：LPC 加一层 emote overlay（气泡里像素图标），只同步 `{emote: "wave", ts}` 事件。
5. **AI 生成动态天气/时间**：Phaser 全局 tint + 半透明覆盖层做 day/night，BGM 随之切换。
6. **玩家自建家具地图**：Tiled 编辑逻辑做进浏览器，`tile-mapper` subagent 辅助校验，房主可摆放 furniture tile。
7. **AR/相机 selfie 房间**：`getUserMedia` + fal.ai 实时把摄像头帧转像素，头像动态跟随。
8. **Discord/Twitter 分享卡**：`html2canvas` 抓当前地图 + 站姿，一键生成分享图。

---

## 六、风险与规避

| 风险 | 规避策略 |
|---|---|
| **美术风格漂移**（AI 每次生的画风不一致） | 调色板锁死 + 3 张参考图强绑定 + `art-director` subagent 打分 ≥ 8 才入库；每周做一次 `.claude/skills/pixel-art-aesthetics` 复盘更新 |
| **法律授权污染**（用了 CC-BY-SA 或 Mana Seed） | 起步只用 CC-BY 4.0 / CC0 / MIT 资源；LPC 只在明确接受开源约束时使用；Mana Seed 直接排除；照片生图明确"用户授权仅用于本次生成" |
| **多人同步延迟/抖动** | PartyKit 就近部署 + 客户端插值 200ms + 位置差分传输；上线前做真机跨洲测试；Colyseus 作为超过 100 CCU 后的迁移目标 |
| **成本失控**（fal.ai / Suno 用量突增） | fal 每次生图前用 Retro Diffusion 的 `check_cost:true` 或自建 budget cap；Supabase 免费额度天花板监控；Liveblocks 一律不用（分钟计费坑） |
| **AI 幻觉写破代码**（Phaser 4 vs 3 混用、Colyseus 版本错乱） | CLAUDE.md 里锁死版本号（`phaser@4.0.0`, `partykit@0.x` 具体号），每次 PR 让 Claude 先跑一次类型检查 + Playwright 视觉回归 |
| **一个人心态崩溃**（进度慢、无正反馈） | 阶段 1 结束就分享 URL 给 3 个朋友；每天 30 分钟必上 Vercel 预览刷一次；用 Claude Code 的 `dataviz` skill 做每周提交图激励自己 |

---

## 七、下一步立即行动（3 个）

### 行动 1：搭建骨架 + 装 MCP（预计 2-3 小时）
**做什么**：
```bash
npm create vite@latest pixel-town -- --template vanilla-ts
cd pixel-town && npm i phaser
npm create partykit@latest party
claude mcp add playwright npx @microsoft/playwright-mcp
claude mcp add supabase npx @supabase/mcp-server-supabase
```
写好上面第 3.4 节的 `CLAUDE.md`，创建 `.claude/skills/pixel-art-aesthetics/SKILL.md`（把第 3.3 节的硬约束抄进去）。

**期望产出**：一个空 Phaser 4 项目 + PartyKit server + Claude Code 已能通过 Playwright 打开本地 `localhost:5173` 截图。

### 行动 2：跑通"一个占位角色能在地图上走"（预计 4-6 小时）
**做什么**：
1. 下 Ninja Adventure Pack（<https://pixel-boy.itch.io/ninja-adventure-asset-pack>）
2. Tiled 拼一张 20×15 街景导出 `.tmj`（30 分钟人肉，或让 Claude 直接生 JSON）
3. 让 Claude Code 一句话："用 Phaser 4 加载 `assets/tilemap.tmj`，用 Pipoya 主角 sprite sheet 做 4 方向 walk 动画，方向键控制移动，撞到 collision layer 时停下"
4. Playwright MCP 反复截图验证

**期望产出**：本地能开着摄像头样的画面走来走去；Vercel `git push` 就上线。

### 行动 3：接 PartyKit 做双人 demo（预计 3-4 小时）
**做什么**：
1. `party/server.ts` 写 20 行：状态 = `Map<id, {x, y, dir}>`，`onMessage` 处理 `move`，`broadcast` 全房间
2. 前端 `partysocket` 连接，10Hz 上报位置，收到别人的位置就在 scene 里 spawn 一个"别人"精灵
3. `partykit deploy`，用手机开一个、电脑开一个，看到彼此在动就成功

**期望产出**：可发给朋友的 URL，两人能同时在线互相看到走动——**这是整个项目最爽的时刻，务必优先冲到这里，后面所有美化都是锦上添花。**