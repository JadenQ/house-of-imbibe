Note: glm-5.2[1M] (the safety classifier) was unavailable when reviewing this subagent's work. Please carefully verify the subagent's actions and output before acting on them.

# 像素角色/头像自定义创建器 (Character Creator) 深度调研

## 一、核心开源模块化像素角色系统

### 1. Universal LPC Spritesheet Character Generator（LPC 系）
- **官方仓库**：https://github.com/LiberatedPixelCup/Universal-LPC-Spritesheet-Character-Generator （1300+ ★，社区活跃，2026 年仍在更新）
- **旧维护分支**：https://github.com/sanderfrenken/Universal-LPC-Spritesheet-Character-Generator （已迁移，422 ★）
- **在线 Demo**：https://liberatedpixelcup.github.io/Universal-LPC-Spritesheet-Character-Generator/
- **技术栈**：纯 JavaScript + HTML + CSS（还有 TypeScript 分支），无重型框架依赖，画布层叠 PNG → 导出 PNG spritesheet
- **图层**：body / head / eyes / nose / ears / hair / beard / clothes (torso/legs/feet) / armor / weapon / hat / accessories 一应俱全；标准姿态包含 walk（4 方向）/ cast / thrust / slash / shoot / hurt，最新 v3 扩展到 run / idle / jump / climb 等
- **画风**：LPC 标准像素画，64×64 tile，四方向行走——**这是 Pokémon Emerald 风 JRPG 事实上最接近的开源基线**
- **License**：CC-BY-SA 3.0/4.0 + GNU GPL 3.0 双许可，部分 CC0，**必须署名所有原始作者**（自带 CREDITS.csv）
- **成本**：完全免费
- **上手难度**：★★☆☆☆——现成的网页版就是一个可编辑器，直接 fork 就能跑；也可以只取 `spritesheets/` 目录当纯资源集
- **AI 友好度**：★★★★★——纯 JS + 目录规范化 + 每个部件独立 PNG，Claude Code 可以直接生成配置驱动式合成器；GPL 传染性会强制你的最终产品也开源

### 2. sanderfrenken/lpc-spritesheet-generator-v2
- **URL**：https://github.com/sanderfrenken/lpc-spritesheet-generator-v2
- 上面项目的重构版，Python + JS，License GPL-3.0；仅 5 ★，作者本人回流主仓，**不推荐单独使用**

### 3. SithJester's Character Sprite Generator
- **原站点已离线**，社区在 RPG Maker 官方论坛集中托管：https://forums.rpgmakerweb.com/threads/sithjesters-rmxp-resources.144609/
- **画风**：VX/XP 风，尺寸较小（约 32×32 头身），偏 SithJester 复古 JRPG
- **License**：需持有 RMXP，可用于商业项目，需署名 "Sithjester"
- **AI 友好度**：★★☆☆☆——纯散图 zip，没有拼装器代码，需要自己写合成层；且许可条款对非 RPG Maker 用户不够友好
- **结论**：作为**画风参考**可以，作为主资源不推荐——LPC 更现代、更完整、更明确开源

### 4. ElizaWy/LPC Revised Paradigm
- **URL**：https://github.com/ElizaWy/LPC
- LPC 分层规范的现代重整版，用于 LPC 拓展；作为 spec 参考

---

## 二、itch.io 上顶级 Modular Pixel Character 资源包（5+）

