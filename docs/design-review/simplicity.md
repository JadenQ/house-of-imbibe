# 设计评审 — 简洁性 (Fowler 异味基线)

**裁定**: `proceed-with-fixes`

**摘要**: 从简洁性视角看，这套设计的骨架是对的（单二进制、SQLite、一个 broadcast 房间、静态底图 + DB 装饰物、脚本 NPC），锁定的 9 条决策在文档里基本都能对上。真正的风险不在架构选型，而在三类具体的重量：\n\n**一、被写成硬验收项的 Speculative Generality。** 最刺眼的是 Slice 3 那条「verified by a second store impl in tests」——为一个已在 Out of Scope 的 R2 迁移，付一个永不上生产的 store 实现的代码成本，而 object_store 本身就是 trait，切换是 3 行构造函数。同类的还有为两行永不变更数据建的 `npcs` 表、被命名为「dialogue tree engine」的两句台词查表、以及『Further Notes』把 R2/第二场景/LLM NPC/后处理四个 Out of Scope 项制度化为「每个切片都必须保持可扩展」——这一条与紧邻的「Lightweight mandate」直接冲突，且是最容易催生空抽象层的表述。\n\n**二、两处会导致返工的语义模糊。** (1) Slice 1 的「10 Hz delta snapshot (only changed players)」没有区分房间级 delta 与 per-connection diff；后者会摧毁 v2 调研里那个 30 行的单 broadcast::channel 模型，换来的是对 30-50 人毫无意义的 12 KB/s 节省——这是第一个切片，读错一次全盘返工，应在动工前钉死为房间级。(2) `docs/pixel-mosaic-game-workflow-v2-rust.md` 按 CONVENTIONS.md 属决策层（会被当权威读），却仍规定 email_tokens / messages 落库 / rooms 表 / axum-login / lettre / imagequant / fal.ai——与 PRD 的 Out of Scope 正面矛盾，且它的 §八「行动 1/2」是抄了就跑的脚手架指令。agent 大概率据此生成错误的 0001_init.sql 和臃肿 Cargo.toml。这两条是我建议在写第一行代码前就修掉的。\n\n**三、模块与 schema 的职责重叠。** 装饰物一个功能被切给 maps / realtime / admin 三个模块（PRD 内部自相矛盾，且 admin 沦为纯转发的 Middle Man）；`/api/library` 横跨 assets 与 generation 两个模块的表，归属不明会长出重复的状态映射；schema 里所有枚举列是裸 TEXT 且**丢掉了 v2 调研原本带的 CHECK 约束**（净退步），`kind` 一名承载三套无关词汇表，back/hand slot 这个核心概念在 schema 里根本没有类型。另外 `avatars.is_active` 引入了一个 schema 表达不了、PRD 也没提的不变式（每人至多一个 active），缺 partial unique index。\n\n**切片划分**：只有 Slice 3 明确过大——它捆了「异步生成管线」和「配件装备 + overlay 渲染」两条零共享代码的纵切，还顺带把只需要前者的 Slice 4 挡在后面。建议拆 3a/3b，3b 变叶子。Slice 1 虽大但 tracer bullet 的价值是真的，值得保持整块（只建议把没有消费者的 admin 提权逻辑挪到 Slice 6）。Slice 7 被 Slice 6 阻塞是排序错误：部署与 admin 零耦合，却让真实用户反馈延迟到第 7 个切片之后，与 v2 文档「早期 tunnel 分享给朋友」的意图相反。\n\n**对一人 vibe coding 是否过重**：总体不过重，但 8 个后端模块偏细（照前述合并后合理数量是 5 个），且「运行时 HSL 换色」是全设计里性价比最低的一处——它要 shader 或逐帧 canvas 操作、对像素画效果还差，而它服务的 Story 7 用「每部件预出 6 套调色板 PNG」就能零代码满足。\n\n结论 proceed-with-fixes：Slice 1 在钉死 delta 语义（一句话）并给 v2 文档加取代标注（两处编辑）后即可开工；其余 major 在各自切片启动前修文档即可，无需推翻任何已锁定决策。

