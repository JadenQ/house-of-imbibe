# v2 修订版工作流（Rust 后端 + Phaser 前端）

---

## 一、v2 修订版技术栈总览

**核心动机**：把 v1 的"Supabase Auth + PartyKit 实时 + Vercel 全托管"这套 SaaS 组合，替换为"**单个 Rust 二进制 + SQLite + 自建注册 + 单 VPS**"的更低层路线。取舍如下：**放弃**了 Supabase 那种"点两下就有的 Auth/Realtime/Storage"，**换来**的是 (1) 全栈可控无供应商锁定，(2) 密码哈希/session/邮件验证全部自己拿捏，(3) 部署就是一个 <15MB 静态二进制 + 一个 `.db` 文件，(4) 每月 €4.5 封顶，(5) AI Coding 时 Rust 后端语料远比拼装 Supabase RLS/Edge Functions/PartyKit 更集中不易翻车。**前端 Phaser 4 + TypeScript + Vite 保持不变**，与后端通过 REST + WebSocket 两个通道通信，前端无需感知后端语言变化。

| 类别 | v1 首选 | v2 首选 | v2 备选 | 一句话理由 |
|---|---|---|---|---|
| 前端引擎 | Phaser 4 | **Phaser 4**（保留） | — | v1 结论继承 |
| 前端语言/打包 | TS + Vite | **TS + Vite**（保留） | — | v1 结论继承 |
| Web 框架 | Next.js API Routes | **Axum 0.8** | Loco.rs 0.14 | Tokio 官方 + 原生 WS + Claude 语料最厚 |
| 实时同步 | PartyKit | **Axum WS + `tokio::broadcast` + `DashMap`** | Naia 0.26 | 客户端零 SDK，浏览器 `new WebSocket()` 直连 |
| 数据库 | Supabase Postgres | **SQLite (WAL) + sqlx 0.8** | Postgres via Neon | 单文件、无 DevOps、AI 写裸 SQL 最顺 |
| Auth | Supabase Auth | **axum-login 0.18 + tower-sessions 0.14 + argon2 0.5** | jsonwebtoken 9.3 | Cookie session 撤销即时生效，无 JWT 黑名单 |
| 邮件 | Supabase | **lettre 0.11 + Resend SMTP** | Brevo（300/日） | Resend 免费额度 + DNS 最简 |
| 限流 | 无 | **tower_governor 0.7** | governor 0.7 | Tower 中间件一行接入 |
| 资产存储 | Supabase Storage | **object_store 0.11（LocalFileSystem 起步）** | R2 via 同一 trait | 本地→R2 零业务代码改动 |
| 图像处理 | 前端 fal.ai | **image 0.25 + fast_image_resize 5.1 + imagequant 4.3**（后端后处理） | oxipng 9.1 | 像素化调色板量化必备 |
| 部署（原型） | Vercel | **本地 cargo run + Cloudflare Tunnel** | Fly.io | 朋友测试零成本 |
| 部署（上线） | Vercel + Supabase | **Hetzner CX22 + Caddy + systemd** | Fly.io | €4.5/月，无 auto-stop，WS 就那样跑着 |
| 监控 | Vercel Analytics | **tracing 0.1 + sentry 0.34 + tokio-console 0.1** | Grafana Cloud | Rust 后端标准三件套 |

**保留**：Phaser 4、TypeScript、Vite、fal.ai 生图（前端触发，后端负责拉回本地）。
**替换**：Supabase 全家桶 → Axum + SQLite + axum-login；PartyKit → 手写 broadcast；Vercel/Supabase Storage → Hetzner + object_store；Supabase Auth 邮件 → Resend SMTP。

---

## 二、后端架构详解

### 1. Web 框架

**首选：Axum 0.8**。备选：Loco.rs 0.14（Rails-like，底层还是 Axum，代码可平滑迁移）。

不选 Actix（中间件生态与 tower 割裂）、Rocket（WS 无原生）、Poem/Salvo（语料量偏少）、Pavex（alpha）。

`src/main.rs` 最小骨架（可直接 `cargo run`）：

