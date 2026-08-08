# PixelLab API · 已核实事实

> **核实日期**：2026-08-01
> **核实方式**：
> - `curl -sL https://api.pixellab.ai/v2/openapi.json` → 388 KB OpenAPI spec，逐 schema 读取参数定义
> - `curl -sL https://api.pixellab.ai/v2/llms.txt` → 官方端点索引
> - MCP JSON-RPC 直接探测：`POST https://api.pixellab.ai/mcp` → `initialize` + `tools/list`
> - `curl -sL https://www.pixellab.ai/pixellab-api` → 官方 estimated price 表
> - CORS：`curl -X OPTIONS` 预检 + 无鉴权 `GET /v2/balance` 观察响应头
> **失效条件**：官方引入 v3 API、改变异步/同步语义、调整尺寸上限或价格。价格为官方标注的 *estimated*，随时可变。
> **本文档不含任何未核实的推测**；标 ⚠️ 的条目为未实际下单验证的部分。

---

## 一、核心结论（先读这段）

1. ✅ **REST API 是唯一可用于「用户在页面上生成」的通道**。Base URL `https://api.pixellab.ai/v2`，鉴权 `Authorization: Bearer <token>`。
2. ✅ **官方 MCP 存在且在线**，但它是**开发期工具**（Claude Code / Cursor 里生成素材），**不能用于面向终端用户的网页** —— 浏览器无 MCP 客户端，且用户不该接触你的 token。
3. ✅ **没有任何单端点能"文字进 → 带行走动画的多方向 sprite 出"**。最短路径是 **2 次调用 + 2 轮轮询**，端到端约 **5–9 分钟**。
4. ✅ **绝大多数生成端点是异步的**：返回 job id → 轮询。UI 必须按分钟级异步任务设计，不能是"点击转圈出图"。
5. ✅ **尺寸有硬上限**：standard/pro 模式角色 ≤128px，v3 ≤256px，`image-to-pixelart` 输出 ≤320px。**不存在 256×384 这类非方形大尺寸输出**。

---

## 二、鉴权与网络事实

| 项 | 事实 | 状态 |
|---|---|---|
| Base URL | `https://api.pixellab.ai/v2` | ✅ |
| 鉴权 header | `Authorization: Bearer <YOUR_API_TOKEN>` | ✅ |
| 无鉴权响应 | `401` + `{"detail":"Not authenticated"}` + `www-authenticate: Bearer` | ✅ 实测 |
| Server | `uvicorn`（FastAPI，故 OpenAPI spec 由代码生成，可信度高） | ✅ |
| CORS | **回显任意 Origin**：`access-control-allow-origin: <你的 Origin>`、`access-control-allow-credentials: true`、`access-control-allow-headers: authorization,content-type`、`access-control-max-age: 600` | ✅ 实测 |
| 额外暴露头 | `access-control-expose-headers: Content-Disposition` | ✅ |

> ⚠️ **CORS 允许浏览器直连，但绝不要这么做**。Bearer token 会明文出现在前端代码/网络面板里，任何用户都能拿走并刷你的余额。**必须经自己的后端代理**。这一点 API 本身不会阻止你犯错。

### 余额查询

```
GET /v2/balance          ✅ 同步
```

---

## 三、异步语义（关键）

生成类端点**返回 job id 而非结果**：

```
POST /v2/animate-character
  → 200 { background_job_ids: [...], directions: [...], status: "processing" }

GET /v2/background-jobs/{job_id}
  → { id, status, created_at, last_response }
     status ∈ processing | completed | failed
     完成时结果在 last_response.images[0]
```

✅ 官方建议轮询间隔 **5–10 秒**。

角色类另有专用查询：

```
GET /v2/characters/{character_id}
  → CharacterDetail { id, name, prompt, size, directions, status,
                      rotation_urls, animations, animation_count,
                      skeletons, template_id, view, tags, group_id }
```

✅ **`rotation_urls` 在 `status != "completed"` 时为 `null`** —— 必须判 status，不能直接取 URL。

`rotation_urls` 字段：`south` / `west` / `east` / `north` 必有；`south-east` / `north-east` / `north-west` / `south-west` 仅 8 方向时非空。

### 少数同步端点

```
POST /v2/image-to-pixelart   ✅ 同步，直接返回 { usage, image: {type:"base64", base64, format} }
POST /v2/lip-sync            ✅ 同步且免费
POST /v2/talking-gif         ✅ 免费
PATCH /v2/characters/{id}/tags   ✅ 同步免费
POST /v2/characters/{id}/portrait ✅ 同步免费
```

---

