# 设计评审 — 规格一致性

**裁定**: `fix-first`

**摘要**: PRD 与 7 个切片对 9 条锁定决策的捕获率约 7/9：决策 6（用户名密码）、7（移动优先双端）、8（内存聊天不落库）、9（Rust/Axum/SQLite 单二进制）被忠实且可验收地写下来了；决策 1（10Hz delta + 插值）、4（异步任务 + 素材库）、5（双管线不互通）方向正确但验收项不足或只落实一半；决策 2（静态底图 + DB 装饰物）和 3（脚本 NPC）存在会导致返工的结构性缺口。三个 blocker：(1) PRD §Realtime 写死「单房、player state 无 scene」，但切片 5 要两个场景 + 切换，decorations 又带 scene 列——切片 1 会按无 scene 实现 WS 协议，切片 5 必须改最底层 seam；(2) PRD §Stack 把 PixelLab 的 **MCP 工具名**（create_character 等）当接口写进 PRD 和切片 3/4 验收项，而 docs/reference/pixellab-api.md 已核实「MCP 不可用于用户侧、参数名与 REST 不同」，且 generation_jobs 无法表达已核实的「2 次调用 + 每方向多个 job id」两阶段流水线；(3) 形象合成发生在客户端还是服务端从未定义，但切片 2/3/4 有三条后端集成测试验收项断言「composite / rendered」，物理上不可执行。双管线一致性方面，「配件只装 A」被切片 4 验收了，但「A/B 不互通编辑」的另一半（对 generated avatar 改 layers 必须拒绝）没有任何验收项，avatars 表也缺 CHECK 约束防止混血行；此外 `/api/avatar` 单数端点与 is_active 多行模型 + 切片 4 的「激活 generated avatar」互不兼容，缺 activate/create 端点。其他实质问题：切片 6 要求 ban 但 users 表无封禁列且无人认领迁移；切片 4 的照片隐私承诺与 PRD 把 params 全量写入 params_json 直接冲突；US19 的 30-50 并发与 delta 语义/10Hz 无任何可证伪断言；PRD §Testing 承诺的前端单测与 Playwright 无切片认领（US33 不可验收）；移动模型（格子步进 vs 自由 x/y）未锁定，与 tile 坐标的装饰/碰撞会对不齐；付费 API + 零门槛注册却无配额/限流/余额告警（事实层明确要求）。唯一明确的 scope creep 是切片 3「写第二个 store 实现来验证可换性」——超出 US31 且违反 PRD 自己的「不测内部」测试原则。建议先做一轮文档修补（多数是 PRD 内几段决策补写 + 验收项改写，不影响切片划分与依赖图），再放切片 1 开工。

共 20 条发现。

---

## BLOCKER

### 0001-house-of-imbibe-prd.md §Implementation Decisions / Realtime room model（第 100-106 行）vs 0006-slice-5 验收项 1-2

**问题**: 锁定决策 2 的「场景」维度与 PRD 的实时模型互相矛盾。PRD 写死「One room per server (MVP)」，player state 为 `{x, y, dir, avatar_snapshot, name}`——**没有 scene 字段**；chat ring buffer 也是「per room」。但切片 5 要求 bar interior + yard **两个场景 + 场景切换**，decorations 表又带 `scene` 列。结果：两个不同场景的玩家会在同一坐标空间互相看到、聊天缓冲区归属不明、decoration 广播该不该按场景过滤未定义。切片 1 会按「单房无 scene」实现 WS 协议与 tick，切片 5 必须回头改协议和房间状态 —— 这是确定性返工，且改的是最底层的 seam。

**建议**: 在 PRD §Realtime room model 明确二选一并写进切片 1 验收项：(a) scene 即 room —— `Rooms: DashMap<SceneId, RoomHandle>`，player state 加 `scene`，每场景独立 tick + 独立 chat ring buffer，场景切换 = 退订旧 room 订阅新 room；或 (b) 单 room 内带 `scene` 字段，tick 广播全量但客户端按 scene 过滤，chat 全局共享。同时补切片 1 验收项：「player state 含 scene 字段，delta/snapshot 帧按 §Realtime 定义的场景语义分发」，并在切片 5 验收项加「场景 A 的玩家不会出现在场景 B 的快照中」。