共 21 条发现。

---

## MAJOR

### PRD『Realtime room model』第 3 条 + Slice 1 (0002) 验收项 4「broadcasts a 10 Hz delta snapshot」

**问题**: hard。「delta snapshot（only changed players）」有两种读法：(a) 房间级 delta——每 tick 只打包本 tick 变化的玩家，一条消息经单个 broadcast::channel 发给所有人；(b) 每连接 delta——为每个订阅者维护「它上次看到什么」再做 diff。(b) 会直接摧毁 v2 调研里那个 30 行的 broadcast 模型：需要 per-connection 状态、per-connection 序列化、lagged 补偿、full-resync 协议。而收益是把 50 人 × 24B × 10Hz ≈ 12 KB/s 压到几 KB/s——对 30-50 并发毫无意义。这是整个工程的第一个切片，读错一次全盘返工。

**建议**: 在 PRD 该条和 Slice 1 验收项里把语义钉死为房间级 delta：「每 100ms tick 遍历 room.state，只序列化自上一 tick 起 x/y/dir/avatar 有变化的玩家，打成一条 JSON 消息经该房间唯一的 broadcast::Sender 发出；所有订阅者收到同一份字节。新连接单独收一次 full snapshot。禁止 per-connection diff / per-connection 序列化」。同时补一句 Lagged 处理：收到 RecvError::Lagged 就给该连接重发 full snapshot。

### docs/pixel-mosaic-game-workflow-v2-rust.md（决策层）§二.3 schema、§二.4 auth 流程、§五 Cargo.toml、§七 阶段 2 checklist vs PRD『Schema』『Out of Scope』

**问题**: hard，Duplicated Code 的最坏形态——两份分歧的副本。按 docs/CONVENTIONS.md，`docs/*.md` 是「决策层／最终采用方案」，会被当权威读。但该文档规定了 `users.email`+`email_verified`、`email_tokens` 表、`messages` 表持久化聊天、`rooms` 表、axum-login、lettre+Resend、tower_governor、sentry、imagequant/oxipng/fast_image_resize 流水线、fal.ai——PRD 明确把 email/聊天落库全部划入 Out of Scope，且改用 PixelLab 而非 fal.ai。AFK agent 拿 §八「行动 1/行动 2」照抄，会生成带 email_tokens + messages 表的 0001_init.sql 和一个塞了 lettre/imagequant/sentry 的 Cargo.toml，然后返工。

**建议**: 两步：(1) 在 v2 文档顶部加状态块「**已被 .scratch/issues/0001 PRD 取代（auth / schema / 聊天 / 生图供应商 / Cargo.toml 四节）；仅 §三 部署、§六 Rust 硬约束仍现行**」，并就地 ❌ 标注 email_tokens / messages / rooms 三张表和 lettre / axum-login / imagequant / fal.ai 条目（CONVENTIONS.md §三要求就地标注不删除）。(2) 在 PRD『Stack (locked)』下新增「MVP 依赖白名单」，正列约 15 个 crate（axum, tokio, tower, tower-http, sqlx, tower-sessions(+sqlx-store), argon2, rand, serde, serde_json, uuid, dashmap, object_store, reqwest, thiserror, anyhow, tracing(+subscriber)），并明写「v2 文档中的 lettre / imagequant / oxipng / fast_image_resize / sentry / tower_governor / validator / rmp-serde / axum-login 均不进 MVP」。

### Slice 3 (0004-slice-3-generation-library-accessories.md) 整体，7 条验收项

**问题**: hard，切片过大。这一片捆了两条互不依赖的纵切：(A) 异步生成基础设施 = PixelLabClient trait + real impl + stub + assets 表 + generation_jobs 表 + object_store + 后台 worker + POST /api/generate + GET /api/jobs/:id + GET /api/library + library UI；(B) 配件装备 = layers_json 的 equipped 结构 + back/hand slot 锚点定义 + 逐帧 overlay 合成渲染 + equip UI。(B) 是纯前端渲染 + 数据模型工作，和 (A) 的 job 生命周期零共享代码。而且 Slice 4（生成形象）只需要 (A)，现在却被整个 Slice 3 挡住。