```rust
use axum::{
    extract::{ws::WebSocketUpgrade, State},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_http::{services::ServeDir, trace::TraceLayer};

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::SqlitePool,
    pub rooms: crate::realtime::Rooms,
    pub assets: Arc<dyn object_store::ObjectStore>,
    pub public_base_url: String,
}

#[derive(Deserialize)] struct LoginReq { email: String, password: String }
#[derive(Serialize)]   struct LoginRes { user_id: String }

async fn login(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginReq>,
) -> Result<Json<LoginRes>, axum::http::StatusCode> {
    // argon2 verify + tower-sessions 写入
    todo!()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().json().init();

    let db = sqlx::SqlitePool::connect("sqlite://data/game.db?mode=rwc").await?;
    sqlx::migrate!("./migrations").run(&db).await?;

    let state = Arc::new(AppState {
        db,
        rooms: crate::realtime::Rooms::default(),
        assets: Arc::new(object_store::local::LocalFileSystem::new_with_prefix("./data/assets")?),
        public_base_url: std::env::var("PUBLIC_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:8080/assets".into()),
    });

    let app = Router::new()
        .route("/api/auth/login", post(login))
        .route("/ws/:room", get(crate::realtime::ws_handler))
        .nest_service("/assets", ServeDir::new("./data/assets"))
        .fallback_service(ServeDir::new("./dist"))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    axum::serve(listener, app).await?;
    Ok(())
}
```

### 2. 实时多人层

**方案**：Axum WS + `tokio::sync::broadcast` + `DashMap<RoomId, Room>`。10Hz tick 广播 room snapshot（50 人 × ~24 字节 ≈ 1.2 KB/frame），JSON 起步够用，量化后再换 MessagePack (`rmp-serde`)。

**权威模型**：客户端上报 `{type:"move", tx, ty}`（目标格 target），服务器 clamp 到合法坐标 → 写入 `Room.players` → 由 10Hz tick 全量广播 → 前端 Phaser 侧做位置插值。聊天独立走 `{type:"chat", text}` 走同一 socket。

`src/realtime.rs` 骨架（~30 行核心）：

```rust
use axum::extract::{ws::{Message, WebSocket, WebSocketUpgrade}, Path, State};
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;

#[derive(Default, Clone)]
pub struct Rooms(pub Arc<DashMap<String, RoomHandle>>);

#[derive(Clone)]
pub struct RoomHandle {
    pub tx: broadcast::Sender<String>,
    pub state: Arc<DashMap<String, PlayerState>>, // player_id -> state
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PlayerState { pub x: f32, pub y: f32, pub name: String, pub avatar: String }

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Path(room): Path<String>,
    State(state): State<Arc<crate::AppState>>,
) -> axum::response::Response {
    ws.on_upgrade(move |sock| handle_socket(sock, state, room))
}

async fn handle_socket(sock: WebSocket, state: Arc<crate::AppState>, room_id: String) {
    let room = state.rooms.0.entry(room_id.clone()).or_insert_with(|| RoomHandle {
        tx: broadcast::channel(256).0,
        state: Arc::new(DashMap::new()),
    }).clone();
    let mut rx = room.tx.subscribe();
    let (mut sink, mut stream) = sock.split();

    let send_task = tokio::spawn(async move {
        while let Ok(m) = rx.recv().await {
            if sink.send(Message::Text(m)).await.is_err() { break; }
        }
    });

    // 10Hz tick: 由 room 第一次创建时 spawn 一个 broadcaster（略），
    // 这里只处理入站消息
    while let Some(Ok(Message::Text(t))) = stream.next().await {
        // parse ClientMsg, update room.state, 不直接广播 — 由 tick 统一发
        let _ = room.tx.send(t); // MVP: 先透传
    }
    send_task.abort();
}
```

演进点标记：MVP 先 pass-through 广播；下一步开 `tokio::time::interval(Duration::from_millis(100))` 单独 spawn 一个 tick task 遍历 `room.state` 打包 snapshot 广播，客户端插值。

### 3. 数据库 + 迁移

**首选**：SQLite + WAL + sqlx 0.8。迁移用 `sqlx-cli`（`sqlx migrate add xxx` 生成纯 SQL 文件）。

启动时执行一次：
```sql
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA foreign_keys = ON;
```

完整 schema DDL（`migrations/0001_init.sql`）：

