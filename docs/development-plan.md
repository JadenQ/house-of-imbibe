# House of Imbibe 开发计划

> 基准：PRD #1 + 切片 #2–#8 + 9 条已锁定 grilling 决策。
> 前置结论：仓库为 greenfield（`docs/` 与 `.scratch/` 之外无代码），所以"预重构"实际是"预设计"——成本极低，是本计划回报最高的一节。

---

## 〇、先修正三处 spec 与已锁决策不一致（进入切片 1 前必须定稿）

| # | 位置 | 问题 | 处置 |
|---|---|---|---|
| A | `docs/pixel-mosaic-game-workflow-v2-rust.md` §2.3 的完整 DDL | 含 `email` / `email_verified` / `email_tokens` / `messages`（聊天落库）/ `avatars.source ∈ (upload,fal_ai,preset)` / `rooms` 表 —— 与决策 6（无邮箱）、决策 8（聊天不落库）、PRD 的 `kind ∈ (modular,generated)` 全部冲突 | **v2 文档的 DDL 不可复制**。以 PRD §Schema 为唯一权威。在 `CLAUDE.md` 里显式写一句 "v2 研究文档的 SQL/邮件/fal.ai 章节已废弃，只保留 crate 选型与部署章节"，否则 AI 编码时会把 email 字段写回来 |
| B | PixelLab 调研报告自相矛盾（能力表写 "❌ 暂无官方 MCP"，下一节写 "⭐ 官方 MCP server"），且 PRD §Stack 写 "via its official MCP/API" | **MCP 是给 IDE/CLI 客户端用的开发期工具，不是给 Rust 服务端运行时调用的**。生产后端应直接调 `api.pixellab.ai` 的 HTTP API（Bearer token, reqwest）。MCP 只用于你在 Claude Code 里手工生成预置素材（切片 2 的 preset parts、切片 5 的 tileset） | 把 PRD 措辞收敛为："运行时 = PixelLab HTTP API；开发期素材生产 = PixelLab MCP（离线，不进代码路径）"。两条路径不共享代码 |
| C | PRD §Realtime 写 "One room per server"，但切片 5 要 bar + yard 两个场景 + 传送 | 若切片 1 把房间当单例（`Room` 而非 `DashMap<SceneId, Room>`），切片 5 必须重写 WS handler、tick、快照、聊天缓冲 | 切片 1 就以 `scene_id` 为房间键（见 §二.6）。同时**现在**定一条：聊天缓冲是**全局单个 ring buffer**（跨场景可见，符合"朋友们的一个酒吧"语义），不是每场景一个——避免切片 5 时才发现语义歧义 |

---

## 一、集成策略与排序

### 1.1 依赖图与波次

```
波次 0（并行，与波次 1 同时开跑，不阻塞任何人）
  ├─ Spike-0  PixelLab 真机验证（半天，手工，无代码）        ← 见 1.3，最高优先级前置
  └─ Spike-1  Tiled 底图草图 + sprite 契约定稿（半天）

波次 1        #2 切片 1  骨架 + 实时脊椎            ← 唯一串行瓶颈
                    │
波次 2（并行）├─ #3 切片 2  Avatar 模型 + Modular
              └─ #6 切片 5  bar/yard 场景 + NPC + menu
                    │
波次 3        #4 切片 3  生成 + 素材库 + 配件
                    │   （建议拆 3a/3b，见 1.2）
                    ├──────────────┐
波次 4（并行）├─ #5 切片 4 照片形象  └─ #7 切片 6 admin
                    │
波次 5        #8 切片 7  部署 + 运维
```

关键路径长度 = 1 → 2 → 3 → 6 → 7（5 个切片）。切片 4（照片形象）**不在关键路径上**，可以和切片 6 并行，也可以在部署后追加——这是一个重要的排期自由度：如果 Spike-0 显示照片管线不可行，切片 4 可以整块延后而不影响上线。

### 1.2 建议的一处拆分（提升并行度）

切片 3 的 `blocked_by` 让切片 6 等了太久，原因只有一条："装饰物素材来自生成"。但 admin 放置装饰物**不需要**生成能力，只需要 `assets` 表 + object_store + 一个 curated 素材集。建议拆：

- **3a — 素材基建**：`assets` 表、`AssetStore` facade、`GET /api/library`、curated 素材导入（把 Spike 期用 MCP 手工生成的 PNG 塞进去）。依赖仅 #2。
- **3b — 生成管线 + 配件**：`PixelLabClient`、`generation_jobs`、worker、配件装配。依赖 3a + #3(切片 2)。

拆完后 `#7 admin` 的依赖变成 `3a + #6`，可以和 3b **并行**。代价：多一个 issue 文件；收益：关键路径缩短一个切片，且 admin/装饰物这条"最容易出实时同步 bug"的线能更早暴露问题。

### 1.3 Spike-0 必须前置（否则切片 4 是盲赌） — ✅ 2026-08-04 已完成

**结论**：照片→4 方向 character = `MiniMax-M3 vision → 1 段文字描述 → create-character-with-4-directions`，**1 次付费、~100 秒、~$0.013**。详见 `.scratch/issues/0009-vision-bridged-pixel-art-pipeline.md` 与 `docs/image2pixel-demo.md`。

原 Spike-0 的多步链路（`create_image_pixflux` → `create_character` → `animate_character`）**不再是默认路径**，但保留作为"对身份一致性要求高的用户"的回退方案（多步链路成本 ~$0.19–0.30，2–4 分钟）。

Spike-0 跑通时验证了 3 个数字 + 1 个判断：

| 项 | 实测值 | 影响 |
|---|---|---|
| 单形象 credit 成本 | 1 generation + ~$0.001 vision ≈ **$0.013** | 比原方案便宜 ~20× |
| 端到端墙钟 | 105 s（4 方向 done），含 21 次 5 s 轮询 | 在 PRD §二.6 的 5–9 分钟包络内 |
| 输出尺寸 | 每个方向 **92×92 RGBA**（character fit 在 ~64×64 中心，余下是动画 headroom） | 切片 1 的 sprite 契约需写明 92×92 canvas / 64×64 effective |
| 4 方向 vs 8 方向 | `create-character-with-4-directions` 出 4 方向（s/w/e/n）；8 方向要 `create-character-with-8-directions` 或 `create-character-v3`，贵 1.3× | 切片 4 起步用 4 方向，8 方向留作未来 |

> ⚠️ **Spike-0 的图是合成的 Pillow 红熊猫，不是真人照片**。切片 4 实现时建议补做 5–10 张真人/真动物照片的人工 A/B（视觉对比 + 多代描述一致性），再冻结接口形状。

### 1.3.1 视觉模型的 provider 抽象（新增，Spike-0 副作用）

因为 `MiniMax-M3` 只是当前实现（API 在 `https://api.minimaxi.com/v1/`，OpenAI-兼容 chat completions，带 `<think>` reasoning tokens），切片 1 落地 trait 时要新增：

```rust
#[async_trait]
pub trait VisionClient: Send + Sync {
    async fn describe(&self, image: ImageRef) -> Result<String, VisionError>;
    fn provider_id(&self) -> &'static str;
    fn max_image_bytes(&self) -> usize;          // 多数 <5 MB，base64 后约 6.7 MB JSON body
}
```

实现：`MiniMaxVision`（当前）、`AnthropicVision`（未来切到 Claude Sonnet 4 / Opus 4.7 时启用）、`StubVision`（测试用，固定 1-paragraph 描述）。
**禁止把 `minimaxi.com` / `api.pixellab.ai` / `api.anthropic.com` 任一硬编码进业务代码**——跟 `PixelLabClient` 同款约束。

### 1.4 每切片的集成验证点（端到端行为，不是单元断言）

原则：验证点必须**同时穿过 HTTP、DB、WS、前端渲染**四层中至少三层，且能被一条自动化测试或一次两标签页手工操作证明。

