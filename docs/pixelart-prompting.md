# PixelLab 提示词写法指南（GBA / 绿宝石画风）

> **核实日期**：2026-08-04
> **核实方式**：
> - `curl -sL https://api.pixellab.ai/v2/openapi.json` → 逐字段读取 `CreateImagePixenRequest` / `CreateCharacterWith4DirectionsRequest` / `CreateImagePixfluxRequest` / `CreateImageBitforgeRequest` 的 properties、default、enum、maxLength
> - 官方文档页：`/docs/options/guidance`、`/docs/options/character`、`/docs/guides/rotating-a-character`、`/docs/tools/text2animation`、`/create-character/new`（表单内的官方提示文案）
> - 社区提示词指南：`rabbitcannon/pixellab-forge-mcp/docs/prompting-guide.md`
> **失效条件**：PixelLab 引入 v3 参数体系、给 pixen 加上 `color_image`/`negative_description`、或改变 `enhance_prompt` 计费。
> **未核实项**：本文档**没有实际下单生成过图像**（按任务约束未调用生成 API）。所有"效果好坏"的判断均来自官方文档措辞与参数结构推导，标 ⚠️；参数存在性与取值均标 ✅（读 spec 可复现）。

配套阅读：`docs/reference/pixellab-api.md`（端点/异步语义/价格/尺寸上限）。本文只讲**怎么写 description**。

---

## 一、五条核心结论（先读这段）

### 1. ✅ 你要用的两个端点**根本没有负面提示词字段**

这是最反直觉的一条。全 spec 中只有 4 个 schema 含 `negative_description`：

| Schema | `negative_description` 的 spec 描述 |
|---|---|
| `CreateImagePixfluxRequest` | **`"(Deprecated)"`** |
| `CreateImageBitforgeRequest` | `"Text description of what to avoid in the generated image"` |
| `InpaintRequest` | 同上 |
| `AnimateWithTextRequest` | `"Negative prompt to guide what not to generate"` |

**`CreateImagePixenRequest` 和 `CreateCharacterWith4DirectionsRequest` 都没有这个字段**。传了会被忽略或 422。

推论：`"3d render, realistic photo, gradients, anti-aliasing, blurry"` 这套 Stable Diffusion 习惯在 PixelLab 上是**死路**——

- 在 pixen / character 上：**无处可传**；
- 在 pixflux 上：字段存在但官方标 **Deprecated**；
- 官方对它的最高评价也只是"**can sometimes be useful**"（`/docs/options/guidance`），措辞本身就很保守。

⚠️ **结论：不要花力气写负面提示词。** 那些负面词想压制的东西（3D、渐变、抗锯齿、写实）在 PixelLab 上是由**专用枚举参数**控制的，见第 3 条。

### 2. ✅ 不要在 description 里写 "pixel art / 16-bit / sprite / GBA"

模型**只会输出像素画**，这些词是纯浪费。官方文档里的示例 description 全部是短名词短语：

> `"cute wizard"`、`"robot"`、`"dragon"`、`"fire elemental"`、`"bouncing slime"` — `/docs/tools/text2animation`

社区指南说得更直接：

> "The AI generates pixel art by default, but you can guide the aesthetic."

⚠️ 唯一有价值的"风格词"是**具体的美术参照**（`"in the style of classic SNES RPGs"`），而不是画质形容词（`"clean outlines"`、`"limited palette"`、`"crisp"`、`"高质量"`）。画质形容词对应的都是**已有的枚举参数**，写进文本只会稀释主体描述。

### 3. ✅ 风格靠**参数**，不靠形容词

`create-character-with-4-directions` 的专用字段（✅ 均读自 spec，spec 明确标注为 **soft guidance — the model may not follow exactly**）：

