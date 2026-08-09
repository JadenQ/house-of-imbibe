# WebSocket 协议目录（切片 1 实时脊椎）

> **权威级**：事实源 = `src/realtime/protocol.rs`（`ClientMsg` / `ServerMsg`）。
> 本文件是其**只读镜像**，行为细节引自 `src/realtime/session.rs` 与 `src/realtime/room.rs`、`src/realtime/mod.rs`。
> 前端镜像 = `web/src/protocol/types.ts`（手工维护）。三者须保持一致；漂移由集成测试 + fixture 桥捕获。
>
> 切片范围：demo（HEAD `819a6ad`）已实现 welcome / snapshot_full / snapshot_delta / chat / chat_backlog / pong / error{unimplemented}；dialogue / decoration_* / scene_changed / kicked 仅**声明形状**，demo **永不发送**。

---

## 1. 传输与序列化约定

- 传输：Axum `WebSocketUpgrade`，端点 `GET /ws/room`（需登录；`current_user` 取 `uid`+`name`）。文本帧 `Message::Text`，每帧一条 JSON。
- 编码：`serde_json`。所有 enum 用 `#[serde(tag = "type", rename_all = "snake_case")]` —— 消息对象有一顶层 `type` 字段（snake_case），其余字段为变体字段。
- 版本字段：**每个变体都带 `v: u8`**，当前恒为 `1`。用于前向兼容的版本协商。
- `Option<T>` 字段：`serde` 默认把 `None` 序列化成 `null`（**字段始终存在**，不是省略）。反序列化时 `null` 或**键缺失**都映射到 `None`。
- 元组 `(i32, i32)` 序列化成 JSON 数组 `[n, n]`。
- 数值类型：Rust `u64/u32/u8/i32/f32` → JSON number；前端 TS 统一为 `number`。
- `AvatarSnapshot` 在线上 = **任意 JSON**（`serde_json::Value`，服务端对 `kind` 透明，不做分支）。

### 1.1 未知 type / 非法帧 —— 静默

- **Client→Server 未知 type**：`ClientMsg` 末尾有 `#[serde(other)] Unknown`，任何未识别的 `type` 反序列化成 `Unknown`，`handle_client` 对其 `no-op`（不断连、不回 error）—— 前向兼容。
- **Client→Server 非法 JSON**：`serde_json::from_str` 失败 → `tracing::warn` 后**丢弃该帧**（`return`，不断连）。
- **Server→Client 未知 type**：前端 `parseMsg` 对不在 `KNOWN_SERVER_TYPES` 集合里的 `type` 返回 `null`（静默忽略）；`JSON.parse` 失败也返回 `null`。

---

## 2. 三个出向通道

服务端对每条连接复用三条通道，`session.rs` 的 `tokio::select!` 同时 drain：

| 通道 | 发送端 | 容量 | 载荷 | per-连接订阅 |
|---|---|---|---|---|
| 房间 broadcast | `room.broadcast_tx` | `broadcast::channel(64)` | 快照流：`SnapshotFull` / `SnapshotDelta`（含 join/leave delta） | `room.broadcast_tx.subscribe()` → `bcast_rx` |
| 全局聊天 broadcast | `rt.chat_tx` | `broadcast::channel(64)` | `Chat`（全局、跨场景；**绝不落库**，禁令 #2） | `rt.chat_tx.subscribe()` → `chat_rx` |
| 连接定向 mpsc | `room.directed[pid]` | `unbounded` | `Welcome` / `ChatBacklog` / `Pong` / `Error`（未来 `Dialogue`） | `enter()` 建 `(dir_tx, dir_rx)`，`dir_rx` 进 select |

- broadcast `send` 是**非阻塞**：receiver 满或无订阅 → 返回 `Err`，`let _ =` 丢弃；落后 receiver 收到 `RecvError::Lagged`（见 §8）。
- 定向 mpsc 是 `unbounded`，所以 `Welcome/SnapshotFull/ChatBacklog/Pong/Error` 入场/控制消息**不会因背压丢失**。

---

## 3. Client → Server 全量目录（`ClientMsg`）

`#[serde(tag = "type", rename_all = "snake_case")]`，每个变体带 `v: u8`。