| 切片 | 集成验证点（"打通了"的证据） |
|---|---|
| **1 骨架** | **两个浏览器标签页**分别注册两个账号，A 移动时 B 屏幕上 A 的方块**平滑插值移动**（不是跳格）；A 发言，B 看到气泡+侧栏；F5 刷新 B 后侧栏仍有最近 50 条；`sqlite3 game.db "select count(*) from sqlite_master where name like '%message%'"` = 0。自动化侧：一个测试进程内起真 Axum + 临时 SQLite + 两个真 WS 客户端，断言 A 的 move 在 ≤200ms 内出现在 B 的 delta 帧里 |
| **2 Modular** | A 在 builder 里把头发改成紫色并保存，**B 的屏幕上 A 的角色头发在不刷新的情况下变紫**。这一条同时验证了：REST 写库 → avatar_snapshot 进房间状态 → delta 广播 → 前端合成层重绘。比"截图看起来对"强得多 |
| **3a 素材基建** | 把一个 PNG 通过导入路径进 `assets` 表 → `GET /api/library` 列出 → 前端 `<img>` 用 `public_url()` 返回的 URL 拉到图。**再把 store 换成第二个 `AssetStore` 实现（内存/临时目录）跑同一套测试全绿**——这就是 R2 可迁移性的机器化证明 |
| **3b 生成+配件** | 提交生成请求 → **HTTP 响应在 100ms 内返回 job_id**（用测试断言响应时间上限，把"禁止阻塞式生成"变成可回归的约束）→ 关掉页面 → 重开 library 看到 done → 装到手部槽 → B 屏幕上看到 A 手里多了个杯子，且**走路动画每一帧杯子都跟手**（锚点契约的真实检验） |
| **4 照片形象** | 上传照片 → job done → 激活 → **同一个 Phaser 渲染代码路径**渲染出来（代码里不应出现 `if kind == generated` 的渲染分支，只有资源装载分支）；`POST /api/avatar/equip` 对 generated 返回 400；job 完成后原图文件**在磁盘上不存在**（测试用 walkdir 断言临时目录已清空） |
| **5 场景+NPC** | 玩家走向吧台**穿不过吧台**（服务端 clamp 生效，前端不作弊也拦得住：用测试直接发一个落在墙里的 `{move}`，断言服务端返回的位置仍在墙外）→ 走到门口触发场景切换，B（在 yard）能看到 A **进入** yard；靠近 bartender 按动作键出对话，跟到 menu 节点拿到 placeholder JSON 并渲染 |
| **6 admin** | admin 在 A 标签页 edit mode 点一个格子放椅子，**B 标签页无刷新出现椅子**；B 用普通账号打 admin 接口全 403；ban 一个在线用户后其 WS 被踢且无法再登录 |
| **7 部署** | 真域名 HTTPS 打开、**WS 在 Caddy 后握手成功**（这是最容易翻车的一点）、`systemctl restart` 后玩家重连恢复、`journalctl` 有结构化日志、跑一次备份脚本并**从备份文件恢复出一个能启动的库**（备份没验证恢复 = 没有备份） |

---

## 二、预重构清单（Make the change easy）

每条格式：**接口形状 → 现在成本 → 不做的返工成本**。

### 2.1 `PixelLabClient` trait —— 关键点是 submit/poll 分离

```rust
// src/generation/provider.rs
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImageRef { pub bytes: Vec<u8>, pub mime: String }   // 不落库，只在内存/临时文件

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum GenSpec {
    Character { description: String, n_directions: u8,
                animations: Vec<String>, style_ref: Option<ImageRef> },
    MapObject { description: String, size: u32, style_ref: Option<ImageRef> },
    Image     { description: String, width: u32, height: u32,
                no_background: bool, init: Option<ImageRef> },
}

pub struct Artifact { pub bytes: Vec<u8>, pub mime: String,
                      pub sheet: Option<SheetLayout>,          // 见 2.4
                      pub provider_meta: serde_json::Value }

#[derive(Debug)]
pub enum GenStatus { Pending, Ready(Vec<Artifact>), Failed { code: String, msg: String } }

pub trait PixelLabClient: Send + Sync {
    /// 只提交，立即返回供应商任务号，绝不等结果
    async fn submit(&self, spec: &GenSpec) -> Result<ProviderJobId, GenError>;
    /// 幂等轮询
    async fn poll(&self, id: &ProviderJobId) -> Result<GenStatus, GenError>;
    fn provider_id(&self) -> &'static str;   // "pixellab" | "stub" | 未来 "otherco"
}
```

- **为什么必须 submit/poll 分离而不是 `async fn generate(spec) -> Artifact`**：真实 PixelLab 是长任务。若 trait 只有 `generate`，worker 会把一个 tokio task 挂几分钟，且**进程重启后这个任务永久丢失**（DB 里 status 停在 `pending`，无从恢复）。分离后 `generation_jobs` 只需多一列 `provider_job_id`，worker 重启后能接着 poll。
- **成本**：现在多一个方法 + 一列 DB 字段，约 30 行。
- **不做的返工成本**：切片 3b 上真 API 时发现要重构 worker 循环、job 状态机、DB schema（要加 migration）、以及所有 stub 测试的时序假设。属于"改一处动四处"。
- **stub 的正确形状**：stub 的 `submit` 记一个内存 map，`poll` **第一次返回 Pending，第二次返回 Ready**。绝不要"stub 同步返回 Ready" —— 那样测试走的是一条真实环境不存在的代码路径，切片 3b 的验收标准里"resolves synchronously"这句话应改成"deterministically resolves within N polls"。
- **多阶段链路的兜底**：如果 Spike-0 证明照片管线是三步链，把链路编排放在**我们的 worker 里**（`generation_jobs` 加 `stage INT`），不要放进 trait。trait 保持"一次调用一个原子操作"，换供应商时才好换。

### 2.2 `AssetStore` facade —— 关键点是 `public_url()`

```rust
// src/assets/store.rs
pub struct StorageKey(pub String);        // "sprites/{owner_id}/{asset_id}.png"

pub trait AssetStore: Send + Sync {
    async fn put(&self, key: &StorageKey, bytes: Bytes, mime: &str) -> Result<()>;
    async fn get(&self, key: &StorageKey) -> Result<Bytes>;
    async fn delete(&self, key: &StorageKey) -> Result<()>;
    /// 唯一允许生成对外 URL 的地方
    fn public_url(&self, key: &StorageKey) -> String;
}
```

内部用 `object_store` crate 实现（`LocalFileSystem` / `AmazonS3Builder`），但业务代码只依赖我们这层 facade。

- **为什么不直接用 `Arc<dyn ObjectStore>`（PRD 原话）**：`object_store` trait 没有 `public_url` 概念。如果不封这一层，前端 URL 会被硬编码成 `/assets/{storage_key}`，散落在 avatar、library、decoration、npc 四处响应构造里。切 R2 时要改的不是"存储层"，而是**所有返回资源的 handler + 前端拼 URL 的地方**。
- **成本**：现在约 60 行 + 一个内存实现给测试用。
- **不做的返工成本**：R2 迁移从"改 1 处配置"变成"改 8 处 handler + 前端 + 重跑所有 JSON 快照测试"。这正是 PRD user story #31 承诺的东西，不封就等于没兑现。
- 附带约定：`storage_key` 一律 `{kind}/{owner_id}/{asset_id}.{ext}`，**不含域名、不含 `/assets` 前缀**。DB 里存 key，不存 URL。

### 2.3 前端 net / game-state / Phaser 边界

```
src/
  net/          WsClient, RestClient        —— 依赖：无（可注入 Transport）
  protocol/     ClientMsg / ServerMsg 类型 + 校验（与 Rust 侧共享 JSON 契约）
  game-state/   纯函数：applyServerMsg / interpolate / walkability 查询
  scene/        Phaser Scene —— 只读 game-state 的 RenderView，只调 net.send
  ui/           摇杆、按钮、聊天面板、builder、library（DOM 层，非 Phaser）
```

核心接口形状：

```ts
// net/transport.ts —— 让测试不需要真 WebSocket
export interface Transport {
  send(raw: string): void;
  onMessage(cb: (raw: string) => void): void;
  onClose(cb: (code: number) => void): void;
  close(): void;
}

// game-state —— 纯函数，无 this、无 Phaser、无 DOM
export function applyServerMsg(s: RoomState, m: ServerMsg): RoomState;
export function interpolate(s: RoomState, nowMs: number, delayMs: number): RenderView;
//  RenderView = { players: Array<{id, x, y, dir, clip, frame, avatar: AvatarSnapshot}>,
//                 decorations: [...], bubbles: [...] }
```