### 0001-house-of-imbibe-prd.md §Stack (locked) / Generation（第 85 行）+ §Generation async UX 的 generation_jobs schema（第 133-135 行）；0004-slice-3 验收项 1、0005-slice-4 验收项 1

**问题**: PRD 与已核实的事实层直接冲突。PRD 写「PixelLab.ai via its official **MCP**/API (`create_character`, `create_image_pixflux`, `create_map_object`, `animate_character`)」，切片 3/4 验收项照抄 `create_character`。但 docs/reference/pixellab-api.md（2026-08-01 核实）§一.2 与 §六 明确：**MCP 不能用于面向终端用户的生成**（浏览器无 MCP 客户端 + token 暴露），用户侧必须走 REST v2；且 §九 指出 **MCP 的参数名与 REST 不同**（`size` vs `image_size` 等），照搬会 422。PRD 列出的四个名字全部是 MCP 工具名，对应的 REST 端点是 `POST /v2/create-character-with-4-directions` / `create-character-v3` / `portrait-character-pro` / `animate-character` / `map-objects`。此外 `generation_jobs` 只有单个 `result_asset_id`，无法表达事实层锁定的「**2 次调用 + 2 轮轮询、每方向一个 background_job_id**」两阶段流水线（reference §三、§四、§十一 建议的 `character_id` / `job_ids` JSON 数组 / `cost_usd` 三列全缺）。切片 3 建的表在切片 4 必然要加列改结构。

**建议**: 1) 把 PRD §Stack 的 Generation 一行改为「PixelLab **REST v2**（`https://api.pixellab.ai/v2`，Bearer token 仅存后端），端点：`create-character-with-4-directions` / `create-character-v3` / `portrait-character-pro` / `animate-character` / `map-objects`；MCP 仅限开发期本地出素材，不进产品代码路径」，并在 Out of Scope 加一条「前端直连 PixelLab / 用 MCP 服务用户请求」。2) `generation_jobs` 补 `provider_character_id TEXT NULL, provider_job_ids TEXT NULL(JSON array), phase TEXT, cost_usd REAL NULL`，并在切片 3 验收项写明「job 支持多阶段（create → poll → animate → poll）与每方向多个 provider job id，worker 轮询间隔 5-10s」。3) 切片 3/4 验收项里的 `create_character` 全部换成 REST 端点名。

### 0001-house-of-imbibe-prd.md §Avatar dual pipeline（第 93、98 行「Rendered by compositing layered PNGs at runtime」/「runtime overlay composition」）vs 0003-slice-2 验收项 3+6、0004-slice-3 验收项 6、0005-slice-4 验收项 6

**问题**: 「合成发生在哪一侧」从未定义，而三个切片的验收项都假设了服务端合成。PRD 说 modular 是「runtime 层叠 PNG 合成」、配件是「runtime overlay 按 slot anchor 逐帧绘制」，配合 §Testing 里「Phaser 渲染不做单元测试」的表述，读起来像**客户端 Phaser 合成**。但切片 3 验收项 6 要求集成测试「assert overlay present in the avatar **composite**」、切片 4 验收项 6 要求「activate -> **assert rendered**」—— 后端集成测试（真 Axum + WS + JSON 断言）根本无法断言客户端渲染结果。二者必有一错：要么后端要产出合成图（则需要 image crate 合成流水线、缓存、失效策略，全部未在 PRD 的 Modules 里出现），要么这两条验收项不可执行。

