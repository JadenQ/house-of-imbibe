# 像素动画、音效、字体与打磨工具调研

> **调研日期**：2026-07-27
> **状态**：**部分过期** —— PixelLab 一行的定价与 MCP 包名已在 2026-08-01 核实并更正，
> 见 [`../reference/pixellab-api.md`](../reference/pixellab-api.md)。
> 音效/字体/shader/素材包部分**未核实**但结论仍适用。

## 一、像素动画工具

### 1.1 桌面/编辑器类

| 工具 | 价格 | 优点 | 缺点 | AI 友好度 |
|---|---|---|---|---|
| **Aseprite** (https://www.aseprite.org/) | $19.99（Steam / itch.io 一次性买断，源码 MIT 但需自行编译免费用） | 行业标准；时间轴、洋葱皮、动画标签、9-slice、色板、Tilemap；CLI 支持（`aseprite -b input.ase --sheet out.png --data out.json`）；LUA 脚本 | 一次性付费 | ★★★★★ CLI + JSON 输出，Claude 可完全脚本化生成 sprite sheet |
| **LibreSprite** (https://libresprite.github.io/) | 免费开源（Aseprite v1.1 fork） | 完全免费，功能覆盖 Aseprite 80% | UI 略旧；新特性慢；无 tilemap 模式 | ★★★★ 有 CLI |
| **Piskel** (https://www.piskelapp.com/) | 免费（网页 + Electron） | 零安装，浏览器直接画；导出 GIF/PNG sheet | 无 CLI；功能简单；无色板管理 | ★★ 只能手动 |
| **Pixelorama** (https://orama-interactive.itch.io/pixelorama) | 免费开源（Godot 引擎写的） | 跨平台；3D 转 2D；洋葱皮；tilemap；活跃更新 | 生态比 Aseprite 小 | ★★★ 有 CLI 但文档少 |
| **Lospec Pixel Editor** (https://lospec.com/pixel-editor/) | 免费网页 | 零门槛；集成 Lospec palette 库 | 功能最少 | ★ |

**首选：Aseprite $19.99** — 值这个钱，CLI + LUA + JSON 输出让 Claude Code 可以「读 .ase → 修改 → 导出 sheet」全自动化；预算硬性为 0 时用 **LibreSprite**。

### 1.2 4-frame walk cycle 生成方案

- **模板法（最快）**：https://opengameart.org/ 搜 "walk cycle template"，或直接用 **Universal-LPC-Spritesheet-Character-Generator**（https://sanderfrenken.github.io/Universal-LPC-Spritesheet-Character-Generator/）— 网页选发型/衣服/肤色，直接下载 64×64 8方向行走 sprite sheet，CC-BY-SA 3.0 / GPL 3.0 授权。GBA 感稍强，配色微调即可。
- **教程**：
  - Saint11 免费系列 https://saint11.org/blog/pixel-art-tutorials/（含 walk cycle 帧解剖）
  - MortMort YouTube channel（GBA 风格教学）
- **AI 补帧**：Aseprite 官方 "In-Between Frames" 脚本 + PixelLab 的 Rotate/Animate skill（见 1.3）

### 1.3 AI 生成 sprite 动画

| 工具 | 价格 | 能力 | AI 友好度 |
|---|---|---|---|
| **PixelLab.ai** (https://www.pixellab.ai/) | **按次计费**（4 方向角色 $0.0105 起；动画 $0.0129/**方向** 起）；免费试用 40 次；有 **Aseprite 插件** + **官方 MCP server**（`https://api.pixellab.ai/mcp`，65 工具） | 文生 sprite、rotate（4/8 方向）、animate（walk/attack/idle 帧生成）、style transfer、tileset | ★★★★★ **有 MCP**，Claude 可直接调用（⚠️ 仅开发期） |

<!-- ❌ 2026-08-01 核实更正：
     (1) 原写"免费额度 ~100 credits/月；Pro ~$10/月"——与 API 按次计费口径混淆了。
         API 是按端点/尺寸按次计费，见 ../reference/pixellab-api.md §七。
         订阅档位本身未能核实（pricing 页客户端渲染，抓不到）。
     (2) 原写 MCP 包名 `@pixellab/mcp`——错误。官方是托管 HTTP 端点
         https://api.pixellab.ai/mcp，仓库为 pixellab-code/pixellab-mcp，无该 npm 包名。
     (3) 动画是**按方向**计费的，4 方向 = 4 倍价格，原文未提。
     (4) MCP 不能用于面向终端用户的网页。 -->
| **Retro Diffusion** (https://www.retrodiffusion.ai/) | 免费 200 credits 首月，$6/月起 ⚠️ 未核实（research/01 写"预付余额无订阅"、research/02 写"$8/月 1000 张"，三处矛盾） | Stable Diffusion 微调专攻像素；ComfyUI 节点；有 API | ★★★★ API + ComfyUI |
| **Scenario.gg** (https://www.scenario.gg/) | $19/月起 | 训练自己的风格 LoRA；生成 asset 集 | ★★★ REST API |
| **Pixel It** (https://giventofly.github.io/pixelit/) | 免费开源 | 照片 → 像素化（无 AI，纯降采样 + 调色板量化） | ★★★★★ JS 库可直接嵌入项目 |

**上传照片 → 像素卡通头像** 的核心链：
1. 前端调用 **Replicate** 或 **fal.ai** 上的 pixel-art LoRA（如 `nerijs/pixel-art-xl`、`retro-diffusion-xl`）出粗图 → 
2. **Pixel It** / **ImageMagick +Riemersma dither** 强制降采样到 32×32 或 64×64 → 
3. 映射到 **DB16 / GB Studio palette**（https://lospec.com/palette-list/nintendo-gameboy）确保色板一致。

---

## 二、Chiptune 音乐工具

| 工具 | 授权 | 特色 | 学习曲线 |
|---|---|---|---|
| **BeepBox** (https://www.beepbox.co/) | 免费 MIT，网页版 | 5 分钟出一首环境曲；分享链接就是曲谱；导出 WAV/MP3/MIDI | 极低 |
| **JummBox** (https://jummb.us/) | BeepBox 增强分支 | 更多乐器/音色，FM 合成；GBA 风更接近 | 低 |
| **ChipTone** (https://sfbgames.itch.io/chiptone) | 免费 | 音效（SFX）— 跳跃、拾取、打击、UI 点击 | 极低 |
| **jsfxr / sfxr** (https://sfxr.me/) | 免费开源 | 单文件网页 SFX 生成器；一键 "Pickup"/"Hit" | 极低 |
| **Bosca Ceoil Blue** (https://yurisizov.itch.io/boscaceoil-blue) | 免费 Godot 重写版 | Terry Cavanagh 原版翻新；网格式作曲；GBA 味浓 | 低 |
| **FamiStudio** (https://famistudio.org/) | 免费开源 | NES 硬件精确；导出 NSF/WAV；Piano roll 现代 | 中 |
| **Furnace** (https://github.com/tildearrow/furnace) | 免费开源 | 支持 GB DMG、GBA 直接的 芯片模拟（含 Game Boy 4 声道）| 中 |
| **SunVox** (https://warmplace.ru/soft/sunvox/) | 免费桌面 / $6 移动 | 模块化合成，功能爆炸 | 高 |

**AI 音乐 GBA BGM**：
- **Suno v4** (https://suno.com/) — $10/月 Pro；prompt 用 `"chiptune, 8-bit, Game Boy Advance, JRPG town theme, upbeat, 4-channel, Pokemon Emerald style, no vocals, loopable, 120bpm"`
- **Udio** (https://www.udio.com/) — $10/月，质量与 Suno 齐平
- **MusicGen** (https://huggingface.co/facebook/musicgen-melody) — 开源；Replicate 一次 $0.006；chiptune 品质弱于 Suno，但可离线批量
- **Stable Audio 2.0** — 短音效强

**首选组合**：BGM 用 **Suno** 生成 → 导出 WAV → **Furnace / FamiStudio** 转成 GB 4 通道版本（可选，保 GBA 真实感）；SFX 用 **jsfxr**（可脚本化：URL query 参数即完整音效定义）。

---

## 三、像素字体（中文/日文支持是关键）

| 字体 | 授权 | 中/日支持 | 像素尺寸 | 链接 |
|---|---|---|---|---|
| **Fusion Pixel Font** 缝合像素字体 | OFL 1.1 | 中日韩全覆盖，思源合并 | 8/10/12 px | https://github.com/TakWolf/fusion-pixel-font |
| **Ark Pixel Font** 方舟像素字体 | OFL 1.1 | 中日韩，作者亲绘 | 10/12/16 px | https://github.com/TakWolf/ark-pixel-font |
| **Zpix (最像素)** | OFL 1.1 | 简繁日 | 12 px | https://github.com/SolidZORO/zpix-pixel-font |
| **Boutique Bitmap 9x9 / 7x7** | OFL 1.1 | 简繁 + 日文假名 | 9/7 px | https://github.com/scott0107000/BoutiqueBitmap9x9 |
| **Cubic 11 / 方舟像素 11** | OFL 1.1 | 简繁 | 11 px | https://github.com/ACh-K/Cubic-11 |
| **Galmuri (韩)** | OFL 1.1 | 中日韩 | 7/9/11 px | https://github.com/quiple/galmuri |
| **VonwaonBitmap** | OFL 1.1 | 简繁 | 12/16 px | https://github.com/Vonwaon/VonwaonBitmap |
| **DamienG PixelFont** 系列 | 免费商用 | 仅拉丁 | 5/6/8/10 px | https://damieng.com/typography/zx-origins/ |
| **Press Start 2P** (Google Fonts) | OFL | 仅拉丁 | 8 px | https://fonts.google.com/specimen/Press+Start+2P |
| **Pixelify Sans** | OFL | 拉丁扩展 | 可变 | https://fonts.google.com/specimen/Pixelify+Sans |

**首选组合**：
- 中文界面 / 对话：**Fusion Pixel 12px**（覆盖最广，OFL 可自由嵌入 Web）+ **Ark Pixel 12px** 备选  
- 英文/数字/UI 标题：**Press Start 2P** 或 **Pixelify Sans**  
- 小号 UI 标签：**Boutique Bitmap 9x9**  
- Web 使用 `@font-face` + `font-display: swap`，woff2 化后 Fusion Pixel 全字集 ~2MB，可按 subset 分片。

---

## 四、CRT / 扫描线 / GBA LCD 后处理 shader

| 资源 | 特点 | 授权 | 链接 |
|---|---|---|---|
| **CRT-Royale** (libretro 官方，最真实) | 阴罩、扫描线、rgb 遮罩 | GPL | https://github.com/libretro/slang-shaders/tree/master/crt |
| **CRT-Geom / CRT-Lottes** | 轻量 | 免费 | 同上 |
| **AGB001 / GBA LCD shader** (libretro) | **专为 GBA 屏幕**：网格化像素 + 轻扫描 | 免费 | https://github.com/libretro/glsl-shaders/tree/master/handheld |
| **crt.pi** | 树莓派轻量 | GPL | libretro repo |
| **Godot CRT shader by pend00** | Godot 直接用 | MIT | https://godotshaders.com/shader/vhs-and-crt-monitor-effect/ |
| **three.js / postprocessing CRT pass** | Web 端；`postprocessing` 库的 `ScanlineEffect` + 自定义 CRT | MIT | https://github.com/pmndrs/postprocessing |
| **glsl-crt** by mattdesl (Web) | 单文件 GLSL，可粘到 Phaser/Pixi 的 filter | MIT | https://github.com/mattdesl/glsl-fxaa (参考同作者) |
| **PixiJS filters** | `@pixi/filter-crt`、`@pixi/filter-old-film` | MIT | https://github.com/pixijs/filters |

**Web 项目最佳路径**：如果渲染层用 **PixiJS**，直接 `import { CRTFilter } from '@pixi/filter-crt';` 一行完事；用 **Phaser 3**，套 `pipeline: 'CRT'` 自定义管线；纯 canvas / three.js 时用 `postprocessing` 的 ScanlineEffect + AGB001 shader 移植。

**GBA 真实感三件套**：① AGB001 LCD 网格 ② 轻微扫描线（scanline strength 0.15） ③ 冷色 LUT（GBA 屏幕偏冷）— GBA 原生分辨率 240×160，画布强制 4×–6× 整数缩放，禁用 mipmap / linear filter。

---

## 五、itch.io / Kenney.nl GBA 风格资源包 Top 10

1. **Kenney - 1-Bit Pack / RPG Urban / Tiny Town** — https://kenney.nl/assets — CC0，商用无忧，量大质稳
2. **Modern Interiors + Modern Exteriors** by LimeZu — https://limezu.itch.io/moderninteriors — 16×16 pixel，GBA 味最正的现代都市；免费 demo + 付费全集
3. **Cozy People Asset Pack** by Shubibubi — https://shubibubi.itch.io/cozy-people — 高质量角色 + walk cycle
4. **Ninja Adventure Asset Pack** by pixel-boy — https://pixel-boy.itch.io/ninja-adventure-asset-pack — 免费 CC-BY 4.0，含 tileset + 角色 + BGM，纯 GBA JRPG 感
5. **Mystic Woods** by Game Endeavor — https://game-endeavor.itch.io/mystic-woods — 免费版 + $5 pro
6. **Sprout Lands** by Cup Nooble — https://cupnooble.itch.io/sprout-lands-asset-pack — 星露谷/GBA 风
7. **Tiny Swords / Tiny Wonder Adventure** by Pixel Frog — https://pixelfrog-assets.itch.io/ — 免费高质
8. **Universal-LPC-Spritesheet-Generator** — https://sanderfrenken.github.io/Universal-LPC-Spritesheet-Character-Generator/ — **网页版直接换装出角色**，本项目核心特性 (2) 最佳基座
9. **Time Fantasy** 系列 by Finalbossblues — https://finalbossblues.com/timefantasy/ — $10 级付费，Pokemon Emerald 风还原度极高
10. **Pokémon-style tileset & sprites bundles**（itch.io 搜 "GBA RPG tileset"）— 例如 **PixelArchipel / Pipoya Free RPG Character Sprites** — https://pipoya.itch.io/pipoya-free-rpg-character-sprites-32x32 — 免费 32×32 四方向

---

## 六、我的首选「资产工具箱」组合（一人+AI 快速起步）

**总预算：≈ $30 一次性 + $10/月 可选**

| 用途 | 选择 | 花费 |
|---|---|---|
| 像素编辑 + 动画 + CLI | **Aseprite** | $19.99 一次 |
| 角色换装原型（用户创建） | **LPC Spritesheet Generator**（fork 内嵌到你的 Web 端） | 免费 |
| 用户照片 → 像素头像 | **Replicate 上 `retro-diffusion` LoRA** + **Pixel It** JS 后处理 + **DB16 调色板量化** | $0.003–0.02 / 张 |
| AI 生成/补动画 | **PixelLab.ai** + 官方 **MCP server**（让 Claude Code 直接调，仅开发期） | 按次计费：$0.0129/方向 起；40 次免费试用 |
| SFX | **jsfxr** (URL 即参数，可代码化) | 免费 |
| BGM | **Suno v4** 生成 → **BeepBox / Furnace** 二次修饰 | $10/月，或先用免费额度 |
| 中文字体 | **Fusion Pixel 12px** (主) + **Boutique Bitmap 9x9** (UI) | 免费 OFL |
| 英文字体 | **Press Start 2P** | 免费 OFL |
| Web CRT/LCD 滤镜 | **PixiJS + @pixi/filter-crt** + AGB001 GLSL 移植 | 免费 |
| 基础 tileset / 角色 | **Ninja Adventure Pack** + **LimeZu Modern Interiors** + **Pipoya Free RPG Sprites** | 免费 + $10 可选付费扩展 |
| 素材站 | Kenney.nl (CC0)、itch.io、OpenGameArt.org、Lospec.com | 免费 |

**理由**：
1. **Aseprite + PixelLab MCP** 是 Claude Code 可自动化的最强像素动画栈；前者提供确定性 CLI，后者提供 AI 生成，两者对 Claude 都是「结构化输入 + 结构化输出」。
   ⚠️ **2026-08-01 补充**：这个组合是**你自己出素材**用的。**用户在页面里生成**必须另走
   REST v2 + 后端代理（异步 5–9 分钟），MCP 在浏览器里无法使用。见
   [`../reference/pixellab-api.md`](../reference/pixellab-api.md) §一、§十一。
2. **LPC 生成器**几乎直接可满足项目特性 (2)「用户换装/发型/颜色」，fork 后把它嵌入前端 React/Vue，避免自己画上百个部件。
3. **Fusion Pixel 12px** 是目前中日韩覆盖最完整的开源像素字体，OFL 授权对商业 Web 部署零风险，直接 subset 上 CDN。
4. **Suno + jsfxr** 是 AI + 参数化的最省时间组合，你不需要懂 tracker 也能出成品；后续再用 Furnace 做「真 GBA 4 通道」版本作为质感升级。
5. **PixiJS + CRTFilter** 一行接入 Web，性能好，比 three.js 后处理链轻十倍——原型阶段最重要的是「立刻看到 GBA 味」。

Sources:
- [Aseprite](https://www.aseprite.org/)
- [LibreSprite](https://libresprite.github.io/)
- [Pixelorama](https://orama-interactive.itch.io/pixelorama)
- [Piskel](https://www.piskelapp.com/)
- [PixelLab.ai](https://www.pixellab.ai/) / [PixelLab OpenAPI](https://api.pixellab.ai/v2/openapi.json) ⭐ 2026-08-01 核实依据
- [Retro Diffusion](https://www.retrodiffusion.ai/)
- [Scenario.gg](https://www.scenario.gg/)
- [BeepBox](https://www.beepbox.co/) / [JummBox](https://jummb.us/)
- [ChipTone](https://sfbgames.itch.io/chiptone) / [jsfxr](https://sfxr.me/)
- [Bosca Ceoil Blue](https://yurisizov.itch.io/boscaceoil-blue)
- [FamiStudio](https://famistudio.org/) / [Furnace](https://github.com/tildearrow/furnace) / [SunVox](https://warmplace.ru/soft/sunvox/)
- [Suno](https://suno.com/) / [Udio](https://www.udio.com/) / [MusicGen](https://huggingface.co/facebook/musicgen-melody)
- [Fusion Pixel Font](https://github.com/TakWolf/fusion-pixel-font) / [Ark Pixel](https://github.com/TakWolf/ark-pixel-font) / [Zpix](https://github.com/SolidZORO/zpix-pixel-font) / [Boutique Bitmap](https://github.com/scott0107000/BoutiqueBitmap9x9) / [Cubic 11](https://github.com/ACh-K/Cubic-11) / [VonwaonBitmap](https://github.com/Vonwaon/VonwaonBitmap)
- [Press Start 2P](https://fonts.google.com/specimen/Press+Start+2P) / [Pixelify Sans](https://fonts.google.com/specimen/Pixelify+Sans)
- [libretro slang-shaders (CRT/GBA AGB001)](https://github.com/libretro/slang-shaders) / [glsl-shaders handheld](https://github.com/libretro/glsl-shaders/tree/master/handheld)
- [PixiJS filters](https://github.com/pixijs/filters) / [postprocessing (three.js)](https://github.com/pmndrs/postprocessing) / [godotshaders CRT](https://godotshaders.com/shader/vhs-and-crt-monitor-effect/)
- [Kenney.nl](https://kenney.nl/assets) / [LimeZu Modern Interiors](https://limezu.itch.io/moderninteriors) / [Ninja Adventure Pack](https://pixel-boy.itch.io/ninja-adventure-asset-pack) / [Sprout Lands](https://cupnooble.itch.io/sprout-lands-asset-pack) / [Pixel Frog](https://pixelfrog-assets.itch.io/) / [Pipoya Sprites](https://pipoya.itch.io/pipoya-free-rpg-character-sprites-32x32) / [Time Fantasy](https://finalbossblues.com/timefantasy/) / [LPC Generator](https://sanderfrenken.github.io/Universal-LPC-Spritesheet-Character-Generator/)

## 修订记录

- **2026-08-01**：核实 PixelLab v2 API。更正：删除错误的 MCP npm 包名
  `@pixellab/mcp`（实为托管端点 `https://api.pixellab.ai/mcp`）；
  修正定价口径（"~100 credits/月 + Pro $10/月" → 按次计费，附精确价格）；
  补充「动画按方向计费」与「MCP 仅限开发期」；
  标注 Retro Diffusion 定价在三份文档中互相矛盾且均未核实。