## 四、路径 A：自然语言 → 4 方向行走 sprite（最短，推荐）

### 步骤 1 — 创建角色

```
POST /v2/create-character-with-4-directions
```

✅ 已核实的请求 schema（`CreateCharacterWith4DirectionsRequest`），required：`description`, `image_size`

| 参数 | 类型 / 取值 | 默认 | 说明 |
|---|---|---|---|
| `description` | string | — | **必填**，角色外观描述 |
| `image_size` | `{width, height}` | — | **必填**，每个方向的图像尺寸 |
| `text_guidance_scale` | number **1–20** | `8.0` | 遵循文本的程度 |
| `view` | `side` \| `low top-down` \| `high top-down` \| `perspective` | `low top-down` | **`low top-down` 就是经典 3/4 RPG 视角** |
| `outline` | `single color black outline` \| `single color outline` \| `selective outline` \| `lineless` | `single color black outline` | 软引导，模型可能不完全遵守 |
| `shading` | `flat shading` \| `basic shading` \| `medium shading` \| `detailed shading` | `basic shading` | 软引导 |
| `detail` | `low detail` \| `medium detail` \| `high detail` | `medium detail` | 软引导 |
| `proportions` | preset 或自定义 | `null` | preset 可选：`default` `chibi` `cartoon` `stylized` `realistic_male` `realistic_female` `heroic`。**仅 humanoid 生效** |
| `template_id` | string | `mannequin` | 四足：`bear` `cat` `dog` `horse` `lion` |
| `isometric` | bool | `false` | |
| `color_image` | Base64Image | `null` | 调色板参考图 |
| `force_colors` | bool | `false` | 强制使用 `color_image` 的颜色 —— **锁 GBA 调色板就靠这个** |
| `directions` | `{方向: Base64Image}` | `null` | 可传已有 sprite 作参考，缺的方向 AI 补。**传了则 `proportions` 被忽略** |
| `seed` | int | `null` | 可复现 |
| `async_mode` | 恒为 `true` | `true` | 角色创建**只有异步** |

✅ `outline` / `shading` / `detail` / `proportions` 在 **pro 模式下被完全忽略**；`shading` / `proportions` 在 **v3 模式下被忽略**。

**参考图约束**（`directions` 字段）：
- 每张图尺寸必须**精确等于** `image_size`，否则 `422`
- 提供任一参考图时 `south` 必填；四足模板还需 `east`；`oblique` 视角需全部 4 个方向

### 步骤 2 — 加行走动画

```
POST /v2/animate-character      （等价：POST /v2/characters/animations）
```

✅ 请求 schema `CreateCharacterAnimationRequest`，required：仅 `character_id`

**三种模式**（不传 `mode` 时自动判定）：

| 模式 | 触发条件 | 成本 | 帧数 |
|---|---|---|---|
| `template` | 提供 `template_animation_id` | 1 generation / 方向 | 由模板固定，不可配 |
| `v3` | 无 template_animation_id 时的默认 | `ceil(w·h·frames/65536)` / 方向 | `frame_count` 4–16，须偶数 |
| `pro` | 显式指定 | 20–40 generations / 方向 | 由角色尺寸定（≤64px→16 帧，>64px→4 帧），**`frame_count` 被忽略** |

关键参数：

| 参数 | 说明 |
|---|---|
| `template_animation_id` | ✅ spec 中示例值为 **`walking-4-frames`**。⚠️ 描述里列出的 ID 清单被截断（`angry`, `attack`, `attack-back`, `attack-left`, `attack-right`, `backflip`, `bark`, `breathing-idle`, `cross-punch`, `crouched-walking`, ...）。**完整清单需 `GET /v2/characters/{id}` 看 `animations` 字段，或用 MCP `get_character`** |
| `action_description` | 自定义模式**必填**。只描述动作（`"walking"`, `"walking quickly"`），**不要写环境/场景** |
| `directions` | ⚠️ **默认值因模式而异**：template 模式默认**全部方向**，v3/自定义模式**只做 south**。想要 4 方向 v3 动画必须显式传 `["south","north","east","west"]` |
| `frame_count` | 4–16，偶数，**仅 v3 生效** |
| `ai_freedom` | 0–900，**仅 template 模式生效**。0 = 严格跟随模板骨架 |
| `text_guidance_scale` | 1–20，**仅 template 模式生效** |
| `keep_first_frame` | 默认 `true` —— 参考帧作为 frame 0，所以 `frame_count=8` 实际存 **9 帧**。设 `false` 才是正好 8 帧。**仅 v3** |
| `custom_start_frame` / `end_frame` | Base64Image，**仅 v3 且单方向**。传 `end_frame` 进入插值模式 |
| `enhance_prompt` | 默认 false。true 时自动扩写 `action_description`，额外 0.05 generation，**仅 v3**（template/pro 传了会 `422`） |
| `animation_group_id` | 追加方向到已有动画组，而非新建 |