- **机械化强制边界**：加一条 lint（eslint `no-restricted-imports` 或 `dependency-cruiser`）——`net/`、`game-state/`、`protocol/` **禁止 import `phaser`**，CI 里跑。
- **成本**：一条 lint 规则 + 目录约定，约 15 分钟。
- **不做的返工成本**：Phaser 一旦渗进状态层（例如直接把 `Phaser.GameObjects.Sprite` 存进玩家状态、或用 `scene.time` 做插值时钟），插值逻辑就再也不能在 node 里单测。PRD user story #33 直接失效，后续每个切片的前端逻辑都只能靠"打开浏览器看"来验证——这是整个项目最大的隐性效率杀手。
- 附带约定：插值时钟用**注入的 `now()`**，不用 `Date.now()` 直调。测试要能"快进时间"。

### 2.4 规范 sprite-sheet 契约（8 方向 × 动画帧）

一份 sidecar JSON（每个 sheet 一份，随 asset 一起存），Rust 与 TS 各有一份类型：

```json
{
  "v": 1,
  "frame_w": 32, "frame_h": 32,
  "origin": [16, 28],
  "dirs": ["s","se","e","ne","n","nw","w","sw"],
  "clips": {
    "idle": { "row0": 0, "frames": 2, "fps": 4,  "loop": true },
    "walk": { "row0": 8, "frames": 4, "fps": 8,  "loop": true }
  },
  "anchors": {
    "hand": { "s": [[20,18],[21,17],[20,18],[19,17]], "se": [...], "...": "每 clip×dir×frame" },
    "back": { "...": "同上" }
  }
}
```

布局规则（写进 `docs/sprite-contract.md`，作为唯一权威）：**行 = 方向（固定顺序 s,se,e,ne,n,nw,w,sw），列 = 帧；每个 clip 占连续 8 行，`row0` 指定起始行**。

- 三条容易漏、漏了很贵的字段：
  1. **`anchors`（配件锚点，逐帧）**。切片 3b 的配件叠加需要它。如果契约里没有，切片 3b 才发现，就必须给切片 2 已经生成的所有 preset sheet **补锚点元数据**，还要改 asset meta schema + 加 migration。**现在加一个可选字段（默认退化为 origin）成本 ≈ 0**。
  2. **`v` 版本号**。将来帧尺寸从 32 变 48（比如做 LOD 近景），老 asset 还能被识别。
  3. **`dirs` 显式列出而非隐式约定**。PixelLab 返回的方向顺序不由我们决定（Spike-0 才知道），显式声明才能在导入时做一次重排而不是让渲染器猜。
- **成本**：一个 JSON schema + 两处类型定义，约 1 小时。
- **不做的返工成本**：所有已生成素材需要重新生成或人工补数据（**花钱**且不可回滚），渲染器与合成器同时改，配件功能可能整块推迟。这是本清单里返工代价最高的一条。

### 2.5 WS 消息协议（type 清单，切片 1 就全部声明，未实现的返回 `error{code:"unimplemented"}`）

```ts
// ClientMsg
| { v:1, type:"move",      tx:number, ty:number }
| { v:1, type:"chat",      text:string }
| { v:1, type:"interact",  target:string }            // npc id / decoration id
| { v:1, type:"dialogue_advance", npc:string, choice?:string }   // 切片 5
| { v:1, type:"ping",      t:number }

// ServerMsg
| { v:1, type:"welcome",        self_id, scene, tick_hz:10, server_time:number }
| { v:1, type:"snapshot_full",  tick, players:[], decorations:[], npcs:[] }
| { v:1, type:"snapshot_delta", tick, t:number, upsert:[], remove:[] }
| { v:1, type:"chat",           from, name, text, ts }
| { v:1, type:"chat_backlog",   items:[] }             // 入场补最近 50 条
| { v:1, type:"dialogue",       npc, node, menu?:MenuPayload }   // 切片 5
| { v:1, type:"decoration_added"|"decoration_removed", ... }     // 切片 6
| { v:1, type:"scene_changed",  scene, spawn:{x,y} }             // 切片 5
| { v:1, type:"kicked",         reason }               // 切片 6 ban
| { v:1, type:"error",          code, msg }
| { v:1, type:"pong",           t }
```

- **必须现在就定的三件事**：
  1. **`tick` + `t`（服务端时间戳）出现在每个快照里**。客户端插值需要一个权威时间基准来排缓冲区；事后加时间戳等于重写插值器。
  2. **前向兼容规则："收到未知 `type` 必须静默忽略"**，双向都遵守。否则切片 5/6 上线时会出现"旧前端 + 新后端 = 白屏"，被迫每次前后端锁步部署。
  3. **`v` 字段**，为将来换 MessagePack / 改结构留门。
- **成本**：把上面这段抄进 `protocol.ts` + Rust `#[serde(tag="type")]` enum，1 小时，未实现的 arm 全部 `=> Err(unimplemented)`。
- **不做的返工成本**：每个后续切片都要改 enum + 改前端 dispatch + 改所有已有测试的断言。更糟的是切片 6 的实时装饰同步会想"REST 改完让客户端自己刷"，从而**违反决策 2**。

### 2.6 补充两条（同样重要，PRD 未点明）

**(a) `scene_id` 作为房间键 + `WalkGrid` 抽象**

```rust
pub trait WalkGrid: Send + Sync {
    fn is_walkable(&self, tx: i32, ty: i32) -> bool;
    fn clamp(&self, from: (i32,i32), to: (i32,i32)) -> (i32,i32);
    fn spawn(&self) -> (i32,i32);
    fn portals(&self) -> &[Portal];   // 切片 5 才有内容，切片 1 空数组
}
```
切片 1 原计划用 `RectGrid`（硬编码矩形）实现它；切片 5 用 `TmjGrid`（解析 .tmj）实现它，**服务端与客户端解析同一个 .tmj 文件**（不允许把碰撞信息在 Rust 里手写第二遍——两份必然漂移）。
> **2026-08-08 demo 偏离**：demo 需要真实酒吧地图（"走到吧台按 E 开酒单"），故改用 `BarGrid`（解析共享的 `assets/maps/bar.json`，Rust `include_str!` + TS `import` 同一份）。`WalkGrid` 接口不变，切片 5 把 `BarGrid` 换成 `TmjGrid` 是加一个 impl + 指向 `.tmj`，非重写。
房间容器从第一天起就是 `DashMap<SceneId, Room>`，`Room` 持有 `Arc<dyn WalkGrid>`。
成本：一个 trait + 20 行矩形实现。不做的成本：切片 5 重写 WS handler / tick / 快照 / 聊天缓冲 / 前端场景管理。

**(b) `NpcRuntime` 与对话引擎的位置**
切片 1 不做 NPC，但要给房间状态留 `npcs: Vec<NpcState>` 字段并进快照（空数组）。原因：`snapshot_full` 的形状一旦发布，加字段比改字段便宜得多，而且前端渲染循环可以从第一天就是"players + npcs + decorations 三类实体"的统一结构，避免切片 5/6 各插一套渲染分支。

---

## 三、逐切片实现要点

### 切片 1 — 骨架 + 实时脊椎

**最难的两点**

1. **10Hz delta tick 与 WS 生命周期**（决策 1）。经典陷阱三个：`broadcast::channel` 满时 `RecvError::Lagged` 不处理 → 玩家永久卡死；tick task 与房间 entry 生命周期不匹配 → 空房间 task 泄漏；`DashMap` entry 持有引用跨 `.await` → 死锁。
   **推荐做法**：房间首次创建时 spawn 一个 `tokio::time::interval(100ms)` 的 tick task，task 持 `Weak` 引用，房间空且连续 N 次 tick 无人则自行退出并清理 entry。delta 计算方式：每个 `PlayerState` 带一个 `dirty: bool`/`rev: u64`，tick 时只打包 `rev > last_sent_rev` 的玩家。**每个连接维护自己的 `last_sent_rev` 会很贵**（O(玩家²)），MVP 用"全房间共享一个 tick 序号 + 本 tick 内变动集合"即可：50 人 × 24 字节 = 1.2KB/帧，10Hz = 12KB/s/连接，50 连接 = 600KB/s 出向——单 VPS 完全够，别过早优化。`Lagged` 一律降级为"给该连接补发一次 `snapshot_full`"。