| 参数 | 默认 | 取值 |
|---|---|---|
| `outline` | `single color black outline` | `single color black outline` / `single color outline` / `selective outline` / `lineless` |
| `shading` | `basic shading` | `flat shading` / `basic shading` / `medium shading` / `detailed shading` |
| `detail` | `medium detail` | `low detail` / `medium detail` / `high detail` |
| `view` | `low top-down` | `side` / `low top-down` / `high top-down` / `perspective` |
| `proportions` | `default` | `default` `chibi` `cartoon` `stylized` `realistic_male` `realistic_female` `heroic`（**仅 humanoid**） |
| `text_guidance_scale` | `8.0` | 1–20 |

⚠️ **`text_guidance_scale` 不要拉满**。官方对 guidance weight 的说明：

> "If it is overdone, then the image can get artifacts like **over-saturation**." — `/docs/options/guidance`

本项目建议保持默认 `8.0`；主体总是画错时先改 description，再考虑升到 10–12，不要直接上 20。

⚠️ **`view` 是弱控制**。官方原文 "**Weakly controls** what perspective the character should be drawn in"，且给了角度：`low top-down` ≈ 俯视 20°，`high top-down` ≈ 35°。GBA 宝可梦的地图人物属于前者 → 本项目统一用 **`low top-down`**（与 `docs/reference/pixellab-api.md` 第十一节一致）。

### 4. ✅ 锁 GBA 调色板只有一条路，而且 pixen 走不了

| | `color_image` | `force_colors` |
|---|---|---|
| `create-character-with-4-directions` / `-8-directions` | ✅ 有 | ✅ **有** |
| `create-character-animation` | ✅ 有 | ✅ 有 |
| `create-image-pixflux` / `-bitforge` | ✅ 有 | ❌ 无 |
| **`create-image-pixen`** | ❌ **无** | ❌ **无** |
| tileset / map-object / rotate / inpaint / resize | ✅ 有 | ❌ 无 |

✅ **全 spec 中只有 character 系列 3 个 schema 有 `force_colors`。**

对本项目的直接含义：

- **角色（人形 + 四足）** → `color_image` + `force_colors: true`，调色板可硬锁；
- **配件 / 单图** → 如果用 `create-image-pixen`，**连调色板参考都传不了**，画风一致性只能靠事后 `reduce-colors` 或自己量化；
- ⚠️ 若配件也必须锁色，应改用 **`create-image-pixflux`**（有 `color_image`，虽无 `force_colors`）或走 `create-map-object`（有 `color_image`）。**这是选型决策点，不是提示词问题。**

### 5. ✅ 提示词宜短；要长请让官方扩写并**缓存**

- 官方对 description 的定义就是 "**A short** description of what the tool should generate."（`/docs/options/guidance`）
- 硬上限 `maxLength: 2000`（character 端点），`minLength: 1`
- 官方表单里的唯一写作指导（`/create-character/new`）：
  > "**Be specific about colors, clothing, features, and style.**"

⚠️ 关于 "<200 字符 vs >500 字符"：**spec 里没有任何证据表明长 prompt 更好**，反而 PixelLab 专门提供了三个扩写端点，说明"把短 prompt 变成富 prompt"是它认为该由**模型**做的事：

```
POST /v2/enhance-pixen-prompt        ✅
POST /v2/enhance-character-v3-prompt ✅
POST /v2/enhance-animation-v3-prompt ✅
```

`create-image-pixen` 还有内联开关 `enhance_prompt: bool`（默认 `false`，额外 **0.05 generation**），spec 说明：

> "automatically expand your description into a richer, more detailed prompt before generating — equivalent to calling /v2/enhance-pixen-prompt first... The expanded text is returned in `enhanced_prompt`."

✅ 响应里 `enhanced_prompt` 和 `enhance_usage` 是独立字段。

💡 **本项目推荐用法**：不要在运行时对每次生成都开 `enhance_prompt`。而是**在素材编写期调一次扩写、把 `enhanced_prompt` 存进代码/配置**，之后一直传这个固定字符串。好处：
1. 省掉每次 0.05 generation；
2. **确定性** —— 配合 `seed` 才能真正复现（扩写是随机的，开着它 `seed` 就不保证同图）；
3. 扩写结果可人工审核，避免它塞进"景深/写实"之类反像素画的词。