**建议**: 拆成两个 issue：Slice 3a「生成任务管线 + 个人素材库」——验收止于「请求 accessory → 拿 job_id → worker 完成 → 出现在 /api/library，状态正确」，不含 equip；Slice 3b「配件装备 + overlay 合成渲染」——blocked_by [3a]。然后把 Slice 4 的 blocked_by 从 [3, 4] 改成 [3, 3a]，Slice 6 的 blocked_by 从 [4, 6] 改成 [3a, 6]。Slice 3b 变成不阻塞任何人的叶子节点。

### Slice 3 (0004) 最后一条验收项「verified by a second store impl in tests」+ PRD User Story 31 + PRD『Further Notes』sustainability guardrails

**问题**: hard，Speculative Generality 被写成了硬验收项。`object_store::ObjectStore` 本身就是 trait，LocalFileSystem → R2 的切换是构造函数里 3 行（v2 文档 §二.5 已给出两段代码）。为了「证明」这个抽象而在测试里再写一个 store impl，是为规格外需求（R2 迁移已在 Out of Scope）付真实代码成本，且这个 impl 永远不会被生产用到。附带成本：object_store 开 `aws` feature 会把 hyper/aws-sigv4 一整串拖进那个号称 <15MB 的单二进制。

**建议**: 删掉该验收项。改为一条约束句：「业务代码只依赖 `Arc<dyn ObjectStore>`，不出现 `LocalFileSystem` 具体类型（除 main.rs 构造处）」——这条 grep 就能验，零额外代码。MVP 阶段 object_store 不开 `aws` feature，切 R2 时再开。同时把 User Story 31 从「asset store abstracted so I can migrate」降级为『Further Notes』里的一行说明，别让它以 story 身份产生验收压力。

### PRD『Modules to build』的 `maps` / `realtime` / `admin` 三条 vs Slice 6 (0007) 验收项 1-3

**问题**: hard，Shotgun Surgery + Middle Man，且 PRD 自相矛盾。装饰物这一个功能被切给三个模块：`maps` = 「decoration CRUD + broadcast」、`realtime` = 「decoration live-sync」、`admin` = 「decoration edit endpoints」。给 decorations 加一个字段（比如 rotation）要改三处。更糟的是 `admin` 若真持有 decoration endpoints，它就只是把请求转给 maps 的 Middle Man——它自己没有装饰物领域逻辑。

**建议**: 在 PRD『Modules to build』里重写职责边界：`world` 模块（合并现 maps + npcs）独占 .tmj 加载、walkable 查询、decorations 表与其 REST handler、以及「写入后调用 realtime 的 `Room::broadcast(msg)`」；`realtime` 只提供 `broadcast(room_id, ServerMsg)` 这一个出口，不认识 decoration 概念；`admin` 缩成「一个 `require_admin` tower layer + members 的 3 个 handler」，明写「admin 不持有任何 decoration handler，装饰物路由挂在 world 上、套 require_admin layer」。

### PRD『Modules to build』`assets` 与 `generation` 两条 + Slice 3 (0004) 验收项 4「GET /api/library lists the member's assets + their job status」

**问题**: hard，职责重叠导致归属不明。`assets` 被定义为「blob store + metadata；library queries」，`generation` 被定义为「jobs 生命周期」。但 /api/library 的返回是 assets ⨯ generation_jobs 的联合视图：pending/failed 的条目根本还没有 assets 行（只有 job 行），done 的条目才有。当前定义下 agent 只能二选一：让 assets 模块去 SELECT generation_jobs（Feature Envy），或者两边各写一份状态映射（Duplicated Code）。

**建议**: 把 /api/library 明确划给 `generation` 模块，并规定它的数据形状：库列表 = `SELECT ... FROM generation_jobs LEFT JOIN assets ON assets.id = generation_jobs.result_asset_id WHERE generation_jobs.owner_id = ?`，即「job 是库条目的主键实体，asset 是它完成后的产物」。把 `assets` 模块降为一个无 handler 的内部 crate 模块：只暴露 `put(bytes, kind) -> AssetId` / `url_for(AssetId)`，不做任何列表查询。这样「素材库」只有一个所有者、一份状态映射。