2. **客户端插值**。缓冲 100–200ms，用服务端 `t` 排队，在 `t_render = t_now - delay` 处对两帧线性插值；本地玩家用**预测 + 服务端纠正**（收到权威位置若偏差 > 阈值则平滑收敛，不要硬 snap，否则移动会抽搐）。
   **推荐做法**：插值器写成纯函数（§2.3），单测里灌造好的快照序列。

**验收测试写法**

```
一个 #[tokio::test]：
  起真 router 到 127.0.0.1:0（拿随机端口），DATABASE_URL 指向 tempfile
  reqwest 客户端 A：register → login（保留 cookie jar）
  reqwest 客户端 B：register → login
  A、B 各开一个真 WS（带 cookie）
  断言 B 收到 welcome + snapshot_full，且 players 含 A
  A 发 {move, tx:5, ty:3}
  在 500ms 超时内轮询 B 的帧，断言出现 snapshot_delta 且含 A 的新位置
  A 发 {move, tx:-999, ty:-999}，断言服务端后续快照里 A 仍在合法区域（clamp 生效）
  A 发 {chat,"hi"}，断言 B 收到 chat
  新开客户端 C，断言 C 收到 chat_backlog 含 "hi"
  最后：sqlx 查询整库所有表名，断言无任何表存有 "hi"（决策 8 的机器化守卫）
```
把这个 harness 抽成 `tests/harness.rs` 的 `spawn_app() -> TestApp { base_url, db, ws(cookie) }`，后续所有切片复用。**这是切片 1 最重要的交付物，比游戏功能重要。**

---

### 切片 2 — Avatar 模型 + Modular

**最难的两点**

1. **图层合成在哪里做**。三种选择：(a) 前端 Phaser 多 Sprite 叠加，(b) 前端离屏 canvas 预合成成一张纹理，(c) 后端 `image` crate 合成后缓存 PNG。
   **推荐 (b)**：50 个玩家 × 4 图层 = 200 个 Sprite，移动端 draw call 压力大且换色难；后端合成会让每次改色都变成一次生成+存储，违背"轻量"。(b) 的做法：拿到 `AvatarSnapshot` 后在 offscreen canvas 上按图层顺序绘制整张 sheet，做完 HSL 换色，`scene.textures.addCanvas(key, canvas)`，之后就是**单 Sprite 渲染**，与 generated 形象完全同一条路径（这正是切片 4 "同一渲染路径"的前提）。缓存键 = `AvatarSnapshot` 的稳定哈希，避免重复合成。
2. **HSL 换色的像素艺术正确性**。朴素 HSL rotate 会毁掉像素画的调色板层次（阴影/高光变脏）。
   **推荐做法**：preset 的可换色图层用**索引化的"色带"**（例如头发 4 档明度写死为 4 个特定 RGB），换色时做**查表替换**而不是逐像素 HSL 数学。每个 preset 附一份 `ramp: [[r,g,b] × 4]`，用户选的目标色生成新 ramp（保留原有明度差）。这样出来的像素画永远干净。写进 preset 元数据里。

**验收测试写法**：整数级断言优于截图。`applyServerMsg` 的单测里断言 `AvatarSnapshot` 正确进出；集成测试断言 `PUT /api/avatar` 后 delta 帧里 avatar 的哈希变了。合成正确性用**一个 node 侧的合成器单测**：喂固定 preset + 固定颜色，断言输出 canvas 的若干采样像素等于预期 RGB（3–5 个采样点，不做全图比对，避免脆弱）。视觉审美走人工/Playwright 截图，不进 CI 断言。

---

### 切片 3a — 素材基建

**最难的一点**：`public_url` 的两种形态（本地 `/assets/...` 由 `ServeDir` 提供；R2 可能需要签名或自定义域）如何在不改业务代码的前提下都成立。
**推荐做法**：`public_url` 返回**绝对 URL**，本地实现用 `PUBLIC_BASE_URL` 拼；前端永远原样使用，绝不自己拼。

**验收测试写法**：参数化测试——同一组断言跑两次，一次 `LocalFileSystem`（temp dir），一次 `InMemory`。这是 PRD user story #31 唯一可信的证明。

---

### 切片 3b — 生成管线 + 配件

**最难的两点**

1. **worker 的崩溃恢复与幂等**（决策 4）。job 表 status 机 `pending → submitted(provider_job_id) → done|failed`；worker 启动时先扫 `submitted` 继续 poll，再扫 `pending` 提交。加 `attempts` + 指数退避 + `max_attempts`，失败写 `error` 文本。**绝不允许"HTTP handler 里 await 生成"**——把这条变成回归测试：断言 `POST /api/generate` 的响应时间 < 100ms（用 stub 但 stub 内部 sleep 300ms 来证明不是巧合）。
2. **配件锚点的逐帧对齐**。这是最容易"看起来差一点点"的地方。
   **推荐做法**：锚点数据由 preset 制作时（离线，用 MCP + 手工）标定并写进 sidecar；运行时合成用 `anchors[clip][dir][frame]` 定位。给 `back` / `hand` 各定义一个 z 序（back 在身体下，hand 在身体上）。做一个开发期调试开关：按某键在锚点位置画 1px 十字，肉眼即可校准。

**验收测试写法**：集成测试全走 stub，断言状态机每一跳；配件合成在 node 侧单测：断言 `hand` 槽的配件像素出现在第 2 帧的期望坐标（±0 像素，锚点是精确整数）。

---

### 切片 4 — 照片形象

> ✅ Spike-0 已完成（2026-08-04）。本切片默认走 **vision-bridged 路径**（`image → VisionClient → text → PixelLabClient.create-character-with-4-directions`），仅在用户对身份一致性要求高时降级到 PRD §五 路径 B（多步链）。
>
> 详见 `.scratch/issues/0009-vision-bridged-pixel-art-pipeline.md` 与 `docs/image2pixel-demo.md`。

**最难的两点**

1. **视觉模型选型与描述质量**（Spike-0 已验证 MiniMax-M3 可用，但未与 Claude Sonnet 4 / Opus 4.7 / GPT-4V 横评）。`VisionClient` 必须抽象，**禁止把 MiniMax 绑死**（见 §1.3.1）。描述 prompt 模板是切片 4 拥有的资产，加单测断言：5 张样本照片 → 描述 ≤80 词、含 species/proportions/colors/distinctive features 四要素。
2. **隐私：原图不落库不留盘**。链路要用到原图 base64 传给 vision 与 PixelLab。
   **推荐做法**：原图存进 `tempfile::TempDir`（进程级），路径记在 job 的 `params_json` 里，job 终态时**在同一个函数里**删除并写 `photo_purged_at`；进程启动时无条件清空整个 temp 目录。**不要**把原图放进 `AssetStore`——一旦进了 object_store 就会被备份、被 R2 同步，隐私承诺失效。
   验收：测试在 job done 后 walkdir 断言 temp 目录为空。

**风格一致性（新增于 Spike-0 后）**

- 发 `create-character-with-4-directions` 时**必须**带 `color_image`（一份固定的 GBA emerald 调色板 PNG，~1 KB）+ `force_colors: true`，否则多人生成的 sprite 调色板会漂，房间里并排看起来割裂。
- `view: "low top-down"`、`outline: "single color black outline"`、`shading: "basic shading"`、`detail: "medium detail"` 是硬默认，写进 `PixelLabClient` 的封装里，不让 caller 改。

**同一渲染路径的守卫**：加一条测试或 lint 断言 `scene/` 目录下不出现 `kind === 'generated'` 的条件分支（只允许在 `game-state`/装载层出现）。这是"双管线不互通但渲染统一"（决策 5）的机械化保障。

**4 方向 vs 8 方向的取舍**

- 切片 4 用 4 方向起步（`create-character-with-4-directions`）。够 MVP，多走对角线时视角退化在低密度房间里肉眼不可见。
- 8 方向是 `create-character-with-8-directions` 或 `create-character-v3`，贵 1.3×、多 ~10 s、风格一致性更难保持。留给"future polish"，不在 MVP。

---

#### 切片 4 附录 — 生成质量的自检–重试闭环（QA Loop）

**问题**：Spike-0 验证的管线（`照片 → vision → 文本 → create-character-with-4-directions`，~100 s / ~$0.013）**一次上传只产一个形象**。质量中庸时用户唯一的出路是"再传一次盲赌"——既浪费钱，也没有任何信息告诉管线上次差在哪。