**建议**: 在 PRD §Avatar dual pipeline 增加一段「合成位置」决策：推荐**客户端合成**（服务端只广播 `avatar_snapshot = {kind, layers/sprite_asset_id, equipped[]}` 这一份纯数据描述，Phaser 侧按 slot anchor 表叠图），并把 slot anchor 表定义为 canonical contract 的一部分。随后改写验收项：切片 3 验收项 6 改成「assert 广播的 avatar_snapshot.equipped 含该配件与 slot」，切片 2 验收项 6 与切片 4 验收项 6 的「render/composite」改成「assert snapshot 字段」+ 另立一条前端单元测试验收项（compose 函数输入 snapshot 输出图层列表）。

## MAJOR

### 0003-slice-2 验收项 2（`PUT /api/avatar` 更新 layers_json）+ 0005-slice-4 验收项 4（仅拒绝 equip）；PRD §Schema 的 avatars 表（第 127-130 行）

**问题**: 锁定决策 5「A/B 互不互通编辑」只被落实了一半。切片 4 只验证了「配件装到 generated 返回 400」，但 PRD 第 96 行的另一半——「a generated avatar cannot have preset parts swapped」——**没有任何验收项**：`PUT /api/avatar` 对一个 `kind='generated'` 的 active avatar 提交 `layers_json` 时的行为未定义、未测试。同时 avatars 表没有任何约束保证 `kind='modular' ⇒ layers_json NOT NULL AND sprite_asset_id IS NULL`（反之亦然），PRD 只在 Modules 里含糊地写了「dual-kind validation」。数据层允许出现两个字段都填的混血行，正是「brittle unification」想避免的。

**建议**: 切片 2 验收项加两条：(a) 迁移含 `CHECK ((kind='modular' AND layers_json IS NOT NULL AND sprite_asset_id IS NULL) OR (kind='generated' AND sprite_asset_id IS NOT NULL AND layers_json IS NULL))`；(b) 集成测试断言 `PUT /api/avatar` 对 generated avatar 返回 400/409。切片 4 验收项 4 扩写为「equip 与 layers 编辑两条路径都对 generated 返回 400」。

### 0003-slice-2 验收项 2（GET/PUT `/api/avatar` 单数资源）vs 0005-slice-4 验收项 2-3（多个 avatar 在库中、可设为 active）；PRD §Schema `avatars.is_active`

**问题**: Avatar 的 API 面在两个切片间不自洽。`is_active` 列 + 切片 4 的「generated avatar 出现在库中并可激活」意味着一个用户有 **N 个 avatar 行**；但切片 2 只定义了单数的 `GET /api/avatar`（返回 active）和 `PUT /api/avatar`（改 active 的 layers），既没有「创建第二个 avatar」也没有「在 N 个中切换 active」的端点。PRD §Generation async UX 也只列了 `/api/jobs/:id` 和 `/api/library`。切片 4 实现时必须现场发明 `POST /api/avatars` / `POST /api/avatars/:id/activate`，而切片 2 已经把前端 avatar builder 绑在单数端点上 —— 前端返工。

**建议**: 在 PRD 里补一节「Avatar API」定死复数资源面：`GET /api/avatars`（列出本人全部，含 kind 与 is_active）、`POST /api/avatars`（创建 modular）、`PUT /api/avatars/:id`（仅 modular，改 layers）、`POST /api/avatars/:id/activate`、`GET /api/avatar/active`（便捷读取）。切片 2 验收项 2 改用这套端点并保留「首次调用自动创建默认 avatar」；切片 4 验收项 3 引用同一个 activate 端点。

### 0007-slice-6 验收项 4（POST ban，login disabled）vs PRD §Schema users 表（第 126 行）

**问题**: 切片 6 要求「ban（禁止登录）」，但 PRD 的 `users(id, username, password_hash, role, created_at)` **没有任何 banned/status 列**，也没有任何切片的验收项包含这次迁移；切片 1 的 auth 登录路径也不会检查封禁状态。实现者要么私自加列（PRD schema 失真），要么用 role='banned' 污染角色枚举（与 `role ∈ member|admin` 冲突）。US28 因此无法被验收项闭环验证。