| `type` | 字段（除 `v`/`type`） | Rust 类型 | demo 处理 |
|---|---|---|---|
| `move` | `tx`, `ty` | `i32`, `i32` | ✅ 实现：见 §6.1 |
| `chat` | `text` | `String` | ✅ 实现：见 §6.2 |
| `interact` | `target` | `String` | ⚠️ 声明未实现 → 回 `error{code:"unimplemented"}`（§9） |
| `dialogue_advance` | `npc`, `choice` | `String`, `Option<String>` | ⚠️ 声明未实现 → 回 `error{code:"unimplemented"}`（§9） |
| `ping` | `t` | `u64` | ✅ 实现：回 `pong{t}` |
| _(未知)_ | — | `#[serde(other)] Unknown` | 静默 no-op（§1.1） |

- `dialogue_advance.choice`：`Option<String>` → 客户端可发 `choice: "..."`、`choice: null` 或**省略**键，三者都映射 `None`/`Some`。
- 工厂：前端 `web/src/protocol/types.ts` 的 `msg` 只提供 `move` / `chat` / `ping` 三个构造器（与"已实现"集合一致；`interact`/`dialogue_advance` 无工厂）。

---

## 4. Server → Client 全量目录（`ServerMsg`）

`#[serde(tag = "type", rename_all = "snake_case")]`，每个变体带 `v: u8`。

| `type` | 字段（除 `v`/`type`） | Rust 类型 | demo 发送? | 通道 | 说明 |
|---|---|---|---|---|---|
| `welcome` | `self_id`, `scene`, `tick_hz`, `server_time` | `u64`, `String`, `u8`, `u64` | ✅ | directed | 连接即发（§7）；`scene="bar"`、`tick_hz=10`、`server_time=now_ms()` |
| `snapshot_full` | `tick`, `t`, `players`, `decorations`, `npcs` | `u64`, `u64`, `Vec<PlayerSnap>`, `Vec<Value>`, `Vec<Value>` | ✅ | directed（入场）/ directed（Lagged 补发） | 全量玩家快照；demo `decorations`/`npcs` 恒为 `[]` |
| `snapshot_delta` | `tick`, `t`, `upsert`, `remove` | `u64`, `u64`, `Vec<PlayerSnap>`, `Vec<u64>` | ✅ | 房间 broadcast | 10Hz tick 增量 + join/leave 即时 delta；`remove` 仅在离场时非空 |
| `chat` | `from`, `name`, `text`, `ts` | `u64`, `String`, `String`, `u64` | ✅ | 全局聊天 broadcast | `push_chat` 入 ring buffer 50 后广播 |
| `chat_backlog` | `items` | `Vec<ChatItem>` | ✅ | directed | 连接即发（§7）+ 聊天 Lagged 补发（§8） |
| `dialogue` | `npc`, `node`, `menu` | `String`, `String`, `Option<Value>` | ❌ 预留（切片5/6） | directed | 形状已稳定 |
| `decoration_added` | `decoration` | `Value` | ❌ 预留 | 房间 broadcast | 形状已稳定 |
| `decoration_removed` | `id` | `u64` | ❌ 预留 | 房间 broadcast | 形状已稳定 |
| `scene_changed` | `scene`, `spawn` | `String`, `(i32,i32)`→`[n,n]` | ❌ 预留 | directed | 形状已稳定 |
| `kicked` | `reason` | `String` | ❌ 预留 | directed | 形状已稳定 |
| `error` | `code`, `msg` | `String`, `String` | ✅（仅 unimplemented） | directed | demo 只发 `code:"unimplemented"`（§9） |
| `pong` | `t` | `u64` | ✅ | directed | 回应 `ping`，`t=now_ms()` |

> "预留"变体在 `protocol.rs` 注释里标明"切片 5/6 才发送；demo 永不发送"。前端 `KNOWN_SERVER_TYPES` 仍包含全部 12 个（解析层已就绪）。

---

## 5. 共享结构

### 5.1 `PlayerSnap`（`snapshot_full.players[]` / `snapshot_delta.upsert[]`）