```sql
-- ============ 用户 ============
CREATE TABLE users (
    id              TEXT PRIMARY KEY,                     -- UUID v4
    email           TEXT NOT NULL UNIQUE COLLATE NOCASE,
    username        TEXT NOT NULL UNIQUE,
    password_hash   TEXT NOT NULL,                        -- argon2id PHC
    email_verified  INTEGER NOT NULL DEFAULT 0,
    avatar_id       TEXT REFERENCES avatars(id) ON DELETE SET NULL,
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_users_email ON users(email);

-- ============ 会话 ============
-- tower-sessions-sqlx-store 自建 tower_sessions 表，这里是业务侧登录会话
CREATE TABLE sessions (
    id              TEXT PRIMARY KEY,                     -- 128-bit random
    user_id         TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    user_agent      TEXT,
    ip              TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at      TEXT NOT NULL                         -- now + 30d
);
CREATE INDEX idx_sessions_user ON sessions(user_id);
CREATE INDEX idx_sessions_expires ON sessions(expires_at);

-- ============ 邮件验证 / 密码重置 令牌 ============
CREATE TABLE email_tokens (
    token_hash      TEXT PRIMARY KEY,                     -- SHA-256(token)，不存明文
    user_id         TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    kind            TEXT NOT NULL CHECK (kind IN ('verify_email','password_reset')),
    expires_at      TEXT NOT NULL,                        -- now + 24h
    consumed_at     TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_email_tokens_user ON email_tokens(user_id);

-- ============ 头像 ============
CREATE TABLE avatars (
    id              TEXT PRIMARY KEY,                     -- UUID v4
    user_id         TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    source          TEXT NOT NULL CHECK (source IN ('upload','fal_ai','preset')),
    asset_path      TEXT NOT NULL,                        -- object_store path
    palette         TEXT,                                 -- JSON: [[r,g,b], ...]
    width           INTEGER NOT NULL,
    height          INTEGER NOT NULL,
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_avatars_user ON avatars(user_id);

-- ============ 房间 ============
CREATE TABLE rooms (
    id              TEXT PRIMARY KEY,                     -- slug: "lounge"
    name            TEXT NOT NULL,
    map_id          TEXT NOT NULL,                        -- tilemap id
    owner_id        TEXT REFERENCES users(id) ON DELETE SET NULL,
    capacity        INTEGER NOT NULL DEFAULT 50,
    is_public       INTEGER NOT NULL DEFAULT 1,
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

-- ============ 聊天消息 ============
CREATE TABLE messages (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    room_id         TEXT NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    user_id         TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    body            TEXT NOT NULL,
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_messages_room_time ON messages(room_id, created_at DESC);
```

Postgres 迁移时的 diff：`TEXT PRIMARY KEY` → `UUID PRIMARY KEY DEFAULT gen_random_uuid()`；`INTEGER NOT NULL DEFAULT 0` → `BOOLEAN NOT NULL DEFAULT FALSE`；`datetime('now')` → `now()`；`AUTOINCREMENT` → `GENERATED ALWAYS AS IDENTITY`。业务代码基本不动（sqlx 使用统一 SQL，编译期 feature flag 切换）。

### 4. 注册/登录系统完整流程

**为什么 session 不 JWT**：一人后端无跨服务需求，session 表让"改密码即踢下线"零成本；JWT 撤销要维护黑名单反而更复杂。

**argon2id 推荐参数**（OWASP 2024）：`m_cost = 19456`（19 MiB）、`t_cost = 2`、`p_cost = 1`。每次哈希 20-40ms、峰值 ~20 MiB。

```
[前端表单 Phaser/Vite]
  │ POST /api/auth/register {email, username, password}
  ▼
[tower_governor 全局 IP 限流 60/min + 登录/注册桶 5/min]
  ▼
[handler]
  ├─ validator: email 格式 / password 强度 (≥8 位)
  ├─ argon2::PasswordHash 生成 hash              (crate: argon2 0.5)
  ├─ INSERT users (email_verified=0)             (crate: sqlx 0.8)
  ├─ 生成 32-byte random token, 存 SHA-256(token) (crate: rand 0.8)
  └─ lettre → Resend SMTP 发验证邮件              (crate: lettre 0.11)
  ▼
用户点邮件链接 → GET /api/auth/verify?token=xxx
  → 校验 SHA-256(token) 存在且未 consumed 且未过期
  → UPDATE users SET email_verified=1; UPDATE email_tokens SET consumed_at=now()
  ▼
POST /api/auth/login {email, password}
  → argon2::verify_password
  → axum-login: AuthSession::login(&user)   (crate: axum-login 0.18)
  → Set-Cookie: id=...; HttpOnly; Secure; SameSite=Lax; Max-Age=2592000
  ▼
后续请求：
  → login_required!(Backend) 中间件
  → handler 里 AuthSession extractor 拿 user
  ▼
POST /api/auth/forgot {email}
  → 无论用户是否存在都返回 200（防枚举）
  → 若存在：生成 reset_token, kind='password_reset', 24h 过期
  → 发邮件
  ▼
POST /api/auth/reset {token, new_password}
  → 校验 token → argon2 rehash → UPDATE users
  → DELETE FROM sessions WHERE user_id=? (踢下线所有设备)
  ▼
POST /api/auth/logout
  → AuthSession::logout()
  → DELETE FROM sessions WHERE id=?
```

