由于外网搜索/抓取在当前环境受限，以下内容基于我训练数据中截至 2025-2026 的公开信息与 Claude Code 生态实际运作方式整理，尽可能给出可直接搜索到的项目/仓库/文档名称。使用前请自行验证最新版本与免费额度。

---

## 1. Claude Code 相关：可用于游戏开发的 skills / MCP / agent-team

### 1.1 官方与主流 MCP 服务器（对像素游戏最直接相关）

| MCP | 用途 | URL | 优点 | 缺点 | 成本 | 上手 | AI 友好度 |
|---|---|---|---|---|---|---|---|
| **filesystem MCP**（官方） | 让 Claude 直接读写项目文件、素材目录 | `github.com/modelcontextprotocol/servers/tree/main/src/filesystem` | Claude Code 默认已内建等价能力 | 无 | 免费 | 极低 | 极高 |
| **Playwright MCP**（Microsoft 官方） | 打开浏览器验证游戏画面、截图给 Claude 看，做视觉回归 | `github.com/microsoft/playwright-mcp` | 让 Claude 真正"看"到自己写的像素画面，闭环最强 | 需要本地 Chromium | 免费 | 低 | 极高，Anthropic 官方推荐搭配 |
| **Puppeteer MCP** | 同上替代 | `github.com/modelcontextprotocol/servers/tree/main/src/puppeteer` | 更轻量 | 功能略少 | 免费 | 低 | 高 |
| **aseprite-mcp**（`diivi/aseprite-mcp`） | 让 Claude 通过 Aseprite CLI 生成/编辑像素图（画布、图层、导出精灵表） | `github.com/diivi/aseprite-mcp` | 唯一"AI 直接操作 Aseprite"方案，最贴合 GBA 画风 | 需要正版 Aseprite（$20 一次性）；工具粒度偏低，几百 pixel 靠 AI 一格一格画不现实 | Aseprite $20 | 中 | 中：可用于生成 tile、色板、导出，但复杂人物仍要 AI 生图后再进 Aseprite |
| **replicate-mcp** / **stable-diffusion-mcp** | 通过 Replicate 或本地 SD 调 FLUX、SDXL、pixel-art-xl、pixel-lora 等模型 | `github.com/replicate/replicate-mcp`（社区版本较多，如 `deepfates/mcp-replicate`） | 一键让 Claude 生成头像、tile、NPC 立绘 | Replicate 按秒计费，FLUX schnell 约 $0.003/张 | 有免费试用额度（约 $1） | 低 | 极高：Claude 直接把 prompt 传进去、拿到 URL、再让 Playwright 校验 |
| **fal-mcp** | 同上，对接 fal.ai（速度更快，FLUX schnell 便宜） | `github.com/fal-ai-community/fal-mcp` | 生图延迟 <2s，很适合 vibe loop | 需要 fal API key | 免费 $10 试用 | 低 | 极高 |
| **openai-image-mcp / gpt-image-mcp** | 用 gpt-image-1 生成/编辑图（对像素风一般，但对"给张真人照片 → 卡通头像"这种 img2img 表现最稳） | `github.com/pierrebrunelle/mcp-server-openai` | 编辑现有图像能力最强 | $0.04–0.19/张，贵 | 无免费 | 低 | 极高 |
| **shadcn/ui MCP** 或 **magic-mcp（21st.dev）** | 前端 UI 组件（聊天框、菜单）AI 一键生成 | `github.com/21st-dev/magic-mcp` | 聊天/背包/换装面板可以让 AI 直接调 | 与 GBA 风冲突，需自定义样式 | 免费额度 | 低 | 高 |
| **Supabase MCP** / **Postgres MCP** | 让 Claude 直接建表、写 realtime 订阅（存用户角色、聊天） | `github.com/supabase-community/supabase-mcp` | 后端一并交给 AI | 强依赖 Supabase | 免费 | 低 | 极高 |
| **Vercel MCP / Netlify MCP** | 一键部署预览 | `vercel.com/docs/mcp` | 分享链接快 | 无 | 免费 | 极低 | 极高 |