---

## 二、description 写作公式

```
[主体名词] + [服装/配色的具体项] + [1–2 个辨识特征]
```

**要写**：颜色、衣着、材质、可辨识的道具（官方：colors, clothing, features）
**不要写**：
- ❌ 视角 / 朝向（`view` 参数管，写进文本会和参数打架）
- ❌ 画风画质词（`pixel art` `16-bit` `crisp` `clean outlines` `limited palette` `high quality`）
- ❌ 否定词（字段不存在，见 §1.1）
- ❌ 背景 / 场景 / 环境（角色端点输出透明底 sprite；写场景会诱导模型画背景元素）
- ❌ 动作（动作属于 `action_description`，且官方明确 "**不要写环境/场景**"）

| ❌ 差 | ✅ 好 |
|---|---|
| `a character` | `a dwarf blacksmith wearing a leather apron, holding a hammer` |
| `a sword` | `a curved silver scimitar with a golden crossguard and red gem in the pommel` |
| `16-bit pixel art bartender sprite, clean outlines, limited palette, GBA style, front view, standing in a bar` | `a bartender in a white shirt and black vest with a red bow tie` |

⚠️ 长度落在 **60–160 字符**通常就够（官方示例甚至只有 2 个词）。超过 ~300 字符时，模型在 48×48 里根本画不下那么多细节，多余描述只会互相干扰。

---

## 三、⚠️ 四方向角色的对称性陷阱（最容易踩）

官方 rotate 指南（`/docs/guides/rotating-a-character`）的实测观察：

> "It's not perfect, as we can see it **especially struggles with the hat**"
> "If the character is **symmetric**, we can use the mirrored direction, i.e., if the `south-east` direction looks bad but the `south-west` direction looks good, we can **flip** the good one."

因为 `create-character-with-4-directions` 内部就是"先生成朝南图，再旋转"，所以：

✅ **强不对称的配件在 4 个方向之间不会保持一致**。写 description 时应避免：

| 避免 | 原因 |
|---|---|
| 眼罩、单边耳环、刘海偏分 | 旋转后会跳到另一边或消失 |
| 单肩护甲、斜挎包、腰间单侧挂件 | 东/西方向左右互换 |
| 帽檐复杂的帽子（三角帽、宽檐帽） | 官方点名的失败案例 |
| 背后的披风/翅膀/长发辫 | 朝北时遮挡主体，朝南时又看不见 |

✅ **改为左右对称的设计**：对称的双肩、居中的领结/纽扣/腰带扣、贴头的短发或圆帽。
副作用是：对称角色可以**水平翻转 east 得到 west**，直接省掉 1/4 的生成成本，也保证了两侧绝对一致。

---

## 四、调色板参考图（`color_image`）

社区/Lospec 命名约定：像素画圈子里 "GBA palette" **不是**一个单一标准调色板（GBA 硬件本身是 15-bit 色、每调色板 16 色 ×16 组，不像 NES/Game Boy 有固定的硬件色表）。所以：

⚠️ **在 description 里写 `"GBA palette"` / `"NES palette"` / `"SNES palette"` 基本无效** —— 它对模型只是模糊的年代风格暗示，不是可执行的约束。真正锁色请用 `color_image` + `force_colors`。

✅ Lospec 上 `gba` tag 下有 6 个可用调色板（`https://lospec.com/palette-list/tag/gba`），与本项目最相关的：

| 调色板 | 说明 |
|---|---|
| **AxulArt 32 color Palette** | 32 色，作者自述 "intended to be similar to that used in arcade and **Game Boy Advance** games"，5,422 次下载，社区认可度最高 |
| **PokeRuby overworld（exterior / interior）** | tag 含 `pokemon, gba, ruby, sapphire, pokeruby, overworld` —— **直接就是宝可梦红/蓝宝石地图调色板**，与本项目"绿宝石画风"同代同源 |