**建议**: PRD §Schema 的 users 加 `is_banned INTEGER NOT NULL DEFAULT 0`（或 `status TEXT CHECK(status IN ('active','banned'))`），切片 6 验收项 4 补「迁移新增封禁列；被封用户 `POST /api/auth/login` 返回 403 且其现有 session 被清除；集成测试覆盖」。

### 0005-slice-4 验收项 5（原始照片不持久化）vs PRD §Generation async UX + §Schema 的 `generation_jobs.params_json`（第 119、134 行）

**问题**: 两处设计互相抵消。PRD 规定 `POST /api/generate {kind, params}` 把 params 整体写进 `generation_jobs.params_json`，而切片 4 的入参就是 `{kind: avatar_generated, **photo**, params}` —— 照片（或其 base64）作为生成参数，按 PRD 的 schema 会被落进 SQLite 的 params_json 并长期留存，直接违反切片 4 验收项 5「原始照片不持久化于 job 之外」的隐私承诺。此外 PRD 正文（Further Notes / Out of Scope）完全没有提照片隐私，这条数据处理承诺只活在一个切片里，容易在别处被推翻。

**建议**: 在 PRD §Generation async UX 明确「二进制上传物**不进 params_json**：照片走临时文件/内存传给 worker，job 完成或失败后即删；params_json 只存标量参数（size/view/seed 等）」，并把这条隐私承诺提到 PRD 正文（Further Notes 或新增 §Privacy）。切片 4 验收项 5 改为可验证形式：「job 完成后断言 params_json 不含图像数据、临时上传文件已删除」。

### PRD User Story 19 + §Realtime（10 Hz delta）vs 0002-slice-1 验收项 4+7

**问题**: 锁定决策 1 的核心承诺（30-50 并发下的 10Hz **delta** + 插值）没有任何能证伪的验收项。切片 1 只要求「asserts **a** delta snapshot frame is received」——一帧到达既不能证明它是 delta（只含变化玩家）、也不能证明 tick 是 10Hz、更不能证明 30-50 并发下可用；而 US19 明写「under 30-50 concurrency」。「client interpolates ~100-200 ms」被列为验收项，但它是前端行为，整套验收里没有任何前端测试来验证它。结果是本项目最关键的性能决策全靠人肉相信。

**建议**: 切片 1 验收项拆细为三条可执行断言：(a) delta 语义 —— 「两个客户端连接，只有 A 移动；断言 B 收到的 delta 帧**不包含**未变化的 C，且新连接收到 full snapshot」；(b) tick 频率 —— 「1 秒内收到的 tick 帧数在 8-12 之间」；(c) 并发冒烟 —— 「50 个 WS 客户端同时连接并各发一次 move，全部在 N 秒内收到含自己位置的帧，无 broadcast Lagged 断连」。插值那条从后端验收项移出，改为前端 `game-state` 模块单元测试验收项（给定 t0/t1 两帧断言插值输出）。

### PRD §Testing Decisions（第 161 行 Playwright + art-director、第 158 行 frontend net/game-state 单元测试）+ User Story 33；全部 7 个切片验收项

**问题**: PRD 承诺的前端测试策略没有任何切片认领。§Testing 明确「Frontend `net` + `game-state` 模块是纯 TS，做单元测试」「视觉正确性由 Playwright 截图 + art-director review loop 覆盖」，US33 也把「前端逻辑与 Phaser 解耦、可在无 canvas 下验证」当成一条 user story。但切片 1-7 的验收项**全部只有 Rust 集成测试**，没有一条要求建立前端单测 runner 或 Playwright 基线。US33 与 PRD 的测试决策因此不可验收，且第一个切片没建的脚手架后面切片也不会建（PRD 自己说「第一个切片建立的 harness 成为后续模式」）。

**建议**: 切片 1 验收项补两条：(a) 「前端 `net` 与 `game-state` 为不依赖 Phaser/DOM 的纯 TS 模块，含 vitest 单元测试：消息编解码、房间状态镜像、插值」；(b) 「Playwright 冒烟 + 首张截图基线（登录→进房→移动）落库，作为后续 art-director review loop 的起点」。若判断 Playwright 应后置，则从 PRD §Testing 删掉它，不要留下无人认领的承诺。