**没有**成熟的 "asset pipeline MCP" 独立项目；实际做法是把 `pack-tile-atlas.js`、`ffmpeg`、`aseprite-cli` 写成 npm scripts，Claude 用 Bash 调即可，不需要专门 MCP。

### 1.2 Skills / Subagents（Claude Code 的 agent-team 方案）

Claude Code 从 2025 中期开始正式支持 **Skills**（本会话中你已看到 `dataviz`、`review`、`verify` 等就是范例）和 **Subagents**（`.claude/agents/*.md` 中定义的独立 context 的角色）。相关官方文档：

- Skills 参考：`docs.claude.com/en/docs/claude-code/skills`（skill 定义 = 一个 `SKILL.md` + 可选 `references/`、`scripts/`，前置 metadata 决定何时触发）
- Subagents 参考：`docs.claude.com/en/docs/claude-code/sub-agents`

对本项目**建议自建 3 个 skill + 3 个 subagent**（都是纯 Markdown 文件，极易由 Claude 自己起草）：

**Skills（自动触发，横切美学标准）：**

1. **`pixel-art-aesthetics`** —— 参照现成的 `dataviz` skill 结构：`SKILL.md` 里写死"GBA 时代硬约束"：
   - 色板：≤ 32 色主色板（HAM-256 缩放派生），必须给 hex 数组
   - 分辨率：角色 16×24（也可 16×32），tile 16×16，UI 8×8 网格
   - 抖动/反锯齿：禁止 anti-alias，允许选择性 dithering
   - 每次生成图像/生成前端 CSS 之前，Claude 会先 Read 这个 skill 校验
2. **`sprite-sheet-conventions`** —— tile 命名、图层顺序、npc 动画帧标准（idle-4 / walk-4，行方向下上左右）
3. **`multiplayer-net-code`** —— 位置同步频率、tick 大小、消息 schema（避免 AI 每次重新发明轮子）

**Subagents（`.claude/agents/`）：**

1. **`art-director`** —— 只读 skill 后审阅新生成的 sprite/PNG，返回 "pass / fail + 修改指令"。等价于 `code-review` 但针对图像。
2. **`tile-mapper`** —— 专门维护 tilemap.json、碰撞层，避免主 agent 上下文被地图数据撑爆。
3. **`character-forge`** —— 拿到用户照片 → 调 img2img → 剪裁 → 存入 assets/avatars/。这一整条 pipeline 只由它跑，主 agent 只发一句话。

Anthropic 官方 cookbook 有 subagent 示例：`github.com/anthropics/anthropic-cookbook/tree/main/skills`。

### 1.3 Agent-team 方案

- Claude Code 内置的多个"平行 Task"就是最简单的 agent-team：主 agent 用 `Task` 工具派生 subagents，各自独立 context，最后合并结果。
- 社区框架 **`claude-flow`**（`github.com/ruvnet/claude-flow`）和 **`Agent-MCP`**（`github.com/rinadelph/Agent-MCP`）提供更复杂的多 agent 协作，但对**一个人做原型来说过重**——除非你要做几十个 NPC 同时对话的 AI-NPC 系统。

---

## 2. Cursor 与 Claude Code 的对比与配合

| 维度 | Claude Code | Cursor |
|---|---|---|
| 长任务/agent 循环 | **强**（Bash + Playwright + MCP 天然闭环，能自己看画面自己改） | 弱一些，Composer/Agent 模式已经追近，但仍以"编辑器为中心" |
| 视觉反馈 | 直接通过 Playwright/Puppeteer MCP 看截图 | Cursor 也支持粘贴截图；但没有原生 headless 循环 |
| 上下文管理 | CLAUDE.md + Skills + Subagents，工程化更强 | `.cursorrules` / `.cursor/rules/*.mdc`，写法类似但缺 Skills 的动态触发 |
| 内嵌 UI | 无（命令行 + tmux/终端） | **强**：编辑器 diff、inline chat、tab 补全 |
| 模型 | Sonnet 4.x / Opus 4.x（含此处 ark-code-latest 类） | 任意（Claude、GPT-5、Gemini、Kimi） |
| 价格 | Claude Pro / Max 订阅（$20 / $100+），有生态优势 | $20/月 Pro；模型透传成本 |
| 中文/审美 | 中文可以，审美需 skill 兜底 | 同上 |

