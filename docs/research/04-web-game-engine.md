# Web 端 2D 像素游戏引擎选型（2025-2026）

## 一、候选引擎逐项对比

### 1. Phaser 4（+ Phaser 3 LTS）
- 官网 / 仓库：https://phaser.io/ ； https://github.com/phaserjs/phaser
- 版本：Phaser v4.0.0 "Caladan" 于 2026-04-10 发布（40K stars），Phaser 3 仍长期维护
- 关键特性：全新 Render Node 架构、`TilemapGPULayer`（整张 tilemap 一次 draw call，可达 4096×4096）、`SpriteGPULayer`（100 万精灵一次 draw call）、统一 Filter 系统（Blur/Bloom/Pixelate/ColorMatrix 等）、WebGL/WebGPU、TypeScript 完整类型 
- Tiled/LDtk：Tiled 是一等公民（`Phaser.Tilemaps.Parsers.Tiled` 原生支持 TMX/JSON/base64/animated tileset/group layer/GID 翻转）；LDtk 需社区插件 https://github.com/mobilex1122/phaser-ldtk-importer（仍 alpha，只有一个维护者）
- 多人：无内置，配合 Colyseus / Playroom Kit / Geckos.io
- 成本：MIT，免费；官方提供 Phaser Editor v5 与 Phaser Game Agent（付费）
- 上手难度：中等，文档最丰富的 Web 2D 引擎
- **AI 友好度：极高**。v4 仓库自带 `skills/` 目录，含 28 个官方 AI Agent Skill 文件（覆盖每个子系统 + v3→v4 迁移），可直接把 Claude Code / Cursor 指向该目录。海量 Stack Overflow / 教程 / GitHub 示例已进入模型训练语料
- 优点：生态最厚、文档最全、性能顶级、AI 训练语料最多；缺点：v4 刚正式发布，插件生态还在迁移，纯 API，需要自己搭 JRPG 逻辑

### 2. Kaplay（Kaboom.js 继任者）
- 官网 / 仓库：https://kaplayjs.com/ ； https://github.com/kaplayjs/kaplay （1.7K stars）
- 版本：`3001.x` 稳定，`4000.0.0-alpha.22`（2025-10）
- API：`kaplay()` + 组件式（`sprite/pos/area/body/health/"tag"`），非常"块状"、贴近 Scratch 思维；`create-kaplay` 一条命令建项目
- TypeScript：内置类型；v4 新增 `KAPLAYOpt.types` / `kaplayTypes()` 支持强类型 scenes、`typed scenes`、`RuleSystem` / `DecisionTree` / `StateMachine`（内置敌人 AI）、逆运动学
- Tiled/LDtk：无原生，只能通过 `loadSprite` + 自己解析 JSON
- 多人：无内置
- 成本：MIT，免费；有官方 KAPLAYGround 在线编辑器 https://play.kaplayjs.com
- 上手难度：极低
- **AI 友好度：极高**。API 短、模式统一、示例 90+；Claude/Cursor 常能一次生成可运行代码。缺点是像素卡通/JRPG 风格的现成模板较少，且训练语料主要还是老 Kaboom
- 优点：vibe coding 天堂；缺点：非 JRPG 定位，做 tile-based JRPG 需要自己搭一切；WebGL 而非 WebGPU

### 3. PixiJS v8
- 官网 / 仓库：https://pixijs.com/ ； https://github.com/pixijs/pixijs
- 版本：v8.19（2026-06），WebGPU + WebGL2，纯渲染器
- Tilemap：`@pixi/tilemap` https://github.com/pixijs/tilemap（`Tilemap` / `CompositeTilemap`）
- LDtk：无官方插件；PixiJS React v8：https://react.pixijs.io/
- **AI 友好度：高**。2026 年 6 月起官方 blog 明确提供 "AI agent skills"；文档 llms.txt 已上线
- 优点：渲染最快、可自由造轮子、能与 React 无缝集成；缺点：只有渲染，"scene/camera/input/physics/tilemap 逻辑"都要自己拼；对个人开发者略重