| 字段 | Rust 类型 | 含义 |
|---|---|---|
| `id` | `u64` | 玩家 id（= DB `uid`） |
| `x` | `f32` | 像素 x = `tx * 16.0 + 8.0`（16px tile，+8 居中） |
| `y` | `f32` | 像素 y = `ty * 16.0 + 8.0` |
| `dir` | `String` | 朝向，`"n"`/`"s"`/`"e"`/`"w"`（4 方向，按主轴判定） |
| `name` | `String` | 显示名 |
| `avatar` | `serde_json::Value` | 形象快照 = 原始 `config_json`（见 §5.3） |
| `avatar_hash` | `String` | per-avatar 稳定哈希（djb2，`format!("{:x}")`）；前端会自己重算缓存键，跨语言精确一致不要求 |
| `target_tx` | `i32` | 目标 tile x（demo = 当前 `p.tx`） |
| `target_ty` | `i32` | 目标 tile y（demo = 当前 `p.ty`） |

`x`/`y` 是渲染像素坐标；`target_tx`/`target_ty` 是 tile 坐标。`avatar_hash` 来自 `room::avatar_hash`（djb2 变体，初值 5381，`h*33+b` wrapping）。

### 5.2 `ChatItem`（`chat_backlog.items[]`；`chat` 是其扁平展开）

| 字段 | Rust 类型 |
|---|---|
| `from` | `u64` |
| `name` | `String` |
| `text` | `String` |
| `ts` | `u64` |

### 5.3 `AvatarSnapshot` —— 服务端透明

线上 = `serde_json::Value`（任意 JSON）。`session.rs` 从 `avatars.config_json` 取出原样透传；缺失/解析失败 → `default_avatar()`：

```jsonc
{ "kind": "modular", "skin": "#f0c8a0", "hair": "#503018", "shirt": "#3868b0", "pants": "#404048" }
```

已知两种 shape（前端 `prepareCharacterSheet` 装载层负责解析，服务端**对 `kind` 不分支**，符合禁令 #3 的精神）：

- `{ "kind": "modular", "skin", "hair", "shirt", "pants" }`（配色）
- `{ "kind": "generated", "character_id", "rotations": [{ "direction", "url" }] }`（生成 4 方向）

---

## 6. 已实现客户端意图的服务端行为（`handle_client`）

### 6.1 `move { tx, ty }`

1. 取 `room.players.get_mut(&pid)`。
2. `from = (p.tx, p.ty)`；`(nx, ny) = room.grid.clamp(from, (tx, ty))`（可走性 clamp）。
3. `dir` 按**主轴**判定：`ny>p.ty→"s"`、`ny<p.ty→"n"`、`nx>p.tx→"e"`、`nx<p.tx→"w"`，否则保留原 `dir`。
4. 若 `(nx,ny) != from || dir != p.dir`：写 `p.tx/p.ty/p.dir`，`p.rev = p.rev.wrapping_add(1)`。
5. `rev` 自增 → 下一个 10Hz tick 的 `snapshot_delta.upsert` 会带上新快照（见 §7）。

### 6.2 `chat { text }`

`rt.push_chat(ChatItem { from: pid, name, text, ts: now_ms() })`：
- 入全局 ring buffer（`VecDeque`，cap 50，`CHAT_CAP`）—— **禁令 #2：绝不落库**。
- 随后 `rt.chat_tx.send(ServerMsg::Chat{...})` 广播（非阻塞）。

### 6.3 `ping { t }`

定向回 `pong { v:1, t: now_ms() }`（注意：回的是 server `now_ms()`，不是回显 client `t`）。

---

## 7. 连接生命周期（`ws_room`）

### 7.1 入场（`on_upgrade` 之前，同步阶段）

1. `current_user(&state, &headers)` → `(uid, name, _)`；未登录 → `401`。
2. 取 avatar：`SELECT config_json FROM avatars WHERE user_id = ?`；缺失/解析失败 → `default_avatar()`。
3. `rt.enter(uid, name, avatar)`：
   - `pid = uid as u64`；建 `(dir_tx, dir_rx)` unbounded。
   - `ensure_bar()`：取/建 `bar` 房间；若 `tick_alive` 为 false（空转被清掉）→ `spawn_tick` 重启 tick（**join = 活 tick 的真相源**）。
   - 注册 `PlayerState { id, name, tx:spawn.x, ty:spawn.y, dir:"s", avatar, rev:1 }`；`room.directed.insert(pid, dir_tx)`。
   - **立即广播自己入场**（不等 100ms tick）：`broadcast_tx.send(SnapshotDelta{ upsert:[self_snap], remove:[] })`。
   - **之后**才 `bcast_rx = broadcast_tx.subscribe()` —— 所以**加入者自己收不到自己的入场 delta**（订阅晚于 send）；它通过下面的 `snapshot_full` 看到自己。
   - 返回 `(room, dir_rx, bcast_rx, chat_rx, pid)`。