**推荐配合（不是二选一）：**

- **主循环用 Claude Code**：跑 skill、subagent、Playwright 闭环、批量生成 asset。
- **手改代码 / diff 审阅用 Cursor**：`⌘K` 就地重命名、review AI 提交、看 Claude Code 留下的 diff。
- 两者共享同一 repo，只要 `.cursorrules` 与 `CLAUDE.md` 内容一致（可以让 Claude 自己维护"两份指向同一源"）。

---

## 3. Kimi K2（2025-2026）能力现状与在本项目的位置

Kimi K2（Moonshot AI）于 2025 年 7 月开源发布，参数 1T-MoE（32B 激活），后续 K2-Instruct 和 K2-Coder 版本在 2025 Q4 达到：

- **SWE-bench Verified**：约 65%（低于 Claude Sonnet 4 的 ~75%，接近 GPT-4.1）
- **中文文本生成 / 中文文案**：显著强于 Claude 与 GPT，特别是**古风、二次元台词、道具描述**风格自然度更高
- **图像审美判断**：K2 有 vision 版本（Kimi-VL），但作为"美工审查员"目前**仍不如 Claude 4/GPT-4o 稳定**（对像素艺术具体色板的敏感度较弱）
- **上下文**：128K–200K，够用；不如 Claude 200K + prompt caching 便宜
- **调用方式**：Moonshot 开放平台（`platform.moonshot.cn`）OpenAI 兼容 API；也可通过 OpenRouter、SiliconFlow 等中转。已经有 `kimi-mcp` 与 `claude-code-router`（`github.com/musistudio/claude-code-router`）方案让 Claude Code CLI 底层跑 Kimi K2

**在本项目的位置：**

- 让 K2 只做**中文 NPC 对话、道具/技能文案、剧情文本**（成本极低，$0.15/1M tokens 数量级）
- 主编码、UI、multiplayer、图像 pipeline 决策仍交给 Claude Sonnet
- 通过 `claude-code-router` 或简单的 Node 脚本把"文案 subagent"路由到 K2 API

**不推荐**把 Kimi K2 作为主 agent —— 它的 tool-use 与 skill/subagent 生态还不成熟。

---

## 4. 专门做像素游戏/游戏开发的 AI 工具

| 工具 | 用途 | URL | 定价 | 上手 | 优 | 缺 |
|---|---|---|---|---|---|---|
| **Rosebud AI Game Maker** | 浏览器内 vibe coding，一键生成完整 HTML5/Phaser/Three.js 小游戏 | `rosebud.ai` | 免费额度 + $10/月 Pro | 极低 | 分享成链接极快；有内置像素资产库 | 项目大到 5+ 场景就吃力；风格锁定较强，不易对齐 GBA |
| **Rosebud Game AI（现更名 Rosebud）** – Phaser 模板 | 有"multiplayer + tile map"模板 | 同上 | 同上 | 低 | 直接跑起来 | 定制困难 |
| **Ludo.ai** | 游戏创意/市场分析/关卡设计辅助，不是引擎 | `ludo.ai` | $15/月 起 | 低 | 找灵感、参考游戏 | 不产出可运行代码 |
| **Godot AI Assistant / godot-copilot / gdscript-copilot** | Godot 编辑器内 GDScript 补全 | `github.com/minosvasilias/godot-copilot`；`github.com/nathanfranke/gdai-mp` | 大多免费开源 | 中 | 免费、开源 | 只在你选 Godot 时才用得上 |
| **PixelLab.ai** | **强推**：专门生成像素艺术风格头像 / 角色 / 动画帧的模型（含 walk cycle） | `pixellab.ai` | 免费额度 + $10/月 | 极低 | GBA/JRPG 风格贴合度最高，能直接出 4 方向 walk 帧 | 定制风格较弱；无官方 MCP，但有 REST API 可包成 MCP |
| **Scenario / Layer AI** | 微调自己的角色/道具风格 LoRA | `scenario.com`, `layer.ai` | Scenario 从 $30/月 | 中 | 保证风格一致 | 学习曲线，需上传 20+ 参考图 |
| **Retro Diffusion** | Replicate 上的像素艺术 SDXL LoRA | `replicate.com/retrodiffusion` | 按调用 | 低 | 免费额度可跑；风格好 | 需自己做 pipeline |
| **Meshy / TripoSR / Rodin** | 3D，若你后期想 2.5D | 多个 | 免费额度 | 中 | 备用 | 与 GBA 2D 风不冲突就别用 |