**一句话设计**：把"判分"拆成**免费的确定性像素检查** + **结构化属性召回** + **粗粒度风格是非题**；三者只产出一个三档判决和一组**固定词表**的修复动作；闭环整体跑在 worker 的 job 状态机里，HTTP 路径零改动；最终选择权交给用户（候选并排），不由分数独裁。

##### A1. 自检指标：不要用 CLIP

CLIP 相似度在这里是错的工具，三条理由：

- **跨域主导**：512 px 照片 vs 64 px、≤32 色、带黑描边的 sprite，余弦相似度基本被域差吃掉，好坏样本分布重叠严重，阈值切不干净
- **答非所问**：它回答不了"是不是 GBA 掌机审美 / 描边干不干净"——那是**像素统计**问题，不是语义相似问题
- **工程代价**：单二进制里塞 ONNX runtime 或再起一个 embedding 服务，违背已锁定的技术栈

改用三层，从免费到便宜：

**L0 — 确定性像素检查（Rust，$0，硬门）**。这些恰恰是视觉模型答不准、而像素统计一答一个准的：

| 检查 | 判据（64×64 档） | 失败含义 |
|---|---|---|
| 非空 | `alpha>0` 像素占比 ∈ [8%, 85%] | 空图 / 糊满整幅 |
| 背景透明 | 四角各 8×8 全 `alpha=0` | 背景没抠干净 |
| 调色板规模 | 量化后独立颜色数 ≤ 32 | 抗锯齿/渐变，压根不是像素画 |
| 描边闭合 | 主体外轮廓上暗色像素的连通占比 ≥ 0.9 | 描边断裂 |
| 主体占比 | bbox 面积/画布 ∈ [0.35, 0.95] 且不触边 | 太小 / 被裁 |
| **方向一致性** | 4 方向量化色直方图两两 L1 距离 ≤ τ | **"每个方向像不同人"——正是 R2 的失败形态** |

L0 全绿才值得为这一版花评审的钱；任一硬项失败直接进重试，且**失败项本身就唯一确定了修复动作**（见 A2 表）。

**L1 — 属性召回（复用同一个 vision 模型，~$0.002）：这才是"像不像输入照片"的可用指标**。

关键改动：**把第一步 vision 的输出从散文段落改成结构化 JSON**（这是对现有 prompt 模板的直接修改，与本节 §最难的两点·第 1 条的"四要素单测"合并）：

```
SubjectSpec {
  subject_type, is_humanoid, template_hint,
  palette: [基本色词 × 2–4], garments: [...], distinctive: [...],
  pose, confidence
}
```

critique 阶段**用同一 schema、同一 prompt** 去描述**输出的 sprite**，得到 `SubjectSpec'`，比对放在 Rust 里做：

```
recall = |attrs(SubjectSpec) ∩ attrs(SubjectSpec')| / |attrs(SubjectSpec)|
```

颜色只在 **11 个基本色词桶**上比对（像素画的色名必然漂移，"chestnut" 与 "brown" 不能算不同）。

为什么这比"把两张图一起丢给模型打分"好：**单图输入**（不依赖多图能力，换 vision 供应商时约束更少）、**可离线用 fixture 复现**、**缺失的属性本身就是重试提示**（修复动作免费得到）、**分数可解释可断言**（"丢了红围巾"能写进测试，"相似度 0.62"不能）。

**L2 — 风格评审（与 L1 同一次调用，$0 增量）**：3–4 个**是非题** + 每题一句理由（GBA 掌机审美？单色描边？无抗锯齿渐变？低俯视 3/4 视角？）。**不要让模型打 1–10 分**——LLM 的绝对分校准很差且随 prompt 漂移；只取是非，聚合成通过率。

**综合与三档判决**：`score = 0.6·recall + 0.4·style_pass_ratio`；`≥0.75` accept / `0.45–0.75` retry / `<0.45` 或 L0 硬失败 → retry（带硬修复动作）。

两条让这个指标不沦为玄学的纪律：

1. **阈值必须校准，不能拍脑袋**。建一个 10–15 张照片的 golden set（人工标 good/bad），把 critique 的**原始 JSON 存成 fixture**；CI 跑一个**离线**测试断言当前阈值在 golden set 上不误杀 good、不放行 bad。prompt 版本或模型一换就重跑。这是把 LLM 评审变成可回归资产的唯一办法。
2. **选优用两两比较，不用绝对分**。LLM 的排序能力显著强于打分。最终从候选里挑"最好的一版"时做一次 pairwise（候选 ≤3，最多比 3 次）；绝对分只用于 accept/retry 这个粗粒度门。

##### A2. 定向重试：固定词表，不是自由改写

**禁止让 LLM 自由重写 prompt**：不可复现、不可测试，且把用户自由文本直连付费图像 API 是滥用面。改为**失败信号 → 修复动作**的固定映射（注意：`color_image` + `force_colors` + `view/outline/shading/detail` 已是本节规定的硬默认，重试**不得**推翻它们，只能在其上收紧）：

| 信号 | 修复动作（每轮至多改 2 个旋钮） |
|---|---|
| recall 缺属性 X | description 追加 `, prominent {X}`；`text_guidance_scale` 8 → 11 |
| 调色板超标 / 抗锯齿 | `shading: flat shading` + `detail: low detail`（`force_colors` 本就是 on） |
| 描边断裂 | `detail: low detail` + 换 `seed`（`outline` 已是硬默认，只能靠降复杂度） |
| 主体太小 / 被裁 | `proportions: {preset: chibi}`；或换 `image_size` 档 |
| 体型模板错判（人被当四足等） | 依 `SubjectSpec.is_humanoid` / `template_hint` 改 `template_id` |
| 方向不一致 | 取最好的 south 帧走 `create-character-v3` 参考图模式（路径 B 的**局部**降级，+$0.041） |
| 全通过但分数中庸 | 只换 `seed` 重摇 |

- **每轮必换 `seed`**：同 prompt 同 seed 很可能拿回同一张图，那次重试就是纯浪费。
- **不重复施加无效动作**：attempt 记 `moves_applied`；某动作施加后对应子分没提升，标记无效，后续轮次不再选它。
- **永不因为重试而丢弃上一轮产物**：重试完全可能更差，"最好的一版"必须始终可选。

**历史必须留**——新表，三个理由都是硬需求（A/B 选择、无效动作判定、成本审计）：

```
generation_attempts(id, job_id, attempt_no, prompt, params_json, provider_job_id,
                    result_asset_id, scores_json, verdict, moves_applied,
                    cost_usd, created_at)
generation_jobs     加 best_attempt_id
```

##### A3. 成本封顶：用钱封顶，不只是次数

- **每 job**：`max_attempts = 3` **且** `max_cost_usd = 0.08` **且** `max_wall_clock = 12 min`，任一触顶即停
- **每用户每日**：job 数配额 + USD 配额（DB `count`/`sum` 即可，R1 已经要求）
- **全局**：日预算熔断 + 启动时与定时 `GET /v2/balance` 低余额告警 + `PIXELLAB_ENABLED=false` 杀开关

最坏情况算术（64×64 / 4 方向），这是全节唯一需要背下来的两个数：

```
1 × 抽取 $0.002 + 3 × (生成 $0.0122 + 评审 $0.002) ≈ $0.045 / 上传
墙钟 ≈ 3 × 100 s + 开销 ≈ 5–6 min      仍在 PRD "5–9 分钟" 承诺内
```

**最省钱的一条规则是在花 PixelLab 的钱之前先拒绝坏输入**：抽取阶段若 `confidence` 低或 `subject_type = none`（糊图、纯风景、多主体合影），直接 `failed` 并给用户**可读的原因**，总花费 $0.002、零 PixelLab 调用。这条比任何重试策略都更能压住 P99 成本。

提前停机三条：达标即停；**边际收益停机**（本轮 score 较上轮提升 < 0.05 就不再重试，改判 `needs_review`）；**不可修复类失败不消耗 attempt**（输入问题、供应商 5xx 走既有的 `attempts` 退避，不算质量重试）。

把这些做成**回归测试**而不是注释：stub 计数调用次数，断言 job 终态时 PixelLab 调用 ≤3、累计 `cost_usd` ≤ 上限。与切片 3b "响应时间 < 100 ms" 同一套路——把护栏变成可回归的事实。