💡 **落地做法**：把选定调色板导出成一张小 PNG（每色 1px 或 8px 色块，横排即可），base64 后作为 `color_image` 常量存在后端，所有角色生成都传同一张 + `force_colors: true`。这样全站画风一致性由**数据**保证，而不是靠每条 prompt 里的形容词碰运气。

⚠️ `force_colors: true` 的副作用未实测：强制量化到 32 色可能让渐变处出现色带。若发现角色发灰/断层，先退回 `force_colors: false`（只作软参考）再评估。

---

## 五、Worked examples

> 以下 3 个示例的**参数组合**均已对照 spec 校验（字段名、enum 值、默认值）；⚠️ **出图效果未实测**。

### 例 1 · 人形 NPC —— 调酒师（`create-character-with-4-directions`）

```jsonc
POST /v2/create-character-with-4-directions
{
  "description": "a bartender in a white shirt with rolled sleeves, a black vest, a red bow tie, and short dark hair",
  "image_size": { "width": 48, "height": 48 },
  "view": "low top-down",
  "proportions": { "type": "preset", "name": "chibi" },
  "outline": "single color black outline",
  "shading": "flat shading",
  "detail": "low detail",
  "text_guidance_scale": 8.0,
  "color_image": { "type": "base64", "base64": "<GBA 调色板 PNG>" },
  "force_colors": true,
  "seed": 20260804
}
```

拆解：
- description **97 字符**，只有主体 + 衣着 + 配色，没有一个画风词 ✅
- 领结、马甲、双袖全部**左右对称** → 抗旋转（§3）✅
- `flat shading` + `low detail`：48×48 画布上 `medium shading` 会糊成一团，宝可梦地图人物本身就是接近平涂 ⚠️
- `chibi` 比例 = GBA 地图人物的大头身 ✅（`proportions` 仅 humanoid 生效）
- 固定 `seed` + 不开 `enhance_prompt` → 可复现 ✅

### 例 2 · 四足 —— 酒吧猫（`create-character-with-4-directions` + `template_id`）

```jsonc
POST /v2/create-character-with-4-directions
{
  "description": "a ginger tabby cat with white paws and a small red collar",
  "image_size": { "width": 48, "height": 48 },
  "template_id": "cat",
  "view": "low top-down",
  "outline": "single color black outline",
  "shading": "flat shading",
  "detail": "low detail",
  "color_image": { "type": "base64", "base64": "<同一张 GBA 调色板>" },
  "force_colors": true,
  "seed": 20260805
}
```

⚠️ 四足专属注意：
1. ✅ `template_id` 四足只有 `bear` `cat` `dog` `horse` `lion` 五个，**没有 `bird` / `fish` / `slime`**。想要其它动物只能挑最接近的骨架，或退回 humanoid `mannequin`。
2. ✅ **`proportions` 对四足无效**（spec: "Only applies to humanoid characters"）—— 传了是静默忽略，别指望用它调猫的身材。
3. ✅ 若要传 `directions` 参考图，**四足必须同时给 `south` 和 `east`**（人形只需 `south`），且每张尺寸必须精确等于 `image_size`，否则 422。
4. description 里**不写 "cat"** 也可以（模板已定），但写上有助于毛色/花纹落位；这里保留 "cat" 是为了 `ginger tabby` 有依附对象。

### 例 3 · 配件/道具 —— 马天尼杯（`create-image-pixen`，单图）

```jsonc
POST /v2/create-image-pixen
{
  "description": "a martini glass with a green olive on a toothpick",
  "image_size": { "width": 32, "height": 32 },
  "view": "low top-down",
  "outline": "single color black outline",
  "detail": "low detail",
  "no_background": true,
  "seed": 20260806
}
```