**CSRF**：cookie `SameSite=Lax` + 写接口检查 `Origin` header，跳过 `axum-csrf` crate 省依赖。

### 5. 资产存储

**核心抽象**：业务代码只依赖 `Arc<dyn object_store::ObjectStore>`。

```rust
// MVP: 本地磁盘
let store: Arc<dyn ObjectStore> = Arc::new(
    LocalFileSystem::new_with_prefix("./data/assets")?
);

// 上线: R2（同一个 trait，业务代码零改动）
let store: Arc<dyn ObjectStore> = Arc::new(
    AmazonS3Builder::from_env()
        .with_endpoint(std::env::var("R2_ENDPOINT")?)
        .with_bucket_name("hoi-assets")
        .with_region("auto")
        .build()?
);
```

**像素化流水线**（用户上传头像 or 拉回 fal.ai 结果）：
```
上传 multipart (crate: axum::extract::Multipart)
  → image::load_from_memory (crate: image 0.25)
  → fast_image_resize 缩到 64×64 (crate: fast_image_resize 5.1)
  → imagequant 量化到 32 色 (crate: imagequant 4.3)
  → image::save PNG
  → oxipng 后处理 (crate: oxipng 9.1)
  → store.put("avatars/{user_id}/{uuid}.png", bytes)
  → INSERT avatars 表
```

**fal.ai 拉回**：**同步**方式（点"确认" → 后端 reqwest 流式下载 → `store.put_multipart` → 返回自家 URL）。3-5 秒等待可接受，避免消息队列。

> ⚠️ **2026-08-01 补充：PixelLab 不能复用这个同步模式**。
> 若接入 PixelLab 生成多方向行走 sprite，其端点是**异步的，端到端 5–9 分钟**，
> 必须落任务表 + 后台 worker 轮询（5–10 秒间隔）+ SSE 通知前端。
> 表结构与完整接入设计见 [`reference/pixellab-api.md`](./reference/pixellab-api.md) §十一。
> 另注意 `avatars.source` 的 CHECK 约束目前只允许 `'upload','fal_ai','preset'`，
> 接入 PixelLab 需加 `'pixellab'`。

---

## 三、部署与运维

### 阶段 1（原型，第 1-4 周）

**目标**：本机跑，通过 Cloudflare Tunnel 暴露给朋友测试。

```bash
# 后端
cargo run --release

# 前端（另一终端）
cd client && pnpm dev  # Vite 默认 5173

# 暴露给外网（不用买域名不用配 DNS）
cloudflared tunnel --url http://localhost:8080
# 得到 https://xxx-yyy.trycloudflare.com，扔给朋友即可
```

Vite 开发模式配 `server.proxy` 把 `/api` 和 `/ws` 代理到 `localhost:8080`。

### 阶段 2（上线）

**目标机器**：Hetzner CX22（€4.5/月，2 vCPU / 4GB / 40GB）。

```bash
# 1) 本机交叉编译（避免 VPS 编译吃内存）
cargo build --release --target x86_64-unknown-linux-musl
strip target/x86_64-unknown-linux-musl/release/game-server

# 2) 推送二进制 + 迁移
scp target/x86_64-unknown-linux-musl/release/game-server root@host:/opt/hoi/
scp -r migrations root@host:/opt/hoi/

# 3) 重启服务
ssh root@host "systemctl restart hoi"
```

`/etc/systemd/system/hoi.service`：
```ini
[Unit]
Description=House of Imbibe
After=network.target

[Service]
Type=simple
User=hoi
WorkingDirectory=/opt/hoi
EnvironmentFile=/opt/hoi/.env
ExecStart=/opt/hoi/game-server
Restart=always
RestartSec=3
LimitNOFILE=65536

[Install]
WantedBy=multi-user.target
```