响应：`{ background_job_ids: [每方向一个], directions: [...], status }`

### 步骤 3 — 取产物

```
GET /v2/characters/{character_id}/zip    ✅ 导出全套方向 + 动画帧
GET /v2/characters/{character_id}        ✅ 拿 rotation_urls / animations
DELETE /v2/characters/{character_id}/animations   ✅ 效果不满意时先删再重试
```

---

## 五、路径 B：用户上传照片 → sprite

⚠️ 比路径 A 慢且贵约 2 倍，且 Pro 档计费。

### 步骤 1 — 半身照 → 朝南全身 sprite

```
POST /v2/portrait-character-pro
GET  /v2/portrait-character-pro/{job_id}     （或走通用 /v2/background-jobs/{job_id}）
```

| 参数 | 取值 | 说明 |
|---|---|---|
| `direction` | `portrait_to_character` \| `character_to_portrait` | 双向可用 |
| `image` | base64 PNG | 输入 |
| `view` | `low top-down` \| `high top-down` \| `side` | |
| `result_size` | **16 / 32 / 48 / 64 / 128 / 160** | ✅ 只有这几档。16–64 = 20 generations；128/160 在 2K 下渲染 = 25 generations |
| `seed` | int | |

✅ 耗时约 **30–80 秒**。✅ 官方说明「保留主体身份/服装」。

### 步骤 2 — 朝南图 → 8 方向角色

```
POST /v2/create-character-v3
```

✅ 两种模式：
- **参考图模式**：传 `reference_image`（**必须朝南**，≤256×256），v3 旋转成 8 方向。成本 `ceil(w·h·8/65536)` generations
- **从零模式**：省略 `reference_image`，Pixen 先生成朝南图再旋转。成本 `1 + ceil(s²·8/65536)`，s = max(w,h)。`image_size` 16–256，默认 64×64

✅ 结果**持久化为 character**，与 `/create-character-with-8-directions` 同一套系统，可继续动画/下载/列表。
✅ 最终画布**自动 padding 约 2 倍**留动画空间（上限 256）。

### 步骤 3 — 同路径 A 步骤 2

---

## 六、❌ 不要走的弯路

| 想法 | 为什么不行 |
|---|---|
| 用 `/v2/image-to-pixelart` 处理用户照片再喂给旋转端点 | ❌ 它只是"照片降采样成像素画"，**出不来朝南站姿的游戏 sprite**，不符合 `create-character-v3` 对参考图的要求 |
| 前端直连 PixelLab API | ❌ token 暴露（CORS 虽允许） |
| 期待单次调用出完整 sprite sheet | ❌ 不存在这样的端点 |
| 用 MCP 服务用户请求 | ❌ MCP 是开发期工具，浏览器无客户端 |
| 长期直接引用 PixelLab 返回的 URL | ⚠️ 应落自己的对象存储（与 v2 工作流对 fal.ai 的处理一致） |

---

## 七、价格（📄 官方 estimated，USD）

来源：https://www.pixellab.ai/pixellab-api

### 角色

| 端点 | 尺寸 | 价格 |
|---|---|---|
| `create-character-with-4-directions` | 48×48 | $0.0105 |
| | 64×64 | $0.0122 |
| `create-character-with-8-directions` | 48×48 | $0.0133 |
| | 64×64 | $0.0173 |
| `create-character-pro` | ≤85×85 | $0.095 |
| | ≤113×113 | $0.125 |
| | ≤168×168 | $0.185 |
| `create-character-v3` | 64×64 | $0.041 |
| | 128×128 | $0.042 |
| | 168×168 | $0.045 |
| `create-character-state` | ≤84×84 / ≤112 / ≤168 | $0.095 / $0.125 / $0.185 |

### 动画（**价格为每个方向**）

| 模式 | 尺寸 | 价格 / 方向 |
|---|---|---|
| template | 64×64 | **$0.0323** |
| template | 128×128 | **$0.0956** |
| v3（4 帧） | 64×64 | **$0.0129** |
| v3（4 帧） | 128×128 | $0.0145 |
| pro | ≤128×128 | $0.095 |
| pro | ≤168×168 | $0.185 |