| # | 资源 & 作者 | 尺寸/画风 | License | 价格 | 备注 |
|---|---|---|---|---|---|
| 1 | **The Mana Seed "Character Base"** by Seliel the Shaper — https://seliel-the-shaper.itch.io/character-base | 32×32 头身，纸娃娃 (paper-doll) 分层，10 肤色，4 方向；含 idle/walk/run/push/pull/jump/till/water/fish/climb/blacksmith/sleep 等海量动作 | 自定 "Mana Seed User License"：一款游戏只需买一次，可用于商业游戏；**不可**用于 NFT/Web3/GenAI/实体商品（明确禁止 AI 训练） | 免费 demo + 完整版 $19.98；Classic Bundle $35.95 含扩展战斗包 | **画风最接近现代 Pokémon Emerald+Stardew**；作者对 GenAI 有敌意，需谨慎 |
| 2 | **Modern Interiors + Modern Exteriors Character Generator** by LimeZu — https://limezu.itch.io/moderninteriors 与 https://limezu.itch.io/modernexteriors | 16×16 & 32×32 & 48×48，写实现代 RPG 风 | 定制许可（≥$1.50）：可商用可修改，禁转售 | Name-your-price（付 ≥ $1.50 拿完整）；官方带**角色生成器工具**（100+ 服装、200 发型、80 配件、9 肤色） | **最接近现代都市 JRPG**，若你的画风偏 Pokémon 后期作品可能更合适；LimeZu 官方内置 character generator |
| 3 | **PixelVerse: Modular Heroes (Early Access)** by Creative Of SPD — https://creativeofspd.itch.io/ultimate-modular-pixel-character-base-early-access | 32×32，4 方向 top-down | **CC-BY 4.0** ✅（署名即可商用，可修改） | Name-your-price ≥ $2 | 每周持续更新；许可条款是 5 个里最干净的付费选项 |
| 4 | **Free CC0 Modular Animated Vector Characters 2D** by RGS_Dev — https://rgsdev.itch.io/free-cc0-modular-animated-vector-characters-2d | 2048×2048 矢量，非像素 | **CC0**（无条件商用） | Name-your-price（可白嫖） | 白色可染色 body parts；**注意：矢量、不是像素画** —— 需要自己做像素化后处理，不适合直接用 |
| 5 | **Modular 16×16 Pixel Character Pack** by player023 — https://player023.itch.io/modular-1616-pixel-character-pack | 16×16，明确 GBA 灵感 | 商用许可（≥$1） | $1（半价） | 头/身/腿/武器分离，**明确是 Game Boy Advance 风** —— 与你的目标画风最贴合 |
| 6 | **Modular Pixel Character Kit** by monapdx — https://monapdx.itch.io/modular-pixel-character-kit | 头肩像素肖像 | Name-your-price，作者未明确 CC 但声明"Free to use" | 免费 | 多发型多肤色，**适合聊天头像**（不适合行走精灵） |
| 7 | **Isometric Character Asset Pack** by Supernova Files — https://supernovafiles.itch.io/isometric-asset-pack | 等距 | **CC0** ✅ | Name-your-price | 若你走等距 Gather.town 路线可用 |

---

## 三、Web 端角色创建器 UI 可 fork 组件

| 项目 | URL | 画风/尺寸 | License | 适配度 |
|---|---|---|---|---|
| **OmegaCreations/CharaKit** | https://github.com/OmegaCreations/CharaKit/ | 通用（喂什么 sprite sheet 出什么） | **MIT** | ★★★★★ 通用 React 组件，喂入 LPC/Mana Seed sprite sheet 即可，含 `pixelScale`、多层 zIndex、导出 PNG/JPEG/WebP、config 导入导出 |
| **DracoBlue/retro-antlitz-kartei** | https://github.com/DracoBlue/retro-antlitz-kartei | 32×40 内建 8-bit 全身像素 | **MIT** | ★★★★☆ 完整 monorepo：`generator` + `animate`（idle/walk/attack）+ `react-editor`；含 seed 到 avatar 的确定性生成——**很接近你要的 "照片→字符串→像素" 融合思路** |
| **Nourivex/pixel-avatar-lib** | https://github.com/Nourivex/pixel-avatar-lib | 8-bit 头像 | Apache 2.0 | ★★★☆☆ DNA 字符串（`0-1-2-3-4-5`）驱动，NPM 直装，适合"聊天头像"，不适合行走精灵 |
| **TheCHARIITH/PixelPeeps** | https://github.com/TheCHARIITH/PixelPeeps | SVG 头像 | MIT | ★★☆☆☆ 确定性生成，只做头像，非像素 |
| **sultanpeyek/build-your-bandit** | https://github.com/sultanpeyek/build-your-bandit | Next.js + Canvas 单角色 pixel avatar | 未标 License（默认专有）——**不能直接抄** | ★★☆☆☆ 只作交互模式参考 |