### PRD『Schema』代码块全部 6 张表（users.role / avatars.kind / assets.kind / generation_jobs.kind / generation_jobs.status / decorations.z_layer）

**问题**: hard，Primitive Obsession + Mysterious Name。(1) 所有枚举列都是裸 TEXT 且**没有 CHECK 约束**——注意 v2 调研文档的 schema 是带 `CHECK (kind IN (...))` 的，PRD 抄写时把约束丢了，这是净退步：一次拼写错误（'sprite-sheet' vs 'sprite_sheet'）会静默写库。(2) `kind` 一个名字承载三套完全无关的词汇表（avatar 的 modular|generated、asset 的 sprite_sheet|accessory|decoration、job 的 avatar_generated|accessory），一个 agent 在代码里看到 `kind` 无法判断域。(3) back/hand slot 是核心领域概念，schema 里根本没有类型，只存活在 layers_json 的自由文本里。

**建议**: 三处改动写进 PRD schema 块：(a) 每个枚举列加 `CHECK (col IN (...))`，包括 users.role、avatars.kind、assets.kind、generation_jobs.kind、generation_jobs.status；(b) 改名消歧：`assets.kind` → `asset_kind`，`generation_jobs.kind` → `job_kind`，`avatars.kind` 保留；(c) 在 PRD 新增一小节「领域类型（Rust 侧）」，规定 `AvatarKind` / `AssetKind` / `JobKind` / `JobStatus` / `Slot{Back,Hand}` / `Direction`(8 变体) 全部是 `#[derive(sqlx::Type, Serialize, Deserialize)] #[sqlx(rename_all="snake_case")]` 的 enum，禁止在 handler 签名或 struct 字段里出现 `role: String` / `kind: String` / `slot: String`。

### PRD『Schema』`avatars(... is_active INTEGER ...)` + Slice 2 (0003) 验收项 2「GET /api/avatar returns the member's active avatar」+ Slice 4 (0005) 验收项 3「set a generated avatar active」

**问题**: hard。`is_active` 引入了一个 schema 无法表达、PRD 也没提的不变式：「每个 owner 至多一行 is_active=1」。没有 partial unique index，两次并发 activate 就能产生两个 active avatar，而 realtime snapshot 取的是「the active avatar」——结果不确定。而且 User Story 里没有任何需求要求「保留一个形象集合并切换」，只要求「我当前长这样」。v2 调研文档用的是 `users.avatar_id` FK，反而更简单。

**建议**: 二选一并写进 PRD：**(推荐)** 删掉 `avatars.is_active`，改在 `users` 加 `active_avatar_id TEXT REFERENCES avatars(id) ON DELETE SET NULL`——单一真相源，无不变式可破，activate = 一条 UPDATE users；或者保留 is_active 但在 schema 块里补上 `CREATE UNIQUE INDEX idx_avatars_one_active ON avatars(owner_id) WHERE is_active = 1;` 并在 Slice 2 验收项里加一条并发 activate 的测试。

### Slice 5 (0006) 验收项 1「collision/walkable layers loaded by both server (movement clamp) and client (rendering)」

**问题**: hard，Duplicated Code 跨语言版。「哪些格子可走」这条规则会有两个实现（Rust 解析 .tmj + TS/Phaser 解析 .tmj），两者一旦对 Tiled 的图层语义理解有微小差异（是看 collision 图层的 gid≠0，还是看 tile property `walkable=false`，边界格算不算），就表现为服务端 clamp 和客户端预测不一致 → 玩家橡皮筋。这个 bug 在 30-50 人房里最难查。