> ✅ **template 模式的动画比 v3 贵 2.5 倍**（64×64：$0.0323 vs $0.0129），但帧数由模板固定、动作更稳。这是个真实的成本/质量取舍。

### 图像操作

| 端点 | 尺寸 | 价格 |
|---|---|---|
| `image-to-pixelart` | 64×64 / 128×128 / 256×256 | $0.006 / $0.00666 / $0.01164 |
| `remove-background` | 64 / 128 / 256 | $0.00554 / $0.00554 / $0.00593 |
| `resize` | 64×64 / 128×128 | $0.01788 / $0.01777 |
| `create-image-pixflux` | 64×64 / 320×320 / 400×400 | $0.00793 / $0.0101 / $0.0132 |
| `create-image-pixen` | 32 / 64 / 256 / 512 | $0.007 / $0.00718 / $0.0089 / $0.0169 |
| `create-image-bitforge` | 32 / 128 / 200 | $0.0071 / $0.00797 / $0.01122 |
| `generate-8-rotations-v3` | 64×64 / 256×256 | $0.0337 / $0.0377 |
| `rotate` | 64×64 / 128×128 | $0.01057 / $0.01091 |
| `inpaint` | 64×64 / 200×200 | $0.00716 / $0.01122 |
| `edit-image` | 64×64 | $0.0118 |
| `map-objects` | 每个 | $0.0099 |
| `create-tileset`（top-down） | 16×16 / 32×32 tiles | $0.0079 / $0.0099 |
| `create-isometric-tile` | 32 / 64 | $0.0156 / $0.0166 |
| `enhance-*-prompt` | 每次 | $0.002 |
| Pro 系列（image/style/UI） | ≤256 / ≤341 / ≤512 | $0.095 / $0.125 / $0.185 |

### 端到端单角色成本估算

| 方案 | 构成 | 合计 |
|---|---|---|
| **路径 A + template 动画** | 4 方向 48×48 $0.0105 + 4 × $0.0323 | **≈ $0.14** |
| **路径 A + v3 动画** | 4 方向 48×48 $0.0105 + 4 × $0.0129 | **≈ $0.062** |
| **路径 B** | portrait-pro ≈$0.095 + v3 旋转 $0.041 + 4×v3 动画 $0.052 | **≈ $0.19–0.30** |

⚠️ 未实际下单验证。generations 与 USD 的换算关系随订阅档变化，`GET /v2/balance` 返回两者。

---

## 八、尺寸硬上限（最易写错的部分）

| 端点 / 模式 | 上限 | 状态 |
|---|---|---|
| 角色 standard / pro 模式 | **128 px** | ✅ |
| 角色 v3 模式 | **256 px** | ✅ |
| `create-character-v3` 参考图 | **256×256** | ✅ |
| `create-character-v3` 从零 `image_size` | 16–256（默认 64×64） | ✅ |
| `image-to-pixelart` 输入 | 16×16 – **1280×1280** | ✅ |
| `image-to-pixelart` 输出 | 16×16 – **320×320** | ✅ 官方建议输出取输入的 **1/4**，且保持宽高比 |
| `create-image-pixflux` | ≤400×400 | 📄 |
| `create-image-pixen` | ≤512×512 | 📄 |
| `create-image-bitforge` | ≤200×200 | 📄 |
| `animate-with-text` | 固定 64×64 | 📄 |
| `animate-with-text-v3` | ≤256×256，≤16 帧 | 📄 |
| `animate-with-skeleton` | ≤128×128 | 📄 |
| `rotate` | ≤128×128 | 📄 |
| `inpaint` | ≤200×200 | 📄 |
| `create-isometric-tile` | 16×16 – 64×64（>24×24 质量更好） | 📄 |
| `portrait-character-pro` `result_size` | 枚举 16/32/48/64/128/160 | ✅ |
| `animate-character` `frame_count` | 4–16，须偶数 | ✅ |
| `description` 长度 | ≤2000 字符 | ✅ |

---

## 九、MCP 服务器（开发期用）

✅ **实测在线**。`POST https://api.pixellab.ai/mcp`，Streamable HTTP，`serverInfo: {name: "PixelLab MCP Server", version: "0.2.0"}`，protocol `2024-11-05`。

```bash
claude mcp add pixellab https://api.pixellab.ai/mcp -t http \
  -H "Authorization: Bearer YOUR_TOKEN"
```

✅ **暴露 65 个工具**（不是早期文档说的 7 个）。分组：