### PRD §Realtime room model（`{type:"move", tx, ty}` 目标格 intent）vs 0002-slice-1 验收项 6（虚拟摇杆）+ §Map & decorations（decorations 用 tile_x/tile_y）

**问题**: 移动模型未锁定：协议是**目标格 intent**（tx, ty，格子语义），输入是**模拟摇杆**（连续方向），而 PRD 从没说角色是「格子逐步移动（宝可梦式）」还是「自由 x/y 移动」。参考的 v2 研究里 PlayerState 是 `f32 x, y`，PRD 的 decorations/npcs 却是整数 tile 坐标。切片 1 会自行选一种（很可能是自由 x/y + 摇杆矢量），而切片 5 的 tile 碰撞层 clamp、切片 6 的「点一个 tile 放装饰」都建立在格子语义上，届时碰撞与放置对不齐。锁定决策 7（摇杆）与 PRD 的 tile-intent 协议之间缺一层设计。

**建议**: 在 PRD §Realtime room model 明确写死：角色为**格子步进**（tile-based stepping，摇杆方向映射为相邻格 target，服务端 clamp 到 walkable tile，客户端在两格间做 tween/插值），player state 同时含 `tile_x/tile_y`（权威）与用于渲染的插值位置；或若选自由移动，则把 `move` 消息改为 `{dx, dy}` 并把 decorations 的放置/碰撞语义一并说明。切片 1 验收项 4 引用该定义。

### 0004-slice-3 验收项 7（「verified by a second store impl in tests」）

**问题**: 这是 scope creep，且与 PRD §Testing Decisions 第 158 行「测外部行为，绝不测内部；不 mock 我们自己的模块」自相矛盾。为了「证明 object_store 可换」而**再写一个 store 实现**，既超出 US31 的范围（US31 只要求存储被抽象），又是在为抽象层本身写测试（测内部 seam）。object_store crate 本就自带 `InMemory`，不需要自研第二个实现。

**建议**: 验收项 7 改为：「业务代码只依赖 `Arc<dyn ObjectStore>`（clippy/review 层面保证）；集成测试套件用 `object_store::memory::InMemory` 跑通同一批断言，证明业务代码对具体实现无感」。不新增任何自研 store 实现。

### PRD §Further Notes（Lightweight mandate、无成本护栏）+ 0004-slice-3 全部验收项；对照 docs/reference/pixellab-api.md §十一.7

**问题**: 锁定决策 6（注册只要用户名+密码、无邮箱无验证）与付费生成 API 叠加，形成无界成本敞口，而 PRD 与切片 3/4 **没有任何配额、限流或余额告警**。事实层 §七 给出真实单价（路径 B 单角色 ≈$0.19-0.30），§十一.7 明确要求「每用户生成次数限额 + usage 入库 + 低余额告警」；v2 研究里的 `tower_governor` 也被 PRD 精简掉了。零门槛注册意味着一个脚本可以刷爆余额，而这是 PRD 唯一的可变现金支出项。

**建议**: PRD §Generation async UX 补一条硬约束：「每用户每日生成上限（可配 env，默认如 5 次）；`generation_jobs` 记录 `cost_usd`；启动与定时任务调用 `GET /v2/balance`，低于阈值拒绝新 job 并记 warn」。切片 3 验收项加「超配额时 `POST /api/generate` 返回 429，集成测试覆盖」。同时在 PRD §Stack 恢复 auth/generate 两个端点的限流（tower_governor 或手写计数），并在切片 1 验收项写明 register/login 限流。

### 0004-slice-3 验收项 4（`GET /api/library` 列出 assets）vs 0005-slice-4 验收项 2（「generated avatar 出现在 member's library」）+ PRD User Story 10