### 4. RPG.js v5（专门 JRPG/MMORPG）
- 官网 / 仓库：https://rpgjs.dev/ ； https://github.com/RSamaium/RPG-JS （1.6K stars，v5 beta，v5 文档 https://v5.rpgjs.dev/）
- 定位：**唯一一个开箱即用的 2D JRPG/MMORPG TypeScript 框架**——一套代码同时跑单机 RPG 与 MMO
- 内置：地图作为多人 room、事件（NPC/宝箱/触发器）、玩家/装备/技能/存档、client-side prediction + server 权威、Vue GUI 叠加、i18n、依赖注入模块系统；渲染基于自家 CanvasEngine（Pixi 之上封装）
- Tiled：原生 TMX；开发用 Vite
- 多人：**内置**（基于 `@signe/room`/PartySocket，可跑在 Node/Express/Fastify/Hono）
- 成本：引擎 MIT 免费；可选 Studio $99 一次性买断（地图编辑器 + AI 素材 500 credits + 10GB）
- **AI 友好度：非常高（同类最高）**。官方发布了 AI Agent Skill：`npx skills add https://github.com/RSamaium/RPG-JS#v5`；README 明确写 "connect Codex/Claude Code/OpenCode agents"；Studio API 让 agent 直接创建地图、事件、物品、导出项目
- 优点：直接免掉 60% 的"从零写 JRPG"工作；缺点：v5 仍 beta，社区小于 Phaser，若需求偏离 JRPG 范式会掉出快车道

### 5. Melon.js
- 官网 / 仓库：https://melonjs.org/ ； https://github.com/melonjs/melonJS （6.3K stars，v19.4 于 2026-05）
- 特性：~150KB minzipped、ES6/TS、WebGL2 GPU tile 渲染、Tiled 一等公民（正交/等距/六边形/animated tileset）、内置 `Light2d` + 后处理 shader、物理/输入/音频/UI 全套
- 多人：无
- **AI 友好度：中**。文档不如 Phaser，训练语料相对少
- 优点：轻量、Tiled 集成最深；缺点：TypeScript 类型完整度不如 Phaser 4，社区活跃度中等

### 6. Excalibur.js
- 官网 / 仓库：https://excaliburjs.com/ ； https://github.com/excaliburjs/excalibur （2.3K stars）
- 定位：TypeScript-first、Actor/Scene 抽象、仍 pre-1.0（0.x）
- Tiled：官方插件 `@excaliburjs/plugin-tiled`
- **AI 友好度：中**。类型极好但样例少
- 优点：TS 体验干净；缺点：pre-1.0，API 可能破坏；训练语料偏少

### 7. ct.js
- 官网 / 仓库：https://ctjs.rocks/ ； https://github.com/ct-js/ct-js（v5.3，314 stars）
- 定位：桌面 IDE（NW.js）+ Pixi 运行时，视觉编辑器 + Catnip 可视化语言 / JS / TS / CoffeeScript
- **AI 友好度：低**。项目文件是 `.ict` 二进制/半结构化，AI 更难自动化编辑
- 优点：所见即所得；缺点：与"vibe coding 全交给 AI"路线相反

### 8. Godot 4 Web 导出
- https://docs.godotengine.org/en/latest/tutorials/export/exporting_for_web.html
- 现状：WASM 默认 ~33MB，需要 single-thread 模式绕过 COOP/COEP；C# 项目**不能**导出到 Web；GDScript AI 语料 << JS/TS
- 结论：不适合 Web 优先 + 个人 vibe coding 场景，跳过

### 9. Playroom Kit（多人 SDK，不是引擎）
- 官网 / 文档：https://joinplayroom.com/ ； https://docs.joinplayroom.com/
- 定位：serverless 多人层（room / player state / RPC / matchmaking），JS + React hooks + Unity；官方明确写 "vibe-coding friendly, works with Cursor / Replit / Lovable"
- 与 Phaser/Kaplay/RPG.js 全部可组合
- 免费额度：官方 free tier（详细阶梯需登录查看）；**不需要自建后端**
- **AI 友好度：极高**（有专门为 AI/vibe coding 打磨的 API 与文档）
- 优点：一个 `insertCoin()` 就跑起来；缺点：房间状态放在他们服务器，长期依赖第三方

### 10. Colyseus 0.17（多人服务器，非引擎）
- 官网 / 仓库：https://colyseus.io/ ； https://github.com/colyseus/colyseus （7K stars，0.17.x 现役）
- 定位：Node.js 权威服务器 + 房间 + 自动 delta 状态同步；SDK 覆盖 TS/React/Unity/Godot/GameMaker/Defold/Haxe
- React hooks 官方支持：`useRoom` / `useRoomState`
- 成本：MIT 免费自托管；Colyseus Cloud 付费
- **AI 友好度：高**。Schema/装饰器 API 简洁，示例多，Claude 能可靠生成 Room 代码

### 11. Rune SDK
- 现状：Rune 已将重心转向手机原生 SDK，Web 生态相对收缩，不建议再作为 Web 优先首选（相对 Playroom / Colyseus 现役势能弱）

