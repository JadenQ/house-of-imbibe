# House of Imbibe 设计评审 · 最终裁定

> 输入：`development-plan.md`（planner）+ 四份视角评审（spec-conformance / simplicity / extensibility / realtime-correctness），共 77 条原始 findings。
> 本文是唯一裁定权威；四份评审为其证据层，冲突时以本文为准。

---

## 一、总体裁定

### `fix-first`

四个视角里 3 个给出 `fix-first`，唯一给 `proceed-with-fixes` 的简洁性视角也明确附加了「Slice 1 在钉死 delta 语义并给 v2 文档加取代标注后才可开工」的前置条件。合计 10 个 blocker，且它们不是分散的小疏漏，而是**集中压在同一处：切片 1 要冻结的实时契约**——`scene` 归属、WS 鉴权、delta 的自愈语义、每帧字节预算、移动模型，五项全部落在切片 1 必须一次做对、后面无法便宜地补的那个 seam 上。三家评审独立地对同一处（PlayerState 无 `scene`）提出 blocker，这本身就是信号。

但**不需要推翻任何已锁定决策，也不需要重划切片**。10 个 blocker 中 9 个的修复形态是「PRD 某一节重写 + 加几个字段/列 + 加一条纪律」，唯一有外部依赖的是 sprite 契约（要等 Spike-0 用真实 PixelLab 输出校对帧尺寸与方向顺序，半天）。骨架选型（单二进制 / SQLite / 一个 broadcast 房间 / 静态底图 + DB 装饰物 / 脚本 NPC）被四份评审一致确认是对的。

因此裁定为 `fix-first` 而非 `proceed-with-fixes`：**先花 §四 的 P0（4 项，1–2 小时）+ P1（13 项，4–6 小时）+ P2（8 个 issue 改写）把文档修到自洽，再开切片 1**。这是本项目回报最高的半天到一天——仓库是 greenfield，此刻改文档的成本近乎零，而这些缺口一旦被切片 1 的代码和测试固化，修复成本会跳到"改协议 + 改前端 + 重跑全部断言"。

| 视角 | 裁定 | findings | blocker | 最严重问题（一句话） |
|---|---|---|---|---|
| 规格一致性 | `fix-first` | 20 | 3 | PRD 写死「单房、player state 无 scene」，但切片 5 要两场景 + 切换，切片 1 会按无 scene 冻结 WS 协议 |
| 简洁性（Fowler） | `proceed-with-fixes` | 21 | 0 | 「delta snapshot」有房间级/每连接两种读法，读错一次全盘返工；且 v2 决策层文档仍规定 email/聊天落库，agent 会照抄出错误的 init.sql |
| 可扩展性（6 增长点） | `fix-first` | 16 | 2 | 被 3 个切片当验收基准的「canonical sprite-sheet contract」从未定义，它是双管线 + 配件 + 换供应商的唯一互操作面 |
| 实时并发正确性 | `fix-first` | 20 | 5 | `avatar_snapshot` 放进每帧状态 → 约 150 KB/s/客户端、整机 ~7.5 MB/s，是既定预算的 ~12 倍，€5 VPS 跑不动；且 WS 鉴权在 PRD 与全部 AC 中完全缺席 |

去重后 **54 条**（10 blocker / 23 major / 18 minor / 3 nit）。三家共提的 `scene` 缺失合并为一条；spec 的 blocker 2 因涉及两个独立修复面（措辞 vs schema 列）拆为两条；「付费 API + 零门槛注册无任何配额」由 major 升为 blocker（它是本项目唯一的现金敞口，且一旦有人注册就可能被刷）。

---

## 二、开发计划（精炼版）

### 2.0 三处必须先定稿的 spec 不一致

| # | 位置 | 冲突 | 处置 |
|---|---|---|---|
| **A** | `docs/pixel-mosaic-game-workflow-v2-rust.md` §2.3 DDL | 含 `email` / `email_tokens` / `messages`（聊天落库）/ `rooms` / `fal.ai` / lettre / imagequant，与决策 6、8 及 PRD Out of Scope 全部冲突。按 CONVENTIONS.md 它属**决策层**，会被当权威读 | v2 文档顶部加状态块「已被 PRD 取代（auth / schema / 聊天 / 生图供应商 / Cargo.toml 五节）；仅 §三 部署、§六 Rust 硬约束现行」，并就地 ❌ 标注三张表与四个 crate；PRD §Stack 下新增「MVP 依赖白名单」（约 15 个 crate，正列） |
| **B** | PixelLab 调研报告自相矛盾（"暂无官方 MCP" vs "官方 MCP server"）；PRD §Stack 写 "via its official MCP/API" 并点名四个 **MCP 工具名** | MCP 是开发期 IDE 工具，**不能**服务终端用户（浏览器无 MCP 客户端 + token 暴露）；且 MCP 参数名与 REST 不同，照搬会 422 | 运行时 = PixelLab **REST v2**（`api.pixellab.ai/v2`，Bearer 仅存后端，端点 `create-character-with-4-directions` / `create-character-v3` / `portrait-character-pro` / `animate-character` / `map-objects`）；开发期素材生产 = MCP（离线，不进代码路径）。两条路径不共享代码 |
| **C** | PRD §Realtime 写 "One room per server"，切片 5 要 bar + yard + 传送 | 若切片 1 把房间当单例，切片 5 必须重写 WS handler / tick / 快照 / 聊天缓冲 | 切片 1 就以 `scene_id` 为房间键（见 seam P1）。同时**现在**定：聊天缓冲是**每场景一个** ring buffer（与气泡语义一致），并把这个选择写进 PRD——别留语义歧义到切片 5 |

### 2.1 波次与并行策略

```
波次 0（与波次 1 同时开跑，不阻塞任何人）
  ├─ Spike-0  PixelLab 真机验证（半天，纯手工，无代码）   ← 最高优先级前置
  └─ Spike-1  Tiled 底图草图 + sprite 契约定稿（半天）

波次 1        #2 切片 1  骨架 + 实时脊椎            ← 唯一串行瓶颈
波次 2（并行）├─ #3 切片 2  Avatar 模型 + Modular
              └─ #6 切片 5  bar/yard 场景 + NPC + menu
波次 3        #4a 切片 3a 素材基建（依赖仅 #2，可提前到波次 2 尾）
波次 4（并行）├─ #4b 切片 3b 生成管线 + 配件   （blocked_by 4a, 3）
              ├─ #5  切片 4  照片形象            （blocked_by 4a, 3）
              └─ #7  切片 6  admin 装饰 + 成员    （blocked_by 4a, 6）
随时插队      #8 切片 7  部署 + 运维             （blocked_by 只需 #2）
```

**关键路径 = 1 → 2 → 3a → 6 → 7**。三处排期自由度必须写进 issue：

1. **切片 4（照片形象）不在关键路径上**。Spike-0 若判定照片管线不可行，切片 4 可整块延后而不影响上线。
2. **切片 7（部署）的 `blocked_by` 从 `[7]` 改为 `[2]`**。部署与 admin 零耦合，却被排在最后 = 把真实用户反馈推迟到第 7 个切片之后，这是 solo 项目最贵的排序错误。切片 1 顺手加一条零成本 AC：`cloudflared tunnel --url http://localhost:8080` 可分享。
3. **切片 3 必须拆 3a/3b**（见下）。

### 2.2 切片 3 的拆分（这是唯一必须动的切片划分）

原 `#4` 捆了两条零共享代码的纵切：(A) 异步生成基础设施（provider trait + real impl + stub + `assets` + `generation_jobs` + store + worker + `/api/generate` + `/api/jobs/:id` + `/api/library`）；(B) 配件装备（equipped/slot 模型 + 逐帧 anchor overlay + equip UI）。而 admin 放置装饰物**不需要**生成能力，只需要 `assets` 表 + store + 一个 curated 素材集。

- **3a 素材基建** — `assets` 表、`AssetStore` facade（含 `public_url`）、job 管线 + worker + `/api/generate` + `/api/jobs/:id` + `/api/library`、配额/成本护栏、curated 素材导入。`blocked_by: [2]`。
- **3b 生成 + 配件** — equipped/slot 模型、逐帧 anchor overlay、equip UI。`blocked_by: [4a, 3]`，**叶子节点，不阻塞任何人**。

收益：`#7 admin` 的依赖变成 `[4a, 6]`，与 3b 并行；关键路径缩短一个切片；且"最容易出实时同步 bug"的 admin/装饰线能更早暴露。代价：多一个 issue 文件。

> 注：把成本/配额护栏放在 **3a**（而非 3b）是刻意的——护栏必须与 job 管线同批落地，不能"以后再加"。

### 2.3 Spike-0 为什么必须前置

PixelLab 调研有两个未闭合空白：**定价不透明** + **`create_character` 是 text-prompt 驱动，不是 photo 驱动**。而切片 4 的验收是"上传照片 → 合规的 8 方向 sprite sheet"。中间很可能是一条多步付费链：

```
照片 → create_image_pixflux(ref_image) 出单张像素立绘
     → create-character-v3(description + style/ref) 出多方向
     → animate-character(walk) 出帧序列
```

三次付费调用、5–9 分钟延迟、**风格一致性不保证**（三步之间人物可能"变脸"）。Spike-0（半天，纯手工，用自己照片跑一遍，不写代码）必须产出三个数字 + 一个判断：

1. 单形象的**总 credit 成本**与**端到端墙钟时间**；
2. 输出 sheet 的**真实网格布局**（帧尺寸、方向顺序、每动作帧数、`keep_first_frame` 是否多一帧、画布自动 padding）——这直接决定 seam P7 的契约，晚定一天就要改一次，且已生成的付费素材不可回滚；
3. 判断：照片→多方向是**一次调用**还是**多步链**？多步链意味着 worker 必须支持多阶段（`phase` 列），这个结论必须在切片 1 定稿前拿到。

**Spike-0 不通过的降级路线**（现在就写进 `#5` 备注）：照片只出**单方向立绘 + 头像**，8 方向走 modular（用照片提取配色作 modular ramp），切片 4 缩范围。

### 2.4 每切片的集成验证点

原则：验证点必须同时穿过 HTTP / DB / WS / 前端渲染四层中至少三层，且能被一条自动化测试或一次两标签页手工操作证明。

| 切片 | 集成验证点（"打通了"的证据） |
|---|---|
| **1 骨架** | 两个标签页各注册一个账号，A 移动时 B 屏幕上 A **平滑插值**（不跳格）；A 发言 B 见气泡 + 侧栏；F5 后侧栏仍有最近 50 条；`select count(*) from sqlite_master where name like '%message%'` = 0。自动化侧：进程内起真 Axum + 临时 SQLite + 两个真 WS，断言 A 的 move ≤200ms 出现在 B 的 delta 帧；**外加**：无 cookie 握手 401、伪造 user_id 的 chat 被回写为真实身份、故意不排空的客户端在 3s 内收到 keyframe 并收敛、50 客户端 loadtest 的带宽数字被打印 |
| **2 Modular** | A 在 builder 里把头发改成紫色并保存，**B 屏幕上 A 的头发不刷新即变紫**。同时验证 REST 写库 → `avatar_changed` 侧信道推送 → 前端合成层重绘。断言 delta 帧里**没有** avatar 载荷；重跑 loadtest 未超预算 |
| **3a 素材基建** | 一个 PNG 走导入路径进 `assets` → `GET /api/library` 列出 → 前端 `<img>` 用响应里的 `public_url()` 绝对 URL 拉到图。**同一套断言换 `object_store::InMemory` 再跑一遍全绿**（不自研第二个 store）——这是 R2 可迁移性的机器化证明。另：超配额 `POST /api/generate` 返 429 |
| **3b 生成 + 配件** | 提交生成 → **HTTP 响应 <100ms 返回 job_id**（stub 内部 sleep 300ms，证明不是巧合）→ 关页面 → 重开 library 看到 done → 装到手部槽 → B 屏幕上 A 手里多了个杯子，且**走路每一帧杯子都跟手**（anchor 契约的真实检验）。stub 首次 poll 必返 Pending |
| **4 照片形象** | 上传照片 → job done → 激活 → **同一条 Phaser 渲染路径**渲出（`scene/` 里不出现 `kind === 'generated'` 分支，只有资源装载分支）；`equip` 与 `PUT layers` 两条路径对 generated 都返 400；job 终态后原图**在磁盘上不存在**（walkdir 断言 temp 目录已清空 + `params_json` 不含图像数据） |
| **5 场景 + NPC** | 玩家**穿不过吧台**（测试直接发一个落在墙里的 move，断言服务端后续快照仍在墙外）→ 走到门口触发场景切换，B（在 yard）看到 A **进入**；任何一帧里 A 都不同时出现在两个场景；靠近 bartender 出对话，跟到 menu 节点拿到 placeholder JSON；**第二个 WS 客户端全程收不到任何 dialogue 帧**（单播断言） |
| **6 admin** | admin 在 A 标签页 edit mode 点格子放椅子，**B 标签页无刷新出现椅子**；故意不排空的第三个客户端排空后其状态与一次全新 REST 拉取一致（rev 缺口重拉）；普通账号打全部 admin 端点 403（表驱动）；ban 一个在线用户后其 WS 被定向关闭且无法再登录 |
| **7 部署** | 真域名 HTTPS 打开、**WS 在 Caddy 后握手成功**（最容易翻车）、`systemctl restart` 后玩家自动重连恢复、`journalctl` 有结构化日志、跑一次 `sqlite3 .backup` 并**从备份恢复出一个能启动的库**（备份没验证恢复 = 没有备份） |

### 2.5 预重构清单 —— 八个 seam（P1–P8）

格式：**接口形状 → 现在成本 → 不做的返工成本**。这八条是本报告在"轻量"方向上唯一的**加法**，合计增量 ≤1 天；其中前六条同时是"能离线测试"的前提，不是为未来付的税。

**P1 · `scene` 作为房间键，并进每一帧**
```rust
pub struct Rooms(DashMap<SceneId, Arc<Room>>);
pub struct Room {
    tx: broadcast::Sender<Arc<str>>,        // 每 tick 序列化一次，克隆是 refcount
    players: DashMap<UserId, PlayerState>,
    walkable: Arc<dyn WalkableMap>,
    decorations_rev: AtomicU64,
    chat: Mutex<VecDeque<ChatLine>>,        // 每场景一个 ring buffer，50 条
}
```
`PlayerState`、`snapshot_full`、`snapshot_delta`、`{move}` 处理全部带 `scene`；MVP 只允许一个值 `"bar"`。tick 按 scene 分组打包，只发给订阅该 scene 的连接。成本：一个字段 + 一次 groupby + 一条 `scene_changed` 消息类型（切片 1 声明但永不触发）。不做：切片 5 是**协议破坏性变更**，同时打爆切片 1/2 建立的全部 WS 断言。

**P2 · 出向双通道：broadcast + per-connection mpsc**
```rust
// 每个连接：send task 对两者 tokio::select!
let mut rx = room.tx.subscribe();          // 快照/聊天/装饰广播
let (unicast_tx, mut unicast_rx) = mpsc::channel::<Arc<str>>(32);
```
join 快照、Lagged 后的 resync、校验错误、NPC dialogue、`kicked` 全部走私有通道。**必须 subscribe-then-snapshot**：先 `subscribe()`，再读状态并捕获 `{seq, chat_seq, decorations_rev}`，发快照，然后排空 `rx` 丢弃已被快照覆盖的帧。成本：约 20 行。不做：切片 5 的 dialogue 被迫广播给全房（隐私 + 带宽双输），切片 6 的定向踢人无路可走，且 join 竞态会长期表现为"重复气泡/丢失椅子"。