4. 在 `dir_tx` 上**排队**三条入场消息（顺序保证，`on_upgrade` 后由 `dir_rx` arm 先 drain）：
   1. `Welcome { v:1, self_id:pid, scene:"bar", tick_hz:10, server_time:now_ms() }`
   2. `SnapshotFull { ... room.snapshot_full(pid) }` —— 让新人看到当前所有玩家（含自己）。
   3. `ChatBacklog { v:1, items: rt.chat_backlog() }` —— 最近 50 条聊天。

> 连接收到的**前三帧**恒为 `welcome` → `snapshot_full` → `chat_backlog`（经定向通道，先于任何 broadcast delta）。

### 7.2 主循环（`on_upgrade` 后）

`tokio::select!` 四臂：

| 臂 | 来源 | 正常 | Lagged | Closed |
|---|---|---|---|---|
| `bcast_rx.recv()` | 房间快照 broadcast | 序列化发文本帧 | §8：定向补发 `SnapshotFull` | `break`（断连） |
| `chat_rx.recv()` | 全局聊天 broadcast | 序列化发文本帧 | §8：定向补发 `ChatBacklog` | `break` |
| `dir_rx.recv()` | 定向 mpsc | 序列化发文本帧（welcome/backlog/pong/error） | — | — |
| `ws_rx.next()` | 客户端帧 | `Text` → `handle_client`；非 Text 忽略；`Err`/`None` → `break` | — | — |

发送失败（`ws_tx.send(...).await.is_err()`）→ `break`。

### 7.3 离场（循环退出后）

`rt.leave(pid)`：`room.players.remove(pid)` + `room.directed.remove(pid)` + `broadcast_tx.send(SnapshotDelta{ upsert:[], remove:[pid] })`（通知他人移除该玩家）。

---

## 8. 10Hz delta tick（`room::spawn_tick`）与 Lagged 补偿

### 8.1 tick task

- `interval(Duration::from_millis(100))` → **10Hz**；`MissedTickBehavior::Delay`（不 Burst 补打）。
- 持 `Weak<Room>` + `rooms: Arc<DashMap<SceneId, Arc<Room>>>` 副本；空转自清理。
- 每 tick：`weak.upgrade()` 拿 `Arc<Room>`（**绝不持 DashMap entry 跨 `.await`**，先 upgrade 再 await）。
  - 房间空：`idle++`；`idle >= IDLE_TICKS_BEFORE_CLEANUP`（=30，≈3s）→ `tick_alive.store(false)`，重检空（防竞态：此刻有人 join 则 `tick_alive=false` 让它重启 tick），仍空则 `rooms.remove(scene)`，`return`。
  - 非空：`idle=0`；`tick.fetch_add(1)`；`last = last_broadcast_rev.load()`；`(delta, max_rev) = room.build_delta(last)`；`last_broadcast_rev.store(max_rev.max(last))`；`broadcast_tx.send(delta)` 非阻塞。

### 8.2 `build_delta(last_rev)`

- 遍历 `players`，凡 `p.rev > last_rev` 的进 `upsert`，并更新 `max_rev`。
- `remove` 恒为 `[]`（tick 内不产生 remove；remove 只在 §7.3 离场时发）。
- 静止玩家（`rev` 未变）不出现在 delta → 带宽友好。`last_broadcast_rev` 是"已广播到的最大 rev"，单调。

### 8.3 Lagged 补偿（`session.rs` select! 两臂）

broadcast channel cap 64；receiver 落后超过 64 → `RecvError::Lagged(skipped)`：

| 落后通道 | 补偿动作 | 经由 |
|---|---|---|
| `bcast_rx`（快照流） | 定向补发 `SnapshotFull`（`room.snapshot_full(pid)`） | `dir_tx` |
| `chat_rx`（聊天） | 定向补发 `ChatBacklog`（`rt.chat_backlog()`） | `dir_tx` |

