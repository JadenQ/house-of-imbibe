# Rust 实时多人同步方案调研

---

## 1. `axum` WebSocket + `tokio` 手写广播（推荐路线）

- **crate**：
  - `axum = "0.8"` — https://docs.rs/axum/0.8/axum/extract/ws/index.html
  - `tokio = { version = "1.40", features = ["full"] }` — https://docs.rs/tokio/1/tokio/sync/broadcast/index.html
  - `tokio-tungstenite = "0.24"`（只有需要脱离 axum 时才直接用）— https://docs.rs/tokio-tungstenite/0.24
  - `dashmap = "6"` — https://docs.rs/dashmap/6
  - `serde = "1"` + `serde_json = "1"` / 或 `rmp-serde = "1.3"`（MessagePack）
- **客户端要求**：**任何浏览器原生 `new WebSocket()` 即可**，Phaser/TS 直接用，无专用 SDK。
- **同步方式**：应用层自己定，本项目 10Hz 位置广播 → 每 100ms 全量 room snapshot（50 人 * ~24 字节 ≈ 1.2KB/frame）完全够；无需 delta。文字聊天独立事件。
- **权威模型**：服务器权威（客户端上报 input/target，服务器 clamp 并广播）；同时把插值/预测放到客户端 Phaser 侧。
- **合适度（10–50 人/房间）**：★★★★★，完美区间。
- **内存/CPU**：单房间 50 人，一个 `broadcast::channel` + 每人一 `Sender` handle，约 <1MB；空闲进程整体 20–40MB，`tokio` runtime 常驻。二进制 release strip 后 ~8–12MB。
- **AI 友好度**：★★★★★，Claude Code 对 axum WS 例子非常熟，官方 example `examples/websockets` 就是模板。
- **生态**：axum 已是事实标准，issue/示例极多。

---

## 2. Naia（`naia-server` / `naia-client`）

- **crate**：
  - `naia-server = "0.26"` — https://docs.rs/naia-server/0.26
  - `naia-shared = "0.26"` — https://docs.rs/naia-shared/0.26
  - JS 客户端：`@naia/client-js`（npm）通过 WebRTC data channel/WebTransport
- **客户端要求**：**必须用 Naia 自家 JS 客户端**，协议是私有二进制，不能用原生 WebSocket 直连 Phaser。
- **同步方式**：内建 tick、组件 diff/delta、可靠+不可靠两条通道、内建插值。
- **权威模型**：服务器权威，自带 client-side prediction hook。
- **合适度**：功能对本项目**过剩**，且引入 Naia JS 依赖 = Phaser 侧需要抽出一层适配。文档相对少，AI 写起来容易翻车。
- **内存**：≈ 40–80MB，二进制 15–25MB。
- **AI 友好度**：★★☆☆☆（API 变动频繁，Claude 训练语料少）。

---

## 3. Lightyear

- **crate**：`lightyear = "0.21"` — https://docs.rs/lightyear/0.21
- **绑定**：虽然理论上可脱离 Bevy，但 API、schema、replication 全部围绕 Bevy ECS 组件构建，脱离后写法极别扭。
- **客户端**：官方支持 Bevy 客户端，**没有 JS/TS 客户端**。要接 Phaser 得手写协议桥，等于放弃它的所有卖点。
- **合适度**：★☆☆☆☆，不选。
- **AI 友好度**：★★☆☆☆，且信号常常和 Bevy 特定版本纠缠。

---

## 4. Renet / Renet2

- **crate**：
  - `renet = "1.0"` — https://docs.rs/renet/1.0
  - `renet2 = "0.9"`（带 WebTransport / netcode2 支持）— https://docs.rs/renet2/0.9