##### A4. 缓存与去重

哈希对象是**归一化后的图**，不是原始文件字节：解码 → 去 EXIF → 长边缩到 512 → 重编码 PNG → **HMAC-SHA256（服务端盐）**。原始字节哈希没用（重存一次、换个截图工具就变），感知哈希又太松（同一个人的两张不同照片会撞成一个）。我们要回答的问题是"**同一张照片又传了一遍吗**"，精确哈希正好命中这个语义。

两层缓存，**语义不同，不要混**：

1. **抽取缓存** `image_hash + prompt_version → SubjectSpec`（TTL 30 天）：命中率高、安全、省 $0.002 与 ~5 s。隐私边界：原图照常 purge，只留**加盐**哈希 + 派生文本（不加盐的话，任何持有某张照片的人都能反查"这张传过没有"）。
2. **结果缓存** `image_hash + params_hash + pipeline_version → best_attempt`：**按 `owner_id` 分区，禁止跨用户复用**——跨用户命中会泄露"别人传过同一张照片"。而且**不静默复用**：同一用户重复上传时前端提示"你已经生成过这张照片的形象：**直接用旧的 / 重新生成（换 seed）**"，因为重传的用户意图通常恰恰是"再试一次"。提供 `force_new: true` 绕过。

`pipeline_version` 必须进 key（prompt 模板、修复词表、vision 模型 id、PixelLab 端点参数，任一变化即 bump），否则改完 prompt 却拿回旧结果，是最难查的一类 bug。
**不要依赖供应商的确定性**：同 seed 是否严格复现**未实测**；缓存是我们自己的存储，不是"重算一次应该一样"的假设。

##### A5. A/B：用户怎么挑

- Library 的 job 详情页出**候选条**：2–3 张并排、统一 3× 整数缩放、默认显示 south，点按/hover 循环 4 方向；每张带分数徽章 + **一句理由**（直接来自 L2 是非题的理由字段）；按分排序，最优项**预选但不独裁**——永远不要只展示自动选中的那一张，那等于把"盲赌"换成"盲信"
- **原照片不展示**（早已 purge）。只有当前会话内可用浏览器本地的 `File` 对象做对照，刷新后消失——UI 文案要写清楚，别让用户以为我们弄丢了
- 选定 → `POST /api/jobs/:id/select {attempt_id}` 写 `avatars.sprite_asset_id`；未选中的候选保留 7 天后 GC（存储便宜，但不是免费）
- **"再来一版"走同一套修复词表的 chips**（更鲜艳 / 描边更干净 / 更 Q 版 / 强调：____），而不是自由 prompt 框。只留一个限长 + 字符白名单的"强调"字段，其余全是白名单动作——与 A2 完全复用同一条代码路径
- 这一切都在 `ui/`（library）里，`scene/` 一行不改，不触碰禁令 3

##### A6. 在异步流水线里的位置

全部在 worker 内，HTTP 路径零改动（禁令 1 不破）。job 状态机扩成：

```
pending
  → extracting            vision: photo → SubjectSpec   ← 坏输入 fail-fast，成功后立刻 purge 原图
  → submitted(attempt n)  provider_job_id
  → grading(attempt n)    L0 像素检查 + L1/L2 vision critique
  → decide ─┬─ accept  → done
            ├─ retry   → submitted(n+1)
            ├─ 触顶    → needs_review     （有候选，没跨线，等用户裁决）
            └─ 无候选  → failed
```

三个要点：

1. **新终态 `needs_review`**：既不是 `done` 也不是 `failed`，library 必须能把它渲染成"需要你挑一张"。这个状态就是"用户不再盲赌"的落点。
2. **原图在抽取成功后立即删**，而不是 job 终态时删。默认管线的重试只需要 `SubjectSpec` 文本，不需要原图——这把原图存活期从 ~10 分钟压到 **~5 秒**，**比本节第 2 点当前的隐私承诺更强**。**例外**：启用路径 B 降级（`portrait-character-pro` 需要原图贯穿全链）时退回"job 终态删"，该分支要显式记在 job 的 `photo_retain_until_done` 标志上，两条路径各有一个 walkdir 断言测试。
3. **崩溃恢复的原子单元是 attempt 不是 job**：重启后扫 `submitted` 续 poll、扫 `grading` 重跑评分；评分幂等（`scores_json` 非空则跳过），重跑最多多花 $0.002。

`GET /api/jobs/:id` 扩成 `{status, stage, attempt, max_attempts, candidates:[{asset_url, score, verdict}]}`——第一张候选在 ~2 分钟就能露面，而不是 6 分钟白屏。这正是 PRD §Generation async UX "library 页是 come back later 界面"的自然延伸：**回来时看到的是几个可选项 + 理由，而不是一个结果**。

**测试形状**：stub `VisionClient` 必须支持**脚本化判决序列**（bad → bad → good），使重试循环无需网络即可确定性验证。三条必须绿的断言：(a) 首轮达标时只调 1 次 PixelLab；(b) 连续不达标恰好调 3 次且终态为 `needs_review`；(c) golden set 校准测试。

##### A7. 对切片 4 / issue #5 的具体修改清单

1. **AC 增补**：新增 `needs_review` 终态 + `POST /api/jobs/:id/select` + "job 完成后 library 至少呈现 1 个、最多 3 个候选，每个带分数与理由"
2. **AC 修改（隐私更强）**：原图删除时机从"job 终态"改为"**抽取成功后立即**"；路径 B 降级分支保留旧语义，两条各一个测试
3. **本节第 2 点的推荐做法**据上一条改写（temp 文件生命周期缩短到单个阶段内）
4. **vision 输出契约变更**：散文段落 → 结构化 `SubjectSpec` JSON；`strip_think()` 之后加 JSON 解析与 schema 校验，解析失败按低 confidence 处理；原"≤80 词四要素"单测改为 schema 断言
5. **新增 migration**：`generation_attempts` 表 + `generation_jobs.best_attempt_id` + `vision_cache` 表
6. **`VisionClient` trait** 与 `PixelLabClient` 同形（submit/poll 分离、领域类型不是供应商请求体），且必须能被 stub 成脚本化判决序列——建议与 `PixelLabClient` 一起在切片 1 只定类型骨架
7. **成本护栏进 AC**：`max_attempts` / `max_cost_usd` / 每用户日配额，各配一条 stub 计数断言（R1 要求的"3b 一次做好"在这里落地）
8. **golden set + 阈值校准测试**作为切片 4 的 DoD 之一：`fixtures/qa-golden/` 存 10–15 组 critique JSON + 人工标注，CI 离线跑
9. **风险登记补一条 R10**：*评审模型误判导致自动重试烧钱或误杀好结果*。缓解 = 三档粗判决（不是连续分）+ 边际收益停机 + golden set 校准 + `needs_review` 兜底交人裁决

---

### 切片 5 — 场景 + NPC + menu

**最难的两点**

1. **服务端与客户端共用同一份 .tmj 碰撞数据**。
   **推荐做法**：`.tmj` 放在 `assets/maps/`，构建时同时被 Vite 打包给前端、被 Rust 在启动时读取解析成 `TmjGrid`。**只解析需要的层**（一个命名约定：`collision` 层的非零 gid = 不可走）。绝不在 Rust 里手写第二份碰撞表。加一个测试：加载 bar.tmj，断言若干已知坐标的可走性（选吧台内/外各 2 点）。
2. **场景切换的原子性**。玩家从 bar 移到 yard 时要：从 bar 房间摘除 → 广播 `remove` → 加入 yard → 广播 `upsert` → 给自己发 `scene_changed` + `snapshot_full`。顺序错会出现"幽灵玩家"（在两个场景都存在）或"闪现"（新场景玩家在旧坐标）。
   **推荐做法**：把切换写成 `Room` 之外的一个函数，持锁顺序固定（先摘后加），并加一个测试：A 在 bar、B 在 yard，A 过门后断言 B 收到 A 的 upsert **且**任何一帧里 A 都没有同时出现在两个场景。