- **不传播错误**：Lagged 不 `break`、不 log error，只补一次全量/积压，receiver 即刻与最新对齐。
- `RecvError::Closed` → `break`（房间/聊天通道关闭，断连）。

> 设计要点：快照流是"可丢可补"的（delta 丢了无所谓，补 full 即对齐）；定向通道是"不可丢"的（unbounded，welcome/backlog/pong/error 必达）。

---

## 9. `error{code:"unimplemented"}` 语义

- `ClientMsg::Interact { .. }` 与 `ClientMsg::DialogueAdvance { .. }` 是**已声明、形状稳定、但 demo 未实现**的客户端意图。
- 服务端经定向通道回：

  ```jsonc
  { "v": 1, "type": "error", "code": "unimplemented", "msg": "not yet in this demo" }
  ```

- 其余客户端意图均已实现（`move`/`chat`/`ping`），不会回 `error`。
- `error.code` 当前唯一取值 = `"unimplemented"`。其他 `code` 为未来扩展预留。
- `ClientMsg::Unknown`（`#[serde(other)]`）→ **不回 error**，静默 no-op（见 §1.1）。

---

## 10. 前端镜像状态（`web/src/protocol/types.ts`）

### 10.1 线上协议漂移 —— **无**

逐变体逐字段对照 `ClientMsg`(5) / `ServerMsg`(12) / `PlayerSnap`(9) / `ChatItem`(4)：

- `type` 标签（snake_case）、字段名、字段类型、`v:1` 版本字段 —— **全部一致**。
- `KNOWN_SERVER_TYPES` 集合（12 项）与 `ServerMsg` 变体集合**完全一致**。
- `parseMsg` 的"未知/非法 → null（静默）"与 Rust `#[serde(other)] Unknown` + 非法 JSON 丢弃**语义一致**。

**结论：线上协议零漂移。** 本节无需列任何线上漂移项。

### 10.2 已知 TS 建模差异（**设计内，非线上漂移**）

下列差异是前端单侧的建模选择，**线上载荷不变**，属设计意图（`protocol.rs` 头注释已声明）：

1. **`AvatarSnapshot` 类型收窄**：
   - Rust = `serde_json::Value`（线上任意 JSON，服务端对 `kind` 透明）。
   - TS = 判别联合 `{kind:"modular",...} | {kind:"generated",...}`（装载层 `prepareCharacterSheet` 解析两种已知 shape）。
   - 影响：严格 TS 会拒绝服务端**合法但非上述两 shape** 的 avatar 载荷。实践中服务端只持久化这两种 shape（`default_avatar()` 亦然），故不触发。**有意为之**，非 bug。

2. **`ClientMsg` 无 `Unknown` 变体**：
   - `#[serde(other)] Unknown` 是**服务端反序列化**的前向兼容兜底；客户端不发送"未知 type"，故 TS `ClientMsg` 联合体只含 5 个已知变体。一致。

3. **`msg` 工厂仅覆盖 `move`/`chat`/`ping`**：
   - 与"已实现意图"集合一致；`interact`/`dialogue_advance`（unimplemented）无工厂。一致。

4. **`ServerMsg.dialogue.menu` / `ClientMsg.dialogue_advance.choice` 的 `Option` 表达**：
   - Rust `Option<T>` → 线上 `null`（始终在键里）。
   - TS 用 `menu?: unknown` / `choice?: string`（可选键）。TS 类型不显式含 `null`，但 `unknown` 已涵盖；`choice` 的 `string | undefined` 与线上 `null`/缺失均可被 Rust 解析为 `None`。语义一致。

### 10.3 前端分层守卫

`web/src/net`、`web/src/game-state`、`web/src/protocol` 三个目录禁止 `import 'phaser'`（`docs/development-plan.md` §2.3；`CLAUDE.md`「其他硬约束」）。由 `web/eslint.config.js` 的 `no-restricted-imports` 守卫。`scene/` 与 `src/main.ts` 是允许 phaser 的层。

> 命名注记：`CLAUDE.md` 文本写作 `net/ game/ protocol/`（`game`），而代码内注释（`types.ts` 顶部、各文件头）与本文档守卫的是 `game-state/`。仓库里 `src/game/`（纯 canvas 角色 sprite 合成）与 `src/game-state/`（纯状态机）是两个并存目录，均不 import phaser。本文档守卫范围按任务定义 = `game-state/`。