| 组 | 工具 |
|---|---|
| 角色 | `create_character` `create_character_state` `animate_character` `get_character` `list_characters` `delete_character` `update_character_tags` `delete_animation` |
| 对象 | `create_map_object` `get_map_object` `create_1_direction_object` `create_8_direction_object` `get_object` `list_objects` `animate_object` `create_object_state` `delete_object` `update_object_tags` `select_object_frames` `dismiss_review` |
| 地图 | `create_topdown_tileset` `create_sidescroller_tileset` `create_isometric_tile` `create_tiles_pro` `create_path_tiles` `create_building_kit` + 各自 get/list/delete |
| 图像 | `create_image_pixflux` `create_image_pixen` `create_image_pro` `get_image` `edit_image` `inpaint_image` `animate_image` |
| 肖像/语音 | `create_portrait_character` `get_portrait_character` `set_character_portrait` `create_vocal_animation` `get_vocal_animation` `create_talking_gif` `get_lip_sync` |
| UI / 字体 | `create_ui_asset` `get_ui_asset` `list_ui_assets` `delete_ui_asset` `create_font` `get_font` |
| 元 | `get_balance` `agent_feedback` `agent_help` `list_projects` `agent_list` `agent_inspect` `agent_talk` |

✅ MCP `create_character` 的**参数名与 REST 不同**（更扁平）：`size`（不是 `image_size`）、`reference_image_base64` / `reference_image_url`、`n_directions`、`body_type`、`proportions` 为 JSON 字符串。**不要把 MCP 的参数名照搬到 REST 调用**。

✅ MCP 特有的实用提示（REST 文档里没有的）：
- `reference_image_url` **优于** base64 —— 官方明确说「inline base64 常被 MCP 客户端截断」
- `animate_character` 的 **pro 模式必须先不带 `confirm_cost` 调一次看价**，再确认
- **质量升级阶梯**：template（1 gen/dir）→ v3（≤96px 约 1 gen/dir，160px 约 4）→ pro。不满意时先 `delete_animation` 再重试

---

## 十、SDK 状态

| SDK | 仓库 | 最后 push | 状态 |
|---|---|---|---|
| **Python** | `pixellab-code/pixellab-python` | **2026-05-27** | ✅ 当前维护中，`pip install pixellab`，27 stars |
| MCP server | `pixellab-code/pixellab-mcp` | 2025-08-10 | ✅ 托管版在线，38 stars |
| **JavaScript** | `pixellab-code/pixellab-js` | **2025-07-07** | ⚠️ **已落后**。README 仍指向 **v1** API（`api.pixellab.ai/v1`），3 stars |

> ⚠️ **Node/Rust 后端不要用 `@pixellab-code/pixellab`** —— 它面向 v1。直接打 v2 REST（本项目后端是 Rust，用 `reqwest` 即可）。

---

## 十一、对本项目的接入设计

结合 v2 工作流（Axum + SQLite + object_store）：

1. **token 只存后端** —— 环境变量，前端只调自己的 `/api/*`
2. **任务表** —— 分钟级异步必须落库，不能只放内存：

```sql
CREATE TABLE pixellab_jobs (
    id              TEXT PRIMARY KEY,
    user_id         TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    kind            TEXT NOT NULL CHECK (kind IN ('character','animation','portrait')),
    character_id    TEXT,                    -- PixelLab character UUID
    job_ids         TEXT,                    -- JSON array of background_job_ids
    status          TEXT NOT NULL,           -- queued|processing|completed|failed
    error           TEXT,
    asset_path      TEXT,                    -- 落地后的 object_store path
    cost_usd        REAL,                    -- 从响应 usage 累加
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at    TEXT
);
```

3. **后台 worker 轮询** —— 5–10 秒间隔（官方建议），tokio task
4. **前端拿状态** —— SSE 或轮询自己的状态接口，**不要**让前端轮询 PixelLab
5. **产物落地** —— 与 v2 工作流处理 fal.ai 的方式一致：`reqwest` 拉回 → `object_store.put()`。
   ⚠️ 但 fal.ai 用的是"3–5 秒同步等待"，**PixelLab 是分钟级，不能复用那个同步模式**
6. **UI 文案** —— 明示「约 5 分钟，完成后通知」；不要无限转圈
7. **成本护栏** —— 每用户生成次数限额；`usage` 字段入库；上线前用 `GET /v2/balance` 做低余额告警
8. **锁 GBA 画风** —— `color_image` + `force_colors: true` 传绿宝石调色板；`view: "low top-down"`；`proportions: {"type":"preset","name":"chibi"}`

---

## 修订记录

- **2026-08-01**：创建。基于 OpenAPI spec + MCP 实测 + 官方定价页核实。同时纠正了 4 份调研文档中的错误结论，详见各文件的修订记录。