### 12. React + Canvas 手撸
- 只推荐用来包一层 UI/HUD/聊天窗口（`react-pixi` 或 `pixi-react-v8` 挂 PixiJS）；核心地图和精灵渲染不建议纯 React state 驱动 —— 帧率与像素对齐会痛

---

## 二、Tiled vs LDtk（不是引擎但决定管线）

- Tiled（https://mapeditor.org）: 事实标准，被 Phaser 4 / melonJS / RPG.js 原生消费，AI 也几乎只知道 Tiled JSON 
- LDtk（https://ldtk.io）: 编辑器更现代/更友好，但 Phaser/melonJS 需社区插件；如果坚持 LDtk，建议 export "Super Simple" PNG+JSON 通用格式
- **强烈建议先选 Tiled**——AI 生成关卡加载代码的成功率最高

---

## 三、组合建议矩阵

| 目标                     | 引擎                       | 多人层            | 地图工具 | 上手速度 |
| ------------------------ | -------------------------- | ----------------- | -------- | -------- |
| **最快出 JRPG 原型**     | RPG.js v5                  | 内置              | Tiled    | ★★★★★    |
| 稳妥、生态最大化         | Phaser 4                   | Colyseus 0.17     | Tiled    | ★★★★     |
| Serverless / 极简后端    | Phaser 4 / Kaplay          | Playroom Kit      | Tiled    | ★★★★★    |
| 极简 vibe coding，非 JRPG| Kaplay                     | Playroom Kit      | 自解析   | ★★★★★    |
| 从零自定义、极致性能     | PixiJS v8 + Pixi React v8  | Colyseus          | Tiled    | ★★       |

---

## 四、最终推荐

### 首选：**RPG.js v5 + 内置多人 + Tiled**
理由：
1. **唯一原生匹配"JRPG + 多人 + Web"三合一**的开源方案；玩家/事件/地图 room/存档/i18n/预测同步全部现成，一个人可省下 1-2 个月造轮子时间
2. 官方 AI Agent Skill（`npx skills add https://github.com/RSamaium/RPG-JS#v5`）为 Claude Code / Cursor 直接提供领域知识，vibe coding 摩擦最低
3. TypeScript-first + Vite，模块化清晰；Node 端可挂 Express/Hono，跟 Vercel/Fly.io/自己 VPS 都好部署
4. 引擎 MIT，Studio 完全可选，锁定风险低
5. 缺点是 v5 尚 beta（可锁 5.0.x）；社区小 → 用 Phaser/Pixi 的通用知识可以补齐（因为其渲染底层就是 Pixi 系）

### 备选：**Phaser 4 + Colyseus 0.17 + Tiled + PixiJS React（HUD）**
理由：
1. 生态最厚、AI 训练语料最多、v4 内置 28 个官方 skills 目录、TilemapGPULayer 性能狂野
2. 万一 RPG.js 卡住某个非 JRPG 需求（如奇怪的相机、复杂 shader、非典型交互），Phaser 4 给你完全的表达自由
3. Colyseus 授权 (MIT) + 房间/schema/matchmaking 已经和 gather.town 类应用高度对齐，官方 React hooks 让聊天/UI 集成简单
4. 缺点：核心 JRPG 玩法（事件、对话、地图切换、AI NPC 行走）需要自己写——AI 能帮，但比 RPG.js 多写几周代码

### 保底建议
无论选哪个，都强制这三条：
- 地图必须用 Tiled（不是 LDtk）—— AI 生成成功率最大化
- 引擎语言必须 TypeScript —— Cursor/Claude 靠类型才能"不瞎写"
- 多人层与渲染引擎解耦（RPG.js 除外，因为它是一体化）—— 后期换引擎不动网络代码

Sources:
- https://phaser.io/news/2025/12/phaser-v4-release-candidate-6-is-out
- https://github.com/phaserjs/phaser/releases/tag/v4.0.0
- https://github.com/kaplayjs/kaplay
- https://kaplayjs.com/
- https://github.com/RSamaium/RPG-JS
- https://rpgjs.dev/
- https://v5.rpgjs.dev/advanced/node-server-production
- https://pixijs.com/blog
- https://excaliburjs.com/ ； https://github.com/excaliburjs/excalibur
- https://melonjs.org/ ； https://github.com/melonjs/melonJS
- https://ctjs.rocks/ ； https://github.com/ct-js/ct-js
- https://docs.godotengine.org/en/latest/tutorials/export/exporting_for_web.html
- https://amann.dev/blog/2025/godot_web_size/
- https://docs.colyseus.io/ ； https://github.com/colyseus/colyseus
- https://docs.joinplayroom.com/
- https://github.com/mobilex1122/phaser-ldtk-importer