**P3 · WS 协议冻结（`docs/ws-protocol.md` 为唯一权威）**
```ts
// 每个 ServerMsg 都带 { v:1, seq:u64, t_ms:u64 }
| { type:"welcome",        self_id, scene, tick_hz:10, protocol_version:1 }
| { type:"snapshot_full",  seq, t_ms, you, scene, players, avatars, decorations, decorations_rev, chat, chat_seq, npcs }
| { type:"snapshot_delta", seq, t_ms, upsert:[{id,x,y,dir,moving,avatar_rev}], remove:[] }
| { type:"player_joined" | "player_left", id }
| { type:"avatar_changed", id, rev, avatar }
| { type:"chat", chat_seq, from, name, text, ts }   | { type:"chat_backlog", items }
| { type:"dialogue", conversation_id, npc, node, text, choices, pending, done }
| { type:"decoration_added" | "decoration_removed", scene, rev, ... }
| { type:"scene_changed", scene, x, y }  | { type:"kicked", reason }
| { type:"error", code, msg }            | { type:"pong", t }
```
切片 1 全部声明，未实现的 arm 一律 `error{code:"unimplemented"}`；**双向"收到未知 `type` 静默忽略"**；`v` 不匹配则客户端关闭并提示刷新。路由钉死为 `/ws/room`（无路径参数；若日后 scene 进 URL，必须校验编译期白名单否则 404，防止伪造建房导致 `DashMap` 无界增长 + 每假房一个 tick task）。成本：抄一遍类型 + Rust `#[serde(tag="type")]`，1 小时。不做：每个后续切片改 enum + 改前端 dispatch + 改全部断言；且旧前端 + 新后端 = 静默错解析。

**P4 · `WalkableMap` trait**
```rust
pub trait WalkableMap: Send + Sync {
    fn is_walkable(&self, tx: i32, ty: i32) -> bool;
    fn clamp(&self, from: (i32,i32), to: (i32,i32)) -> (i32,i32);
    fn spawn(&self, name: &str) -> (i32,i32);
    fn portals(&self) -> &[Portal];        // 切片 1 空数组
}
```
切片 1 用 `RectMap`（硬编码矩形）实现；切片 5 换 `TmjMap`。**服务端是可走性的唯一解析者**：`GET /api/scenes/:id` 返回 `{w, h, rev, bits}` 位图，客户端只用 .tmj 渲染视觉层、碰撞一律读这个位图。成本：一个 trait + 20 行。不做：切片 5 要在 tick 热路径上做手术；且双解析漂移 = 橡皮筋，是 30–50 人房里最难查的 bug。

**P5 · `AssetStore` facade —— 关键是 `public_url()`**
```rust
pub struct StorageKey(pub String);   // "{kind}/{owner_id}/{asset_id}.{ext}"，内容寻址、写入后不可变
pub trait AssetStore: Send + Sync {
    async fn put(&self, key:&StorageKey, bytes:Bytes, mime:&str) -> Result<()>;
    async fn get(&self, key:&StorageKey) -> Result<Bytes>;
    async fn delete(&self, key:&StorageKey) -> Result<()>;
    fn public_url(&self, key:&StorageKey) -> String;   // 唯一允许生成对外 URL 的地方，返回绝对 URL
}
```
内部用 `object_store`（`LocalFileSystem` / `AmazonS3Builder`），业务代码只依赖此 facade；DB 只存 key 不存 URL；前端只用响应里的完整 URL，绝不自己拼。MVP 不开 `aws` feature（避免把 hyper/aws-sigv4 拖进 <15MB 单二进制）。成本：约 60 行 + 一个测试实现。不做：R2 迁移从"改一个 env"退化为"改 8 处 handler + 前端 + 重跑全部 JSON 快照测试"，或"用 Rust 进程代理全部字节"（R2 的意义全没了）。

**P6 · `SpriteProvider` trait —— submit/poll 两段式 + 领域动词**
```rust
pub trait SpriteProvider: Send + Sync {
    async fn submit(&self, req: &SpriteRequest) -> Result<ProviderRef, GenError>;
    async fn poll(&self, r: &ProviderRef) -> Result<JobOutcome, GenError>;
    fn provider_id(&self) -> &'static str;      // "pixellab" | "stub"
}
pub enum JobOutcome { Pending { done_of_total: (u8,u8) },
                      Done(Vec<DirectionSheet>),
                      Failed { code: String, msg: String } }
```
**不要**叫 `PixelLabClient`，**不要**按 MCP 的四个工具名建模，**不要**只有 `generate() -> Bytes`。真实是 5–9 分钟、2 次调用 + 2 轮轮询、每方向一个独立 job；`ProviderRef` 落 `provider_ref` 列，worker 是"读 pending 行 → poll → 推进"的**可重启**循环。多阶段编排放在 worker 的 `phase` 列，**不进 trait**（trait 保持"一次调用一个原子操作"）。stub 必须**可编程**（内部队列 + `advance()`），且**首次 poll 必返 Pending**——"stub 同步 resolve"会让测试走一条真实环境不存在的路径。成本：一个方法 + 两列 DB，约 30 行。不做：切片 3b 上真 API 时重构 worker 循环 + 状态机 + schema（要加 migration）+ 全部 stub 测试的时序假设；且进程重启后在飞任务永久丢失。

**P7 · sprite-sheet 契约 + `validate_sheet()`**
```json
{ "v": 1, "frame_w": 32, "frame_h": 32, "origin": [16, 28],
  "dirs": ["s","sw","w","nw","n","ne","e","se"],
  "clips": { "idle": {"row0":0,"frames":2,"fps":4,"loop":true},
             "walk": {"row0":8,"frames":4,"fps":8,"loop":true} },
  "anchors": { "hand": {"s":[[20,18],[21,17],[20,18],[19,17]], "...": "每 clip×dir×frame"},
               "back": {"...": "同上"} } }
```
布局规则写进 `docs/sprite-contract.md`（唯一权威）：**行 = 方向（固定顺序），列 = 帧；每个 clip 占连续 8 行，`row0` 指定起始行**。三个容易漏、漏了很贵的字段：**`anchors`**（切片 3b 需要；现在加可选字段默认退化为 `origin`，成本≈0；否则要给切片 2 已生成的全部 preset 补锚点 + 改 meta schema + 加 migration）、**`v`**（帧尺寸将来从 32 变 48 时老 asset 还能识别）、**`dirs` 显式**（PixelLab 的方向顺序不由我们决定，显式声明才能在导入时重排而不是让渲染器猜）。配一个纯函数 `validate_sheet(bytes, meta) -> Result<()>`，worker 落库前强制过。**必须等 Spike-0 校对后定稿**（`keep_first_frame` 多一帧、v3 默认只做 south、画布自动约 2× padding 上限 256 都会直接约束它）。成本：一个 schema + 两处类型 + 一个校验器，约 1 小时。不做：**本清单里返工代价最高的一条**——已生成的付费素材全部作废且不可回滚，渲染器与合成器同时改，配件功能可能整块推迟。

**P8 · `publish_room_event(scene, event)` —— 唯一广播出口**
所有装饰/场景/成员变更走这一个函数，**禁止在 handler 里直接 `room.tx.send()`**。顺序纪律：**先 DB 事务提交，再 bump `rev`，再广播**；绝不在事务里广播。`rev` 单调递增，每个事件携带变更后的 `rev`，快照携带当前 `rev`；客户端丢弃 `rev <= local_rev`，检测到缺口（`rev > local_rev + 1`）则 `GET /api/decorations?scene=` 全量重拉——这条同时兜住了 `Lagged` 丢事件。成本：约 30 行纪律。不做：一整类"我看到椅子他没看到"的玄学 bug，且多进程时失败方式是静默的。

### 2.6 切片 1 的 Definition of Done

必须**全部**满足才可宣布切片 1 完成、并行启动切片 2 与切片 5。

**A. 功能可见**
- [ ] `cargo run` 单进程在一个端口同时服务 `/api`、`/ws`、`./dist`；Vite dev 模式 proxy 配好，两种模式都能玩
- [ ] register / login / logout / `GET /api/me` 全通；argon2id m=19456,t=2,p=1 且跑在 `spawn_blocking` 里（每次 hash 20–40ms CPU，2 vCPU 上登录突发会拖停整房）；register/login 有限流
- [ ] 注册事务内即创建默认 avatar 并设 active（`GET` 无写副作用）
- [ ] 两标签页可见彼此移动，远端玩家**插值**（延迟 150ms、缓冲上限 2 帧、饥饿时保持最后位置**不外推**）；本地玩家预测 + 服务端纠正，偏差 >0.5 tile 才平滑收敛（不硬 snap）
- [ ] 服务端 clamp 生效且**不可瞬移**：move 意图限相邻格（或改为 `{input, dx, dy}` 速度意图），服务端按固定速度逐格推进并重检每个中间格
- [ ] 聊天气泡 + 侧栏最近 50 条 + 新入场 backlog；文本 ≤200 字符服务端强制、纯文本渲染（`textContent`，绝不 `innerHTML`）；DB 内无任何聊天痕迹
- [ ] 手机横屏**真机**（非模拟器）跑通：DOM 摇杆 + 动作键，整数缩放、无平滑、稳定帧率；世界渲进单个 240×160 RenderTexture，**升采样只在一处发生**（AC 与注释都写明）；UI 全走 DOM 覆盖层；保留一个显式 "tap to enter" 过场（顺带作为将来 BGM 的 audio unlock）

**B. 抽象到位（八个 seam）**
- [ ] P1 房间键为 `SceneId`，`scene` 进 PlayerState 与每一帧；`scene_changed` 已声明
- [ ] P2 出向双通道 + subscribe-then-snapshot + `chat_seq`/`decorations_rev` 计数器
- [ ] P3 `docs/ws-protocol.md` 列全部 type，未实现者返 `unimplemented`；未知 type 静默忽略已实现并有测试；每帧带 `v/seq/t_ms`
- [ ] P4 `WalkableMap` trait + `RectMap` 实现
- [ ] P5 `AssetStore` trait 定义完毕（含 `public_url`）+ 一个实现 + 一个测试实现
- [ ] P6 `SpriteProvider` + `SpriteRequest`/`ProviderRef`/`JobOutcome` 定义完毕 + 可编程 stub（无 job 表也可）
- [ ] P7 `docs/sprite-contract.md` + sidecar schema 定稿（`v`/`dirs`/`clips`/`anchors`）且**已用 Spike-0 真实输出校对**帧尺寸与方向顺序
- [ ] P8 `publish_room_event` 是唯一广播出口
- [ ] 领域枚举全为 `sqlx::Type` enum（禁 `kind: String` / `dir: String`）；WS 自动重连（指数退避 + jitter + 重连后整份重取快照）

**C. 实时正确性（可回归）**
- [ ] 3s keyframe 自愈；`RecvError::Lagged(n)` 走私有通道补发 full snapshot（**有测试**：小 channel 容量 + 故意不排空的客户端）
- [ ] 显式 `player_joined` / `player_left`；`moving` 字段 + 停止时补发一帧
- [ ] Ping 20s / 2 次 Pong 未回即关；入站 idle 60s 超时；同用户第二次连接先以 `session_replaced` 关旧连接（单连接不变式有测试）
- [ ] tick task 每 scene 只 spawn 一次（`entry().or_insert_with()`，绝不 `contains_key` 后 `insert`），无人时回收；连接/断开/重连 20 次后剩一个 task、零残留玩家
- [ ] `MissedTickBehavior::Delay`；每帧序列化**一次**并广播 `Arc<str>`；短字段名
- [ ] WS 升级**前**跑 session 提取，无登录态返 401 不升级；校验 `Origin`；入站令牌桶（chat 2/s 突发 5、input 20/s、interact 5/s）+ `max_message_size(4 KiB)`；身份只来自 session，任何入站消息不得携带 user id（有伪造测试）
- [ ] pragma 经 `after_connect` 施加到**每个**池连接：`journal_mode=WAL` / `synchronous=NORMAL` / `foreign_keys=ON` / `busy_timeout=5000`，有测试断言 `PRAGMA journal_mode` 返回 `wal`
- [ ] 10Hz tick 与 WS handler **零 DB 查询**（2 秒空转窗口内断言零 query）
- [ ] `MAX_PLAYERS_PER_SCENE=60` 硬上限 + 干净的 close code 拒绝
- [ ] delta 帧**不含** avatar 载荷；每帧字节预算 ≤2 KB @50 人、≤8 KB/s/客户端，数字写进 PRD

**D. 测试基建（后续切片复用的资产）**
- [ ] `tests/harness.rs` 提供 `spawn_app()`（随机端口 + 临时 SQLite + 带 cookie 的 WS 客户端工厂），被至少一个完整端到端测试使用
- [ ] 全部测试**离线**可跑（零外网），`cargo nextest run` 绿；`cargo clippy --all-targets -- -D warnings` 干净
- [ ] TS 侧 `net` + `game-state` 有 vitest 单测（含时间快进的插值测试、丢帧后单调不瞬移、`<script>` payload 的转义测试）
- [ ] Rust→TS fixture 桥：Rust 测试生成代表性 ServerMsg JSON 到 `fixtures/`，TS 测试消费之（协议漂移探测器）
- [ ] 前端目录分层 `net / protocol / game-state / scene / ui`，CI lint 禁止前三者 import `phaser`；插值时钟为**注入的 `now()`**
- [ ] `loadtest`：50 个已认证 WS 客户端随机走动 + 1 chat/10s 持续 60s，断言 (a) 服务端零 `Lagged`、(b) p95 帧间隔 <150ms、(c) 平均 ≤8 KB/s/客户端、(d) 稳态 RSS 低于上限，并**打印实测数字**作为切片 2/6 的回归门
- [ ] `migrations/0001_init.sql` + 提交 `.sqlx/` + `justfile` 封装 `sqlx migrate run` / `sqlx prepare`；测试用 `sqlx::migrate!()` 建库

**E. 项目卫生**
- [ ] `CLAUDE.md`：v2 文档的 Rust 硬约束 10 条 + 三条本项目禁令——(1) 禁止在 HTTP 请求路径上等待生成，(2) 禁止把聊天写进任何表，(3) 禁止在 `scene/` 里出现 avatar `kind` 分支；并注明"v2 研究文档的 SQL/邮件/fal.ai 章节已废弃"
- [ ] 一次 `cloudflared tunnel` 分享验证过（外网可访问 + WS 通），提前暴露切片 7 的 WS-over-proxy 风险

**并行放行判据**：启动切片 2 只需 A + B 的 P5/P7/前端分层 + D 的 harness；启动切片 5 只需 A + B 的 P1/P2/P3/P4 + D 的 harness。两者都**不依赖** P6，所以 P6 若受 Spike-0 阻塞可只交付类型骨架——但 **P7 必须等 Spike-0 校对完再定稿**，这是切片 1 唯一的外部依赖，也是把 Spike-0 排在波次 0 的根本原因。

---

## 三、评审发现（去重 + 按严重度排序）

4 个视角共 **77 条** findings，去重合并后编号如下。编号被 §四 / §五 引用，请勿变更。

> 视角简称：`spec` 规格一致性 · `简洁` Fowler 异味基线 · `扩展` 可扩展性 · `实时` 实时并发正确性

### BLOCKER — 必须在切片 1 开工前解决（10 条）