**建议**: 把「可走性」的唯一解析者定为服务端：服务端 boot 时解析 .tmj，暴露 `GET /api/scenes/:id` 返回 `{width, height, walkable: <行主序 bitmap 或 base64 位图>}`；客户端只用 Phaser 的 Tiled loader 渲染**视觉图层**，碰撞判定一律读这个 bitmap，绝不自己解析 collision 图层。改写该验收项为「服务端是 walkable 的唯一解析者；客户端从 GET /api/scenes/:id 取 bitmap；集成测试断言同一坐标在服务端 clamp 与 bitmap 中判定一致」。

## MINOR

### PRD『Bartender NPCs』+『Schema』`npcs(id, scene, npc_def_id, x, y)` + Slice 5 (0006) 验收项 3

**问题**: Speculative Generality。规格是「两个 NPC，固定位置，脚本台词，无运行时编辑」（锁定决策 3）。为两行永不变更的数据建一张表 + 一次 migration + 一套读取路径，而底图本身却是静态 .tmj 文件——同样静态的东西用了两种截然不同的存储机制。`npc_def_id` 这个间接层指向的 npc_def 又不在任何表里（PRD 说是 JSON），于是形成 DB → 文件的跨介质引用。

**建议**: 删掉 `npcs` 表。NPC 定义与位置一起放进随 .tmj 一同 ship 的静态资源 `assets/scenes/bar/npcs.json`：`[{id, sprite, tile_x, tile_y, dialogue:[...]}]`，boot 时读进内存 `HashMap<NpcId, NpcDef>`。少一张表、少一次 migration、少一层 npc_def_id 间接。若哪天要运行时编辑 NPC，那时再加表——那是加表，不是重写。

### PRD『Bartender NPCs』「dialogue tree as JSON」/『Modules to build』「`npcs` - scripted dialogue tree engine」+ Slice 5 (0006) 验收项 4

**问题**: Speculative Generality（命名驱动的过度设计）。实际需求是：靠近 + 按键 → 出一句话 → 某句话能掀开菜单。把它命名为「dialogue tree engine」会让 agent 造出节点图解释器：条件跳转、变量、访问过标记、玩家选项分支——规格里一个都没要。

**建议**: 把『Modules to build』里的「scripted dialogue tree engine」改成「npc dialogue lookup」，并在 PRD 里把对话数据结构钉死为最小形状：`dialogue: Vec<Node>`，`Node { text: String, opens_menu: bool }`，交互时按顺序推进、到末尾回到第 0 个。明写「MVP 不支持条件分支、变量、玩家选项；需要时再扩，不预留钩子」。Slice 5 验收项 4 相应改为「interact 返回当前 node 的 {text, opens_menu}」。

### PRD『Avatar dual pipeline』+『Schema』`avatars.layers_json` + Slice 3 (0004) 验收项 5「stored in the avatar's `layers_json` equipped list」

**问题**: Mysterious Name。列名叫 `layers_json`，实际装两个不同概念：预置部件组合（base/hair/outfit + 颜色）**和** equipped 配件引用列表（含 slot）。PRD 自己的措辞就是「stores a `layers` JSON describing preset base parts **and** an `equipped` list」。后续 agent grep `layers` 找装备逻辑会找不到，或反过来把配件当图层塞进部件数组。

**建议**: 列改名为 `modular_config_json`，并在 PRD 里给出对应的 Rust 类型：`ModularConfig { base: BaseParts, equipped: Vec<Equipped { asset_id: AssetId, slot: Slot }> }`。同步改 Slice 2 验收项 1、Slice 2 验收项 2（PUT 更新 modular_config_json）、Slice 3 验收项 5 的措辞。

### PRD『Avatar dual pipeline』 + 隐含于 Slice 2 验收项 3/4、Slice 3 验收项 5、Slice 4 验收项 3/4

**问题**: Repeated Switches。`avatars.kind` 会在至少 5 处被 match：渲染路径选择、PUT /api/avatar 校验、equip 拒绝 400、realtime snapshot 序列化、library 列表。锁定决策 5 说「两条管线同动作动画规范」，正是为了让渲染端不必知道 kind——但 PRD 没把这个契约写下来，agent 大概率会把 kind 一路透到 Phaser 场景里再 switch。

