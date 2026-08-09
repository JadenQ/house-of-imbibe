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
- **管线策略（2026-08-09 用户定稿，取代旧「直转先」锁定）**：
  - **主路径**：上传图片 → MiniMax-M3 vision 描述文字 → `create-character-with-4-directions`（4 方向，~$0.013）→ `animate-character`（行走动画，4 方向，v3 模式 ~$0.013/方向）→ 下载 PNG 进 AssetStore。~$0.06/角色，4 方向 + 行走。
  - **回退**：MiniMax 429 / 不可用 → 用户手动填写描述文字 → `create-character-with-4-directions`（文字）→ animate。
  - ⚠️ **放弃「直转先」**：`image-to-pixelart` 直转只产出单张 south 像素图，**不是可走多方向角色**（pixellab-api §六），直转后仍需 v3 旋转+animate，并不比 vision→text 省钱/省步，故不采用直转为主路径。直转只在地图背景层（D1）用。
  - generated 角色 **4 方向**（south/north/west/east），对角靠镜像；canonical 8 方向契约降级为 4 方向渲染（D2 定稿）。
- 切片 4 落地时必须新增 `VisionClient` trait，**与 `PixelLabClient` 同形状**（submit/poll 分离 + 领域类型，不绑供应商）。MiniMax-M3 仅是当前实现，Anthropic / OpenAI / Gemini 任一都要能热插拔。
- 像素画调色板锁 GBA emerald：发 `create-character-with-4-directions` 时**必须**带 `color_image`（一份固定的 GBA palette PNG）+ `force_colors: true`，否则会漂。
- 已知 live API 与 OpenAPI spec 三处不一致（见 demo 注释）：image-to-pixelart 要 `image_size` + `output_size`；create-character 返回 `background_job_id` 单数；create-image-pixen 拒绝 `model`/`negative_description`/`seed` 字段。

## 新增（2026-08-09）地图 + 人物生成方案（用户已定稿，见 issue #0010）

- **地图三层架构**：①视觉背景层（**D1=生成整张背景图**：文字→`create-image-pixen` 或 图片→`image-to-pixelart`，240×160，~$0.007）②可走/碰撞网格层（admin 手标 walkable/blocked，服务端 clamp 读它）③家具装饰对象层（= decorations 方案，issue #7）。PixelLab 生成的扁平像素图 ≠ 可走 tilemap，三层必须解耦。
- **人物生成定稿**：2a modular = **代码手绘部件扩展**（发型/上衣/下装/鞋子样式选项 + 捏脸 UI，非 PNG preset）；2b/2c = vision→text 或手动文字 → `create-character-with-4-directions`(4方向) → `animate-character`(行走)；**D2=4方向+行走**（非8方向，对角镜像）；**D3=一个用户一个形象、可重新生成覆盖**（avatars.user_id PK + upsert）；**D4=generated 角色也允许配 accessories**（放宽 PRD non-interoperability，手物/背饰 overlay 装到 generated）。
- generated avatar `config_json` 契约 = `{kind:"generated", character_id, frames: {south:[key…], north:[…], west:[…], east:[…]}}`（每方向帧 key 数组，1 帧=静站，3 帧=行走）；前端 `loadGeneratedSheet` 按 `/api/assets/{key}` 取图合成 3列×4行 sheet。

## 新增（2026-08-09）交互 UX + Admin 方案（design-an-interface 盘问后定稿）

- **Admin = Design 2 独立管理台**：is_admin 才显示「管理」入口 → 独立全屏 DOM console（非 Phaser），tabs:地图/成员/装饰。编辑全在 DOM/net，**scene 保持只读**（不违禁令#3 精神）；移动端优先（大目标 DOM）。地图 tab：放大可编辑网格 + walkable 笔刷 + 装饰放置 +「重新生成背景」（D1，文字→create-image-pixen）。成员 tab：表 + 提升/降级/封禁。
- **点单 = 只读看单**：menu 仅展示，**不下单**（PRD menu 内容 TBD/接口 only）。
- **异步形象 = 非阻塞 library**：生成提交后**可先入场玩**（默认/当前形象），job 后台轮询，完成时通知 + 热替换/刷新；**不再阻塞模态干等**（修掉 avatarCreate 现状的 5–9min 阻塞 poll）。
- modular 样式字段（hairStyle/topStyle/bottomStyle/shoeStyle/shoes）**必须**由 `put_avatar` 完整持久化 + 经 WS 快照透传给远端（AvatarSnapshot=serde_json::Value 已透明，只需 put_avatar 不剥离）。

## 其他硬约束

- **移动端横屏优先**（设计第一原则，2026-08-08 用户锁定）：所有前端开发/改进必须支持移动端浏览器横屏适配——整数缩放、`imageSmoothingEnabled=false`、左虚拟摇杆 + 右动作键（DOM/CSS 实现，**不进 Phaser 渲染层**），桌面键盘（WASD/方向键 + 动作键）为回退方案。移动端适配优先级**高于**桌面：任何新功能先保证移动端横屏可用，再做桌面。当前 demo 仅桌面（触控是已知缺口，下个优先级）。
- 注册 = 用户名+密码，**无邮箱、无第三方登录**；argon2id（m=19456,t=2,p=1，即 argon2 crate 默认值）
- PixelLab Bearer token **只存后端**，绝不进前端代码/网络面板
- 前端分层：`net/` `game/` `protocol/` **禁止 import phaser**；`scene/` 只读状态、只调 net
- DB 里存 storage key 不存 URL；`public_url()` 是唯一允许拼 URL 的地方
- 聊天缓冲全局单个 ring buffer（跨场景可见）
- issue 跟踪在 `.scratch/issues/`，不用 GitHub Issues（本机无 gh）

## 运行

```bash
# 推荐：just（或 make，目标同名）—— 见 justfile / Makefile
just dev        # vite :5173（HMR，/api 与 /ws 代理到 :8080）+ cargo run :8080（API+WS+web/dist）
just test       # cargo test --all-targets + cd web && npm run test（离线）
just build      # 前端 build → cargo build --release
just run        # 单二进制生产运行 :8080
just migrate    # 显式 DATABASE_URL=sqlite:data/hoi.db sqlx migrate run（头号踩坑：迁移前必须有 DATABASE_URL）
scripts/run.sh  # 生产后台启动（pidfile 在 data/hoi.pid）；scripts/stop.sh 停止

# 等价的原生命令（无 just 时）
cargo run                      # :8080，服务 API + WS + web/dist
cd web && npm install && npm run dev   # :5173，/api 与 /ws 代理到 :8080
cd web && npm run build && cargo run --release   # 生产
```