| # | 视角 | 位置（修订目标） | 问题 | 建议 |
|---|---|---|---|---|
| **B1** | spec | 0001-house-of-imbibe-prd.md §Implementation Decisions / Realtime room model（第 100-106 行）vs 0006-slice-5 验收项 1-2 | 锁定决策 2 的「场景」维度与 PRD 的实时模型互相矛盾。PRD 写死「One room per server (MVP)」，player state 为 `{x, y, dir, avatar_snapshot, name}`——**没有 scene 字段**；chat ring buffer 也是「per room」。但切片 5 要求 bar interior + yard **两个场景 + 场景切换**，decorations 表又带 `scene` 列。结果：两个不同场景的玩家会在同一坐标空间互相看到、聊天缓冲区归属不明、decoration 广播该不该按场景过滤未定义。切片 1 会按「单房无 scene」实现 WS 协议… | 在 PRD §Realtime room model 明确二选一并写进切片 1 验收项：(a) scene 即 room —— `Rooms: DashMap<SceneId, RoomHandle>`，player state 加 `scene`，每场景独立 tick + 独立 chat ring buffer，场景切换 = 退订旧 room 订阅新 room；或 (b) 单 room 内带 `scene` 字段，tick 广播全量但客户端按 scene 过滤，chat 全局共享。同时补切片 1 验收项：「player state 含 scene 字段，delta/snapshot 帧按 §Realtime 定义的场景语义分发」，… |
| **B2** | spec | 0001-house-of-imbibe-prd.md §Stack (locked) / Generation（第 85 行）+ §Generation async UX 的 generation_jobs schema（第 133-135 行）；0004-slice-3 验收项 1、0005-s… | PRD 与已核实的事实层直接冲突。PRD 写「PixelLab.ai via its official **MCP**/API (`create_character`, `create_image_pixflux`, `create_map_object`, `animate_character`)」，切片 3/4 验收项照抄 `create_character`。但 docs/reference/pixellab-api.md（2026-08-01 核实）§一.2 与 §六 明确：**MCP 不能用于面向终端用户的生成**（浏览器无 MCP 客户端 + token 暴露），用户侧必须走 REST v2；且 §九 指出 **MCP… | 1) 把 PRD §Stack 的 Generation 一行改为「PixelLab **REST v2**（`https://api.pixellab.ai/v2`，Bearer token 仅存后端），端点：`create-character-with-4-directions` / `create-character-v3` / `portrait-character-pro` / `animate-character` / `map-objects`；MCP 仅限开发期本地出素材，不进产品代码路径」，并在 Out of Scope 加一条「前端直连 PixelLab / 用 MCP 服务用户请求」。2) `generatio… |
| **B3** | spec | 0001-house-of-imbibe-prd.md §Avatar dual pipeline（第 93、98 行「Rendered by compositing layered PNGs at runtime」/「runtime overlay composition」）vs 0003-sli… | 「合成发生在哪一侧」从未定义，而三个切片的验收项都假设了服务端合成。PRD 说 modular 是「runtime 层叠 PNG 合成」、配件是「runtime overlay 按 slot anchor 逐帧绘制」，配合 §Testing 里「Phaser 渲染不做单元测试」的表述，读起来像**客户端 Phaser 合成**。但切片 3 验收项 6 要求集成测试「assert overlay present in the avatar **composite**」、切片 4 验收项 6 要求「activate -> **assert rendered**」—— 后端集成测试（真 Axum + WS + JSON 断言）根本无法断言… | 在 PRD §Avatar dual pipeline 增加一段「合成位置」决策：推荐**客户端合成**（服务端只广播 `avatar_snapshot = {kind, layers/sprite_asset_id, equipped[]}` 这一份纯数据描述，Phaser 侧按 slot anchor 表叠图），并把 slot anchor 表定义为 canonical contract 的一部分。随后改写验收项：切片 3 验收项 6 改成「assert 广播的 avatar_snapshot.equipped 含该配件与 slot」，切片 2 验收项 6 与切片 4 验收项 6 的「render/composite」改成「as… |
| **B4** | 扩展 | PRD #1 §Implementation Decisions › Realtime room model（`players: Map<id, {x, y, dir, avatar_snapshot, name}>`）+ Slice 1 (#2) AC 第 4 条 | PlayerState 与 delta snapshot 协议里没有 `scene` 字段，但 Slice 5 (#6) 要求 bar/yard 两个场景 + 场景切换，且 `decorations`/`npcs` 表都已经有 `scene` 列。这意味着：(a) 服务端 clamp 时不知道该用哪张 collision layer；(b) 在 yard 的玩家会被广播进 bar 客户端（坐标空间不同，渲染成鬼影）；(c) `decoration_added` 广播也无法按场景过滤。增长点 3（加第三个场景）因此不是加法——它要求改 snapshot/delta 协议、改客户端解析、改所有 Slice 1/2 建立的 WS 断言，三… | 在 Slice 1 就把 `scene: String` 放进 PlayerState、full snapshot、delta、以及 `{move}` 的服务端处理里，MVP 只允许一个值 `"placeholder"`。10Hz tick 按 scene 分组打包，只向订阅该 scene 的连接发送。同时把 `{type:"scene_changed", scene, x, y}` 定为一条协议消息（Slice 1 可以只有一个 scene，永不触发）。Slice 1 成本：一个字段 + 一次 groupby + 一条消息类型；Slice 5 之后再补成本：协议破坏性变更。 |
| **B5** | 扩展 | PRD #1 §Avatar dual pipeline（"same canonical sprite-sheet contract - 8 directions × the same animation set (idle, walk, …) on the same grid layout"）；S… | "canonical sprite-sheet contract" 在 PRD 里被引用 4 次、被 3 个切片当作验收基准，但从未被定义：帧尺寸、网格布局、方向枚举顺序、动画名集合、每动画帧数、accessory 的 per-slot anchor 坐标系，全部缺失。这个契约是四个东西的唯一互操作面——管线 A（模块化合成）、管线 B（PixelLab 生成）、配件叠加、以及未来换生成商/本地模型。没有书面契约 + 校验器，增长点 2 根本无法验证"新供应商产出合规"，只能靠肉眼看图；同时 Slice 2/3/4 三个切片会各自假设一套布局，返工面覆盖渲染器 + 合成器 + 生成 worker。另外，fact layer 已确认… | 在 Slice 2 开工前，把契约写成一份可执行规格（`docs/reference/sprite-sheet-contract.md` 或 PRD 一个新小节）：frame w×h、每行一个方向、方向顺序固定为 `[south, south-west, west, north-west, north, north-east, east, south-east]`、动画名枚举 + 各自帧数、sheet 总尺寸公式、accessory 每 slot 的 anchor 是 per-direction-per-frame 的 (dx,dy) 表。然后在 Slice 2 加一个纯函数校验器 `validate_sheet(bytes, me… |
| **B6** | 实时 | PRD 0001 §Implementation Decisions > Realtime room model (L100-104: `players: Map<id, {x, y, dir, avatar_snapshot, name}>`, "One room per server (MVP)… | PlayerState has no `scene` field and the delta snapshot has no scene scoping, yet Slice 5 adds two scenes with transitions, and decorations/npcs are already keyed by `scene`. As specced, after Slice 5 everyone is in one flat coordinate space: yard players are broadcast to bar clients at overlapping tile coords, the mov… | Put `scene` into the protocol in Slice 1, even with a single placeholder scene. Concretely: (a) key room state as `DashMap<SceneId, Scene>` where `Scene { tx: broadcast::Sender<Arc<str>>, players: DashMap<UserId, PlayerState>, walkable: Arc<WalkableGrid>, decorations_rev: AtomicU64 }`; (b) add `scene: String` to Player… |
| **B7** | 实时 | Slice 1 0002 AC #4 (L23 "WS /ws/room: on connect the server sends a full snapshot") and PRD 0001 §Realtime room model (L102-103, inbound message list) | WS authentication is never specified anywhere in the PRD or in any slice AC. The inbound message list has no identify/hello, so player identity must come from the session cookie on upgrade — but nothing says so, and nothing requires rejecting an unauthenticated upgrade. An agent implementing AC #4 literally will most p… | Add to Slice 1 ACs: (1) `GET /ws/room` runs the tower-sessions extractor *before* `ws.on_upgrade` and returns 401 without upgrading if there is no logged-in user — `user_id` and `role` are captured from the session into the connection task and are the only identity source; no inbound message may carry a user id. (2) Th… |
| **B8** | 实时 | PRD 0001 §Realtime room model (L104 "broadcasts a **delta snapshot** at 10 Hz (only changed players). New connections get a full snapshot on join") +… | Pure deltas over a single `tokio::sync::broadcast` channel have no recovery path, and the spec provides none. Three concrete failure modes: (a) `broadcast::Receiver` returns `RecvError::Lagged(n)` when a slow mobile client falls behind the 256-slot buffer — the dropped frames are gone forever, and because deltas only c… | Specify the frame contract in the PRD and test it in Slice 1: (1) every server frame carries `{v:1, seq:u64, t_ms:u64}`; (2) the tick emits a **full keyframe every N ticks** (N=30, i.e. 3s) and deltas in between — a lagged client self-heals within 3s; (3) explicitly handle `Err(RecvError::Lagged(_))` in the send task b… |
| **B9** | 实时 | PRD 0001 §Realtime room model (L102, `avatar_snapshot` inside per-frame player state) vs §Avatar dual pipeline (L91-98) and Slice 2 0003 AC #4 | Carrying `avatar_snapshot` in the per-frame state blows the bandwidth budget that decision 1 depends on. A modular snapshot is a layers JSON with base/hair+color/outfit+color plus an `equipped` list of accessory refs (UUIDs) — realistically 250-350 bytes. The research doc's sizing (L101: "50 人 × ~24 字节 ≈ 1.2 KB/frame")… | Make avatars a versioned side-channel, stated in the PRD: (1) delta/keyframe player entries contain only `{id, x, y, dir, moving, avatar_rev}` — target ≤32 B/player; (2) full avatar payloads are sent only in the join snapshot and in a `{type:"avatar_changed", id, rev, avatar}` broadcast frame; (3) a client seeing an un… |
| **B10** | 实时 | PRD 0001 §Realtime room model (L103-104: inbound `{type:"move", tx, ty}` target-tile intent; "Server clamps movement to walkable tiles") + Slice 1 000… | The movement model is under-specified in a way that makes "server-authoritative" (locked decision 1) vacuous. The intent is a *target tile*, but nothing says who integrates motion between the current tile and the target, at what speed, or with what per-step collision. An implementation that sets `player.x = tx; player.… | Replace tile-target intent with a direction/velocity intent and integrate on the server: inbound `{type:"input", dx, dy, seq}` where `(dx,dy)` is a normalized 8-way direction (or zero); each 100 ms tick the server advances the player by `speed * dt` (pin `speed` in the PRD, e.g. 4 tiles/s) and rejects any step whose de… |

### MAJOR — 会导致返工或违背锁定决策（38 条）

| # | 视角 | 位置（修订目标） | 问题 | 建议 |
|---|---|---|---|---|
| **M1** | spec | 0003-slice-2 验收项 2（`PUT /api/avatar` 更新 layers_json）+ 0005-slice-4 验收项 4（仅拒绝 equip）；PRD §Schema 的 avatars 表（第 127-130 行） | 锁定决策 5「A/B 互不互通编辑」只被落实了一半。切片 4 只验证了「配件装到 generated 返回 400」，但 PRD 第 96 行的另一半——「a generated avatar cannot have preset parts swapped」——**没有任何验收项**：`PUT /api/avatar` 对一个 `kind='generated'` 的 active avatar 提交 `layers_json` 时的行为未定义、未测试。同时 avatars 表没有任何约束保证 `kind='modular' ⇒ layers_json NOT NULL AND sprite_asset_id IS NULL`（反… | 切片 2 验收项加两条：(a) 迁移含 `CHECK ((kind='modular' AND layers_json IS NOT NULL AND sprite_asset_id IS NULL) OR (kind='generated' AND sprite_asset_id IS NOT NULL AND layers_json IS NULL))`；(b) 集成测试断言 `PUT /api/avatar` 对 generated avatar 返回 400/409。切片 4 验收项 4 扩写为「equip 与 layers 编辑两条路径都对 generated 返回 400」。 |
| **M2** | spec | 0003-slice-2 验收项 2（GET/PUT `/api/avatar` 单数资源）vs 0005-slice-4 验收项 2-3（多个 avatar 在库中、可设为 active）；PRD §Schema `avatars.is_active` | Avatar 的 API 面在两个切片间不自洽。`is_active` 列 + 切片 4 的「generated avatar 出现在库中并可激活」意味着一个用户有 **N 个 avatar 行**；但切片 2 只定义了单数的 `GET /api/avatar`（返回 active）和 `PUT /api/avatar`（改 active 的 layers），既没有「创建第二个 avatar」也没有「在 N 个中切换 active」的端点。PRD §Generation async UX 也只列了 `/api/jobs/:id` 和 `/api/library`。切片 4 实现时必须现场发明 `POST /api/avatars`… | 在 PRD 里补一节「Avatar API」定死复数资源面：`GET /api/avatars`（列出本人全部，含 kind 与 is_active）、`POST /api/avatars`（创建 modular）、`PUT /api/avatars/:id`（仅 modular，改 layers）、`POST /api/avatars/:id/activate`、`GET /api/avatar/active`（便捷读取）。切片 2 验收项 2 改用这套端点并保留「首次调用自动创建默认 avatar」；切片 4 验收项 3 引用同一个 activate 端点。 |
| **M3** | spec | 0007-slice-6 验收项 4（POST ban，login disabled）vs PRD §Schema users 表（第 126 行） | 切片 6 要求「ban（禁止登录）」，但 PRD 的 `users(id, username, password_hash, role, created_at)` **没有任何 banned/status 列**，也没有任何切片的验收项包含这次迁移；切片 1 的 auth 登录路径也不会检查封禁状态。实现者要么私自加列（PRD schema 失真），要么用 role='banned' 污染角色枚举（与 `role ∈ member\|admin` 冲突）。US28 因此无法被验收项闭环验证。 | PRD §Schema 的 users 加 `is_banned INTEGER NOT NULL DEFAULT 0`（或 `status TEXT CHECK(status IN ('active','banned'))`），切片 6 验收项 4 补「迁移新增封禁列；被封用户 `POST /api/auth/login` 返回 403 且其现有 session 被清除；集成测试覆盖」。 |
| **M4** | spec | 0005-slice-4 验收项 5（原始照片不持久化）vs PRD §Generation async UX + §Schema 的 `generation_jobs.params_json`（第 119、134 行） | 两处设计互相抵消。PRD 规定 `POST /api/generate {kind, params}` 把 params 整体写进 `generation_jobs.params_json`，而切片 4 的入参就是 `{kind: avatar_generated, **photo**, params}` —— 照片（或其 base64）作为生成参数，按 PRD 的 schema 会被落进 SQLite 的 params_json 并长期留存，直接违反切片 4 验收项 5「原始照片不持久化于 job 之外」的隐私承诺。此外 PRD 正文（Further Notes / Out of Scope）完全没有提照片隐私，这条数据处理承诺只… | 在 PRD §Generation async UX 明确「二进制上传物**不进 params_json**：照片走临时文件/内存传给 worker，job 完成或失败后即删；params_json 只存标量参数（size/view/seed 等）」，并把这条隐私承诺提到 PRD 正文（Further Notes 或新增 §Privacy）。切片 4 验收项 5 改为可验证形式：「job 完成后断言 params_json 不含图像数据、临时上传文件已删除」。 |
| **M5** | spec | PRD User Story 19 + §Realtime（10 Hz delta）vs 0002-slice-1 验收项 4+7 | 锁定决策 1 的核心承诺（30-50 并发下的 10Hz **delta** + 插值）没有任何能证伪的验收项。切片 1 只要求「asserts **a** delta snapshot frame is received」——一帧到达既不能证明它是 delta（只含变化玩家）、也不能证明 tick 是 10Hz、更不能证明 30-50 并发下可用；而 US19 明写「under 30-50 concurrency」。「client interpolates ~100-200 ms」被列为验收项，但它是前端行为，整套验收里没有任何前端测试来验证它。结果是本项目最关键的性能决策全靠人肉相信。 | 切片 1 验收项拆细为三条可执行断言：(a) delta 语义 —— 「两个客户端连接，只有 A 移动；断言 B 收到的 delta 帧**不包含**未变化的 C，且新连接收到 full snapshot」；(b) tick 频率 —— 「1 秒内收到的 tick 帧数在 8-12 之间」；(c) 并发冒烟 —— 「50 个 WS 客户端同时连接并各发一次 move，全部在 N 秒内收到含自己位置的帧，无 broadcast Lagged 断连」。插值那条从后端验收项移出，改为前端 `game-state` 模块单元测试验收项（给定 t0/t1 两帧断言插值输出）。 |
| **M6** | spec | PRD §Testing Decisions（第 161 行 Playwright + art-director、第 158 行 frontend net/game-state 单元测试）+ User Story 33；全部 7 个切片验收项 | PRD 承诺的前端测试策略没有任何切片认领。§Testing 明确「Frontend `net` + `game-state` 模块是纯 TS，做单元测试」「视觉正确性由 Playwright 截图 + art-director review loop 覆盖」，US33 也把「前端逻辑与 Phaser 解耦、可在无 canvas 下验证」当成一条 user story。但切片 1-7 的验收项**全部只有 Rust 集成测试**，没有一条要求建立前端单测 runner 或 Playwright 基线。US33 与 PRD 的测试决策因此不可验收，且第一个切片没建的脚手架后面切片也不会建（PRD 自己说「第一个切片建立的 harnes… | 切片 1 验收项补两条：(a) 「前端 `net` 与 `game-state` 为不依赖 Phaser/DOM 的纯 TS 模块，含 vitest 单元测试：消息编解码、房间状态镜像、插值」；(b) 「Playwright 冒烟 + 首张截图基线（登录→进房→移动）落库，作为后续 art-director review loop 的起点」。若判断 Playwright 应后置，则从 PRD §Testing 删掉它，不要留下无人认领的承诺。 |
| **M7** | spec | PRD §Realtime room model（`{type:"move", tx, ty}` 目标格 intent）vs 0002-slice-1 验收项 6（虚拟摇杆）+ §Map & decorations（decorations 用 tile_x/tile_y） | 移动模型未锁定：协议是**目标格 intent**（tx, ty，格子语义），输入是**模拟摇杆**（连续方向），而 PRD 从没说角色是「格子逐步移动（宝可梦式）」还是「自由 x/y 移动」。参考的 v2 研究里 PlayerState 是 `f32 x, y`，PRD 的 decorations/npcs 却是整数 tile 坐标。切片 1 会自行选一种（很可能是自由 x/y + 摇杆矢量），而切片 5 的 tile 碰撞层 clamp、切片 6 的「点一个 tile 放装饰」都建立在格子语义上，届时碰撞与放置对不齐。锁定决策 7（摇杆）与 PRD 的 tile-intent 协议之间缺一层设计。 | 在 PRD §Realtime room model 明确写死：角色为**格子步进**（tile-based stepping，摇杆方向映射为相邻格 target，服务端 clamp 到 walkable tile，客户端在两格间做 tween/插值），player state 同时含 `tile_x/tile_y`（权威）与用于渲染的插值位置；或若选自由移动，则把 `move` 消息改为 `{dx, dy}` 并把 decorations 的放置/碰撞语义一并说明。切片 1 验收项 4 引用该定义。 |
| **M8** | spec | 0004-slice-3 验收项 7（「verified by a second store impl in tests」） | 这是 scope creep，且与 PRD §Testing Decisions 第 158 行「测外部行为，绝不测内部；不 mock 我们自己的模块」自相矛盾。为了「证明 object_store 可换」而**再写一个 store 实现**，既超出 US31 的范围（US31 只要求存储被抽象），又是在为抽象层本身写测试（测内部 seam）。object_store crate 本就自带 `InMemory`，不需要自研第二个实现。 | 验收项 7 改为：「业务代码只依赖 `Arc<dyn ObjectStore>`（clippy/review 层面保证）；集成测试套件用 `object_store::memory::InMemory` 跑通同一批断言，证明业务代码对具体实现无感」。不新增任何自研 store 实现。 |
| **M9** | spec | PRD §Further Notes（Lightweight mandate、无成本护栏）+ 0004-slice-3 全部验收项；对照 docs/reference/pixellab-api.md §十一.7 | 锁定决策 6（注册只要用户名+密码、无邮箱无验证）与付费生成 API 叠加，形成无界成本敞口，而 PRD 与切片 3/4 **没有任何配额、限流或余额告警**。事实层 §七 给出真实单价（路径 B 单角色 ≈$0.19-0.30），§十一.7 明确要求「每用户生成次数限额 + usage 入库 + 低余额告警」；v2 研究里的 `tower_governor` 也被 PRD 精简掉了。零门槛注册意味着一个脚本可以刷爆余额，而这是 PRD 唯一的可变现金支出项。 | PRD §Generation async UX 补一条硬约束：「每用户每日生成上限（可配 env，默认如 5 次）；`generation_jobs` 记录 `cost_usd`；启动与定时任务调用 `GET /v2/balance`，低于阈值拒绝新 job 并记 warn」。切片 3 验收项加「超配额时 `POST /api/generate` 返回 429，集成测试覆盖」。同时在 PRD §Stack 恢复 auth/generate 两个端点的限流（tower_governor 或手写计数），并在切片 1 验收项写明 register/login 限流。 |
| **M10** | spec | 0004-slice-3 验收项 4（`GET /api/library` 列出 assets）vs 0005-slice-4 验收项 2（「generated avatar 出现在 member's library」）+ PRD User Story 10 | 「个人素材库」的资源边界不一致。切片 3 把 library 定义为 `assets` 表的查询（sprite_sheet / accessory / decoration + job 状态），但切片 4 说完成时创建的是一条 **`avatars` 行**（kind='generated'）并称其「出现在库中」。US10 要求库里同时能看到「生成的 avatar 和配件及其状态」。avatars 与 assets 是两张表，`/api/library` 按切片 3 的定义不会返回 avatars 行。锁定决策 4 的「完成后回库查看」这一核心 UX 因此在两个切片里指向不同的东西。 | 在 PRD §Generation async UX 定义 library 的返回结构：`GET /api/library` 返回 `{assets: [...], avatars: [...], jobs: [...]}`（或统一为带 `type` 的条目流），并说明 generated avatar 在库中以 avatar 条目呈现、其底层 sprite 以 asset 条目呈现。切片 3 验收项 4 与切片 4 验收项 2 都引用这一结构。 |
| **M11** | 简洁 | PRD『Realtime room model』第 3 条 + Slice 1 (0002) 验收项 4「broadcasts a 10 Hz delta snapshot」 | hard。「delta snapshot（only changed players）」有两种读法：(a) 房间级 delta——每 tick 只打包本 tick 变化的玩家，一条消息经单个 broadcast::channel 发给所有人；(b) 每连接 delta——为每个订阅者维护「它上次看到什么」再做 diff。(b) 会直接摧毁 v2 调研里那个 30 行的 broadcast 模型：需要 per-connection 状态、per-connection 序列化、lagged 补偿、full-resync 协议。而收益是把 50 人 × 24B × 10Hz ≈ 12 KB/s 压到几 KB/s——对 30-50 并发毫无意… | 在 PRD 该条和 Slice 1 验收项里把语义钉死为房间级 delta：「每 100ms tick 遍历 room.state，只序列化自上一 tick 起 x/y/dir/avatar 有变化的玩家，打成一条 JSON 消息经该房间唯一的 broadcast::Sender 发出；所有订阅者收到同一份字节。新连接单独收一次 full snapshot。禁止 per-connection diff / per-connection 序列化」。同时补一句 Lagged 处理：收到 RecvError::Lagged 就给该连接重发 full snapshot。 |
| **M12** | 简洁 | docs/pixel-mosaic-game-workflow-v2-rust.md（决策层）§二.3 schema、§二.4 auth 流程、§五 Cargo.toml、§七 阶段 2 checklist vs PRD『Schema』『Out of Scope』 | hard，Duplicated Code 的最坏形态——两份分歧的副本。按 docs/CONVENTIONS.md，`docs/*.md` 是「决策层／最终采用方案」，会被当权威读。但该文档规定了 `users.email`+`email_verified`、`email_tokens` 表、`messages` 表持久化聊天、`rooms` 表、axum-login、lettre+Resend、tower_governor、sentry、imagequant/oxipng/fast_image_resize 流水线、fal.ai——PRD 明确把 email/聊天落库全部划入 Out of Scope，且改用 PixelLab 而… | 两步：(1) 在 v2 文档顶部加状态块「**已被 .scratch/issues/0001 PRD 取代（auth / schema / 聊天 / 生图供应商 / Cargo.toml 四节）；仅 §三 部署、§六 Rust 硬约束仍现行**」，并就地 ❌ 标注 email_tokens / messages / rooms 三张表和 lettre / axum-login / imagequant / fal.ai 条目（CONVENTIONS.md §三要求就地标注不删除）。(2) 在 PRD『Stack (locked)』下新增「MVP 依赖白名单」，正列约 15 个 crate（axum, tokio, tower, to… |
| **M13** | 简洁 | Slice 3 (0004-slice-3-generation-library-accessories.md) 整体，7 条验收项 | hard，切片过大。这一片捆了两条互不依赖的纵切：(A) 异步生成基础设施 = PixelLabClient trait + real impl + stub + assets 表 + generation_jobs 表 + object_store + 后台 worker + POST /api/generate + GET /api/jobs/:id + GET /api/library + library UI；(B) 配件装备 = layers_json 的 equipped 结构 + back/hand slot 锚点定义 + 逐帧 overlay 合成渲染 + equip UI。(B) 是纯前端渲染 + 数据模型工作，和… | 拆成两个 issue：Slice 3a「生成任务管线 + 个人素材库」——验收止于「请求 accessory → 拿 job_id → worker 完成 → 出现在 /api/library，状态正确」，不含 equip；Slice 3b「配件装备 + overlay 合成渲染」——blocked_by [3a]。然后把 Slice 4 的 blocked_by 从 [3, 4] 改成 [3, 3a]，Slice 6 的 blocked_by 从 [4, 6] 改成 [3a, 6]。Slice 3b 变成不阻塞任何人的叶子节点。 |
| **M14** | 简洁 | Slice 3 (0004) 最后一条验收项「verified by a second store impl in tests」+ PRD User Story 31 + PRD『Further Notes』sustainability guardrails | hard，Speculative Generality 被写成了硬验收项。`object_store::ObjectStore` 本身就是 trait，LocalFileSystem → R2 的切换是构造函数里 3 行（v2 文档 §二.5 已给出两段代码）。为了「证明」这个抽象而在测试里再写一个 store impl，是为规格外需求（R2 迁移已在 Out of Scope）付真实代码成本，且这个 impl 永远不会被生产用到。附带成本：object_store 开 `aws` feature 会把 hyper/aws-sigv4 一整串拖进那个号称 <15MB 的单二进制。 | 删掉该验收项。改为一条约束句：「业务代码只依赖 `Arc<dyn ObjectStore>`，不出现 `LocalFileSystem` 具体类型（除 main.rs 构造处）」——这条 grep 就能验，零额外代码。MVP 阶段 object_store 不开 `aws` feature，切 R2 时再开。同时把 User Story 31 从「asset store abstracted so I can migrate」降级为『Further Notes』里的一行说明，别让它以 story 身份产生验收压力。 |
| **M15** | 简洁 | PRD『Modules to build』的 `maps` / `realtime` / `admin` 三条 vs Slice 6 (0007) 验收项 1-3 | hard，Shotgun Surgery + Middle Man，且 PRD 自相矛盾。装饰物这一个功能被切给三个模块：`maps` = 「decoration CRUD + broadcast」、`realtime` = 「decoration live-sync」、`admin` = 「decoration edit endpoints」。给 decorations 加一个字段（比如 rotation）要改三处。更糟的是 `admin` 若真持有 decoration endpoints，它就只是把请求转给 maps 的 Middle Man——它自己没有装饰物领域逻辑。 | 在 PRD『Modules to build』里重写职责边界：`world` 模块（合并现 maps + npcs）独占 .tmj 加载、walkable 查询、decorations 表与其 REST handler、以及「写入后调用 realtime 的 `Room::broadcast(msg)`」；`realtime` 只提供 `broadcast(room_id, ServerMsg)` 这一个出口，不认识 decoration 概念；`admin` 缩成「一个 `require_admin` tower layer + members 的 3 个 handler」，明写「admin 不持有任何 decoration ha… |
| **M16** | 简洁 | PRD『Modules to build』`assets` 与 `generation` 两条 + Slice 3 (0004) 验收项 4「GET /api/library lists the member's assets + their job status」 | hard，职责重叠导致归属不明。`assets` 被定义为「blob store + metadata；library queries」，`generation` 被定义为「jobs 生命周期」。但 /api/library 的返回是 assets ⨯ generation_jobs 的联合视图：pending/failed 的条目根本还没有 assets 行（只有 job 行），done 的条目才有。当前定义下 agent 只能二选一：让 assets 模块去 SELECT generation_jobs（Feature Envy），或者两边各写一份状态映射（Duplicated Code）。 | 把 /api/library 明确划给 `generation` 模块，并规定它的数据形状：库列表 = `SELECT ... FROM generation_jobs LEFT JOIN assets ON assets.id = generation_jobs.result_asset_id WHERE generation_jobs.owner_id = ?`，即「job 是库条目的主键实体，asset 是它完成后的产物」。把 `assets` 模块降为一个无 handler 的内部 crate 模块：只暴露 `put(bytes, kind) -> AssetId` / `url_for(AssetId)`，不做任何列表查询… |
| **M17** | 简洁 | PRD『Schema』代码块全部 6 张表（users.role / avatars.kind / assets.kind / generation_jobs.kind / generation_jobs.status / decorations.z_layer） | hard，Primitive Obsession + Mysterious Name。(1) 所有枚举列都是裸 TEXT 且**没有 CHECK 约束**——注意 v2 调研文档的 schema 是带 `CHECK (kind IN (...))` 的，PRD 抄写时把约束丢了，这是净退步：一次拼写错误（'sprite-sheet' vs 'sprite_sheet'）会静默写库。(2) `kind` 一个名字承载三套完全无关的词汇表（avatar 的 modular\|generated、asset 的 sprite_sheet\|accessory\|decoration、job 的 avatar_generated\|acce… | 三处改动写进 PRD schema 块：(a) 每个枚举列加 `CHECK (col IN (...))`，包括 users.role、avatars.kind、assets.kind、generation_jobs.kind、generation_jobs.status；(b) 改名消歧：`assets.kind` → `asset_kind`，`generation_jobs.kind` → `job_kind`，`avatars.kind` 保留；(c) 在 PRD 新增一小节「领域类型（Rust 侧）」，规定 `AvatarKind` / `AssetKind` / `JobKind` / `JobStatus` / `Sl… |
| **M18** | 简洁 | PRD『Schema』`avatars(... is_active INTEGER ...)` + Slice 2 (0003) 验收项 2「GET /api/avatar returns the member's active avatar」+ Slice 4 (0005) 验收项 3「set a… | hard。`is_active` 引入了一个 schema 无法表达、PRD 也没提的不变式：「每个 owner 至多一行 is_active=1」。没有 partial unique index，两次并发 activate 就能产生两个 active avatar，而 realtime snapshot 取的是「the active avatar」——结果不确定。而且 User Story 里没有任何需求要求「保留一个形象集合并切换」，只要求「我当前长这样」。v2 调研文档用的是 `users.avatar_id` FK，反而更简单。 | 二选一并写进 PRD：**(推荐)** 删掉 `avatars.is_active`，改在 `users` 加 `active_avatar_id TEXT REFERENCES avatars(id) ON DELETE SET NULL`——单一真相源，无不变式可破，activate = 一条 UPDATE users；或者保留 is_active 但在 schema 块里补上 `CREATE UNIQUE INDEX idx_avatars_one_active ON avatars(owner_id) WHERE is_active = 1;` 并在 Slice 2 验收项里加一条并发 activate 的测试。 |
| **M19** | 简洁 | Slice 5 (0006) 验收项 1「collision/walkable layers loaded by both server (movement clamp) and client (rendering)」 | hard，Duplicated Code 跨语言版。「哪些格子可走」这条规则会有两个实现（Rust 解析 .tmj + TS/Phaser 解析 .tmj），两者一旦对 Tiled 的图层语义理解有微小差异（是看 collision 图层的 gid≠0，还是看 tile property `walkable=false`，边界格算不算），就表现为服务端 clamp 和客户端预测不一致 → 玩家橡皮筋。这个 bug 在 30-50 人房里最难查。 | 把「可走性」的唯一解析者定为服务端：服务端 boot 时解析 .tmj，暴露 `GET /api/scenes/:id` 返回 `{width, height, walkable: <行主序 bitmap 或 base64 位图>}`；客户端只用 Phaser 的 Tiled loader 渲染**视觉图层**，碰撞判定一律读这个 bitmap，绝不自己解析 collision 图层。改写该验收项为「服务端是 walkable 的唯一解析者；客户端从 GET /api/scenes/:id 取 bitmap；集成测试断言同一坐标在服务端 clamp 与 bitmap 中判定一致」。 |
| **M20** | 扩展 | Slice 1 (#2) AC 第 4 条 `WS /ws/room`；PRD #1 §Realtime（对比 docs/pixel-mosaic-game-workflow-v2-rust.md §二.2 用的是 `/ws/:room` + `DashMap<RoomId, Room>`） | PRD 把调研文档里带 room 参数的路由收窄成了字面量 `/ws/room`，路径里没有 room 标识。增长点 6 里最便宜的那一档水平扩展（同一进程内多房间分片，30-50 CCU 一房）因此需要同时改路由、改客户端 URL、改 `Rooms` 的 keying；增长点 3 如果最终决定用"一场景一房间"也一样。这是一个纯粹白送的耦合点。 | Slice 1 就用 `GET /ws/room/:room_id`，服务端保留 `Rooms: DashMap<RoomId, RoomHandle>`，MVP 只接受 `default`（其余返回 404），客户端 URL 从配置里拼。成本约等于零，省掉后续一次前后端同步改动。 |
| **M21** | 扩展 | PRD #1 §Schema › `generation_jobs(...)` 与 `assets(...)`；Slice 3 (#4) AC 第 2 条 | `generation_jobs` 只有 `params_json / result_asset_id / error`，没有 `provider` 和供应商侧句柄。而 fact layer（docs/reference/pixellab-api.md §三、§十一）确认真实流程会产出必须持久化的供应商态：`character_id`（PixelLab 侧 UUID）、`background_job_ids`（每方向一个）。增长点 2（换供应商/本地模型）因此不是加法：换了之后无法分辨历史 job/asset 出自谁、无法恢复进程重启时在飞的任务、无法灰度并行两家。`assets` 同样缺 provider 溯源，未来"把某供应商产… | Slice 3 的 migration 里就加：`generation_jobs` 增 `provider TEXT NOT NULL DEFAULT 'pixellab'`、`provider_ref TEXT NULL`（存 character_id / job_ids JSON）、`cost_usd REAL NULL`（fact layer §十一.7 要求的成本护栏）；`assets` 增 `provider TEXT NULL`。四个列，一次 migration，零业务逻辑改动。 |
| **M22** | 扩展 | PRD #1 §Implementation Decisions › Generation（"`create_character`, `create_image_pixflux`, `create_map_object`, `animate_character`"）；Slice 3 (#4) AC… | 两个问题叠加，共同破坏增长点 2。(1) trait 的形状是供应商动词：PRD 直接点名 PixelLab 的四个工具名当接口，一旦 `PixelLabClient` 按这四个方法建模，换供应商就是重塑 trait + 改所有调用点，而不是加一个 impl。(2) stub "同步 resolve" 让 trait 只需要一个阻塞式 `generate() -> Bytes` 就能通过全部测试，但真实供应商是 5-9 分钟、2 次调用 + 2 轮轮询、每方向一个独立 job（fact layer §一.3、§三、§四步骤 2）。于是没有任何测试会覆盖 pending→轮询→done 的时间线、部分方向失败、进程重启后续跑——而这恰… | 把 trait 定成领域动词 + submit/poll 两段式：`trait SpriteProvider { async fn submit(&self, req: SpriteRequest) -> Result<ProviderRef>; async fn poll(&self, r: &ProviderRef) -> Result<JobOutcome /* Pending{done_of_total} \| Done(Vec<DirectionSheet>) \| Failed(String) */>; }`。`ProviderRef` 落 `provider_ref` 列，worker 变成"读 pending 行 →… |
| **M23** | 扩展 | PRD #1 §Implementation Decisions › Generation："PixelLab.ai via its official MCP/API (`create_character`, `create_image_pixflux`, `create_map_object`,… | 这一行与已核实的事实层直接冲突，且违反 docs/CONVENTIONS.md §一 的"调研层结论不允许被当成事实引用"。docs/reference/pixellab-api.md §一.2 与 §六 明确：MCP 是开发期工具，用户侧生成**必须**走 REST v2；§九 进一步说明 MCP 的参数名与 REST 不同（`size` vs `image_size`、`n_directions`、`proportions` 为 JSON 字符串），"不要把 MCP 的参数名照搬到 REST 调用"。PRD 列的这四个名字全是 MCP 工具名，对应的 REST 端点其实叫 `create-character-with-4-dir… | 把这行改成："PixelLab REST v2（`https://api.pixellab.ai/v2`，Bearer token 仅存后端），端点见 docs/reference/pixellab-api.md §四/§五；MCP 仅用于开发期在编辑器内出素材，不进产品代码路径。" 并在 Slice 3/4 的 AC 里点明真实 impl 用 `reqwest` 打 v2（fact layer §十.3 已排除 JS SDK）。 |
| **M24** | 扩展 | PRD #1 §Implementation Decisions › Storage；Slice 3 (#4) AC 最后一条；对照 docs/pixel-mosaic-game-workflow-v2-rust.md §二.1 的 `.nest_service("/assets", ServeDi… | 设计只保证了"写"这一侧的抽象（`Arc<dyn ObjectStore>`），但"读"这一侧完全没有缝：前端拿到的素材 URL 从哪来、长什么样，没有任何一处定义。调研骨架给的范式是直接 `ServeDir` 本地目录，如果照抄，`/assets/<path>` 这个 URL 形状就会散落进前端、`meta_json`、以及 Phaser 的 loader 里。切到 R2 时只剩两条差路：把所有素材字节从 Rust 进程代理转发（R2 的意义全没了），或者改成签名 URL——那就要同时改前端和已入库的引用。增长点 1 于是不是"换一行 builder"。 | Slice 3 引入一个 `AssetUrls` seam：`fn url_for(&self, storage_key: &str) -> String`，本地实现返回 `{PUBLIC_ASSET_BASE}/{key}`，R2 实现返回 CDN/签名 URL；前端只从 API 响应里读完整 URL，绝不自己拼路径。同时明确 `assets.storage_key` 永远是 store 相对 key（不含 scheme、不含 `./data`、不是 URL）。Slice 3 已有的"第二个 store impl in tests"要把 `url_for` 也覆盖进去，否则那条 AC 只验证了一半。 |
| **M25** | 扩展 | PRD #1 §Testing Decisions（"assert on JSON / WS frames"）；Slice 1 (#2) AC 第 7 条 | 六个增长点里有四个（场景、LLM NPC、装饰同步、分片）都要动 WS 协议，但协议本身没有单一事实来源（Rust 侧 serde 类型与前端 TS 类型各写一份，靠人对齐），也没有版本号。加上测试直接断言 JSON 帧，等于把线格式钉进测试——调研文档 §二.2 明确规划了"JSON 起步、之后换 MessagePack"，那次切换会一次性打爆全部前后端断言。另外浏览器会缓存旧前端，服务端协议演进后没有任何握手能识别版本不匹配。 | Slice 1 做三件小事：(1) full snapshot / hello 帧里加 `protocol_version: 1`，客户端不匹配就提示刷新；(2) 协议消息的判别式与字段写进一份 `docs/reference/ws-protocol.md`（或用 `schemars` 从 Rust 类型导出 JSON Schema，前端 TS 类型由它生成），作为 Rust 与 TS 共同的基准；(3) 前端 `net` 模块里隔出一个 codec（`encode`/`decode`），单测断言解码后的对象而非原始帧字符串。 |
| **M26** | 扩展 | Slice 5 (#6) AC 第 2 条："walking through the bar door moves the player between bar and yard scenes" | 场景切换的触发条件与目标落点没有说明来源。如果门的位置、目标场景、落地坐标是写在 Rust/TS 代码里的（最自然的实现），那增长点 3（加第二个场景/房间）就必须改后端代码 + 前端代码 + 重新部署二进制，不是加法。同理，新场景的出生点也无处声明。 | Slice 5 就规定 portal 与 spawn point 从 Tiled 的 object layer 读：`.tmj` 里放 `portals`（矩形 + 属性 `to_scene`、`to_x`、`to_y`）和 `spawns`（属性 `name`）两个 object layer，服务端启动时解析成场景注册表。加一个场景 = 丢一个 `.tmj` 进 assets + 一行注册。同时把 collision 来源也做成 seam（Slice 1 的 clamp 依赖一个 `WalkableMap` 结构，Slice 1 用硬编码矩形构造它，Slice 5 换成从 `.tmj` 构造），避免 Slice 5 重写 clam… |
| **M27** | 扩展 | PRD #1 §Implementation Decisions › Bartender NPCs（"client sends {type:"interact", target: npc_id} -> server returns the current dialogue node"）；Slice… | 三处让增长点 4 变成返工而非加法：(1) 协议是隐式请求-响应"interact 之后紧接着来的那帧就是我的对白"，没有 correlation id、没有 conversation id、没有"正在思考"状态。LLM NPC 是 0.5-5 秒延迟 + 可能流式 + 可能超时不回，届时必须改协议和客户端渲染逻辑。(2) "the current dialogue node" 暗示服务端持有每玩家会话态，但 PRD 没有定义它存在哪、何时过期——LLM 需要的正是这个会话态。(3) `npc_def` 只有"dialogue tree as JSON"一种形状，没有行为判别式，也没有对应的表/文件位置定义，加 LLM 行为等于改数… | Slice 5 把对白回复定成独立消息类型而非顺序耦合的响应：`{type:"dialogue", conversation_id, npc_id, node_id, text, choices, pending: bool, done: bool}`，客户端按 conversation_id 路由、允许同一 conversation 收到多帧（先 `pending:true` 再补文本）。服务端保留一个 `DashMap<(player_id, npc_id), Conversation>` 带 TTL。`npc_defs` 落表（或明确落文件）并带 `behavior TEXT NOT NULL DEFAULT 'scripte… |
| **M28** | 扩展 | PRD #1 §Stack › Frontend（"Logical resolution 240×160, integer-scaled, imageSmoothingEnabled=false"）；Slice 1 (#2) AC 第 6 条；§Out of Scope（"CRT/LCD post-… | 增长点 5 的加法性取决于渲染路径结构，而 Slice 1 只说了"整数缩放"没说缩放发生在哪一层。如果 Slice 1 直接让 Phaser Scale Manager 对主 canvas 做整数 zoom、并把摇杆/聊天/库/形象编辑器都画在 Phaser 场景里，那后加 CRT 会遇到两个真问题：(a) 扫描线/曲面畸变必须跑在**升采样之后**的目标上，240×160 上打 shader 出不来效果，届时要把世界改成先渲进一个 240×160 的 RenderTexture 再单独升采样——那是渲染路径重构；(b) shader 会连带扭曲 UI 与聊天文字。BGM 另有一个小坑：移动端音频需要用户手势解锁，若登录后自动进房… | Slice 1 就固定：世界渲染进单个 240×160 的 RenderTexture / 单摄像机，升采样只在**一处**发生（写进 AC 与注释："the one place scaling happens"）；摇杆、按钮、聊天面板、库、形象编辑器全部走 DOM 覆盖层，不进 Phaser 场景；确认用 WebGL renderer 而非 Canvas。另外保留一个显式的"tap to enter"过场（顺带作为将来 BGM 的 audio unlock 手势）。这三条现在决定成本≈0。 |
| **M29** | 扩展 | Slice 1 (#2) AC 第 4/5 条（10Hz delta + broadcast）；对照 docs/pixel-mosaic-game-workflow-v2-rust.md §七 阶段 2 踩坑预警（`RecvError::Lagged`、空房间 DashMap 不回收） | 调研文档已经点出的两个坑没进任何切片的验收标准：(1) `broadcast::channel` 满时慢客户端拿到 `Lagged`，如果只是忽略，该客户端从此静默丢帧、状态永久漂移——这在 30-50 CCU 下偶发，在增长点 6 的更高并发下变成主要故障模式，且表现为"某些人看别人卡住"这种极难归因的 bug；(2) 空房间的 DashMap entry 与 tick task 不回收，多房间分片后会持续泄漏。锁定决策 1 明确要求 10Hz delta + 插值，那么"delta 丢失如何恢复"是这个方案的必答题，不是可选优化。 | Slice 1 加两条 AC：收到 `RecvError::Lagged(n)` 时立即给该连接重发一次 full snapshot（delta 协议的恢复语义写进 ws-protocol 文档），并加一个集成测试用小 channel 容量 + 故意不读的客户端来触发它；房间最后一个连接断开时回收 entry 并 abort tick task。两处都是 Slice 1 的十几行。 |
| **M30** | 实时 | Slice 1 0002 AC #4/#7; PRD 0001 §Realtime room model (L104 "New connections get a full snapshot on join") and §Map & decorations (L110) and chat ring… | The join path has an unavoidable snapshot/stream race that the spec does not address, and every later slice reuses this path. If a connection builds the snapshot from DashMap and *then* calls `tx.subscribe()`, any mutation in between (a move, a chat line, an admin decoration add) is lost forever — a permanent ghost/mis… | Mandate the subscribe-then-snapshot pattern in Slice 1 and make later slices inherit it: (1) `let mut rx = scene.tx.subscribe();` **before** reading state; (2) build the snapshot capturing `{seq, chat_seq, decorations_rev}` under the same logical point; (3) send the snapshot; (4) then drain `rx`, discarding frames whos… |
| **M31** | 实时 | Slice 1 0002 (no reconnect/heartbeat AC) + PRD 0001 §Realtime room model (L100-105) + locked decision 7 (mobile-first) | Disconnect, heartbeat and reconnect semantics are entirely absent, which is a mobile-first-specific hole. Three concrete problems: (a) a backgrounded mobile tab keeps the TCP socket half-open for minutes — the player stays in DashMap as a frozen ghost since there is no ping/pong liveness check; (b) on reconnect the sam… | Add Slice 1 ACs: (1) server sends WS `Ping` every 20 s and closes the connection after 2 missed `Pong`s; inbound idle timeout 60 s. (2) Player state is keyed by `user_id`; on a second connect for the same user the older connection is closed with a `session_replaced` close frame before the new one registers — assert sin… |
| **M32** | 实时 | Slice 5 0006 AC #1 ("collision/walkable layers loaded by both server (movement clamp) and client (rendering)") + PRD 0001 §Map & decorations (L108-110… | Two independent parsers of the same `.tmj` — Phaser's Tiled loader in TS and a hand-rolled parser in Rust — will diverge, and divergence in a collision grid is exactly what produces rubber-banding under client prediction. The usual divergence sources are all unaddressed: tileset `firstgid` offsets, tile flip/rotation f… | Make the server the sole authority over walkability and stop the client from deriving it: add `GET /api/maps/:scene/collision` returning a versioned, RLE-or-bitmask walkability grid (`{w, h, rev, bits}`) that the client consumes for prediction, and have the client use the `.tmj` only for rendering. Keep one Rust parser… |
| **M33** | 实时 | Slice 6 0007 AC #2 ("On decoration add/remove, the server broadcasts {decoration_added}/{decoration_removed}") + PRD 0001 §Map & decorations (L110, RE… | Decoration mutation crosses two transports (REST write, WS broadcast) with no ordering discipline, so several races are open: (a) a client that fetches the decoration list over REST and then subscribes can miss an add that landed in between, or double-apply one already in its list — with no version there is no way to t… | (1) Add a monotonic `decorations_rev` per scene; the join snapshot carries `decorations` + `rev`, and every event carries the post-mutation `rev`. Clients drop events with `rev <= local_rev` and, on detecting a gap (`rev > local_rev + 1`), refetch `GET /api/decorations?scene=` to resync — this also covers Lagged. (2) M… |
| **M34** | 实时 | PRD 0001 §Realtime room model (L105, chat fire-and-forget) + Slice 1 0002 AC #5; locked decision 8 | Inbound WS messages have no rate limit, size cap, or sanitization. `tower_governor` (research doc L18/L598) is HTTP middleware and does not see WS frames, so one authenticated client can push chat or move frames at socket speed. Because everything shares one `broadcast` channel, a single flooder overruns the 256-slot b… | Add Slice 1 ACs: (1) per-connection token buckets on inbound frames — chat 2/s burst 5, input 20/s, interact 5/s; over-budget frames are dropped (not queued) and repeated abuse closes the socket with a policy close code. (2) `WebSocketUpgrade::max_message_size(4 KiB)` and a chat text cap (e.g. 200 chars, rejected serve… |
| **M35** | 实时 | PRD 0001 §Generation async UX (L118-121) + §Further Notes lightweight mandate (L179) + Slice 3 0004 AC #3; SQLite WAL from research doc §3 (L166-170) | SQLite has exactly one writer, and the generation worker is the one component that can hold it for minutes. Per the PixelLab research doc, generation is asynchronous with 5-9 minute end-to-end latency and requires polling. If the worker opens a transaction to mark a job `running`, then `.await`s the PixelLab call or th… | (1) Slice 1 AC: pragmas applied via `SqlitePoolOptions::after_connect` on **every** pooled connection — `journal_mode=WAL`, `synchronous=NORMAL`, `foreign_keys=ON`, `busy_timeout=5000` — with a test asserting `PRAGMA journal_mode` returns `wal`. (2) Slice 3 AC: the worker uses one short transaction per state transition… |
| **M36** | 实时 | PRD 0001 §Testing Decisions (L157-162) + Slice 1 0002 AC #7 + PRD Solution (L28, "30-50 concurrent members") | The headline non-functional requirement — 30-50 CCU at 10Hz on a 2 vCPU / €5 VPS — has no test, no budget, and no owner in any slice. The test plan is entirely functional-contract-based (one WS client, one move, one chat). Every bandwidth and CPU regression in Slices 2 (avatar in state), 5 (two scenes) and 6 (decoratio… | Add to Slice 1 a `cargo run --bin loadtest` (or `#[ignore]`d test) reusing the same harness: open N=50 authenticated WS clients, drive random movement + 1 chat/10 s for 60 s, and assert (a) zero `Lagged` events server-side, (b) p95 inter-frame interval < 150 ms, (c) mean bytes/client/s under a stated budget (recommend… |
| **M37** | 实时 | PRD 0001 §Bartender NPCs (L115, "server returns the current dialogue node") + Slice 5 0006 AC #4; research doc realtime skeleton (L143-147, send task… | The connection's outbound path as sketched has only one source — the broadcast receiver — but several frames are inherently unicast: the join snapshot, a post-Lagged resync, validation errors, and the NPC dialogue node (which must go only to the interacting player, not the whole room). With the current design either di… | In Slice 1, give each connection a private `mpsc::Sender<Arc<str>>` alongside the broadcast receiver and have the send task `tokio::select!` over both; route the join snapshot and any resync through the private channel. Add `req_id` to inbound messages and echo it on unicast replies so the client can correlate. In Slic… |
| **M38** | 实时 | Slice 6 0007 AC #4 ("POST ban (login disabled)") + Slice 1 0002 AC #2 (logout) — no interaction with the WS layer specified | Ban and logout are specified purely as HTTP-side state changes, but identity on a WS connection is captured once at upgrade and never revalidated. A banned or logged-out user's socket therefore stays open indefinitely: they keep moving, keep chatting, and keep receiving the room stream until they voluntarily disconnect… | Add to Slice 6: maintain an in-memory `revoked: DashSet<UserId>` (or check a cheap epoch counter per user); on ban and on logout, look up that user's live connections and close them with a policy close code, and have the WS inbound loop drop frames from a revoked user. Test: connect as member, ban via admin REST from a… |

### MINOR — 建议修，成本低（21 条）

| # | 视角 | 位置 | 问题 → 建议 |
|---|---|---|---|
| m1 | spec | 0006-slice-5 验收项 3-4 vs PRD §Schema `npcs` 表（第 138 行）+ §Bartender NPCs（第 114-115 行） | 两个缺口：(a) PRD 声明了 `npcs(id, scene, npc_def_id, x, y)` 表，但切片 5 的验收项只说「两个 NPC 在固定位置、npc_def 对话树是 JSON」，**没有任何切片认领这张表的迁移**，也没说 `npc_def` 本体存在哪（DB 表？随包 JSON 资产？）。(b) `{interac… → 切片 5 验收项补：(a)「迁移新增 `npcs` 表并 seed 两个 bartender；`npc_def` 对话树作为随包 JSON 资产（路径写明），由 id 引用」；(b) 明确对话状态语义 —— 建议无状态化：`{interact, npc_id, node_id?}`，服务端按 node_id 返回下一节点，会话状态留在客户… |
| m2 | spec | 0002-slice-1 验收项 5（chat 重播）+ PRD §Realtime room model（第 105 行） | 聊天消息缺两项在原始研究中被点名的约束，PRD 与切片 1 都未捕获：(a) **文本消毒/转义**（v2 研究阶段 2 明确要求「服务端 clamp 坐标 + 反 XSS 消息（ammonia 或 escape）」）—— 聊天要渲染成头顶气泡，未消毒即 XSS 面；(b) **长度上限与发送频率限制** —— fire-and-forg… → PRD §Realtime room model 补一句「chat 文本服务端强制 ≤N 字符并转义/消毒后再广播；每连接发送速率上限 M 条/10s，超限丢弃」。切片 1 验收项 5 加断言：超长文本被截断/拒绝、含 HTML 的文本被转义后广播。 |
| m3 | spec | PRD User Story 5（「pick a default placeholder avatar」）vs 0003-slice-2 验收项 2（「assigns/returns a default on first… | US5 说的是「**挑选**一个默认占位形象」（存在若干占位可选），切片 2 实现成「首次调用**自动分配**一个默认」。二者不等价：按切片 2 的验收项，US5 的「pick」永远不会被实现或验收。另外切片 1 用的是「block avatar」纯色方块占位，与切片 2 的默认 avatar 是不是同一个东西也没说。 → 二选一并统一：要么把 US5 改写为「首次登录自动获得一个默认形象，可立即进房」（最轻，与锁定决策 9 一致），要么在切片 2 验收项加「`GET /api/avatar/presets` 含 ≥3 个默认整体形象，首次进入时 UI 让用户选一个」。同时说明切片 1 的方块占位在切片 2 被默认 modular avatar 取代。 |
| m4 | spec | 0003-slice-2 验收项 4（「pick hair/outfit/**accessory** + recolor」）vs 0004-slice-3 验收项 5（配件作为 equipped list + back/… | 「配件」在两个切片里是两个不同的数据概念：切片 2 把 accessory 当作 `layers_json` 里的一个**预置图层**（和 hair/outfit 同级、可调色），切片 3 把 accessory 当作 `assets` 表里的**可装备物**（equipped 列表 + slot anchor 叠加）。PRD 第 93… → 在 PRD §Avatar dual pipeline 明确统一：预置配件与生成配件**共用同一个 equipped/slot 模型**（预置配件只是 owner 为 system 的 asset），`layers_json` 只保留 body/hair/outfit 等基础部件。切片 2 验收项 4 改为「预置配件通过 equipped… |
| m5 | spec | 0007-slice-6 验收项 2（广播 `{decoration_added}` / `{decoration_removed}`）+ PRD §Map & decorations（第 110 行） | 装饰物实时同步是锁定决策 2 的核心，但广播帧的**载荷形状从未定义**：是否带 `scene`？是否带完整 `{id, tile_x, tile_y, asset_id, z_layer}` 供客户端直接渲染，还是只给 id 让客户端回拉 REST？在场景维度未定（见 blocker 1）的情况下，客户端无法判断该不该应用这次更新。切片… → PRD §Map & decorations 写死帧结构：`{type:"decoration_added", scene, decoration:{id, tile_x, tile_y, asset_id, z_layer, asset_url}}` 与 `{type:"decoration_removed", scene, id}`（… |
| m6 | 简洁 | PRD『Bartender NPCs』+『Schema』`npcs(id, scene, npc_def_id, x, y)` + Slice 5 (0006) 验收项 3 | Speculative Generality。规格是「两个 NPC，固定位置，脚本台词，无运行时编辑」（锁定决策 3）。为两行永不变更的数据建一张表 + 一次 migration + 一套读取路径，而底图本身却是静态 .tmj 文件——同样静态的东西用了两种截然不同的存储机制。`npc_def_id` 这个间接层指向的 npc_def 又… → 删掉 `npcs` 表。NPC 定义与位置一起放进随 .tmj 一同 ship 的静态资源 `assets/scenes/bar/npcs.json`：`[{id, sprite, tile_x, tile_y, dialogue:[...]}]`，boot 时读进内存 `HashMap<NpcId, NpcDef>`。少一张表、少一次… |
| m7 | 简洁 | PRD『Bartender NPCs』「dialogue tree as JSON」/『Modules to build』「`npcs` - scripted dialogue tree engine」+ Slice 5… | Speculative Generality（命名驱动的过度设计）。实际需求是：靠近 + 按键 → 出一句话 → 某句话能掀开菜单。把它命名为「dialogue tree engine」会让 agent 造出节点图解释器：条件跳转、变量、访问过标记、玩家选项分支——规格里一个都没要。 → 把『Modules to build』里的「scripted dialogue tree engine」改成「npc dialogue lookup」，并在 PRD 里把对话数据结构钉死为最小形状：`dialogue: Vec<Node>`，`Node { text: String, opens_menu: bool }`，交互时按顺序推… |
| m8 | 简洁 | PRD『Avatar dual pipeline』+『Schema』`avatars.layers_json` + Slice 3 (0004) 验收项 5「stored in the avatar's `layers_… | Mysterious Name。列名叫 `layers_json`，实际装两个不同概念：预置部件组合（base/hair/outfit + 颜色）**和** equipped 配件引用列表（含 slot）。PRD 自己的措辞就是「stores a `layers` JSON describing preset base parts **a… → 列改名为 `modular_config_json`，并在 PRD 里给出对应的 Rust 类型：`ModularConfig { base: BaseParts, equipped: Vec<Equipped { asset_id: AssetId, slot: Slot }> }`。同步改 Slice 2 验收项 1、Slice 2… |
| m9 | 简洁 | PRD『Avatar dual pipeline』 + 隐含于 Slice 2 验收项 3/4、Slice 3 验收项 5、Slice 4 验收项 3/4 | Repeated Switches。`avatars.kind` 会在至少 5 处被 match：渲染路径选择、PUT /api/avatar 校验、equip 拒绝 400、realtime snapshot 序列化、library 列表。锁定决策 5 说「两条管线同动作动画规范」，正是为了让渲染端不必知道 kind——但 PRD 没把… → 在『Avatar dual pipeline』末尾补一段契约：「`kind` 只在两处被 match —— (1) DB 读出时反序列化为 `enum Avatar { Modular(ModularConfig), Generated { sprite_asset_id: AssetId } }`；(2) 构建 realtime sna… |
| m10 | 简洁 | PRD『Realtime room model』`players: Map<id,{x,y,dir,...}>`、『Schema』`decorations(scene, tile_x, tile_y)` / `npcs(… | Data Clumps + Primitive Obsession。`(scene, x, y)` 三元组在 decorations / npcs / player state / move 意图 / walkable clamp 里反复同现，`dir` 没有类型（极易变成裸字符串 "n"/"ne"，前后端各拼一套）。而且 move 用… → PRD 里定义两个共享类型并统一全部字段名：`TilePos { scene: SceneId, x: u16, y: u16 }`、`Direction` 8 变体 enum（线上表示为 `"s"\|"se"\|...` 固定小写缩写）。所有 DB 列统一 `tile_x`/`tile_y`，所有线上消息统一 `{ "pos": {"x… |
| m11 | 简洁 | PRD『Further Notes』第一条 sustainability guardrails | judgement，但这条正是 Speculative Generality 的制度化。「every slice must keep the PixelLab dependency behind the trait, the asset store behind object_store, and the frontend logic o… → 缩到一条真正有当下回报的：「PixelLabClient trait —— 唯一理由是让集成测试完全离线、零成本（Testing Decisions 已依赖它）」。其余三项改写为不产生验收压力的表述：object_store 降为「main.rs 之外不出现具体 store 类型」（见另一条 finding），「frontend logi… |
| m12 | 简洁 | Slice 2 (0003) 验收项 3「recoloring via HSL shift on recolorable layers」+ PRD User Story 7 | judgement，但这是本设计里最贵的一处「小功能」。运行时 HSL 位移要么写 WebGL shader、要么逐帧 canvas 像素操作（8 方向 × N 帧 × 每次换色），且对像素画效果很差——GBA 风格的色块会因色相位移变糊、描边跟着变色。它服务的 Story 7 只要求「同轮廓的两个人看起来不同」。 → 把该验收项换成零代码方案：每个可换色部件预出 N 套调色板变体作为独立 PNG（如 hair_01_brown.png / hair_01_blond.png ...，6 色档）。选色 = 换贴图 key，前端一行代码，无 shader、无 canvas 像素操作、美术效果可控（也正是 GBA 时代的做法）。Story 7 完全满足。若坚… |
| m13 | 简洁 | Slice 7 (0008) frontmatter `blocked_by: [7]` + PRD 依赖图 `6(admin) -> 7(deploy)` | judgement。部署配置（Caddyfile / systemd unit / env 配置 / tracing / 备份脚本）与 admin 功能零耦合，却被排在最后、被整个功能集阻塞。结果是在完成 6 个切片之前拿不到任何真实用户反馈——而 v2 文档 §三阶段 1 的整个设计意图恰恰相反（Cloudflare Tunnel 早期… → 把 Slice 7 的 blocked_by 从 [7] 改为 [2]（只需要「一个能跑的二进制」）。它随时可以插队做完，之后每个切片都能直接推上线。同时在 Slice 1 验收项里加一行零成本的「`cloudflared tunnel --url http://localhost:8080` 可分享给朋友测试」——这不是新工作，是一条命… |
| m14 | 简洁 | Slice 1 (0002) 验收项 3「First registered user (or an ADMIN_USERNAME env bootstrap) is promoted to admin role」 | judgement，Speculative Generality 落在最关键路径上。Slice 1 里没有任何东西消费 `role`——第一个 role-gated 端点在 Slice 6。现在就实现「首个用户提权 或 env 变量提权」两条分支（含两者冲突时怎么办、env 里的用户还没注册时怎么办这类边界），是在 tracer bull… → Slice 1 只保留 schema：`users.role TEXT NOT NULL DEFAULT 'member' CHECK (role IN ('member','admin'))`。把提权逻辑（含 ADMIN_USERNAME env 分支和它的边界）整条移到 Slice 6 验收项——那里刚好需要一个 admin 才能测… |
| m15 | 扩展 | PRD #1 用户故事 18 / §Avatar dual pipeline（8 directions）；对照 docs/reference/pixellab-api.md §七"端到端单角色成本估算"（该表按 **4… | PRD 把 8 方向定为 canonical 契约，但事实层的成本估算全部建立在 4 方向上，且动画**按方向计费**：8 方向 v3 4 帧 @64px = 8 × $0.0129 ≈ $0.103，路径 B 端到端从 ~$0.19-0.30 涨到 ~$0.24-0.40。另外 v3 模式动画默认只做 south，8 方向必须显式传全部… → 要么在 PRD 里显式承认并接受这个成本（写一行：8 方向使每角色动画成本约翻倍、每个预置部件美术量翻倍），要么把 canonical 契约定为 4 方向 + 水平镜像出斜向（方向枚举与 sheet 布局仍按 8 方向定义，只是斜向由镜像填充），后者让"以后升级到真 8 方向"仍是纯数据替换。无论选哪个，都要写进第 2 条那份 sprit… |
| m16 | 扩展 | PRD #1 §Schema › `assets(id, owner_id, kind, storage_key, meta_json, created_at)` | 缺 `content_type`、`byte_size`、内容哈希，也没有"key 不可变"的约定。这三样都是增长点 1 的直接依赖：R2/S3 的 PUT 需要显式 content-type，否则对象以 `application/octet-stream` 返回、浏览器不当图片处理；迁移时没有 size/哈希就无法校验"本地 N 个对象… → Slice 3 的 `assets` 加 `content_type TEXT NOT NULL`、`byte_size INTEGER NOT NULL`、`sha256 TEXT NULL`，并在 PRD 里写死一条：storage_key 一律 UUID/内容寻址、写入后不可变，覆盖即新 key + 更新引用。这样 R2 迁移可以做… |
| m17 | 扩展 | PRD #1 §Map & decorations（"the server broadcasts {decoration_added}\|{decoration_removed} over WS"）；Slice 6 (#… | 装饰物的权威态在 DB（跨进程可共享，好），但通知只走进程内 `tokio::broadcast`。增长点 6 走到多进程那一档时，A 进程上管理员摆的椅子不会推给 B 进程的玩家——而且失败方式是静默的（DB 有、部分在线玩家看不见，刷新才出现），排查成本远高于修复成本。 → Slice 6 把所有装饰变更收敛到一个 `publish_room_event(scene, event)` 函数（唯一的广播出口），MVP 实现就是进程内 broadcast。将来换成跨进程总线（或最朴素的"DB 事件表 + 各进程轮询"）只需替换这一个函数体。现在只是"别在 handler 里直接 `room.tx.send()`"… |
| m18 | 实时 | PRD 0001 §Realtime room model (L83/L104: "clients interpolate (~100-200 ms)") + Slice 1 0002 AC #4 ("client in… | Two interpolation details are left to chance and both are visible on mobile. (a) At a 10 Hz tick the frame interval is exactly 100 ms, so a 100 ms interpolation buffer ha… → Pin the numbers in the PRD: interpolation delay = 150 ms (1.5 frames), buffer clamped at 2 frames, and on buffer starvation hold the last position rather than extrapolati… |
| m19 | 实时 | Slice 1 0002 AC #4 (10 Hz tick) — implementation detail not covered by any AC; research doc §2 (L159, `tokio::… | `tokio::time::interval` defaults to `MissedTickBehavior::Burst`: after any stall (a blocking argon2 hash on the same worker thread, a GC-ish pause, a long DB write) the t… → Specify `interval.set_missed_tick_behavior(MissedTickBehavior::Delay)` (or `Skip`) on the room tick, and have every frame carry the server `t_ms` so the client interpolat… |
| m20 | 实时 | PRD 0001 §Realtime room model (L104, "New connections get a full snapshot on join") — snapshot contents never… | "Full snapshot" is undefined, and four later slices each need to extend it (avatar in #3, scene + npc state in #6, decorations in #7). Without a written frame schema each… → Write the snapshot schema into the PRD: `{v:1, type:"snapshot", seq, t_ms, you:{id,name,x,y,dir}, scene, players:[...], avatars:{id->payload}, decorations:[...], decorati… |
| m21 | 实时 | PRD 0001 §Realtime room model (L104, `WS /ws/room`) vs research doc §2 (L87/L127-139, `/ws/:room` with `DashMa… | The route shape is inconsistent between the PRD (fixed `/ws/room`) and the skeleton the agents will copy (`/ws/:room`, room id taken from the path and `or_insert_with`-cr… → Pin `/ws/room` with no path parameter in Slice 1, or if scene appears in the URL, validate it against a compile-time allowlist of known scenes and reject anything else wi… |

### NIT — 可选（8 条）

| # | 视角 | 位置 | 问题 → 建议 |
|---|---|---|---|
| n1 | spec | .scratch/issues/*.md YAML frontmatter 的 `id` 字段 vs docs/agents/issue-tracker.md §Conventions 第 27 行 | 约定要求「Issue `id` 是与文件名前缀匹配的**零填充**整数（`0001`, `0002`…）」，但 8 个文件的 frontmatter 全部写成 `id: 1` … `id: 8`（YAML 里还是整数而非字符串）。正文交叉引用又用 `#2`…`#8` 的非填充形式。约定与实际不一致，未来用脚本按约定解析会对不上。 → 二选一：把 frontmatter 改为 `id: "0001"` 等字符串形式；或修改 issue-tracker.md 的约定为「`id` 为整数，**文件名前缀**零填充」——后者改动更小且与现状及 `blocked_by: [2]` 的整数写法一致，推荐后者。 |
| n2 | spec | 0001-house-of-imbibe-prd.md §Child issues 末行 Dependency graph（第 194 行） | 同一节里混用两套编号：上方列表用 issue 号（`#2 - Slice 1`…`#8 - Slice 7`），紧接着的 dependency graph 用**切片号**（`1(skeleton) -> {2(modular), 5(scenes)}`）。因为切片号恒等于 issue 号减一，这行图极易被误读成 issue 依赖（例如误… → 把该行统一改为 issue 号并标注，如：`#2(skeleton) -> {#3(modular), #6(scenes)}`；`#3 -> #4(generation) -> #5(generated-avatar)`；`#4 + #6 -> #7(admin) -> #8(deploy)`，并在句首写「以 issue 号表示」。 |
| n3 | 简洁 | PRD『Modules to build (seams respected)』8 个后端模块 | judgement。auth / assets / generation / avatars / realtime / maps / npcs / admin 对一人 MVP 偏细。结合前面几条（admin 应只剩一个 layer + members handler、npcs 应是静态配置、assets 应无 handler），照单执行会… → 把该清单改写成 5 个模块的目标文件布局并注明合并关系：`auth`、`avatars`（含 presets 与合成配置）、`generation`（吞掉 assets 的 store 包装与 library 查询）、`realtime`（房间 + 聊天 + broadcast 出口）、`world`（.tmj + walkable +… |
| n4 | 简洁 | Slice 2 (0003) 验收项 2「GET /api/avatar returns the member's active avatar (or assigns/returns a default on first… | GET 带写副作用（首次调用会 INSERT 并绑定默认形象），违反读写直觉，也让「GET 是幂等的」这个假设失效——重试、预取、双标签页同时首登都会踩到。 → 把默认形象的创建挪到 register handler 里（注册事务内一条 INSERT + 设 active），GET /api/avatar 保持纯读、必然有结果。验收项改为「注册后 GET /api/avatar 立即返回默认形象；GET 无写副作用」。 |
| n5 | 简洁 | PRD『Child issues』末行依赖图「1(skeleton) -> {2(modular), 5(scenes)}; 2 -> 3 -> 4; 3 + 5 -> 6 -> 7」 | Mysterious Name。这行用的是 slice 序号，上面的 child issue 列表和各文件 frontmatter 用的是 issue id（相差 1）。于是「2」在同一份文档里既指 issue #2（skeleton）又指 slice 2（modular）。AFK agent 解析依赖时容易错位一格。 → 依赖图统一改用 issue id 并标注 slice 名：`#2(slice1 skeleton) -> {#3(slice2 modular), #6(slice5 scenes)}; #3 -> #4 -> #5; #4 + #6 -> #7 -> #8`。或直接删掉这行——每个 issue 的 frontmatter `blocke… |
| n6 | 扩展 | Slice 1 (#2) AC 全部；对照 docs/pixel-mosaic-game-workflow-v2-rust.md §六 Rust 硬约束 2 与 9 | Slice 2/3/6 都以"新加 migration"的方式做加法演进（这本身是对的），但 Slice 1 的验收标准里没有"建立 `migrations/` 目录 + 提交 `.sqlx` 离线查询数据"这一项。硬约束 2 要求 schema 变更后必须 `cargo sqlx prepare` 并提交 `.sqlx`；如果 Slic… → Slice 1 加一条 AC：`migrations/0001_init.sql` + 提交 `.sqlx` 目录 + 一个 `justfile`/`Makefile` 目标封装 `sqlx migrate run` 与 `sqlx prepare`，并在测试里用 `sqlx::migrate!()` 建库；README 写一行"改 sc… |
| n7 | 实时 | Slice 1 0002 AC #4 ("server clamps to a walkable area") vs Slice 5 0006 AC #1 (real Tiled collision) | Slice 1's placeholder clamp is a rectangle and Slice 5 replaces it with a real grid. If Slice 1 inlines a bounds check in the tick loop, Slice 5 has to surgically edit th… → Require the clamp in Slice 1 to sit behind a `WalkableMap` abstraction (`fn is_walkable(&self, tx: i32, ty: i32) -> bool`) with a `RectMap` impl, so Slice 5 only adds a `… |
| n8 | 实时 | Research doc §2 (L101, "JSON 起步够用，量化后再换 MessagePack") vs PRD 0001 — no serialization or budget statement | The PRD never records the serialization decision or a byte budget, so "switch to MessagePack later" has no trigger condition and the per-tick JSON `String` is cloned once… → State in the PRD: JSON for MVP, serialize the frame exactly once per tick and broadcast `Arc<str>` (not `String`) so the clone is a refcount bump; short field keys (`i,x,… |

---

## 四、PRD / Issue 必修订项（实施前）

分三批：**P0** 是"不改就一定会写错代码"的硬矛盾（纯决策，无代码）；**P1** 是 PRD 章节改写；**P2** 是 issue 文件的机械改写。P0+P1 合计约 6–8 小时，其中 sprite 契约需等 Spike-0 的半天真机验证。

### P0 — 四处硬矛盾，必须先定稿（1–2 小时）

| 序 | 位置 | 冲突 | 定稿为 |
|---|---|---|---|
| **1** | PRD §Realtime room model vs 切片 5 双场景 vs `decorations.scene` 列 | PlayerState 无 `scene` 字段，但切片 5 要 bar+yard 两场景、装饰表已按 scene 键控。而两片被标为"可并行" → 必然返工 | **scene 即 room**：`DashMap<SceneId, Scene>`，`scene` 进 PlayerState / snapshot / delta / `{move}` 处理；MVP 只有一个值；`scene_changed` 现在就声明为协议消息（切片 1 永不触发）。**（B1, B6, B8）** |
| **2** | PRD §Stack 的 Generation 一行 + 切片 3/4 AC 照抄 MCP 工具名 | PRD 写 "via its official **MCP**/API" 并点名 `create_character` 等 MCP 工具名；但 MCP 是 IDE/CLI 的**开发期**工具，不能服务终端用户请求（浏览器无 MCP 客户端 + token 必然暴露） | 运行时 = **PixelLab REST v2**（`api.pixellab.ai/v2`，Bearer token 仅存后端）；MCP 仅限开发期离线出素材，**不进产品代码路径**。Out of Scope 新增"前端直连 PixelLab / 用 MCP 服务用户请求"。**（B2, M23）** |
| **3** | PRD §Avatar dual pipeline 的"runtime 合成"措辞 vs 切片 2/3/4 AC 的 "assert composite / assert rendered" | 合成发生在服务端还是客户端**从未定义**，而三个切片的 AC 都隐含假设了服务端合成——后端集成测试无法断言 Phaser 渲染结果，这些 AC 现在无法通过 | **客户端合成**：服务端只广播 `avatar_snapshot = {kind, layers/sprite_asset_id, equipped[]}` 这份纯数据；slot anchor 表进 canonical contract。三处 AC 改为断言**广播数据**而非渲染结果。**（B3）** |
| **4** | PRD 4 次引用 "canonical sprite-sheet contract"、3 个切片以它为验收基准，但它从未被定义 | 帧尺寸 / 网格布局 / 方向枚举顺序 / 动画名集合 / 每动画帧数 / accessory per-slot anchor 全缺失 → 切片 2/3/4 各自假设一套，返工面覆盖渲染器 + 合成器 + worker，且**已付费生成的素材全部作废（不可回滚）** | 写 `docs/sprite-contract.md`：frame w×h、方向顺序固定、动画名 + 帧数、sheet 尺寸公式、`anchors` 为 per-direction-per-frame 的 (dx,dy) 表、sidecar `v/dirs/clips/anchors`；加纯函数 `validate_sheet()` 在 worker 落库前强制。**必须用 Spike-0 真实输出校对后再定稿。（B5, B10）** |

**同时需要你显式签字一处取舍 —— 8 方向 vs 4 方向**：8 方向让每角色动画成本与每个预置部件的美术量**翻倍**，而"solo builder with limited art"正是本项目前提；这是全设计里唯一"轻量"与"效果"真正对立、且代价随内容量线性增长的地方。**建议**：契约按 8 槽位定义、MVP 只做 4 主方向、`dirs` 声明实际存在的方向、渲染器按最近方向回退 —— 将来升 8 方向是纯数据替换，不动代码。

### P1 — PRD 章节改写（4–6 小时）

| 序 | 文件 · 章节 | 改成什么 |
|---|---|---|
| 5 | PRD §Realtime room model（帧契约，整段重写） | 每帧带 `{v, seq, t_ms}`；3s 一个 full **keyframe** + 其间 delta（慢客户端 3s 自愈）；显式处理 `RecvError::Lagged` 并补发 full snapshot；显式 `player_joined` / `player_left` 帧；`moving` 字段 + 停止时补一帧；subscribe-then-snapshot 消除 join 竞态；`seq` 缺口检测。**（B7, M1, M2）** |
| 6 | PRD §Realtime room model（鉴权段，新增） | WS 升级**前**跑 tower-sessions extractor，无登录态返 401 且**不升级**；`user_id`/`role` 只来自 session，**任何入站消息都不得携带身份**；Ping 20s / 2 次 Pong 未回即关；入站 idle 60s 超时；同用户第二次连接以 `session_replaced` 关旧连接；入站令牌桶限流。**（B4, M3）** |
| 7 | PRD §Realtime room model（带宽段，新增） | 帧内每玩家只放 `{id, x, y, dir, moving, avatar_rev}`（目标 ≤32 B/player）；avatar 全量走**侧信道**（join snapshot + `avatar_changed` 广播）；预算写进 PRD：**≤2 KB/帧、≤8 KB/s/客户端**；每 tick 只序列化一次并广播 `Arc<str>`；`MAX_PLAYERS_PER_SCENE=60` 硬上限。**（B9, M4）** |
| 8 | PRD §Realtime room model（移动模型段） | 把"目标格意图"改为**速度/方向意图** `{input, dx, dy, seq}`，服务端每 100 ms 按固定 `speed`（写死，如 4 tiles/s）推进并**重检每个中间格**，杜绝瞬移；客户端预测 + 服务端纠正，偏差 >0.5 tile 才平滑收敛（不硬 snap）。**（B10-realtime）** |
| 9 | PRD §Schema（整块重写） | 所有枚举列加 `CHECK` 约束；`avatars` 加双管线 CHECK（modular 必须有 `layers_json` 且 `sprite_asset_id` 为 NULL，generated 反之）；删 `is_active`，改为 `users.active_avatar_id` 外键（消掉"每 owner 至多一行"的隐式不变式）；`generation_jobs` 加 `provider / provider_ref / phase / cost_usd / attempts`；**删 `npcs` 表**（改随包 JSON）；`decorations` 加 `UNIQUE(scene,tile_x,tile_y,z_layer)`；统一 `tile_x/tile_y` 拼写；新增「领域类型（Rust 侧）」小节规定 `AvatarKind/AssetKind/JobKind/JobStatus/Slot/Direction` 全为 `sqlx::Type` enum，禁止 `kind: String`。**（M10, M11, M18, M19, M20, M14, B9）** |
| 10 | PRD 新增 §Avatar API | 复数资源面：`GET /api/avatars`、`POST /api/avatars`（仅 modular）、`PUT /api/avatars/:id`（仅 modular）、`POST /api/avatars/:id/activate`、`GET /api/avatar/active`；默认 avatar 在 register 事务内创建，GET 无写副作用。**（M11, m6）** |
| 11 | PRD 新增 §Privacy | 二进制上传物不进 `params_json`；照片走进程级 temp dir，终态即删并写 `photo_purged_at`，启动无条件清空；**禁止把原图放进 AssetStore**。**（M21）** |
| 12 | PRD §Storage | `AssetStore` facade 含 `public_url()->绝对 URL`；DB 只存 key 不存 URL；`storage_key` 格式固定、写入后不可变、内容寻址、可 `Cache-Control: immutable`；前端只用响应里的完整 URL。**（M14）** |
| 13 | PRD §Generation async UX | submit/poll 两段式 trait（名为 `SpriteProvider`，领域动词，`GenSpec` 是我们的类型）；多阶段编排在 worker 的 `phase` 列而非 trait；stub 可编程且首次 poll 必返 Pending；worker 每次状态跃迁一个短事务、**网络 `.await` 不跨事务**、CPU 图像处理走 `spawn_blocking`；每用户日配额 + `cost_usd` + 余额告警 + params 哈希永久缓存 + `PIXELLAB_ENABLED=false` 降级开关。**（P6, B9, M5, M16）** |
| 14 | PRD §Modules to build | 压缩为 5 个：`auth` / `avatars` / `generation`（含 store 包装与 library 查询）/ `realtime`（只暴露 `publish_room_event`）/ `world`（.tmj + walkable + decorations + npc defs）；`admin` 不作为模块存在，只是 `require_admin` layer + 3 个 members handler。**（M23）** |
| 15 | PRD §Testing Decisions | 删掉 Playwright；写明 vitest 认领 `net`/`game-state` 单测；Rust→TS fixture 桥；50 客户端 loadtest 为切片 1 交付物并作为切片 2/6 的回归门。**（M22, M6）** |
| 16 | PRD §Further Notes | sustainability guardrails 改写为三条可 grep 约束（见 m9）；加「10Hz tick 与 WS handler 永不碰 DB」；删 LLM NPC / 第二场景 / R2 作为设计驱动力的提法。**（m9, M5）** |
| 17 | PRD §Out of Scope | 新增：前端直连 PixelLab / MCP 服务用户请求；**MVP 装饰物不碰撞**；水平分片（多进程/跨机）。**（M17）** |

### P2 — issue 文件改写（与 P1 同批做完，纯机械）

| 序 | issue | 改什么 |
|---|---|---|
| 18 | `#2 切片 1` | AC 按本报告 §2.4 DoD 全量重写；删掉 admin 提权（移至 #7）；加 migrations + `.sqlx` + justfile；加 pragma（含 `busy_timeout`）；加 loadtest；加 `cloudflared tunnel` 一行。 |
| 19 | `#3 切片 2` | AC2 改用复数 avatar 端点；AC3 HSL→ramp LUT；AC4 改为断言 `avatar_changed` 推送而非帧内 avatar；新增双管线 CHECK + `PUT` 对 generated 返 400；新增"重跑 loadtest 未超预算"。 |
| 20 | **拆分 `#4`** → `#4a 切片 3a 素材基建` / `#4b 切片 3b 配件装备` | 3a：assets 表 + AssetStore facade（含 `public_url`）+ job 管线 + worker + `/api/generate` + `/api/jobs/:id` + `/api/library` + 配额/成本护栏；参数化测试用 `InMemory`（不自研 store）。3b：equipped/slot 模型 + 逐帧 anchor overlay，`blocked_by: [4a, 3]`，叶子节点。 |
| 21 | `#5 切片 4` | AC1 换 REST 端点名；AC4 扩为 equip + layers 双路径 400；AC5 改为可验证形式（walkdir + params_json 断言）；`blocked_by` 改 `[4a, 3]`；**写入 Spike-0 不通过的降级路线**（照片只出单方向立绘 + 头像，8 方向走 modular，用照片配色作 modular ramp）。 |
| 22 | `#6 切片 5` | AC1 改为"服务端是 walkable 唯一解析者 + `GET /api/scenes/:id` bitmap + Tiled 导出契约 + golden fixture"；AC2 加 portal/spawn 从 object layer 读 + 场景切换原子性测试；AC3 删 `npcs` 表改随包 JSON；AC4 改为推送式 `dialogue` + conversation_id + 单播断言。 |
| 23 | `#7 切片 6` | AC2 写死装饰帧结构 + `rev` + 缺口重拉 + 提交后广播顺序 + 409 + DELETE 幂等；AC4 加 `is_banned` 迁移 + 定向踢连接 + session 清除；新增 admin 端点表驱动 403 测试；新增 admin 提权逻辑（从 #2 移入）；`blocked_by` 改 `[4a, 6]`。 |
| 24 | `#8 切片 7` | `blocked_by` 改 `[2]`；AC 加 `websocat` 手工握手验证 + 备份恢复演练 + `sqlite3 .backup`。 |
| 25 | `docs/agents/issue-tracker.md` §Conventions | 约定改为「`id` 为整数，文件名前缀零填充」；PRD §Child issues 末行依赖图删除或统一为 issue 号。 |

---

## 五、更新后的风险登记（Top 8）

| # | 风险 | 触发条件 | 缓解（owner 切片） |
|---|---|---|---|
| **R1** | **带宽/CPU 超预算，€5 VPS 跑不动 30–50 CCU** — 从 planner 的 R3 升级为头号风险，因为 B5 揭示原设计超预算约 12×，而这个指标此前**没有任何测试** | 40 人同房；或切片 2 把 avatar 塞回帧；或切片 5/6 加了新广播 | 帧内只放 `{id,x,y,dir,moving,avatar_rev}`；avatar 走侧信道；每 tick 序列化一次 + `Arc<str>`；预算数字写进 PRD（≤2 KB/帧、≤8 KB/s/客户端）；**切片 1 交付 50 客户端 loadtest 并作为切片 2/6 的回归门**；`MAX_PLAYERS_PER_SCENE=60` 硬上限；MessagePack 切换有可测量触发条件 |
| **R2** | **实时状态永久 desync**（Lagged 丢帧 + 无 keyframe + 无 player_left + join 竞态 + 无心跳）→ 表现为"某些人看别人卡住"这类几乎无法归因的玄学 bug | 移动端 4G 抖动、前后台切换、单人刷屏挤爆 broadcast | 3s keyframe 自愈 + `Lagged` 补发 full snapshot + 显式 join/left 帧 + `seq` 缺口检测 + subscribe-then-snapshot + Ping/Pong + 同用户单连接不变式 + 入站令牌桶；全部在切片 1 有集成测试（含故意不排空客户端触发 Lagged） |
| **R3** | **PixelLab 成本/延迟失控 + 余额被刷爆** — 零门槛注册叠加付费 API，是本项目唯一的现金支出项 | 单形象 >预算、端到端 >5 分钟、或一个脚本刷注册 | Spike-0 前置量化（成本/墙钟/真实 sheet 布局）；每用户日配额 → 429；`cost_usd` 入库；启动+定时查 balance 低于阈值拒新 job；params 哈希永久缓存；`PIXELLAB_ENABLED=false` 降级开关；register/login 限流。**护栏必须在 3a 一次做好，不能"以后再加"** |
| **R4** | **sprite 契约返工 = 已生成付费素材全部作废**（不可回滚） | 切片 2/3/4 各自假设一套布局；或 anchors 到 3b 才发现要补 | 契约在切片 1 定稿（等 Spike-0 校对）+ `validate_sheet()` 纯函数在 worker 落库前强制；`anchors` 现在就加为可选字段（默认退化为 origin，成本≈0）；`v` + `dirs` 显式；4 方向 vs 8 方向的成本取舍在契约里显式签字 |
| **R5** | **SQLite 写并发死锁 / `SQLITE_BUSY` 全站不可用** — 从 planner 的 R6 升级，因为 M5 指出 worker 若在事务内 await 生成会攥住写锁**几分钟** | worker 长任务 + 多 admin 改装饰 + session 写入撞在一起 | pragma 施加到每个池连接（WAL / NORMAL / FK / **`busy_timeout=5000`**）并有测试断言；worker 每次状态跃迁一个短事务、网络 await 绝不跨事务、图像处理 `spawn_blocking`；写路径限制连接数；聊天不落库已消掉最大写压力源；tick 与 WS handler 零查询（有断言） |
| **R6** | **照片→8 方向不可行 / 风格不一致** | Spike-0 输出的多方向人物"每个方向像不同人"，或多步链之间变脸 | 降级路线现在就写进 #5：照片只出单方向立绘 + 头像，8 方向走 modular（用照片提取配色作 ramp）；**切片 4 不在关键路径上，可整块延后不阻塞上线**；多阶段每步产物存中间 asset，失败从断点重试不重跑付费步骤 |
| **R7** | **可走性双解析漂移 → 橡皮筋**；以及场景/装饰实时同步漂移 | Tiled 重导出改变语义；两 admin 并发；客户端重连期间发生变更 | 服务端是 .tmj 唯一解析者 + `GET /api/scenes/:id` bitmap + Tiled 导出契约启动时校验 + golden fixture CI；MVP 装饰物不碰撞（避免动态网格）；`decorations_rev` + 缺口重拉 + 提交后广播 + UNIQUE 409 + DELETE 幂等；所有变更走唯一出口 `publish_room_event` |
| **R8** | **范围膨胀 + 抽象层空转**（solo 项目双向风险：既会"顺手多做一点"，也会为四个 Out of Scope 的未来预留而长出空壳） | 每个切片的"就加个小 if"；或 sustainability guardrails 被字面执行 | 每切片 AC 即范围合同，动手前重读 Out of Scope；guardrails 收敛为三条可 grep 约束；渲染层单一路径 + `scene/` 禁 `kind` 分支 lint + `equip` 对 generated 硬 400；README 写明双管线不互通是**有意的**、不是待办；切片 7 之后才允许 polish |

**从 planner 风险登记里降级/移除的**：原 R5（移动端性能）降为切片 1 DoD 的一条（真机横屏 + LRU 64 张合成纹理 + 远处玩家不播动画 + 摇杆走 DOM），不再单列；原 R8（契约漂移）已被 fixture 桥 + `docs/ws-protocol.md` 单一权威覆盖，并入 R2/R4。

---

## 六、可持续性与轻量签字

**结论：修订后满足，修订前不满足。**

"轻量"这一项**原设计本来就满足**，且四份评审一致确认骨架选型是对的（单二进制 / SQLite / 一个 broadcast 房间 / 静态底图 + DB 装饰物 / 脚本 NPC）。本报告在轻量方向上做的是**减法**：删 `npcs` 表、删 `avatars.is_active`、删 Playwright、删自研第二个 store、删 8 个模块压到 5 个、删 HSL shader、把 admin 从模块降为一个 layer、把 sustainability guardrails 从四个未来目标收敛为三条可 grep 约束。唯一的加法是 P1–P8 那八个 seam，合计增量 ≤1 天——而其中 6 个（scene 键、双通道、协议冻结、WalkableMap、AssetStore、provider trait）**同时是"能离线测试"的前提**，不是为未来付的税。

"可扩展"这一项**原设计不满足**，且不满足的方式是危险的——文档里写着六个增长点都是"加法"，但实际上有两个（加第二个场景、换生成商）是重写，两个（LLM NPC、R2 迁移）是协议/前端级改动。这种"以为是加法"比"知道是重写"更贵。

**最小补丁就是六条，全部是字段/列/一个函数/一条纪律**：

1. `scene` 进房间键与每一帧（→ 加场景 = 丢一个 .tmj + 一行注册）
2. per-connection mpsc + dialogue 为服务端推送语义（→ LLM 升级 = 加一个 behavior 分支，而非改协议）
3. `AssetStore::public_url()` 归口 + DB 只存 key（→ R2 迁移 = 改一个 env）
4. provider trait 是 submit/poll + 领域动词 + 可编程 stub（→ 换生成商 = 加一个 impl；且**这是唯一让集成测试完全离线零成本的东西**）
5. sprite 契约 + `validate_sheet()`（→ 这是"换生成商"和"双管线渲染统一"的唯一回归网）
6. 协议冻结 + `v/seq/t_ms` + 未知 type 静默忽略（→ 前后端不必锁步部署，换传输层不打爆全部断言）

**唯一需要 owner 显式签字的取舍**（我不替他决定，但给出建议）：**8 方向 vs 4 方向**。8 方向让每角色动画成本约翻倍、每个预置部件美术量翻倍，而"solo builder with limited art"正是本项目的前提；这是全设计里唯一一个"轻量"与"效果"真正对立、且代价随内容量线性增长的地方。建议契约按 8 槽位定义、MVP 只做 4 主方向、`dirs` 声明实际存在方向、渲染器按最近方向回退——这样将来升 8 方向是纯数据替换。

**放行条件**：§四 的 P0（4 项，1–2 小时）+ P1（13 项，4–6 小时，其中 sprite 契约等 Spike-0 半天）+ P2（8 个 issue 改写）全部完成后，切片 1 立即开工；切片 7（部署）可与切片 1 之后的任何时点插队，不必等到最后。