`/etc/caddy/Caddyfile`：
```
game.yourdomain.com {
    reverse_proxy localhost:8080
    encode zstd gzip
    # WebSocket 自动透传，Caddy 无需额外配置
}
```

**TLS**：Caddy 自动 Let's Encrypt，一次配置终身自动续期。

### 监控/日志

**tracing 结构化 JSON**（`main.rs` 顶部）：
```rust
use tracing_subscriber::{fmt, EnvFilter};
fmt()
    .json()
    .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,tower_http=debug".into()))
    .init();
```

**Sentry**（生产 panic + error 捕获）：
```rust
let _guard = sentry::init((
    std::env::var("SENTRY_DSN").ok(),
    sentry::ClientOptions { release: sentry::release_name!(), ..Default::default() },
));
```
中间件层：`.layer(sentry_tower::NewSentryLayer::<axum::Request>::new_from_top())`

**tokio-console**（仅本地/临时开）：`Cargo.toml` 加 `console-subscriber = "0.4"` 作为 feature，`RUSTFLAGS="--cfg tokio_unstable"` 编译，需要时启动前 SSH 端口转发 `-L 6669:localhost:6669`。生产默认关。

**日志观察**：`ssh host journalctl -u hoi -f`。

### 备份策略

**SQLite** 阶段：
```bash
# 每日 03:00 crontab
0 3 * * * sqlite3 /opt/hoi/data/game.db ".backup /tmp/game-$(date +\%F).db" && \
  rclone copy /tmp/game-*.db r2:hoi-backups/ && rm /tmp/game-*.db
```
用 `.backup` 命令（非 `cp`）保证 WAL 一致性。上传到 R2（首月免费额度足够）。

**Postgres**（未来切换后）：`pg_dump | gzip | rclone copy` 每日进 R2。

---

## 四、v1 → v2 迁移清单

| v1 工具 | v2 替换 | 理由 | 前端改动量 |
|---|---|---|---|
| Supabase Auth | axum-login 0.18 + tower-sessions 0.14 + argon2 0.5 | 自建撤销即时生效；无 SaaS 锁定 | **极小**（登录接口 URL 从 Supabase SDK 改成 `fetch('/api/auth/login')`） |
| Supabase Postgres | SQLite + sqlx 0.8 (WAL) | 单文件零 DevOps；上线也 €0 加钱 | **0**（前端从不直连 DB） |
| Supabase Realtime | Axum WS + tokio::broadcast | 原生 `new WebSocket()`，无 supabase-js 依赖 | **中等**（去掉 supabase realtime channel API，改用 `new WebSocket('wss://.../ws/lounge')`） |
| Supabase Storage | object_store 0.11 (Local → R2) | 抽象层统一，MVP 零外部依赖 | **极小**（URL 域名换掉） |
| PartyKit | Axum WS + `DashMap` 房间 | 少一个部署目标，少一门 JS runtime | **中等**（拆掉 partykit/client，换回原生 WS）|
| Supabase Auth 邮件模板 | lettre + Resend SMTP | 完全自控模板；免费 3000/月 | **0** |
| Vercel Analytics | tracing + Sentry | 服务端指标，前端另开 Plausible | 前端可选加 Plausible |
| Next.js API Routes | Axum 0.8 handlers | 单二进制 + 语言统一 | **0**（前端只关心 HTTP 契约） |
| Vercel 部署 | Hetzner + Caddy + systemd | €4.5 封顶，WS 无 idle timeout | **0** |
| fal.ai 直接前端调 | **保留**前端触发 + 后端拉回本地 | 生图 URL 24h 过期，必须落地 | **0**（前端仍用 fal.ai JS SDK 触发） |
| Phaser 4 / TS / Vite | **保留** | v1 结论不动 | **0** |

---

## 五、Cargo.toml 骨架