**NPC 对话引擎**：纯数据驱动 `{ nodes: { id: { text, choices:[{label,next}], menu?:MenuId } } }`，服务端持有"每玩家×每 NPC 的当前节点"在**内存**里（不落库，重连回到根节点即可，符合轻量原则）。menu payload 单独一个 JSON 文件，接口 `MenuPayload { id, sections:[{title, items:[{name, desc, price?}]}] }`——设计 TBD 但结构定死，将来换内容不改代码。

---

### 切片 6 — admin 装饰物 + 成员管理

**最难的两点**

1. **REST 写入与 WS 广播的一致性**（决策 2）。若先广播后写库、或写库成功但广播失败，客户端与 DB 会漂移。
   **推荐做法**：**先 DB 事务提交，成功后再广播**；给每个 `decorations` 变更分配一个单调递增的 `rev`（可用 `decorations` 的 rowid 或一个独立计数器），`decoration_added/removed` 消息携带 `rev`，`snapshot_full` 携带当前 `rev`。客户端若收到 `rev` 不连续，就重新 `GET /api/decorations` 全量拉一次。这条"rev + 缺口重拉"是 30 行代码换掉一整类"我看到椅子他没看到"的玄学 bug。
2. **同格冲突与踢人**。两个 admin 同时点同一格：DB 上给 `(scene, tile_x, tile_y, z_layer)` 加 UNIQUE 约束，第二个返回 409。ban 一个在线用户：需要一条从 REST handler 到 WS 连接的定向通道——房间里存 `player_id -> mpsc::Sender<ServerMsg>`（这个 map 切片 1 就该有，因为 `error` 和 `dialogue` 都是定向消息，不是广播）。**若切片 1 只做了 broadcast 没做定向通道，切片 5 的 dialogue 就会被迫广播给所有人**（隐私+带宽双输）。← 这条并入 §二 的预重构：**切片 1 必须同时具备 broadcast（快照/聊天）与 per-connection mpsc（定向响应）两条出向路径**。

**验收测试写法**：两个 WS 客户端 + REST admin 操作，断言广播到达与 `rev` 连续；非 admin 打全部 admin 端点断言 403（用一个端点清单表驱动，防止将来新增端点忘记 gate）。

---

### 切片 7 — 部署 + 运维

**最难的两点**

1. **Caddy 后的 WebSocket**（Caddy 2 默认透传，但配了 `encode` / 自定义 header / 反代路径重写时容易踩坑）+ **长连接与 systemd 重启**。
   **推荐做法**：Caddyfile 保持最小（`reverse_proxy localhost:8080` + `encode`），部署后**必须**用 `websocat wss://域名/ws/...` 手工验证一次握手。前端加自动重连（指数退避 + 重连后重新走 `welcome/snapshot_full`）——这个重连逻辑其实**应该在切片 1 就写**，因为它同时解决开发期 `cargo run` 重启导致的白屏。
2. **备份的可恢复性**。用 `sqlite3 .backup`（非 `cp`，WAL 一致性），并且**跑一次恢复演练**写进 runbook。

**其他**：`sqlx` 离线编译（提交 `.sqlx/`）否则 CI/交叉编译会因缺 DATABASE_URL 而炸；交叉编译 musl 静态二进制在本机做，别在 4GB VPS 上编译。

---

## 四、风险登记

| # | 风险 | 触发条件 | 规避 / 缓解 |
|---|---|---|---|
| R1 | **PixelLab 成本/延迟失控**（定价不透明；照片管线可能是 3 步付费链） | 单形象成本 > 预算，或端到端 > 5 分钟，或月度用量随成员数线性爆炸 | Spike-0 前置量化（§1.3）；`generation_jobs` 加**每用户配额**（每日 N 次，DB count 即可）与全局日预算熔断；结果**永久缓存**，相同 params 哈希直接复用旧 asset；配置化 `PIXELLAB_ENABLED=false` 的降级开关（关掉后 UI 只提供 modular）；成本护栏必须在 3b 一次做好，不能等"以后再加" |
| R2 | **照片→8 方向不可行 / 风格不一致**（`create_character` 是 text 驱动，非 photo 驱动） | Spike-0 输出的 8 方向人物"每个方向像不同人" | 降级路线现在就写进 issue #5：照片只出**单方向立绘 + 头像**，8 方向走 modular（用照片提取的配色作为 modular 基底的 ramp）。切片 4 不在关键路径上，可整块延后；不要让它阻塞上线 |
| R3 | **30–50 并发同步抖动**（`Lagged` 丢帧、tick 抖动、移动端插值卡顿） | 真实 40 人同房，或移动端 4G 抖动 | 决策 1 的 delta + 插值；`Lagged` 降级为补发 full snapshot；tick 用固定 interval + `MissedTickBehavior::Delay`；**在切片 1 就写一个负载脚本**（50 个假 WS 客户端随机走动）并跑在开发机上，把"50 人能跑"变成可回归的事实而不是上线当天的祈祷；JSON → MessagePack 的切换点已由 `v` 字段预留 |
| R4 | **形象双管线复杂度渗漏**（决策 5 的"互不互通"被后来的"要不要给照片形象加个帽子"打破） | 任何一次"就加个小 if" | 渲染层单一路径 + `equip` 对 generated 硬 400（已在切片 4 AC 里）+ §三 的 lint 守卫（`scene/` 禁出现 kind 分支）。产品层面写一句话进 README：这是**有意的**不互通，不是待办 |
| R5 | **移动端性能 / 横屏适配**（240×160 整数缩放 + 50 个合成纹理 + DOM 摇杆） | iOS Safari 中低端机、横屏 vs 竖屏切换、地址栏遮挡 | 合成纹理**按 AvatarSnapshot 哈希缓存**并设上限（LRU，例如 64 张）；远处玩家降级为 idle 帧不播动画；摇杆用 DOM/CSS 不进 Phaser 渲染；**在切片 1 就在真机上跑一次**（不是模拟器），把"横屏 + 无平滑 + 60fps" 变成切片 1 的 DoD 之一而不是最后惊喜 |
| R6 | **SQLite 写并发 / `SQLITE_BUSY`** | 多个 admin 同时改装饰 + worker 写 job + session 写入撞在一起 | WAL + `busy_timeout=5000` + `synchronous=NORMAL`（启动时 PRAGMA）；**写路径全部走单一 pool 且 `max_connections` 对写做限制**（sqlx sqlite 建议：读池多连接，写走 1 连接的独立池，或全局 pool size 小）；聊天不落库（决策 8）已经消掉了最大写压力源；tower-sessions 用 sqlite store 时注意它的清理任务频率。真出现持续 BUSY 才考虑 PG（不在 MVP 范围） |
| R7 | **admin 实时同步冲突/漂移**（客户端装饰物状态与 DB 不一致） | 广播丢失、两 admin 并发、客户端重连期间发生变更 | `rev` 单调号 + 缺口重拉（§三 切片 6）；`(scene,tile,z)` UNIQUE → 409；`snapshot_full` 始终携带装饰全量 + 当前 rev，重连自动收敛 |
| R8 | **契约漂移**（Rust `ServerMsg` 与 TS `ServerMsg` 手工维护两份，逐渐不一致） | 任意切片新增消息字段 | MVP 不引入代码生成（违背轻量），改为：`protocol/` 目录下 TS 类型 + Rust enum 相邻放置并在 `docs/ws-protocol.md` 里作为单一权威；**集成测试用真实 Rust 序列化输出的 JSON 喂给 TS 侧的 `applyServerMsg` 单测**（把几个代表性帧存成 fixture 文件，由 Rust 测试生成、TS 测试消费）——这条 fixture 桥是低成本的漂移探测器，切片 1 建立 |
| R9 | **单人 solo 项目的范围膨胀** | 每个切片都"顺手多做一点" | 每个切片的 AC 就是范围合同；`Out of Scope` 清单（CRT/BGM/LLM/分片/多房间）在每次动手前重读一遍。切片 7 之后才允许考虑 polish |

---

## 五、可持续性自检（6 个增长点是否"加法"）