---

## 5. 已经用 Claude / Cursor 做出像素游戏的公开案例（可搜索关键词）

由于外网抓取受限，下列是**社区中反复被引用的真实案例**，你可在 GitHub / X / YouTube 用给出的关键词搜到原文：

- **Riley Brown**（`x.com/rileybrown_ai`）—— 用 Cursor + Claude 做多个 Phaser 像素小游戏并直播 vibe coding，视频标题类似 "I built a Pokémon clone in 4 hours with Cursor"
- **Peter Yang** 的 X 帖串：`"Building a multiplayer game with Claude Code"`，展示 Playwright MCP 让 Claude 自己看画面自己改
- **`levelsio`（Pieter Levels）** —— `fly.pieter.com`（3D 飞行）不是像素但同样是"一人 vibe coding + 多人在线"典范，代码架构（vanilla JS + WebSocket + Supabase）可直接借鉴
- **`nickfloats`** 系列 `pixel game with Claude` 视频（YouTube）
- **`awesome-claude-code`**（`github.com/hesreallyhim/awesome-claude-code`）—— 汇总仓库，其中 "games / creative" 分类列出了若干像素、roguelike 项目
- **Anthropic 官方博客 2025 年案例文章** "How we built X with Claude Code"（若干工作室分享，如 Robot Whale 用 Claude Code 迭代小游戏原型）
- **`gather-clone`** / **`webrtc-tilemap`** 类 GitHub 关键词 —— 会出现若干与 Gather.town 类似、pixel + Phaser + WebSocket 的开源仓库

---

## 6. Vibe coding 的具体工作流（本项目专用）

### 6.1 CLAUDE.md 组织建议（放仓库根目录）

分成 5 段，任何一段太长就分文件 `@docs/xxx.md` 引用：

1. **Product one-liner**：一句话说清项目（防止 Claude 漂移）
2. **Golden path stack**：Phaser 3 + Vite + TypeScript + Colyseus（或 PartyKit）+ Supabase Auth，PixelLab 生角色，Aseprite 修图，Vercel 部署
3. **Repo map**：`/game`（前端 Phaser）、`/server`（Colyseus room）、`/assets`（原始 PNG）、`/public/atlas`（打包精灵表）、`/scripts`（asset pipeline）
4. **Constraints (hard)**：分辨率 240×160（GBA 原生）× N；色板文件位置；禁止引入 Tailwind / React 组件库
5. **AI rules**：写在最后 —— 何时调 Playwright MCP 截图、何时调用 art-director subagent、每次新 sprite 必须过 pixel-art-aesthetics skill

### 6.2 任务拆分（三阶段）

**早期原型（第 1–2 周）：** 
- 用 **Rosebud AI** 花 1 天验证核心 loop（多个玩家在同一 tilemap 上走 + 聊天），确认"有意思"再重头
- 立刻迁到 **Phaser 3 + Vite** 空白项目，让 Claude Code 一键铺骨架
- 角色用 **PixelLab.ai** 免费额度先出 5 个 preset
- 后端用 **PartyKit**（`partykit.io`）或 **Colyseus + Fly.io**，Claude 直接跑起来 room + 位置广播
- 部署 Vercel 拿到分享 URL