⚠️ **pixen 的三个坑（全部 ✅ 读自 spec）**：

1. **`detail` 的 enum 和 character 端点不一样**：pixen 是 `low detail` / `medium detail` / **`highly detailed`**，**没有 `high detail`**。写 `"high detail"` → 422。且 pixen 的 `detail` **默认就是 `highly detailed`**，对 32×32 的小图标偏高，建议显式降到 `low detail`。
2. **`no_background` 默认是 `false`**。配件要透明底**必须显式传 `true`**，否则会得到带背景的方图，还得再花一次 `remove-background`。
3. **pixen 没有 `shading`、没有 `text_guidance_scale`、没有 `color_image`、没有 `negative_description`、没有 `isometric`**。它的可调项只有：`description` `image_size` `outline` `detail` `view` `direction` `no_background` `background_removal_task` `seed` `enhance_prompt`。

⚠️ 因此配件想和角色**同调色板**，pixen 做不到（§1.4）。若一致性要求高，改用 `create-image-pixflux`（有 `color_image`，≤400×400）：

```jsonc
POST /v2/create-image-pixflux
{
  "description": "a martini glass with a green olive on a toothpick",
  "image_size": { "width": 32, "height": 32 },
  "outline": "single color black outline",
  "shading": "flat shading",
  "detail": "low detail",
  "text_guidance_scale": 8,
  "color_image": { "type": "base64", "base64": "<同一张 GBA 调色板>" },
  "seed": 20260806
  // negative_description 字段虽存在但官方标 (Deprecated)，不要用
}
```

---

## 六、速查表

| 想控制的东西 | ✅ 正确做法 | ❌ 错误做法 |
|---|---|---|
| 3/4 俯视角 | `view: "low top-down"` | 在 description 写 `"3/4 top-down view"` |
| 干净描边 | `outline: "single color black outline"` | 写 `"clean outlines, crisp"` |
| 无渐变/无抗锯齿 | `shading: "flat shading"` + `detail: "low detail"` | 写负面词 `"no gradients, no anti-aliasing"` |
| 限制调色板 | `color_image` + `force_colors: true`（仅 character） | 写 `"limited palette, GBA palette"` |
| 大头身 | `proportions: {type:"preset", name:"chibi"}` | 写 `"chibi proportions"`（浪费 token，且四足无效） |
| 排除写实/3D | 用不着——模型只出像素画 | 写 `"not 3d render, not realistic photo"` |
| 更丰富的 prompt | 生成期调一次 `enhance-*-prompt`，**缓存结果** | 手写 500+ 字符长 prompt |
| 透明底（pixen） | `no_background: true` | 依赖默认值（默认是 `false`！） |
| 四方向一致性 | 设计成左右对称 | 加眼罩/斜挎包/宽檐帽 |

---

## 七、与项目约束的衔接

- 生成走**异步 job + 轮询**，禁止在 HTTP 请求路径上等待（`CLAUDE.md` 禁令 1）；本文只管请求体怎么填。
- `color_image` 的调色板 PNG 与 Bearer token 一样**只存后端**，前端不接触。
- description / seed / 参数组合建议随 `pixellab_jobs` 一起落库（或至少记进素材元数据），否则复现不了历史素材。
- 角色 `kind`（人形/四足）的差异**只体现在请求构造层**（`template_id`、`proportions` 是否生效），⚠️ 不得泄漏到 `scene/`（禁令 3）——两者产出的都是同构的 4 方向 sprite。

---

## 修订记录

- **2026-08-04**：创建。基于 OpenAPI spec 逐字段核实 + 官方文档页 + 社区提示词指南。核心修正了三个常见误解：负面提示词在目标端点上不存在（且 pixflux 上已废弃）、pixen 无法锁调色板、长 prompt 无证据优于短 prompt（官方倾向短 prompt + 模型扩写）。**未实际下单验证出图效果。**
