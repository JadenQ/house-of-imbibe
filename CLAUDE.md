# House of Imbibe — 项目约束（AI 编码前必读）

GBA/宝可梦绿宝石画风的 Web 像素社交酒吧。技术栈已锁定：**Rust + Axum 0.8 + SQLite(sqlx 0.8) 单二进制** / **Phaser 4 + TS + Vite**，逻辑分辨率 240×160 整数缩放。

## 文档权威级

- `.scratch/issues/0001-house-of-imbibe-prd.md` = PRD 唯一权威
- `docs/development-plan.md` = 切片排序与预设计
- `docs/reference/pixellab-api.md` = PixelLab **实测**事实层（curl 验证）
- `docs/image2pixel-demo.md` = `image → LLM vision → text → PixelLab` 管线 demo 用法与踩坑记录
- ⚠️ `docs/pixel-mosaic-game-workflow-v2-rust.md` 的 **SQL DDL / 邮箱验证 / fal.ai 章节已废弃**——不要把 email 字段写回来。只有 crate 选型与部署章节仍有效。

## 三条项目专属禁令

1. **禁止在 HTTP 请求路径上等待生成**（PixelLab 端到端 5–9 分钟，必须异步 job + 轮询）
2. **禁止把聊天写进任何表**（内存 ring buffer 50 条，绝不落库）
3. **禁止在 `scene/` 里出现 avatar `kind` 分支**（双管线渲染统一，只允许装载层分支）

## 新增（2026-08-04）切片 4 输入管线

- `src/bin/image2pixel.rs` 是 photo→avatar 的 demo binary。所有调用方式、踩坑、API 字段名差异（OpenAPI vs live）都在 `docs/image2pixel-demo.md`。
- **管线策略（已锁定）**：
  - **主路径**：上传图片 → `image-to-pixelart`（PixelLab 直转，同步，~$0.007，图片 ≤1280×1280）
  - **回退路径**：图片过大 / `image-to-pixelart` 失败 → MiniMax-M3 vision 描述文字 → `create-image-pixen` / `create-character-with-4-directions`（文字生成，~$0.013，30–120 s）
  - **回退的回退**：MiniMax 429 / 不可用 → 用户手动填写描述文字 → 走文字生成
  - 调用顺序：**先尝试直转，失败再走 vision→text，最后兜底手动文字**。不要跳过直转直接走 vision。
- 切片 4 落地时必须新增 `VisionClient` trait，**与 `PixelLabClient` 同形状**（submit/poll 分离 + 领域类型，不绑供应商）。MiniMax-M3 仅是当前实现，Anthropic / OpenAI / Gemini 任一都要能热插拔。
- 像素画调色板锁 GBA emerald：发 `create-character-with-4-directions` 时**必须**带 `color_image`（一份固定的 GBA palette PNG）+ `force_colors: true`，否则会漂。
- 已知 live API 与 OpenAPI spec 三处不一致（见 demo 注释）：image-to-pixelart 要 `image_size` + `output_size`；create-character 返回 `background_job_id` 单数；create-image-pixen 拒绝 `model`/`negative_description`/`seed` 字段。

## 其他硬约束

- 注册 = 用户名+密码，**无邮箱、无第三方登录**；argon2id（m=19456,t=2,p=1，即 argon2 crate 默认值）
- PixelLab Bearer token **只存后端**，绝不进前端代码/网络面板
- 前端分层：`net/` `game/` `protocol/` **禁止 import phaser**；`scene/` 只读状态、只调 net
- DB 里存 storage key 不存 URL；`public_url()` 是唯一允许拼 URL 的地方
- 聊天缓冲全局单个 ring buffer（跨场景可见）
- issue 跟踪在 `.scratch/issues/`，不用 GitHub Issues（本机无 gh）

## 运行

```bash
# 后端（首次编译较慢）
cargo run                      # :8080，服务 API + web/dist
# 前端开发（另开终端）
cd web && npm install && npm run dev   # :5173，/api 代理到 :8080
# 生产
cd web && npm run build && cargo run --release
```