```toml
[package]
name = "hoi-server"
version = "0.1.0"
edition = "2021"
rust-version = "1.80"

[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
strip = true
panic = "abort"

[dependencies]
# Web / async
axum = { version = "0.8", features = ["ws", "macros", "multipart"] }
tokio = { version = "1.42", features = ["full"] }
tower = "0.5"
tower-http = { version = "0.6", features = ["fs", "trace", "cors", "compression-zstd"] }
tower_governor = "0.7"
futures-util = "0.3"

# 数据库
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite", "macros", "migrate", "chrono", "uuid"] }

# Auth / session
axum-login = "0.18"
tower-sessions = "0.14"
tower-sessions-sqlx-store = { version = "0.15", features = ["sqlite"] }
argon2 = "0.5"
rand = "0.8"
sha2 = "0.10"

# 邮件
lettre = { version = "0.11", default-features = false, features = ["tokio1-rustls-tls", "smtp-transport", "builder"] }

# 序列化 / 时间 / ID
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rmp-serde = "1.3"           # MessagePack（后期广播压缩用）
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }

# 实时房间
dashmap = "6"

# 资产存储
object_store = { version = "0.11", features = ["aws"] }
mime_guess = "2"

# 图像处理
image = "0.25"
fast_image_resize = "5.1"
imagequant = "4.3"
oxipng = { version = "9.1", default-features = false, features = ["parallel"] }

# HTTP 客户端（拉 fal.ai 结果）
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "stream", "json"] }

# 错误 / 日志 / 监控
thiserror = "2"
anyhow = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
sentry = { version = "0.34", default-features = false, features = ["rustls", "backtrace", "contexts", "panic", "tower"] }
sentry-tower = "0.34"

# 验证 / 工具
validator = { version = "0.19", features = ["derive"] }

[dev-dependencies]
tokio-test = "0.4"
reqwest = { version = "0.12", features = ["rustls-tls", "json"] }

[features]
default = []
# 开启后启用 console-subscriber，用于 tokio-console 排查
tokio-console = ["dep:console-subscriber"]

[dependencies.console-subscriber]
version = "0.4"
optional = true
```

---

## 六、AI 工作流对 Rust 的补充

### CLAUDE.md 新增硬约束（Rust 专属）

```markdown
## Rust 硬约束（违反直接不合并）

1. **禁 unwrap/expect in handler**：所有 axum handler 返回 `Result<T, AppError>`，用 `?` 传播。AppError 实现 IntoResponse，映射到 HTTP 状态码。仅测试代码可 unwrap。
2. **sqlx 一律用 query!/query_as! 宏**：编译期校验 SQL 列名/类型。禁用 query_unchecked!、禁止拼接字符串 SQL。schema 变更后必须 `cargo sqlx prepare` 更新 .sqlx 目录并提交。
3. **禁 std::sync::Mutex 跨 await**：并发状态用 tokio::sync::Mutex 或 DashMap；async 上下文中持有 std::Mutex 跨 .await 会死锁。
4. **所有公共 API 加 #[tracing::instrument(skip(state))]**：state 不打进日志（含密码 hash）。密码、token 明文永不进 tracing。
5. **crate 版本**：严格按 Cargo.toml 里 pin 的版本写代码。不确定的 API 先 `cargo doc --open` 或让我确认，不要凭记忆写。
6. **禁引入新 crate 无审批**：新增依赖需在 PR 里说明理由 + 二进制大小影响；避免 openssl（用 rustls）。
7. **broadcast::Sender 不 clone 进循环**：房间状态用 Arc<DashMap>，广播通道每房间一个，玩家 handle 里存 subscribe 后的 Receiver。
8. **测试**：每个 handler 至少一个 #[tokio::test]，跑 axum_test 或自建 TestServer。
9. **迁移**：只往前加，不改历史。改 schema 就新加 migration 文件，别改老文件。
10. **feature flag**：tokio-console 用 feature gate，生产构建默认不带。
```

### 推荐 MCP / Skills 与工具

- **rust-analyzer** 作为 LSP：Claude Code 通过内置的诊断能捕获类型错误，不用等 cargo check
- **cargo-nextest**：`cargo nextest run` 并行测试更快，AI 报错信息更清晰
- **cargo-machete**：定期扫未用依赖
- **bacon**：`bacon test` / `bacon clippy` 后台增量运行，比让 Claude 每次跑 `cargo build` 快很多
- **sqlx-cli**：`sqlx migrate add xxx` + `sqlx prepare`；把 `.sqlx` 目录纳入版本控制以便 CI 离线构建
- 目前 crates.io 上**没有**稳定成熟的"rust-analyzer MCP"或"cargo MCP" — 直接在 CLAUDE.md 里写规则 + 让 Claude 每次改完跑 `cargo clippy --all-targets -- -D warnings` 更靠谱

### Claude Code 写 Rust 的常见幻觉