| 增长点 | 现状判定 | 结论与切片 1 需补什么 |
|---|---|---|
| **1. 迁移到 R2** | ⚠️ 条件满足 | PRD 只说"用 `Arc<dyn ObjectStore>`"，**不够**——缺 `public_url` 归口。补 §2.2 的 `AssetStore` facade + "DB 只存 key 不存 URL" 约定 + 双实现参数化测试。补完后迁移 = 改一个 env + 一个 builder 分支。**（切片 1 只需定义 trait；实现可留到 3a，但 trait 与 URL 归口必须在切片 1）** |
| **2. 换生成商** | ⚠️ 条件满足 | trait 存在但形状不对。必须是 **submit/poll 分离 + `GenSpec` 是我们自己的领域类型（不是 PixelLab 的请求体）**（§2.1）。另需：`assets.meta_json` 里记 `provider_id` + `provider_meta`，将来才能识别"这是老供应商产的"。补完后换商 = 新增一个 impl。**（切片 1 定义 trait 与 GenSpec；stub 实现即可，真实现留 3b）** |
| **3. 加第二个场景** | ❌ 现状不满足 | PRD 明说 "one room per server"，若照字面实现，切片 5 就是重写。**切片 1 必补**：`DashMap<SceneId, Room>` + `Arc<dyn WalkGrid>` + `Portal` 概念占位 + `scene_changed` 消息类型（未实现也要声明）+ 前端 scene 概念（哪怕只有一个 `"placeholder"`）。补完后加场景 = 加一个 .tmj + 一行注册 |
| **4. NPC 升级 LLM** | ⚠️ 条件满足 | 关键不是对话内容，而是**定向异步响应通道**。切片 1 必补：per-connection `mpsc::Sender<ServerMsg>`（§三 切片 6 的 R 点），以及 `dialogue` 是**服务端主动推送的独立消息**而不是"interact 的同步返回值"。若做成 request/response 语义，LLM 的 2 秒延迟就会阻塞 WS 处理循环，届时要重写。补完后升 LLM = worker 里换一个 dialogue provider（同 `PixelLabClient` 的 trait 套路） |
| **5. CRT/BGM 后处理** | ✅ 满足（Phaser 4 pipeline 是 post-process，加法） | 唯一前提：**渲染分辨率与缩放策略集中在一处**（一个 `renderConfig.ts`），不要在多个 scene 里各写 `setScale`。切片 1 顺手做到即可，成本 0 |
| **6. 扩到 >50 CCU / 分片** | ⚠️ 部分满足 | 单进程 + `DashMap` 分场景已经是天然分片单元（`SceneId`），垂直扩容（更大 VPS）是纯加法。**真正的水平分片（多进程/跨机）不是加法**——需要跨进程消息总线，与"轻量/无 Redis"直接冲突。判定：**这不该在 MVP 里预留**，但要预留**观测**：切片 1 加一个 `GET /api/metrics`（或 tracing 里定期打）暴露 `rooms/players/tick_lag_ms/broadcast_lagged_count`，让"该扩容了"是数据驱动的。同时 `v` 字段与协议前向兼容规则让将来换传输层不至于锁步 |

**汇总：切片 1 必须补 4 项**（否则后续是重写不是加法）：
1. `DashMap<SceneId, Room>` + `Arc<dyn WalkGrid>` + Portal 占位（增长点 3）
2. per-connection 定向 mpsc 通道 + dialogue 为推送语义（增长点 4）
3. `AssetStore` facade 含 `public_url`（增长点 1）
4. `PixelLabClient` 的 submit/poll trait 形状 + `GenSpec` 领域类型（增长点 2，只需 trait + stub）

加上 §二 的 sprite 契约（含 anchors）与协议全量 type 清单，这就是切片 1 的"预设计"总账，估计增量 ≤ 1 天工作量。

---

## 六、切片 1 的 Definition of Done

必须**全部**满足才可宣布切片 1 完成、并行启动切片 2 与切片 5。分四组：

### A. 功能可见（PRD AC 的直接兑现）
- [ ] `cargo run` 单进程在一个端口上同时服务 `/api`、`/ws`、`./dist`；Vite dev 模式下 proxy 配好，两种模式都能玩
- [ ] register / login / logout / `GET /api/me` 全通；argon2id 参数 m=19456,t=2,p=1；首个注册用户或 `ADMIN_USERNAME` 被置为 admin，且**有测试断言这条 bootstrap 逻辑**
- [ ] 两标签页可见彼此移动，远端玩家**插值**（肉眼无跳格）；本地玩家有预测且服务端纠正不抽搐
- [ ] 服务端 clamp 生效（客户端作弊发非法坐标无效）
- [ ] 聊天气泡 + 侧栏最近 50 条；新入场者收到 backlog；DB 内无任何聊天痕迹
- [ ] 手机横屏真机（非模拟器）跑通：左摇杆 + 右动作键，整数缩放、无平滑、稳定帧率；桌面 WASD/方向键 + 动作键

### B. 抽象到位（§五 汇总的 4 项 + §二 的 6 条）
- [ ] 房间键为 `SceneId`；`WalkGrid` trait 存在，`RectGrid` 实现之
- [ ] 出向双通道：broadcast（快照/聊天）+ per-connection mpsc（定向 error/未来 dialogue）
- [ ] `AssetStore` trait 定义完毕（含 `public_url`），至少一个实现 + 一个测试用实现
- [ ] `PixelLabClient` trait + `GenSpec`/`Artifact`/`GenStatus` 类型定义完毕 + stub 实现（尚无 job 表也可，只要类型定型）
- [ ] `docs/sprite-contract.md` + sidecar JSON schema 定稿（含 `v`、`dirs`、`clips`、`anchors`），且**已用 Spike-0 的真实 PixelLab 输出校对过**帧尺寸与方向顺序
- [ ] `docs/ws-protocol.md` 列出全部 ClientMsg/ServerMsg type（含切片 5/6 的），未实现者返回 `error{code:"unimplemented"}`；双向"未知 type 静默忽略"规则已实现并有测试
- [ ] 快照携带 `tick` + 服务端 `t`；插值器是纯函数、时钟可注入
- [ ] 前端目录分层 `net / protocol / game-state / scene / ui`，且 CI 里有 lint 禁止前三者 import `phaser`
- [ ] WS 自动重连（指数退避 + 重连后重取 snapshot_full）

### C. 测试基建（后续切片复用的资产）
- [ ] `tests/harness.rs` 提供 `spawn_app()`（随机端口 + 临时 SQLite + 带 cookie 的 WS 客户端工厂），并被至少一个完整端到端测试使用（§三 切片 1 的测试清单全绿）
- [ ] 全部测试**离线**可跑（无任何外网请求），`cargo nextest run` 绿
- [ ] TS 侧 `net` + `game-state` 有单测（含插值时间快进），`vitest` 绿
- [ ] Rust→TS 的 fixture 桥建立：Rust 测试生成若干代表性 ServerMsg JSON 到 `fixtures/`，TS 测试消费之（R8 的漂移探测器）
- [ ] `cargo clippy --all-targets -- -D warnings` 干净；`.sqlx/` 已提交（离线编译可行）
- [ ] 50 个假 WS 客户端的负载脚本存在且跑过一次，记录了 tick 延迟与带宽数字

### D. 项目卫生
- [ ] `CLAUDE.md` 存在，含 v2 文档的 Rust 硬约束 10 条 + 三条本项目专属禁令：(1) 禁止在 HTTP 请求路径上等待生成，(2) 禁止把聊天写进任何表，(3) 禁止在 `scene/` 里出现 avatar `kind` 分支；并注明"v2 研究文档的 SQL/邮件/fal.ai 章节已废弃"
- [ ] `justfile`/`Makefile` 固化 `sqlx migrate run` 的 DATABASE_URL 前置步骤（v2 文档标记的头号踩坑）
- [ ] 一次 Cloudflare Tunnel 分享验证过（外网可访问 + WS 通），提前暴露切片 7 的 WS-over-proxy 风险

### 并行放行判据
- **启动切片 2（Modular）只需**：A 组 + B 组的 sprite 契约、`AssetStore` trait、前端分层、C 组 harness。
- **启动切片 5（场景/NPC）只需**：A 组 + B 组的 `SceneId`/`WalkGrid`/定向通道/协议清单、C 组 harness。
- 两者都**不依赖** `PixelLabClient`（那是切片 3 的事），所以 B 组第 4 项若临时受 Spike-0 阻塞，可以只交付类型骨架而不影响放行——但 **sprite 契约必须等 Spike-0 校对完再定稿**，这是切片 1 唯一的外部依赖，也是把 Spike-0 排在波次 0 的根本原因。