---

## 四、用户自定义结果 → sprite sheet + walking animation

1. **合成方式**：把选中的每一层 PNG 按固定坐标平铺在 canvas 上（LPC 布局是 13 行 × 21 列 × 64px 网格），逐帧输出到一张大 PNG。LPC 生成器已经写好这套逻辑，直接抄 `main.js` 的 `getMergedImage()` 逻辑即可。
2. **导出**：`canvas.toBlob()` → 存 R2/Supabase Storage → 引擎（Phaser/PixiJS）作为 SpriteSheet 装载：
   ```js
   scene.load.spritesheet('me', url, {frameWidth:64, frameHeight:64});
   scene.anims.create({key:'walk-down', frames: scene.anims.generateFrameNumbers('me',{start:104,end:111}), frameRate:8, repeat:-1});
   ```
3. **网络同步**：只需要同步 `{layers: {body:'light', hair:'short_ash', top:'shirt_red', ...}}` 这个 JSON，其他客户端各自合成——**每个用户 <200 bytes 状态**，比传 sprite sheet 便宜 3 个数量级。

---

## 五、照片 → 像素融合方案（脸部/发色抽取 + 手动调其余）

Claude Code 可自动化的最小管线：

1. **前端上传** → 调用外部照片转像素模型（GPT-Image、Sora、Flux Pix2Pix Lora，或 Replicate 上的 `fofr/face-to-sticker`：https://replicate.com/fofr/face-to-sticker） → 得到一张 64×64 头像
2. **色卡抽取（纯前端可做，无需 AI）**：
   - 用 https://github.com/lokesh/color-thief 或 https://github.com/Vibrant-Colors/node-vibrant 从上传照片 dominant color
   - 上半脸区域取 skin tone，头顶三角区域取 hair color（简单 grabcut 或直接 y<20% 区域）
3. **注入到 LPC 生成器**：LPC 的每个 body/hair 层都是**灰度+可染色版本**（`recolor()` 用 HSL 偏移即可），直接把提取的色值套上去
4. **其余项（发型形状/衣服/配件）**：用 CharaKit 的组件让用户在剩下的选项里挑
5. **头像可选二路**：聊天头像用 Replicate 生的 64×64 face，行走精灵用 LPC 合成——**两套并行，一致性靠肤色/发色统一**

---

## 六、版权友好资源包一览（用于混搭）

- **CC0**：RGS_Dev 矢量包、Supernova 等距包、LPC 中标注 CC0 的部分部件、https://opengameart.org（大量 CC0/CC-BY tilesets & sprites）
- **CC-BY 4.0**：PixelVerse (Creative Of SPD)、部分 LPC 部件
- **CC-BY-SA 4.0**：LPC 主体（**传染性 copyleft，需衍生也 SA**）
- **GPL 3.0**：LPC 主体另一可选许可
- **定制商用**：Mana Seed（禁 NFT/GenAI）、LimeZu（禁转售，其余全开）

---

## 首选推荐

**「LPC 资源包 + OmegaCreations/CharaKit React 组件」组合**

- **资源**：https://github.com/LiberatedPixelCup/Universal-LPC-Spritesheet-Character-Generator 的 `spritesheets/` 目录（CC-BY-SA 3.0 + GPL 3.0）
- **拼装器**：https://github.com/OmegaCreations/CharaKit（MIT，Canvas 渲染，支持 pixelScale、多层 zIndex、导出 PNG、配置 JSON 导入导出）
- **确定性种子生成** 从 https://github.com/DracoBlue/retro-antlitz-kartei 借鉴 `configFromSeed()` 逻辑（MIT）