- **协议**：底层是 netcode.io（UDP）+ 多 channel（可靠有序 / 不可靠）。renet2 加了 `WebTransportServer` 走 HTTP/3。
- **客户端**：JS 端要用浏览器 WebTransport API + `netcode.io` 的 JS port（生态不成熟），或用 Rust wasm 的 `renet_client`。**不能拿裸 WebSocket 连**。
- **合适度**：适合 FPS/高频输入。本项目 10Hz 位置 + 聊天用 UDP 通道纯属浪费复杂度。★★☆☆☆。
- **内存**：40–60MB。AI 友好度：★★★☆☆，例子够但都是 Bevy demo。

---

## 5. matchbox_socket（WebRTC P2P）

- **crate**：`matchbox_socket = "0.12"` — https://docs.rs/matchbox_socket/0.12；配套 `matchbox_server`
- **模型**：服务器只做 signaling，实际状态走 P2P mesh。
- **合适度**：50 人房间 = 50×49/2 ≈ 1225 条 P2P 连接，客户端浏览器直接爆炸。**不适合本项目**，只适合 2–8 人。★☆☆☆☆。

---

## 6. 手写 shared-server（Axum WS + `broadcast::channel` + `DashMap`）

= 方案 1 的落地形态，见下面代码。这是我的首选。

---

## 7. Colyseus Rust 兼容层

- 现状：**不存在**成熟实现。GitHub 上有 `colyseus-rs`（客户端零散尝试）和几个 archived 的服务端实验，均无近期提交、无 crates.io 发行版。Colyseus 官方服务器只有 Node 实现，schema 二进制格式虽有文档但没人维护 Rust server 端。
- **结论**：跳过。要用 Colyseus 就得跑 Node，与"自建 Rust 后端 + 单二进制"目标冲突。

---

## 首选：**方案 6（Axum WS + `broadcast` + `DashMap` 手写）**

**理由**：
1. 客户端零依赖，Phaser/TS 用 `new WebSocket()` 就能连，也方便浏览器 DevTools 调试。
2. 需求（10–50 人 / 10Hz / 位置+聊天）用不到 delta/预测/可靠 UDP，任何"游戏网络框架"都是过度设计。
3. axum + tokio 是 Rust web 语料的绝对主流，Claude Code 生成质量最高。
4. 单二进制 <15MB，空闲内存 ~30MB，完全在预算内。
5. 未来若要升级到 delta 广播、房间分片、redis pub/sub，也是最容易增量演进的架构。

---

## 最小 broadcast 骨架（<30 行）

```rust
use axum::{extract::{ws::{Message, WebSocket, WebSocketUpgrade}, State}, routing::get, Router};
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::broadcast;

type Rooms = Arc<DashMap<String, broadcast::Sender<String>>>;

#[tokio::main]
async fn main() {
    let rooms: Rooms = Arc::new(DashMap::new());
    let app = Router::new().route("/ws/:room", get(ws_handler)).with_state(rooms);
    let l = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(l, app).await.unwrap();
}

async fn ws_handler(ws: WebSocketUpgrade, State(rooms): State<Rooms>,
                    axum::extract::Path(room): axum::extract::Path<String>) -> axum::response::Response {
    ws.on_upgrade(move |s| handle(s, rooms, room))
}

async fn handle(socket: WebSocket, rooms: Rooms, room: String) {
    let tx = rooms.entry(room).or_insert_with(|| broadcast::channel::<String>(256).0).clone();
    let mut rx = tx.subscribe();
    let (mut sink, mut stream) = socket.split();
    let send = tokio::spawn(async move { while let Ok(m) = rx.recv().await { if sink.send(Message::Text(m)).await.is_err() { break; } } });
    while let Some(Ok(Message::Text(t))) = stream.next().await { let _ = tx.send(t); }
    send.abort();
}
```

依赖行：
```toml
axum = { version = "0.8", features = ["ws"] }
tokio = { version = "1.40", features = ["full"] }
dashmap = "6"
futures-util = "0.3"
```

后续演进点：把 `String` 换成 `serde_json::Value` 或 MessagePack 字节；`DashMap` 的 room entry 里再挂一个 `DashMap<PlayerId, PlayerState>` 做服务器权威快照；开一个 10Hz `tokio::time::interval` tick 广播 room snapshot 替代 pass-through。