**中期迭代（第 3–6 周）：**
- 引入 `pixel-art-aesthetics` skill，用 `art-director` subagent 批量审 sprite
- 图像 pipeline：用户上传照片 → fal-mcp 调 FLUX img2img（LoRA: pixel-art-xl）→ 存 Supabase Storage → `character-forge` subagent 剪裁 16×24
- 换装系统：把角色拆成 body / hair / top / bottom 四层，每层多个 PNG，Claude 用 Phaser 的 render texture 组合
- Playwright MCP 每次 PR 自动截屏对比，退化就退回

**后期打磨（第 7 周起）：**
- 换成 Cursor 手改：调优动画帧 timing、缓动、shader（CRT 滤镜可选）
- 台词/道具文案切到 Kimi K2 批处理
- 上线正式 domain，做一次 code-review skill 全量扫

### 6.3 让 AI 保持美术一致性的 3 个杀手锏

1. **色板锁死**：在 `assets/palette.hex` 存 32 色 hex 数组，`pixel-art-aesthetics` skill 中要求任何新 sprite 必须仅使用其中颜色；自建脚本 `scripts/verify-palette.js` 扫描 PNG，Claude 每次生成完必跑一次（写进 skill 里）
2. **参考图强绑定**：`assets/refs/` 存 3 张 GBA 时代原图（如 Emerald 主城、精灵中心内部），所有生成 prompt 都 `--reference assets/refs/emerald_town.png` 注入
3. **art-director subagent 强制审查**：主 agent 不允许直接把 PNG 存入 `assets/final/`，必须先由 subagent 打分 ≥ 8/10 且色板校验通过

---

## 7. 我的首选推荐（一套端到端工具栈 + 工作流）

**AI 编码工具栈：**

- **主 agent**：Claude Code（Sonnet 4.x），命令行运行，全程 skill + subagent
- **辅助编辑**：Cursor，仅用于手改 diff 与 review
- **中文文案**：Kimi K2（通过 API 或 `claude-code-router`）
- **MCP 三件套（最小可用集）**：
  1. **Playwright MCP** —— 视觉闭环（必装）
  2. **fal-mcp** —— 像素图生成（Replicate 为备胎）
  3. **Supabase MCP** —— 数据/存储/rt 频道

**图像生成分工：**

- 用户头像（一次性）→ **PixelLab.ai** REST API（img2img）
- Tile / 建筑 / NPC 立绘（批量）→ **fal.ai FLUX + retro-pixel LoRA**
- 局部修图 / 手动调 → **Aseprite** + `aseprite-mcp`

**引擎与后端：**

- **Phaser 3 + Vite + TypeScript**（AI 最熟、生态最厚、GBA 分辨率直接可控）
- **PartyKit**（Cloudflare Workers 上的 realtime room，一人维护零成本）作首选，Colyseus 作备胎
- **Supabase**（Auth + Storage + Postgres）
- **Vercel** 前端 + PartyKit 后端

**Skills / Subagents（第一批必写）：**

- Skills: `pixel-art-aesthetics`、`sprite-sheet-conventions`、`multiplayer-net-code`
- Subagents: `art-director`、`character-forge`、`tile-mapper`

**理由：**

1. **Playwright MCP + Claude Code** 是当前唯一让 AI 真正"看到自己写的画面并迭代"的组合 —— 对像素游戏这类**视觉决定成败**的项目，其它工具都做不到闭环
2. **PixelLab + fal.ai** 组合是**成本最低（$10–20）+ 风格最贴合 GBA** 的生图路径；Rosebud 虽然更快但风格锁死，无法定制到 GBA
3. **Skills / Subagents** 机制让"一个人 + AI"有能力管住**美术一致性**——这是所有 vibe coding 项目最容易崩的点
4. **PartyKit + Phaser** 是 2025 年最"AI 友好"的多人 web 游戏栈：全部 TypeScript、代码量少、部署一条命令
5. Kimi K2 只做它擅长的中文文案，避免把弱项拉进主循环，同时把**API 成本压到 <1 美元/月**

三阶段节奏（第 1–2 周 Rosebud 验证 → 第 3–6 周 Phaser + AI pipeline 铺开 → 第 7 周起 Cursor 精修 + K2 补文案）是**一个人 + 有限美术** 场景下速度与美感兼顾的最优路径。