### 理由
1. **画风匹配**：LPC 是唯一在体量、动作种类、社区支持上都接近 Pokémon Emerald 风 JRPG 的**开源**资源库；商业替代 (Mana Seed / LimeZu) 更精致但许可含糊、且 Mana Seed 明确敌视 GenAI 流程
2. **AI 友好**：LPC 每个部件是独立 PNG + 严格网格，Claude Code 可以直接生成"从 JSON 到合成 sprite sheet"的代码；CharaKit 是纯 React + Canvas，AI 二次开发难度极低
3. **可扩展**：后续想混入 itch.io 商用包（PixelVerse CC-BY 或 player023 的 16×16 GBA 包）时，只需按 LPC 网格重切图即可，架构不动
4. **成本**：0 元起步，全程可离线；只需在游戏内 "Credits" 页保留 CREDITS.csv
5. **风险**：LPC 是 CC-BY-SA/GPL 双许可 —— 你的**画面合成结果**需保持 SA/GPL；若未来商业化且不想开源，就在启动初期切换到 PixelVerse (CC-BY 4.0)

### 工作量估算（单人 vibe coding + Claude Code）
- **Day 1**：fork LPC 生成器网页版，剥离掉不需要的服饰保留基础 5-7 层（body / hair / top / bottom / shoes / hat / accessory）——2-4 小时
- **Day 2**：接入 CharaKit（或直接改 LPC 原生 JS）作为 React 组件嵌入你的 Next.js 项目，导出 sprite sheet URL 存 R2/Supabase——3-6 小时
- **Day 3**：接入 `color-thief` 从上传照片抽 skin+hair 色，注入 LPC 的 palette recolor 流程——2-4 小时
- **Day 4**：把 selection JSON 通过 WebSocket 广播，其他客户端本地合成同一个 sheet——2-3 小时
- **Day 5**：接 Replicate `fofr/face-to-sticker` 做聊天头像的照片像素化路径——1-2 小时

**总计约 10-20 小时可产出一个可用原型**，产出物：完整分层选择器 + 4 方向行走动画 + 上传照片抽色 + 多端合成同步。

## Sources
- [LiberatedPixelCup/Universal-LPC-Spritesheet-Character-Generator](https://github.com/LiberatedPixelCup/Universal-LPC-Spritesheet-Character-Generator)
- [LPC Generator Live Demo](https://liberatedpixelcup.github.io/Universal-LPC-Spritesheet-Character-Generator/)
- [sanderfrenken 旧版仓库](https://github.com/sanderfrenken/Universal-LPC-Spritesheet-Character-Generator)
- [ElizaWy/LPC Revised Paradigm](https://github.com/ElizaWy/LPC)
- [Sithjester's RMXP Resources 合集帖](https://forums.rpgmakerweb.com/threads/sithjesters-rmxp-resources.144609/)
- [Mana Seed Character Base — Seliel the Shaper](https://seliel-the-shaper.itch.io/character-base)
- [Mana Seed User License](https://selieltheshaper.weebly.com/user-license.html)
- [Modern Interiors — LimeZu](https://limezu.itch.io/moderninteriors)
- [PixelVerse: Modular Heroes — Creative Of SPD](https://creativeofspd.itch.io/ultimate-modular-pixel-character-base-early-access)
- [Free CC0 Modular Animated Vector Characters — RGS_Dev](https://rgsdev.itch.io/free-cc0-modular-animated-vector-characters-2d)
- [Modular 16×16 Pixel Character Pack — player023](https://player023.itch.io/modular-1616-pixel-character-pack)
- [Modular Pixel Character Kit — monapdx](https://monapdx.itch.io/modular-pixel-character-kit)
- [Isometric Character Asset Pack — Supernova Files](https://supernovafiles.itch.io/isometric-asset-pack/devlog/1013194/pack-goes-cc0)
- [OmegaCreations/CharaKit](https://github.com/OmegaCreations/CharaKit/)
- [DracoBlue/retro-antlitz-kartei](https://github.com/DracoBlue/retro-antlitz-kartei)
- [Nourivex/pixel-avatar-lib](https://github.com/Nourivex/pixel-avatar-lib)
- [TheCHARIITH/PixelPeeps](https://github.com/TheCHARIITH/PixelPeeps)
- [color-thief](https://github.com/lokesh/color-thief)
- [node-vibrant](https://github.com/Vibrant-Colors/node-vibrant)
- [Replicate fofr/face-to-sticker](https://replicate.com/fofr/face-to-sticker)