# 瓦片地图（Tilemap）设计与工具调研

## 一、编辑器对比

### 1. Tiled Map Editor（www.mapeditor.org）
- **用途**：业界事实标准的 2D 瓦片地图编辑器，Godot/Phaser/Unity/PixiJS/melonJS 均原生支持其格式。
- **导出格式**：`.tmx`（XML）、`.tmj`/`.json`（JSON，Web 项目首选）、`.lua`、CSV；支持多图层（Tile Layer / Object Layer / Image Layer / Group）、自定义属性、动画瓦片、地形（Terrains/Wang tiles）、Infinite map。
- **优点**：格式稳定十几年，生态最大；JSON 结构清晰，**Claude/Cursor 可直接生成或修改**；命令行 `tiled --export-map` 可自动化；有 JS/Python 脚本扩展 API。
- **缺点**：UI 稍显传统，Object 逻辑要靠自定义属性。
- **成本**：完全免费（GPL 版）或 itch.io 上打赏；商用无授权限制。
- **上手难度**：★★☆☆☆
- **AI 友好度**：★★★★★ — 其 JSON schema 已在 Claude 训练语料中大量出现，让它输出 `.tmj` 几乎零幻觉。

### 2. LDtk – Level Designer Toolkit（ldtk.io，作者 Deepnight，Dead Cells 团队）
- **用途**：现代化 2D 关卡编辑器，主打"层规则（Auto-Layer Rules）"—— 只画一层就能自动出边缘、阴影、装饰。
- **导出格式**：`.ldtk`（JSON，schema 公开且带官方 TypeScript 定义 `ldtk.d.ts`），可选简化版 `simplified` JSON。
- **优点**：Entity + Field 系统天然适合 RPG NPC/触发器；Auto-Tiling 规则强大，省 tileset 制作时间；有 [ldtk-ts](https://github.com/estivo/ldtk-ts)、`iLDtk`（Phaser 3 插件）、[phaser-ldtk](https://github.com/Phaser-Ldtk-Importer)。
- **缺点**：格式相对新，社区示例少于 Tiled；碰撞需自己约定字段。
- **成本**：MIT，完全免费。
- **上手难度**：★★★☆☆
- **AI 友好度**：★★★★☆ — schema 公开且带官方 JSON Schema 文件（可直接喂给 Claude 做结构约束），但生态样本比 Tiled 少。

### 3. Ogmo Editor 3（ogmo-editor-3.github.io）
- **用途**：轻量 JSON 关卡编辑器，Matt Thorson（Celeste 作者）出品。
- **导出格式**：`.json`，格式极简。
- **优缺点**：简单直接，但 Auto-Tile、Wang、动画等高级特性欠缺；停更多年，生态最小。
- **AI 友好度**：★★★☆☆ — 格式简单易生成，但可直接用的运行时/加载器很少。
- **结论**：仅推荐单人极简项目，本项目不选。

---

## 二、推荐的免费 GBA / 16-bit Tileset（重点资源）

| # | 名称 | 作者 | 授权 | 链接 / 备注 |
|---|------|------|------|------|
| 1 | **Modern Exteriors / Modern Interiors / Serene Village** | LimeZu | 免费个人+商用（署名，禁止 NFT），付费版扩包 | limezu.itch.io — **像素风格最接近现代 Gather.town / JRPG 融合**，48×48 巨型包，含大量家具、NPC、动画帧 |
| 2 | **Cute Fantasy RPG** | Kenmi | Free / Pro | kenmi-art.itch.io/cute-fantasy-rpg — 干净可爱风，16×16 |
| 3 | **Ninja Adventure Asset Pack** | Pixel-Boy & AAA | CC-BY 4.0 | pixel-boy.itch.io/ninja-adventure-asset-pack — 巨型免费包，含 tileset、精灵、UI，**风格极接近宝可梦** |
| 4 | **Tiny 16 / Tiny 16: Basic** | Lanea Zimmerman (Sharm) | CC-BY 3.0 | opengameart.org/content/tiny-16-basic — OpenGameArt 老牌高质量 16×16 |
| 5 | **LPC – Liberated Pixel Cup 合集** | 多作者 | CC-BY-SA 3.0 / GPL 3 | opengameart.org/lpc — 上千角色/服装可换装组合，**天然适合"自定义头像换装"需求** |
| 6 | **Roguelike/RPG pack** | Kenney | CC0 | kenney.nl/assets/roguelike-rpg-pack — 完全公共领域，无授权风险；风格更"顶视方块"，不算 GBA 风，但可作占位 |
| 7 | **Mystic Woods** | Game Endeavor | Free / Paid | game-endeavor.itch.io/mystic-woods — 森林/村庄场景精美，16×16 |
| 8 | **Cozy People / Cozy Farm** | Shubibubi / Sithjester | 免费署名 | RPG Maker 系列老资源，OpenGameArt 亦有搬运 |

> **授权提醒**：LimeZu 与 Ninja Adventure 是"美感 vs 简单授权"的最佳平衡；Kenney 系列 CC0 是**你不确定授权时的兜底默认**；LPC 因是 CC-BY-SA，用了会传染整个资源目录，谨慎评估。

---

## 三、AI 生成 tileset 的可行性

**结论：目前不推荐直接让扩散模型出可拼接 tileset。** 关键难点：
1. Stable Diffusion / DALL·E 输出边缘不对齐、色板漂移，无法做无缝拼接。
2. 需要严格约束 grid、palette、tile-to-tile 连续性——传统模型没有这些先验。

**可用的折中方案**：
- **[Retro Diffusion](https://retrodiffusion.ai/)** — 专做像素艺术的扩散模型，支持 tileable/palette 锁定，有 API，能生成对齐的 16×16/32×32 tile；商业订阅 $8+/月。
- **[PixelLab.ai](https://www.pixellab.ai/)** — 生成 sprite / 简单 tile，有编辑器和 API，MCP-ready 潜力好。
- **[Scenario.gg](https://www.scenario.gg/)** — 训练自定义 LoRA，可以喂 15 张 Ninja Adventure 图训练风格一致的 tileset 补丁。
- **手工流**：从上面免费包中**用 Aseprite / Piskel 拼接改色**，让 Claude 输出色板转换脚本（Python + PIL）比让它画瓦片可靠得多。
- **风格迁移已有素材**：让 Cursor 写 `paletteRemap.py`，把 LimeZu 的调色板全部映射到 GBA 15-bit RGB555，得到统一 GBA 感。

**AI 最擅长的 tileset 任务**：色板归一、tile 切分/去重、autotile 规则生成、bitmask 转 wang-tile 索引——这些是**代码级**任务，让 Claude 写。

---

## 四、程序化生成（WFC / PCG）

### JS/TS 主推
1. **wavefunctioncollapse (npm) by kchapelier** — `github.com/kchapelier/wavefunctioncollapse`，作者是 mxgmn WFC 原始算法的第一批移植者，**Overlapping + Tiled** 模型都支持；接口简单，Claude 会写。
2. **wfc-tool / mxgmn/WaveFunctionCollapse** — `github.com/mxgmn/WaveFunctionCollapse`（原始 C#），仅参考用，AI 可读其示例 XML。
3. **[fast-wfc](https://github.com/math-fehr/fast-wfc)** WebAssembly 绑定 — 性能好，适合大图。
4. **[boris-marinov/collapse](https://github.com/BorisTheBrave/DeBroglie)** DeBroglie —— 功能最强（backtracking、约束区域），有 CLI，可离线跑；**推荐用它离线生成好 map，Web 端只加载 JSON**。
5. **rot.js** — `ondras.github.io/rot.js/`：dungeon / cave / uniform / digger，虽是 roguelike 风格，但生成结构再让 WFC 贴皮很实用。
6. **[Poisson-disk / mikola-lysenko/ndarray]** — 用于随机撒点（NPC 起始位置、树木）。

### 推荐工作流
1. **手绘 12–20 张原型瓦片小样**（或从 Ninja Adventure 抽），标好邻接约束。
2. 用 **DeBroglie CLI** 输入约束 JSON、tileset PNG，输出 `map.png` + `map.json`。
3. 用 Claude 写一个 `wfc-to-tiled.ts`，把 DeBroglie 结果转成 **Tiled `.tmj`** 或 **LDtk `.ldtk`**，就能立刻在编辑器里手工润色。

---

## 五、让 Claude 输出 Tiled 兼容 JSON 的实践

- Tiled `.tmj` schema 已由 Claude 训练时看过大量样本，直接给它 prompt：
  > "生成一个 30×20 的 Tiled 1.10 map JSON，tileset 引用 `serene-village.tsx`（firstgid=1），有 `ground` `objects` `collision` `overhead` 四层，草地为主中间有一条 3 格宽的路穿过村庄，边缘用 tile id 34/35/36 组成 wang 边缘。"
- 让 Claude 生成 **wang tile 索引表** / **autotile 规则**（bitmask → tile id），比让它逐格画像素图靠谱 10 倍。
- 用 **JSON Schema 校验**：从 `github.com/mapeditor/tiled/tree/master/util/schema` 拿 schema，喂给 Claude 做结构强约束。
- LDtk 提供官方 [ldtk.json schema](https://ldtk.io/json/) —— **AI 友好度高于 Tiled**（结构完全声明式）。

---

## 六、光照 / 深度层 / 遮挡

- **图层约定**（Tiled / LDtk 通用）：
  - `ground`（不透明底层）
  - `decoration`（草、花，不阻挡）
  - `collision`（不渲染，仅碰撞几何）
  - `overhead` / `over-player`（树冠、屋檐——y 排序时永远在角色之上）
  - `light`（半透明遮罩，混合叠加，用于夜晚/室内）
- **Y 排序**：可动物体（角色、NPC、大树干）按 y 坐标动态插入到 overhead 之下 —— **Phaser 3 用 `scene.children.depth = y` 就够**。
- **光照**：
  - 简单方案：`light` 层放 128×128 径向渐变 PNG，`multiply` 混合。
  - 进阶方案：**[phaser3-rex-plugins/lightMask](https://rexrainbow.github.io/phaser3-rex-notes/)** 或 pixi-lights，可实时探照。
  - Shader 方案：全局 tint + point light（Claude 写 GLSL 30 行）。
- **深度/遮挡实践**：LimeZu 类资源已经预切好 tall-object 上下两半，直接分别放 `collision`（下半）与 `overhead`（上半），无需运行时切割。

---

## 七、首选组合推荐（一人 vibe coding 优选）

**最终推荐：LimeZu 免费包 + LDtk 编辑器 + DeBroglie WFC + Phaser 3 (或 PixiJS) + Claude 自动化脚本**

理由：
1. **美术零门槛**：LimeZu 的现代小镇风格既有 Gather.town 的社交感又保留 JRPG 温度，直接商用；不够用时再叠 Ninja Adventure（授权兼容）。
2. **编辑器**：LDtk 的 **Auto-Layer Rules 让你只画 collision 就自动出草地/边缘/阴影**——这对一个人做原型是决定性的。JSON schema 完备，Claude 写规则表和加载器都精准。
3. **PCG**：DeBroglie（跑 CLI 离线）→ 你审美通过的关卡导出 → Claude 写 `debroglie-to-ldtk.ts` 转换器 → 在 LDtk 内二次美化。
4. **AI 友好度**：LDtk 官方 JSON Schema + 现成 TS 类型，让 Cursor/Claude 生成/修改关卡数据几乎零幻觉；比全流程手工在 Tiled 拖拽快 5–10 倍。
5. **兜底**：如果 LDtk 生态遇到坑，5 分钟就能切回 Tiled——因为地图逻辑本身就是"tile 数组 + entity 列表"，可以让 Claude 写转换器双向互通。

**备胎**：若你更看重生态成熟度（尤其打算之后用 Godot 或 melonJS），把 LDtk 换成 **Tiled**，其他不变。