**建议**: 在『Avatar dual pipeline』末尾补一段契约：「`kind` 只在两处被 match —— (1) DB 读出时反序列化为 `enum Avatar { Modular(ModularConfig), Generated { sprite_asset_id: AssetId } }`；(2) 构建 realtime snapshot 时解析为渲染契约 `AvatarView { sheet_url: String, overlays: Vec<OverlayRef> }`（generated 的 overlays 恒为空）。前端 net/game-state/phaser-scene 三层都只见 AvatarView，代码中不出现 kind 字段。」这条一句话能省掉后面三个切片的分叉判断。

### PRD『Realtime room model』`players: Map<id,{x,y,dir,...}>`、『Schema』`decorations(scene, tile_x, tile_y)` / `npcs(scene, x, y)`、Slice 1 验收项 4 `{move, tx, ty}`

**问题**: Data Clumps + Primitive Obsession。`(scene, x, y)` 三元组在 decorations / npcs / player state / move 意图 / walkable clamp 里反复同现，`dir` 没有类型（极易变成裸字符串 "n"/"ne"，前后端各拼一套）。而且 move 用 `tx, ty`、decorations 用 `tile_x, tile_y`、npcs 用 `x, y`——同一个概念三种拼写。

**建议**: PRD 里定义两个共享类型并统一全部字段名：`TilePos { scene: SceneId, x: u16, y: u16 }`、`Direction` 8 变体 enum（线上表示为 `"s"|"se"|...` 固定小写缩写）。所有 DB 列统一 `tile_x`/`tile_y`，所有线上消息统一 `{ "pos": {"x":..,"y":..} }`。前端在 net 模块里镜像同名 TS 类型，作为 Slice 1 的契约测试对象。

### PRD『Further Notes』第一条 sustainability guardrails

**问题**: judgement，但这条正是 Speculative Generality 的制度化。「every slice must keep the PixelLab dependency behind the trait, the asset store behind object_store, and the frontend logic out of Phaser - so later slices (R2 migration, a second scene, LLM NPC upgrade, post-processing) are additive」——它为四个明确 Out of Scope 的未来（R2、第二场景、LLM NPC、后处理）给每一个切片加了持续性负担，与紧邻下一条的「Lightweight mandate」直接张力。

**建议**: 缩到一条真正有当下回报的：「PixelLabClient trait —— 唯一理由是让集成测试完全离线、零成本（Testing Decisions 已依赖它）」。其余三项改写为不产生验收压力的表述：object_store 降为「main.rs 之外不出现具体 store 类型」（见另一条 finding），「frontend logic out of Phaser」降为「net / game-state 写成不 import phaser 的纯 TS 模块」——这是可 grep 的具体约束，而不是「为将来的 post-processing 保持解耦」这种会催生额外抽象层的目标。删掉 LLM NPC / 第二场景 / R2 作为设计驱动力的提法。

### Slice 2 (0003) 验收项 3「recoloring via HSL shift on recolorable layers」+ PRD User Story 7

**问题**: judgement，但这是本设计里最贵的一处「小功能」。运行时 HSL 位移要么写 WebGL shader、要么逐帧 canvas 像素操作（8 方向 × N 帧 × 每次换色），且对像素画效果很差——GBA 风格的色块会因色相位移变糊、描边跟着变色。它服务的 Story 7 只要求「同轮廓的两个人看起来不同」。

**建议**: 把该验收项换成零代码方案：每个可换色部件预出 N 套调色板变体作为独立 PNG（如 hair_01_brown.png / hair_01_blond.png ...，6 色档）。选色 = 换贴图 key，前端一行代码，无 shader、无 canvas 像素操作、美术效果可控（也正是 GBA 时代的做法）。Story 7 完全满足。若坚持要连续调色，至少限定为 Phaser 的 `setTint` 单色乘法（一行），并接受它只对灰度底图有效。

### Slice 7 (0008) frontmatter `blocked_by: [7]` + PRD 依赖图 `6(admin) -> 7(deploy)`

