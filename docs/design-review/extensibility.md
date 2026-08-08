# 设计评审 — 可扩展性 (6 个增长点)

**裁定**: `fix-first`

**摘要**: 从可扩展性视角评审：设计的宏观判断是对的（trait 隔离生成商、object_store 抽象、前端逻辑出 Phaser、HTTP+WS 作为主测试缝），六个增长点里有 4 个方向正确但缺关键细节，2 个（场景扩展、供应商替换）目前实际上不是加法。

两个 blocker 都是文档级修补、成本很低但影响面横跨多个切片：(1) realtime PlayerState 与 snapshot/delta 协议里没有 `scene` 字段，而 Slice 5 要两个场景、decorations/npcs 表都已带 scene 列——不补的话服务端 clamp 无从选 collision layer、跨场景玩家互相串台、装饰广播无法按场景过滤，加第三个场景要求协议破坏性变更；(2) 被 3 个切片当验收基准的 "canonical sprite-sheet contract" 从未定义（帧尺寸/网格/方向序/动画集/配件 anchor），它同时是管线 A、管线 B、配件、未来换供应商的唯一互操作面，且必须按 fact layer 已确认的 PixelLab 约束（keep_first_frame 多一帧、v3 默认只做 south、画布自动 2x padding）来选。

主要 major：`/ws/room` 字面路由丢掉 room id（同进程多房间分片本来是白送的）；`generation_jobs` 缺 `provider`/`provider_ref`/`cost_usd`，而真实流程会产出必须落库的 character_id 与 per-direction job ids；生成 trait 被 PixelLab 动词塑形 + stub 同步 resolve，导致没有任何测试覆盖 pending→轮询→done 的时间线（真实是 5-9 分钟、2 调用 2 轮询、每方向独立 job），换供应商时最易不兼容的正是这个形状；PRD 引用 MCP 工具名作为集成面，与 docs/reference/pixellab-api.md §六 及 CONVENTIONS.md 的事实层规则直接冲突；资产只抽象了写侧、没有 URL 解析缝（照抄调研里的 ServeDir 范式会把本地路径焊进前端），R2 迁移会退化成"用 Rust 进程代理全部字节"；WS 协议没有版本号也没有单一事实来源，而测试直接断言 JSON 帧；场景 portal/spawn 若写死在代码里则加场景不是加法；NPC interact 是隐式请求-响应、无 conversation id、无 pending 态、npc_def 无 behavior 判别式，LLM 升级必然改协议；渲染路径没规定升采样发生在哪一层，CRT 后处理需要"先渲进 240×160 目标再单点升采样"，现在决定成本≈0、之后是渲染重构；broadcast `Lagged` 无恢复语义（delta 方案的必答题）与空房间不回收，两者都是调研文档已点出的坑却没进任何 AC。

建议在 Slice 1/2 开工前把上述改动折进 PRD 与切片 AC——全部是字段、列、一个函数或一条纪律级别的补充，没有一项需要新增依赖或违背"轻量优先"与单二进制的锁定决策。

共 16 条发现。

---

## BLOCKER

### PRD #1 §Implementation Decisions › Realtime room model（`players: Map<id, {x, y, dir, avatar_snapshot, name}>`）+ Slice 1 (#2) AC 第 4 条