**问题**: 「个人素材库」的资源边界不一致。切片 3 把 library 定义为 `assets` 表的查询（sprite_sheet / accessory / decoration + job 状态），但切片 4 说完成时创建的是一条 **`avatars` 行**（kind='generated'）并称其「出现在库中」。US10 要求库里同时能看到「生成的 avatar 和配件及其状态」。avatars 与 assets 是两张表，`/api/library` 按切片 3 的定义不会返回 avatars 行。锁定决策 4 的「完成后回库查看」这一核心 UX 因此在两个切片里指向不同的东西。

**建议**: 在 PRD §Generation async UX 定义 library 的返回结构：`GET /api/library` 返回 `{assets: [...], avatars: [...], jobs: [...]}`（或统一为带 `type` 的条目流），并说明 generated avatar 在库中以 avatar 条目呈现、其底层 sprite 以 asset 条目呈现。切片 3 验收项 4 与切片 4 验收项 2 都引用这一结构。

## MINOR

### 0006-slice-5 验收项 3-4 vs PRD §Schema `npcs` 表（第 138 行）+ §Bartender NPCs（第 114-115 行）

**问题**: 两个缺口：(a) PRD 声明了 `npcs(id, scene, npc_def_id, x, y)` 表，但切片 5 的验收项只说「两个 NPC 在固定位置、npc_def 对话树是 JSON」，**没有任何切片认领这张表的迁移**，也没说 `npc_def` 本体存在哪（DB 表？随包 JSON 资产？）。(b) `{interact, npc_id}` 返回「**current** dialogue node」，但「current」相对什么无定义 —— 每玩家的对话推进状态既不在 schema 里，也不在 PRD 的 room state（`players` 只有 x/y/dir/avatar/name）里。实现者必须自行发明状态存放位置。

**建议**: 切片 5 验收项补：(a)「迁移新增 `npcs` 表并 seed 两个 bartender；`npc_def` 对话树作为随包 JSON 资产（路径写明），由 id 引用」；(b) 明确对话状态语义 —— 建议无状态化：`{interact, npc_id, node_id?}`，服务端按 node_id 返回下一节点，会话状态留在客户端；若要服务端保存，则写明存在 room state 的 per-player 字段。

### 0002-slice-1 验收项 5（chat 重播）+ PRD §Realtime room model（第 105 行）

**问题**: 聊天消息缺两项在原始研究中被点名的约束，PRD 与切片 1 都未捕获：(a) **文本消毒/转义**（v2 研究阶段 2 明确要求「服务端 clamp 坐标 + 反 XSS 消息（ammonia 或 escape）」）—— 聊天要渲染成头顶气泡，未消毒即 XSS 面；(b) **长度上限与发送频率限制** —— fire-and-forget 广播 + 50 条 ring buffer 下，一条超长文本或高频刷屏会直接打到所有 30-50 个客户端。锁定决策 8 只锁了「实时气泡 + 内存 50 条不落库」，这两项是它的必要配套。

**建议**: PRD §Realtime room model 补一句「chat 文本服务端强制 ≤N 字符并转义/消毒后再广播；每连接发送速率上限 M 条/10s，超限丢弃」。切片 1 验收项 5 加断言：超长文本被截断/拒绝、含 HTML 的文本被转义后广播。

### PRD User Story 5（「pick a default placeholder avatar」）vs 0003-slice-2 验收项 2（「assigns/returns a default on first call」）

**问题**: US5 说的是「**挑选**一个默认占位形象」（存在若干占位可选），切片 2 实现成「首次调用**自动分配**一个默认」。二者不等价：按切片 2 的验收项，US5 的「pick」永远不会被实现或验收。另外切片 1 用的是「block avatar」纯色方块占位，与切片 2 的默认 avatar 是不是同一个东西也没说。

**建议**: 二选一并统一：要么把 US5 改写为「首次登录自动获得一个默认形象，可立即进房」（最轻，与锁定决策 9 一致），要么在切片 2 验收项加「`GET /api/avatar/presets` 含 ≥3 个默认整体形象，首次进入时 UI 让用户选一个」。同时说明切片 1 的方块占位在切片 2 被默认 modular avatar 取代。