**问题**: judgement。部署配置（Caddyfile / systemd unit / env 配置 / tracing / 备份脚本）与 admin 功能零耦合，却被排在最后、被整个功能集阻塞。结果是在完成 6 个切片之前拿不到任何真实用户反馈——而 v2 文档 §三阶段 1 的整个设计意图恰恰相反（Cloudflare Tunnel 早期分享给朋友测试）。对一人 vibe coding 项目，这是最贵的一种排序错误：反馈延迟到最后。

**建议**: 把 Slice 7 的 blocked_by 从 [7] 改为 [2]（只需要「一个能跑的二进制」）。它随时可以插队做完，之后每个切片都能直接推上线。同时在 Slice 1 验收项里加一行零成本的「`cloudflared tunnel --url http://localhost:8080` 可分享给朋友测试」——这不是新工作，是一条命令。

### Slice 1 (0002) 验收项 3「First registered user (or an ADMIN_USERNAME env bootstrap) is promoted to admin role」

**问题**: judgement，Speculative Generality 落在最关键路径上。Slice 1 里没有任何东西消费 `role`——第一个 role-gated 端点在 Slice 6。现在就实现「首个用户提权 或 env 变量提权」两条分支（含两者冲突时怎么办、env 里的用户还没注册时怎么办这类边界），是在 tracer bullet 里加一个当下无人观察的行为。

**建议**: Slice 1 只保留 schema：`users.role TEXT NOT NULL DEFAULT 'member' CHECK (role IN ('member','admin'))`。把提权逻辑（含 ADMIN_USERNAME env 分支和它的边界）整条移到 Slice 6 验收项——那里刚好需要一个 admin 才能测 403，动机真实。PRD『Further Notes』的 Bootstrap admin 条目相应改指 Slice 6。

## NIT

### PRD『Modules to build (seams respected)』8 个后端模块

**问题**: judgement。auth / assets / generation / avatars / realtime / maps / npcs / admin 对一人 MVP 偏细。结合前面几条（admin 应只剩一个 layer + members handler、npcs 应是静态配置、assets 应无 handler），照单执行会产出若干近乎空壳的 mod.rs，其中 admin 是 Middle Man、assets 是薄包装。

**建议**: 把该清单改写成 5 个模块的目标文件布局并注明合并关系：`auth`、`avatars`（含 presets 与合成配置）、`generation`（吞掉 assets 的 store 包装与 library 查询）、`realtime`（房间 + 聊天 + broadcast 出口）、`world`（.tmj + walkable + decorations + npc defs）；`admin` 不作为模块存在，只是 `middleware::require_admin` + `auth::members` 里的 3 个 handler。这样 agent 不会先建 8 个目录再往里找东西填。

### Slice 2 (0003) 验收项 2「GET /api/avatar returns the member's active avatar (or assigns/returns a default on first call)」

**问题**: GET 带写副作用（首次调用会 INSERT 并绑定默认形象），违反读写直觉，也让「GET 是幂等的」这个假设失效——重试、预取、双标签页同时首登都会踩到。

**建议**: 把默认形象的创建挪到 register handler 里（注册事务内一条 INSERT + 设 active），GET /api/avatar 保持纯读、必然有结果。验收项改为「注册后 GET /api/avatar 立即返回默认形象；GET 无写副作用」。

### PRD『Child issues』末行依赖图「1(skeleton) -> {2(modular), 5(scenes)}; 2 -> 3 -> 4; 3 + 5 -> 6 -> 7」

**问题**: Mysterious Name。这行用的是 slice 序号，上面的 child issue 列表和各文件 frontmatter 用的是 issue id（相差 1）。于是「2」在同一份文档里既指 issue #2（skeleton）又指 slice 2（modular）。AFK agent 解析依赖时容易错位一格。

**建议**: 依赖图统一改用 issue id 并标注 slice 名：`#2(slice1 skeleton) -> {#3(slice2 modular), #6(slice5 scenes)}; #3 -> #4 -> #5; #4 + #6 -> #7 -> #8`。或直接删掉这行——每个 issue 的 frontmatter `blocked_by` 已是唯一真相源，这行是它的分歧副本。