1. **crate 名幻觉**：常写 `tokio-tungstenite` 但项目已经用 axum::ws；写 `sqlx::query!` 但忘了它需要 DATABASE_URL 环境变量。→ CLAUDE.md 明确列出"仅用如下 crate 名"。
2. **版本 API mismatch**：axum 0.6 → 0.7 → 0.8 之间 handler 签名和 State extractor 差异大；Claude 常混写。→ 在 CLAUDE.md 里贴一段 "axum 0.8 handler 签名标准范式" 让它模仿。
3. **sqlx 宏 vs 函数**：`query!` 是宏（编译期校验）、`sqlx::query()` 是函数（运行时）。Claude 常混用。→ 硬约束 4 强制宏。
4. **tower vs actix middleware**：Claude 偶尔会把 actix 的 `App::wrap` 写到 axum 里。→ 硬约束"只用 tower::Layer / tower_http"。
5. **异步 trait**：Rust 1.75+ 已原生支持，但 Claude 常无脑加 `#[async_trait]`。→ 明确"async fn in trait 直接写，除非需要 dyn"。
6. **错误处理**：Claude 会写一大坨 unwrap 或者反过来手写 100 行的自定义 Error enum。→ 提供 `AppError` 模板，规定用 `thiserror::Error` + `IntoResponse` 一次到位。

---

## 七、修订后路线图（4 阶段）

### 阶段 1（第 1-2 周）：Rust 骨架 + SQLite + 注册登录 + 单人 demo

**目标**：能注册、能登录、登录后进入一个空房间看到自己的角色能走动（单人本地）。

**Checklist**：
- [ ] `cargo new hoi-server` + 上面 Cargo.toml
- [ ] `migrations/0001_init.sql` 建表
- [ ] Axum + tower-sessions + axum-login 骨架
- [ ] `POST /api/auth/register` + argon2 hash + 邮件验证（Resend）
- [ ] `POST /api/auth/verify` + `POST /api/auth/login` + `POST /api/auth/logout`
- [ ] `POST /api/auth/forgot` + `POST /api/auth/reset`
- [ ] `tower_governor` 限流（全局 60/min，auth 桶 5/min）
- [ ] Phaser 场景：登录页 + 空房间地图（tilemap）+ 角色四方向移动
- [ ] Vite proxy 配好 `/api` → `localhost:8080`

**AI 分工**：Claude 一次生成 auth 模块（handlers + sqlx 查询 + tests）；你写 Phaser 场景，让 Claude 只生成 `apiClient.ts`。

**踩坑预警**：sqlx 宏首次编译必须先 `DATABASE_URL=sqlite:./data/game.db sqlx migrate run`，否则宏找不到表报错；把这条写进 `justfile` / `Makefile` 别让 AI 每次现查。

### 阶段 2（第 3-4 周）：WebSocket 多人同步 + 聊天

**目标**：多标签页登录不同账号 → 同一房间互相看得到走动 + 打字聊天。

**Checklist**：
- [ ] `GET /ws/:room` 用 axum ws 升级（校验 session cookie）
- [ ] `Rooms: DashMap<String, RoomHandle>` + `broadcast::channel(256)`
- [ ] 10Hz `tokio::time::interval` tick，广播 room snapshot
- [ ] 客户端消息类型：`{type:"move", tx, ty}` / `{type:"chat", body}`
- [ ] 服务端 clamp 坐标 + 反 XSS 消息（`ammonia` 或直接 escape）
- [ ] `messages` 表持久化聊天，进房加载最近 50 条
- [ ] Phaser 侧：远端玩家插值移动、聊天气泡

**AI 分工**：让 Claude 写协议 enum + serde 反序列化 + tick loop；你 review broadcast::Receiver 的生命周期（AI 最容易在这里泄漏 task）。

**踩坑预警**：`broadcast::channel` 满时会丢老消息（`RecvError::Lagged`），需要 handle 掉否则玩家卡住；空房间要定期回收 DashMap entry 否则内存慢慢涨。

### 阶段 3（第 5-6 周）：头像 / 角色系统 + 照片转像素

**目标**：用户能上传照片 → 像素化 → 作为游戏内角色头像。