### 0003-slice-2 验收项 4（「pick hair/outfit/**accessory** + recolor」）vs 0004-slice-3 验收项 5（配件作为 equipped list + back/hand slot 叠加）

**问题**: 「配件」在两个切片里是两个不同的数据概念：切片 2 把 accessory 当作 `layers_json` 里的一个**预置图层**（和 hair/outfit 同级、可调色），切片 3 把 accessory 当作 `assets` 表里的**可装备物**（equipped 列表 + slot anchor 叠加）。PRD 第 93 行确实同时写了「layers（含 accessory）」**和**「equipped list」，但没说两者关系。实现上会出现两套配件渲染路径和两套 UI，违背锁定决策 9 的轻量优先。

**建议**: 在 PRD §Avatar dual pipeline 明确统一：预置配件与生成配件**共用同一个 equipped/slot 模型**（预置配件只是 owner 为 system 的 asset），`layers_json` 只保留 body/hair/outfit 等基础部件。切片 2 验收项 4 改为「预置配件通过 equipped slot 装配（与切片 3 同一机制，本切片只用预置资产）」。

### 0007-slice-6 验收项 2（广播 `{decoration_added}` / `{decoration_removed}`）+ PRD §Map & decorations（第 110 行）

**问题**: 装饰物实时同步是锁定决策 2 的核心，但广播帧的**载荷形状从未定义**：是否带 `scene`？是否带完整 `{id, tile_x, tile_y, asset_id, z_layer}` 供客户端直接渲染，还是只给 id 让客户端回拉 REST？在场景维度未定（见 blocker 1）的情况下，客户端无法判断该不该应用这次更新。切片 6 的集成测试只断言「收到 decoration_added」，不校验字段，所以任何形状都能通过。

**建议**: PRD §Map & decorations 写死帧结构：`{type:"decoration_added", scene, decoration:{id, tile_x, tile_y, asset_id, z_layer, asset_url}}` 与 `{type:"decoration_removed", scene, id}`（客户端无需额外 REST 往返）。切片 6 验收项 2/6 改为断言这些字段齐全，并断言只有同场景客户端收到（或客户端按 scene 过滤）。

## NIT

### .scratch/issues/*.md YAML frontmatter 的 `id` 字段 vs docs/agents/issue-tracker.md §Conventions 第 27 行

**问题**: 约定要求「Issue `id` 是与文件名前缀匹配的**零填充**整数（`0001`, `0002`…）」，但 8 个文件的 frontmatter 全部写成 `id: 1` … `id: 8`（YAML 里还是整数而非字符串）。正文交叉引用又用 `#2`…`#8` 的非填充形式。约定与实际不一致，未来用脚本按约定解析会对不上。

**建议**: 二选一：把 frontmatter 改为 `id: "0001"` 等字符串形式；或修改 issue-tracker.md 的约定为「`id` 为整数，**文件名前缀**零填充」——后者改动更小且与现状及 `blocked_by: [2]` 的整数写法一致，推荐后者。

### 0001-house-of-imbibe-prd.md §Child issues 末行 Dependency graph（第 194 行）

**问题**: 同一节里混用两套编号：上方列表用 issue 号（`#2 - Slice 1`…`#8 - Slice 7`），紧接着的 dependency graph 用**切片号**（`1(skeleton) -> {2(modular), 5(scenes)}`）。因为切片号恒等于 issue 号减一，这行图极易被误读成 issue 依赖（例如误以为 issue #1 即 PRD 阻塞 issue #2/#5）。

**建议**: 把该行统一改为 issue 号并标注，如：`#2(skeleton) -> {#3(modular), #6(scenes)}`；`#3 -> #4(generation) -> #5(generated-avatar)`；`#4 + #6 -> #7(admin) -> #8(deploy)`，并在句首写「以 issue 号表示」。