**问题**: PlayerState 与 delta snapshot 协议里没有 `scene` 字段，但 Slice 5 (#6) 要求 bar/yard 两个场景 + 场景切换，且 `decorations`/`npcs` 表都已经有 `scene` 列。这意味着：(a) 服务端 clamp 时不知道该用哪张 collision layer；(b) 在 yard 的玩家会被广播进 bar 客户端（坐标空间不同，渲染成鬼影）；(c) `decoration_added` 广播也无法按场景过滤。增长点 3（加第三个场景）因此不是加法——它要求改 snapshot/delta 协议、改客户端解析、改所有 Slice 1/2 建立的 WS 断言，三个切片同时返工。

**建议**: 在 Slice 1 就把 `scene: String` 放进 PlayerState、full snapshot、delta、以及 `{move}` 的服务端处理里，MVP 只允许一个值 `"placeholder"`。10Hz tick 按 scene 分组打包，只向订阅该 scene 的连接发送。同时把 `{type:"scene_changed", scene, x, y}` 定为一条协议消息（Slice 1 可以只有一个 scene，永不触发）。Slice 1 成本：一个字段 + 一次 groupby + 一条消息类型；Slice 5 之后再补成本：协议破坏性变更。

### PRD #1 §Avatar dual pipeline（"same canonical sprite-sheet contract - 8 directions × the same animation set (idle, walk, …) on the same grid layout"）；Slice 2 (#3) AC 第 3 条、Slice 3 (#4) AC 第 5 条、Slice 4 (#5) AC 第 1 条

**问题**: "canonical sprite-sheet contract" 在 PRD 里被引用 4 次、被 3 个切片当作验收基准，但从未被定义：帧尺寸、网格布局、方向枚举顺序、动画名集合、每动画帧数、accessory 的 per-slot anchor 坐标系，全部缺失。这个契约是四个东西的唯一互操作面——管线 A（模块化合成）、管线 B（PixelLab 生成）、配件叠加、以及未来换生成商/本地模型。没有书面契约 + 校验器，增长点 2 根本无法验证"新供应商产出合规"，只能靠肉眼看图；同时 Slice 2/3/4 三个切片会各自假设一套布局，返工面覆盖渲染器 + 合成器 + 生成 worker。另外，fact layer 已确认 PixelLab 的真实约束会直接约束这个契约（`keep_first_frame=true` 时 `frame_count=8` 实际产出 9 帧；v3 模式动画默认只做 south，8 方向必须显式传；最终画布自动 padding 约 2 倍、上限 256），契约必须按这些事实选，不能事后适配。

**建议**: 在 Slice 2 开工前，把契约写成一份可执行规格（`docs/reference/sprite-sheet-contract.md` 或 PRD 一个新小节）：frame w×h、每行一个方向、方向顺序固定为 `[south, south-west, west, north-west, north, north-east, east, south-east]`、动画名枚举 + 各自帧数、sheet 总尺寸公式、accessory 每 slot 的 anchor 是 per-direction-per-frame 的 (dx,dy) 表。然后在 Slice 2 加一个纯函数校验器 `validate_sheet(bytes, meta) -> Result<()>`，Slice 3/4 的 worker 落库前必须过它。这一个校验器就是增长点 2 的回归网。

## MAJOR

### Slice 1 (#2) AC 第 4 条 `WS /ws/room`；PRD #1 §Realtime（对比 docs/pixel-mosaic-game-workflow-v2-rust.md §二.2 用的是 `/ws/:room` + `DashMap<RoomId, Room>`）

**问题**: PRD 把调研文档里带 room 参数的路由收窄成了字面量 `/ws/room`，路径里没有 room 标识。增长点 6 里最便宜的那一档水平扩展（同一进程内多房间分片，30-50 CCU 一房）因此需要同时改路由、改客户端 URL、改 `Rooms` 的 keying；增长点 3 如果最终决定用"一场景一房间"也一样。这是一个纯粹白送的耦合点。

**建议**: Slice 1 就用 `GET /ws/room/:room_id`，服务端保留 `Rooms: DashMap<RoomId, RoomHandle>`，MVP 只接受 `default`（其余返回 404），客户端 URL 从配置里拼。成本约等于零，省掉后续一次前后端同步改动。

### PRD #1 §Schema › `generation_jobs(...)` 与 `assets(...)`；Slice 3 (#4) AC 第 2 条

**问题**: `generation_jobs` 只有 `params_json / result_asset_id / error`，没有 `provider` 和供应商侧句柄。而 fact layer（docs/reference/pixellab-api.md §三、§十一）确认真实流程会产出必须持久化的供应商态：`character_id`（PixelLab 侧 UUID）、`background_job_ids`（每方向一个）。增长点 2（换供应商/本地模型）因此不是加法：换了之后无法分辨历史 job/asset 出自谁、无法恢复进程重启时在飞的任务、无法灰度并行两家。`assets` 同样缺 provider 溯源，未来"把某供应商产出的素材全部重生成"这类操作无从下手。

**建议**: Slice 3 的 migration 里就加：`generation_jobs` 增 `provider TEXT NOT NULL DEFAULT 'pixellab'`、`provider_ref TEXT NULL`（存 character_id / job_ids JSON）、`cost_usd REAL NULL`（fact layer §十一.7 要求的成本护栏）；`assets` 增 `provider TEXT NULL`。四个列，一次 migration，零业务逻辑改动。

### PRD #1 §Implementation Decisions › Generation（"`create_character`, `create_image_pixflux`, `create_map_object`, `animate_character`"）；Slice 3 (#4) AC 第 1 条（"stub returns canned sprite bytes and resolves synchronously"）

**问题**: 两个问题叠加，共同破坏增长点 2。(1) trait 的形状是供应商动词：PRD 直接点名 PixelLab 的四个工具名当接口，一旦 `PixelLabClient` 按这四个方法建模，换供应商就是重塑 trait + 改所有调用点，而不是加一个 impl。(2) stub "同步 resolve" 让 trait 只需要一个阻塞式 `generate() -> Bytes` 就能通过全部测试，但真实供应商是 5-9 分钟、2 次调用 + 2 轮轮询、每方向一个独立 job（fact layer §一.3、§三、§四步骤 2）。于是没有任何测试会覆盖 pending→轮询→done 的时间线、部分方向失败、进程重启后续跑——而这恰恰是换供应商时形状最容易不兼容的地方。

**建议**: 把 trait 定成领域动词 + submit/poll 两段式：`trait SpriteProvider { async fn submit(&self, req: SpriteRequest) -> Result<ProviderRef>; async fn poll(&self, r: &ProviderRef) -> Result<JobOutcome /* Pending{done_of_total} | Done(Vec<DirectionSheet>) | Failed(String) */>; }`。`ProviderRef` 落 `provider_ref` 列，worker 变成"读 pending 行 → poll → 推进"的可重启循环。stub 改成可编程的（内部一个队列 + 手动 `advance()`），让集成测试能断言 pending 状态、部分完成、失败重试。命名也别叫 `PixelLabClient`。

### PRD #1 §Implementation Decisions › Generation："PixelLab.ai via its official MCP/API (`create_character`, `create_image_pixflux`, `create_map_object`, `animate_character`)"

**问题**: 这一行与已核实的事实层直接冲突，且违反 docs/CONVENTIONS.md §一 的"调研层结论不允许被当成事实引用"。docs/reference/pixellab-api.md §一.2 与 §六 明确：MCP 是开发期工具，用户侧生成**必须**走 REST v2；§九 进一步说明 MCP 的参数名与 REST 不同（`size` vs `image_size`、`n_directions`、`proportions` 为 JSON 字符串），"不要把 MCP 的参数名照搬到 REST 调用"。PRD 列的这四个名字全是 MCP 工具名，对应的 REST 端点其实叫 `create-character-with-4-directions` / `create-character-v3` / `map-objects` / `animate-character`。从可扩展性看，照着 MCP 的工具形状去实现真实 impl，会把"供应商 + 传输方式"两层都焊进 trait，增长点 2 的成本从"加一个 impl"涨到"重写一层"。

**建议**: 把这行改成："PixelLab REST v2（`https://api.pixellab.ai/v2`，Bearer token 仅存后端），端点见 docs/reference/pixellab-api.md §四/§五；MCP 仅用于开发期在编辑器内出素材，不进产品代码路径。" 并在 Slice 3/4 的 AC 里点明真实 impl 用 `reqwest` 打 v2（fact layer §十.3 已排除 JS SDK）。

### PRD #1 §Implementation Decisions › Storage；Slice 3 (#4) AC 最后一条；对照 docs/pixel-mosaic-game-workflow-v2-rust.md §二.1 的 `.nest_service("/assets", ServeDir::new("./data/assets"))` + `public_base_url`

**问题**: 设计只保证了"写"这一侧的抽象（`Arc<dyn ObjectStore>`），但"读"这一侧完全没有缝：前端拿到的素材 URL 从哪来、长什么样，没有任何一处定义。调研骨架给的范式是直接 `ServeDir` 本地目录，如果照抄，`/assets/<path>` 这个 URL 形状就会散落进前端、`meta_json`、以及 Phaser 的 loader 里。切到 R2 时只剩两条差路：把所有素材字节从 Rust 进程代理转发（R2 的意义全没了），或者改成签名 URL——那就要同时改前端和已入库的引用。增长点 1 于是不是"换一行 builder"。

**建议**: Slice 3 引入一个 `AssetUrls` seam：`fn url_for(&self, storage_key: &str) -> String`，本地实现返回 `{PUBLIC_ASSET_BASE}/{key}`，R2 实现返回 CDN/签名 URL；前端只从 API 响应里读完整 URL，绝不自己拼路径。同时明确 `assets.storage_key` 永远是 store 相对 key（不含 scheme、不含 `./data`、不是 URL）。Slice 3 已有的"第二个 store impl in tests"要把 `url_for` 也覆盖进去，否则那条 AC 只验证了一半。

### PRD #1 §Testing Decisions（"assert on JSON / WS frames"）；Slice 1 (#2) AC 第 7 条

**问题**: 六个增长点里有四个（场景、LLM NPC、装饰同步、分片）都要动 WS 协议，但协议本身没有单一事实来源（Rust 侧 serde 类型与前端 TS 类型各写一份，靠人对齐），也没有版本号。加上测试直接断言 JSON 帧，等于把线格式钉进测试——调研文档 §二.2 明确规划了"JSON 起步、之后换 MessagePack"，那次切换会一次性打爆全部前后端断言。另外浏览器会缓存旧前端，服务端协议演进后没有任何握手能识别版本不匹配。

**建议**: Slice 1 做三件小事：(1) full snapshot / hello 帧里加 `protocol_version: 1`，客户端不匹配就提示刷新；(2) 协议消息的判别式与字段写进一份 `docs/reference/ws-protocol.md`（或用 `schemars` 从 Rust 类型导出 JSON Schema，前端 TS 类型由它生成），作为 Rust 与 TS 共同的基准；(3) 前端 `net` 模块里隔出一个 codec（`encode`/`decode`），单测断言解码后的对象而非原始帧字符串。

### Slice 5 (#6) AC 第 2 条："walking through the bar door moves the player between bar and yard scenes"

**问题**: 场景切换的触发条件与目标落点没有说明来源。如果门的位置、目标场景、落地坐标是写在 Rust/TS 代码里的（最自然的实现），那增长点 3（加第二个场景/房间）就必须改后端代码 + 前端代码 + 重新部署二进制，不是加法。同理，新场景的出生点也无处声明。

**建议**: Slice 5 就规定 portal 与 spawn point 从 Tiled 的 object layer 读：`.tmj` 里放 `portals`（矩形 + 属性 `to_scene`、`to_x`、`to_y`）和 `spawns`（属性 `name`）两个 object layer，服务端启动时解析成场景注册表。加一个场景 = 丢一个 `.tmj` 进 assets + 一行注册。同时把 collision 来源也做成 seam（Slice 1 的 clamp 依赖一个 `WalkableMap` 结构，Slice 1 用硬编码矩形构造它，Slice 5 换成从 `.tmj` 构造），避免 Slice 5 重写 clamp。

### PRD #1 §Implementation Decisions › Bartender NPCs（"client sends {type:"interact", target: npc_id} -> server returns the current dialogue node"）；Slice 5 (#6) AC 第 4 条；PRD §Schema（有 `npcs(... npc_def_id ...)` 但没有 `npc_defs` 表）

**问题**: 三处让增长点 4 变成返工而非加法：(1) 协议是隐式请求-响应"interact 之后紧接着来的那帧就是我的对白"，没有 correlation id、没有 conversation id、没有"正在思考"状态。LLM NPC 是 0.5-5 秒延迟 + 可能流式 + 可能超时不回，届时必须改协议和客户端渲染逻辑。(2) "the current dialogue node" 暗示服务端持有每玩家会话态，但 PRD 没有定义它存在哪、何时过期——LLM 需要的正是这个会话态。(3) `npc_def` 只有"dialogue tree as JSON"一种形状，没有行为判别式，也没有对应的表/文件位置定义，加 LLM 行为等于改数据模型。

**建议**: Slice 5 把对白回复定成独立消息类型而非顺序耦合的响应：`{type:"dialogue", conversation_id, npc_id, node_id, text, choices, pending: bool, done: bool}`，客户端按 conversation_id 路由、允许同一 conversation 收到多帧（先 `pending:true` 再补文本）。服务端保留一个 `DashMap<(player_id, npc_id), Conversation>` 带 TTL。`npc_defs` 落表（或明确落文件）并带 `behavior TEXT NOT NULL DEFAULT 'scripted'`。这三样在 Slice 5 是几十行，LLM 升级就变成加一个 behavior 分支。

### PRD #1 §Stack › Frontend（"Logical resolution 240×160, integer-scaled, imageSmoothingEnabled=false"）；Slice 1 (#2) AC 第 6 条；§Out of Scope（"CRT/LCD post-processing shaders, BGM/SFX"）

**问题**: 增长点 5 的加法性取决于渲染路径结构，而 Slice 1 只说了"整数缩放"没说缩放发生在哪一层。如果 Slice 1 直接让 Phaser Scale Manager 对主 canvas 做整数 zoom、并把摇杆/聊天/库/形象编辑器都画在 Phaser 场景里，那后加 CRT 会遇到两个真问题：(a) 扫描线/曲面畸变必须跑在**升采样之后**的目标上，240×160 上打 shader 出不来效果，届时要把世界改成先渲进一个 240×160 的 RenderTexture 再单独升采样——那是渲染路径重构；(b) shader 会连带扭曲 UI 与聊天文字。BGM 另有一个小坑：移动端音频需要用户手势解锁，若登录后自动进房、没有任何"点击进入"节点，后加 BGM 就得补一个门。

**建议**: Slice 1 就固定：世界渲染进单个 240×160 的 RenderTexture / 单摄像机，升采样只在**一处**发生（写进 AC 与注释："the one place scaling happens"）；摇杆、按钮、聊天面板、库、形象编辑器全部走 DOM 覆盖层，不进 Phaser 场景；确认用 WebGL renderer 而非 Canvas。另外保留一个显式的"tap to enter"过场（顺带作为将来 BGM 的 audio unlock 手势）。这三条现在决定成本≈0。

### Slice 1 (#2) AC 第 4/5 条（10Hz delta + broadcast）；对照 docs/pixel-mosaic-game-workflow-v2-rust.md §七 阶段 2 踩坑预警（`RecvError::Lagged`、空房间 DashMap 不回收）

**问题**: 调研文档已经点出的两个坑没进任何切片的验收标准：(1) `broadcast::channel` 满时慢客户端拿到 `Lagged`，如果只是忽略，该客户端从此静默丢帧、状态永久漂移——这在 30-50 CCU 下偶发，在增长点 6 的更高并发下变成主要故障模式，且表现为"某些人看别人卡住"这种极难归因的 bug；(2) 空房间的 DashMap entry 与 tick task 不回收，多房间分片后会持续泄漏。锁定决策 1 明确要求 10Hz delta + 插值，那么"delta 丢失如何恢复"是这个方案的必答题，不是可选优化。

**建议**: Slice 1 加两条 AC：收到 `RecvError::Lagged(n)` 时立即给该连接重发一次 full snapshot（delta 协议的恢复语义写进 ws-protocol 文档），并加一个集成测试用小 channel 容量 + 故意不读的客户端来触发它；房间最后一个连接断开时回收 entry 并 abort tick task。两处都是 Slice 1 的十几行。

## MINOR

### PRD #1 用户故事 18 / §Avatar dual pipeline（8 directions）；对照 docs/reference/pixellab-api.md §七"端到端单角色成本估算"（该表按 **4 方向** 计算）

**问题**: PRD 把 8 方向定为 canonical 契约，但事实层的成本估算全部建立在 4 方向上，且动画**按方向计费**：8 方向 v3 4 帧 @64px = 8 × $0.0129 ≈ $0.103，路径 B 端到端从 ~$0.19-0.30 涨到 ~$0.24-0.40。另外 v3 模式动画默认只做 south，8 方向必须显式传全部 8 个方向。模块化管线那侧代价更实在：预置部件（发型/服装/配件）每一件都要画 8 方向 × 全动画帧，是 4 方向的 2 倍美术量——对"solo builder with limited art"这个前提是个未被说明的长期负担，而且它会直接决定以后加新配件/新动作的边际成本。

**建议**: 要么在 PRD 里显式承认并接受这个成本（写一行：8 方向使每角色动画成本约翻倍、每个预置部件美术量翻倍），要么把 canonical 契约定为 4 方向 + 水平镜像出斜向（方向枚举与 sheet 布局仍按 8 方向定义，只是斜向由镜像填充），后者让"以后升级到真 8 方向"仍是纯数据替换。无论选哪个，都要写进第 2 条那份 sprite-sheet 契约里。

### PRD #1 §Schema › `assets(id, owner_id, kind, storage_key, meta_json, created_at)`

**问题**: 缺 `content_type`、`byte_size`、内容哈希，也没有"key 不可变"的约定。这三样都是增长点 1 的直接依赖：R2/S3 的 PUT 需要显式 content-type，否则对象以 `application/octet-stream` 返回、浏览器不当图片处理；迁移时没有 size/哈希就无法校验"本地 N 个对象是否完整搬到 R2"；如果 key 是可变的（例如 `avatars/{user_id}/current.png`），套 CDN 后缓存永远不对，只能靠短 TTL 削掉 R2 的收益。

**建议**: Slice 3 的 `assets` 加 `content_type TEXT NOT NULL`、`byte_size INTEGER NOT NULL`、`sha256 TEXT NULL`，并在 PRD 里写死一条：storage_key 一律 UUID/内容寻址、写入后不可变，覆盖即新 key + 更新引用。这样 R2 迁移可以做成"复制 + 逐对象哈希校验"的离线脚本，且素材可直接 `Cache-Control: immutable`。

### PRD #1 §Map & decorations（"the server broadcasts {decoration_added}|{decoration_removed} over WS"）；Slice 6 (#7) AC 第 2 条

**问题**: 装饰物的权威态在 DB（跨进程可共享，好），但通知只走进程内 `tokio::broadcast`。增长点 6 走到多进程那一档时，A 进程上管理员摆的椅子不会推给 B 进程的玩家——而且失败方式是静默的（DB 有、部分在线玩家看不见，刷新才出现），排查成本远高于修复成本。

**建议**: Slice 6 把所有装饰变更收敛到一个 `publish_room_event(scene, event)` 函数（唯一的广播出口），MVP 实现就是进程内 broadcast。将来换成跨进程总线（或最朴素的"DB 事件表 + 各进程轮询"）只需替换这一个函数体。现在只是"别在 handler 里直接 `room.tx.send()`"这一条纪律。

## NIT

### Slice 1 (#2) AC 全部；对照 docs/pixel-mosaic-game-workflow-v2-rust.md §六 Rust 硬约束 2 与 9

**问题**: Slice 2/3/6 都以"新加 migration"的方式做加法演进（这本身是对的），但 Slice 1 的验收标准里没有"建立 `migrations/` 目录 + 提交 `.sqlx` 离线查询数据"这一项。硬约束 2 要求 schema 变更后必须 `cargo sqlx prepare` 并提交 `.sqlx`；如果 Slice 1 没把这条流程连同 CI 一起立起来，后续每个加表的切片都会在"离线/CI 构建时 sqlx 宏找不到表"上各摔一次。

**建议**: Slice 1 加一条 AC：`migrations/0001_init.sql` + 提交 `.sqlx` 目录 + 一个 `justfile`/`Makefile` 目标封装 `sqlx migrate run` 与 `sqlx prepare`，并在测试里用 `sqlx::migrate!()` 建库；README 写一行"改 schema 后必须跑 prepare"。