**Checklist**：
- [ ] `POST /api/avatars/upload` multipart，走 image → fast_image_resize → imagequant 流水线
- [ ] object_store 抽象层 + LocalFileSystem，存 `avatars/{user_id}/{uuid}.png`
- [ ] `POST /api/avatars/fal` 前端拿到 fal.ai URL 后传给后端 → reqwest 拉回 → 走同一流水线
- [ ] `avatars` 表 CRUD + 用户绑定
- [ ] Phaser 侧动态加载头像贴图作为 sprite
- [ ] `imagequant` 参数：quality (60, 100)，`speed=1`（质量优先），最大 32 色

**AI 分工**：Claude 写完整像素化流水线（一个函数 `pixelize(bytes: &[u8]) -> Result<Vec<u8>>`），你只 review 尺寸/色深参数。

**踩坑预警**：`image::load_from_memory` 对超大图（>10MB）会 OOM，multipart 层要设 `DefaultBodyLimit::max(5 * 1024 * 1024)`；imagequant 是 C 库有 `unsafe`，跑 `cargo audit` 确认无 CVE。

### 阶段 4（第 7-8 周）：上线 + 打磨

**目标**：域名可访问，Sentry 有数据，有备份。

**Checklist**：
- [ ] Hetzner CX22 开机，`hoi` 用户，SSH key，ufw + fail2ban
- [ ] Caddy 配 `game.yourdomain.com`，DNS A 记录
- [ ] systemd unit + `.env` 生产配置
- [ ] Sentry DSN 接入，触发一次 panic 确认收得到
- [ ] object_store 保持 LocalFileSystem（先不切 R2，DAU 上百再说）
- [ ] SQLite 每日 `.backup` cron + rclone 到 R2 备份桶
- [ ] 决策点：DAU 破 500 或写并发看到 `SQLITE_BUSY` → 切 Neon Postgres（`Cargo.toml` feature 切换 + 迁移文件跑一遍）
- [ ] Plausible 或 self-host umami 加前端埋点

**AI 分工**：让 Claude 写 systemd unit / Caddyfile / 备份 shell 脚本；你 SSH 上去手动跑一遍，别让 AI 直接 ssh。

**踩坑预警**：Hetzner 默认镜像时区 UTC，cron `datetime('now')` 也是 UTC，前端展示要转本地；`SQLITE_BUSY` 用 `PRAGMA busy_timeout = 5000` 缓解，但真的持续报就是切 PG 的信号。

---

## 八、下一步立即行动（3 个）

### 行动 1（30 分钟）：搭骨架 + 跑起来

```bash
mkdir hoi-server && cd hoi-server
cargo init
# 复制上面第五节 Cargo.toml
mkdir -p migrations data src
# 复制第二节 §3 的 SQL 到 migrations/0001_init.sql
# 复制第二节 §1 的 main.rs 骨架到 src/main.rs
cargo install sqlx-cli --no-default-features --features sqlite
DATABASE_URL=sqlite:./data/game.db?mode=rwc sqlx migrate run
cargo run
# 期望：curl http://localhost:8080 返回 404 (dist 目录空)，日志有 tracing 输出
```
**产出**：能跑的 Axum + SQLite + tracing 骨架，一个 `.db` 文件。

### 行动 2（2 小时）：写完整 auth 模块

给 Claude Code 这个 prompt：
> 参照 CLAUDE.md 硬约束，在 `src/auth/` 下实现完整注册/邮件验证/登录/登出/忘记密码/重置密码 6 个 handler。使用 axum-login 0.18 + tower-sessions 0.14 + argon2 0.5，argon2 参数 m=19456/t=2/p=1。sqlx 全部用 `query!` 宏。为每个 handler 写一个 `#[tokio::test]`，用 sqlx::SqlitePool::connect(":memory:") + `sqlx::migrate!()` 建库。邮件先用 mock（trait `EmailSender`，测试用 fake，生产用 lettre + Resend）。

**产出**：`src/auth/mod.rs` + `handlers.rs` + `email.rs` + 6 个 tests 全绿。

### 行动 3（1 小时）：配 Cloudflare Tunnel 分享给朋友

```bash
brew install cloudflared   # 或对应平台的安装方式
cargo run --release
# 另一终端
cloudflared tunnel --url http://localhost:8080
# 复制 https://xxx-yyy-zzz.trycloudflare.com 发给朋友
# 让他们注册测试 → 你在本地 tail journalctl 或 tracing 输出看真实流量
```
**产出**：一个可以发给朋友的 URL、第一批真实注册用户、第一波日志/panic 反馈。零成本。