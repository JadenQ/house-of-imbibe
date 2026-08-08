# 自建注册/登录系统 + 数据库层调研（Rust）

## 一、Auth 层

### 1. 密码哈希

| Crate | 版本 | docs.rs | 说明 |
|---|---|---|---|
| `argon2` | 0.5.3 | https://docs.rs/argon2 | RustCrypto 官方实现，OWASP 2024 推荐首选（Argon2id）。纯 Rust，无 C 依赖。 |
| `bcrypt` | 0.17 | https://docs.rs/bcrypt | 老牌但抗 GPU 差；仅在需要兼容旧系统时选。 |
| `scrypt` | 0.11.0 | https://docs.rs/scrypt | 抗内存硬度好，但生态使用少于 argon2。 |

推荐参数（OWASP 2024，`Params::new`）：
- Argon2id, `m_cost = 19456`（19 MiB），`t_cost = 2`，`p_cost = 1`
- 二进制大小影响 < 200 KB，运行时每次哈希约 20–40 ms、峰值 ~20 MiB —— 完全符合 <200MB 目标

AI 友好度：极高，`argon2` API 只有两个函数（`hash_password` / `verify_password`），Claude Code 一次就能写对。

---

### 2. 会话方案：JWT vs Cookie Session

**结论：一人 vibe coding 强烈推荐"服务端 session + Cookie"，不要用 JWT。**

原因：
- JWT 撤销困难（改密码、封号无法立刻生效），需要维护黑名单 → 等于又做了一份 session 表
- 你只有一个 Rust 后端，不是跨服务分布式场景，JWT 的"无状态"优势用不上
- Cookie session 直接靠 DB 的一行 `sessions` 表即可，AI Coding 时逻辑最简单

| Crate | 版本 | docs.rs | 用途 |
|---|---|---|---|
| `axum-login` | 0.18 | https://docs.rs/axum-login | 一体化：session store + `AuthUser` trait + `login_required!` 宏，几乎照抄示例即可跑 |
| `tower-sessions` | 0.14 | https://docs.rs/tower-sessions | `axum-login` 底层依赖，SQLite/Postgres/Redis store 全支持 |
| `tower-sessions-sqlx-store` | 0.15 | https://docs.rs/tower-sessions-sqlx-store | 直接把 session 存进你自己的 SQLite/PG，零额外基础设施 |
| `jsonwebtoken` | 9.3 | https://docs.rs/jsonwebtoken | 如果一定要 JWT（例如原生 App 后续对接）再引入 |

`axum-login` 生态活跃度：GitHub 1.8k+ star，`maxcountryman` 也维护 tower-sessions，两个 crate 是同一套配方，文档完整、示例齐全，Claude Code 对它非常熟。

---

### 3. 一体化 Auth 备选

| 方案 | 是否推荐 | 理由 |
|---|---|---|
| **axum-login** | 首选 | 轻量、只做 auth，不绑架你的路由风格 |
| **loco-rs** (0.16) — https://docs.rs/loco-rs | 备选 | Rails 风格全家桶，自带 auth/mailer/queue/CLI 生成器；但把你锁进它的架构，社交游戏这种自定义 WS/tick 场景反而束手束脚 |
| **rauthy** (0.30) — https://sebadob.github.io/rauthy/ | 不推荐 | 是独立 OIDC server，得再跑一个进程，违背"单二进制"目标 |
| **oauth2** (5.0) — https://docs.rs/oauth2 | 按需 | 只有要接 Google/Discord/GitHub 登录时才引入 |

---

### 4. 邮件验证

- `lettre` 0.11 — https://docs.rs/lettre — SMTP 客户端事实标准；直接把 Resend / Postmark / Brevo 当作 SMTP relay 用
- 免费额度对比（2026）：
  - **Resend**：每天 100 封、每月 3,000 封（对新账号验证 + 忘记密码完全够）
  - **Postmark**：100 封/月免费试用后按量付费，交付率业界最好
  - **Brevo (原 Sendinblue)**：300 封/天永久免费，量最大
  - 推荐先用 Resend（Rust/JS 开发者友好，DNS 配置最简单）

---

### 5. Rate Limiting

| Crate | 版本 | docs.rs | 说明 |
|---|---|---|---|
| `tower_governor` | 0.7 | https://docs.rs/tower_governor | Tower 中间件，一行接入 axum；内存级、每 IP/每 key 限流 |
| `governor` | 0.7 | https://docs.rs/governor | 底层实现，`tower_governor` 就是它的封装 |

一人项目直接 `tower_governor` 全局中间件即可，登录/注册接口再叠一层更严格的（如 5 次/分钟）。

---

### 6. CSRF

- `axum-csrf` 0.11 — https://docs.rs/axum-csrf — 与 `tower-sessions` 兼容
- 或者更省事：登录接口只接受 `SameSite=Lax` cookie + 关键写操作检查 `Origin` header，可跳过独立 CSRF crate。

---

## 二、Database 层

### 1. DB 选型

| 方案 | 二进制/内存 | 优点 | 缺点 |
|---|---|---|---|
| **SQLite + sqlx** (WAL) | 单文件、内存 < 50 MB | 零 DevOps，本地开发/生产同一份代码；WAL 下多读单写并发对社交聊天完全够（< 数千并发） | 无跨机复制（Litestream 可解） |
| **Postgres + sqlx** | 需外部进程/托管 | 生产级、并发写强、JSONB、pgvector | 多一个组件；免费托管有冷启动 |
| **Turso (libSQL)** | edge 分布式 SQLite | 边缘部署延迟低、免费额度大 | 生态还在早期，Claude Code 对它熟悉度低于原生 sqlite |
| **sled 0.34 / redb 2.2** | 纯 Rust 嵌入式 KV | 无需 schema、极快 | 没有 SQL、迁移/查询要自己写，不适合有关系模型的 auth |

- 免费 Postgres 托管：Neon（Free tier: 0.5 GB + 分支）、Fly Postgres、Supabase 只用 DB
- Neon 冷启动 ~300ms；对一个像素社交游戏可接受

### 2. ORM/查询库

| Crate | 版本 | docs.rs | AI 友好度 |
|---|---|---|---|
| **sqlx** | 0.8.3 | https://docs.rs/sqlx | 极高：写原生 SQL，Claude 直接输出 SQL 字符串，`query!` 宏在编译期校验列名类型 |
| **SeaORM** | 1.1 | https://docs.rs/sea-orm | 中等：ActiveModel 概念多一层，代码生成器 `sea-orm-cli` 需要额外学习 |
| **Diesel** | 2.2 | https://docs.rs/diesel | 低（对 AI）：宏 DSL 复杂，编译错误信息劝退，Claude 常写错 |

一人 vibe coding 首选 **sqlx**：所见即所得，Claude Code 写起来最顺；迁移用 `sqlx-cli`（`sqlx migrate add`）纯 SQL 文件，改起来无心智负担。

---

## 三、一人 vibe coding 推荐栈（完整注册流程）

```
[前端表单 Phaser/Vite]
  ↓ fetch POST /api/auth/register  (JSON: email, password, username)
[axum + tower_governor 限流]
  ↓
[handler]
  ├── 校验 email / 密码强度
  ├── argon2id hash → users 表 (email_verified=false)
  ├── 生成 verify_token → email_verification_tokens 表
  └── lettre → Resend SMTP 发送验证邮件
  ↓
用户点邮件链接 → GET /api/auth/verify?token=...
  → 校验 token / 过期 → users.email_verified=true → 删 token
  ↓
POST /api/auth/login (email, password)
  → argon2 verify → axum-login 创建 session
  → Set-Cookie: id=...; HttpOnly; Secure; SameSite=Lax
  ↓
后续请求：axum-login 的 AuthSession extractor 拿到 user
  ↓
忘记密码：POST /api/auth/forgot
  → 生成 reset_token 写 email_verification_tokens (kind='reset')
  → 发邮件；用户提交新密码时校验 token → argon2 rehash → 使所有 sessions 失效
```

### Cargo.toml 依赖清单（复制即用）

```toml
[dependencies]
axum = "0.8"
tokio = { version = "1", features = ["full"] }
tower = "0.5"
tower-http = { version = "0.6", features = ["cors", "trace"] }
tower_governor = "0.7"
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite", "macros", "migrate", "chrono", "uuid"] }
axum-login = "0.18"
tower-sessions = "0.14"
tower-sessions-sqlx-store = { version = "0.15", features = ["sqlite"] }
argon2 = "0.5"
lettre = { version = "0.11", default-features = false, features = ["tokio1-rustls-tls", "smtp-transport", "builder"] }
serde = { version = "1", features = ["derive"] }
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
rand = "0.8"
thiserror = "2"
tracing = "0.1"
tracing-subscriber = "0.3"
```

### 最小 Schema DDL（SQLite / 也几乎直接兼容 Postgres）

```sql
-- 用户表
CREATE TABLE users (
    id              TEXT PRIMARY KEY,                     -- UUID v4
    email           TEXT NOT NULL UNIQUE COLLATE NOCASE,
    username        TEXT NOT NULL UNIQUE,
    password_hash   TEXT NOT NULL,                        -- argon2id PHC 字符串
    email_verified  INTEGER NOT NULL DEFAULT 0,           -- 0/1
    avatar_url      TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_users_email ON users(email);

-- 会话表 (tower-sessions-sqlx-store 会自建它自己的表；
-- 下面这个是"业务上"的登录会话，可直接用 axum-login 的默认表结构，或自定义)
CREATE TABLE sessions (
    id              TEXT PRIMARY KEY,                     -- 128-bit random，Cookie 值
    user_id         TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    user_agent      TEXT,
    ip              TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at      TEXT NOT NULL                         -- e.g. now + 30 days
);
CREATE INDEX idx_sessions_user ON sessions(user_id);
CREATE INDEX idx_sessions_expires ON sessions(expires_at);

-- 邮件验证 & 密码重置令牌（复用一张表）
CREATE TABLE email_verification_tokens (
    token           TEXT PRIMARY KEY,                     -- 32-byte hex，仅存 hash 更安全
    user_id         TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    kind            TEXT NOT NULL CHECK (kind IN ('verify_email','password_reset')),
    expires_at      TEXT NOT NULL,                        -- e.g. now + 24 h
    consumed_at     TEXT,                                 -- 一次性使用
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_evt_user ON email_verification_tokens(user_id);
CREATE INDEX idx_evt_expires ON email_verification_tokens(expires_at);

-- 开启 WAL（在应用启动时执行一次）
-- PRAGMA journal_mode = WAL;
-- PRAGMA synchronous = NORMAL;
-- PRAGMA foreign_keys = ON;
```

Postgres 版本替换：`TEXT PRIMARY KEY` → `UUID PRIMARY KEY DEFAULT gen_random_uuid()`；`INTEGER NOT NULL DEFAULT 0` → `BOOLEAN NOT NULL DEFAULT FALSE`；`datetime('now')` → `now()`。

---

## 四、首选与理由

**首选栈：axum 0.8 + sqlx 0.8 (SQLite/WAL) + axum-login 0.18 + tower-sessions 0.14 + argon2 0.5 + lettre 0.11 (Resend SMTP) + tower_governor 0.7**

理由：
1. **单二进制、内存 < 50MB**：SQLite 内嵌 + Rust 静态编译，符合"< 200 MB + 部署简单"的硬约束。备份就是 `cp game.db`，比 Postgres 心智负担低一个数量级。
2. **AI 友好度最高**：sqlx 写裸 SQL + axum-login 官方示例几乎照抄，Claude Code 一次就能生成能跑的代码；SeaORM/Diesel/loco-rs 都会引入 AI 容易踩坑的抽象层。
3. **生态最活跃**：这套组合是 2025-2026 年 Rust web 一人开发的事实标准，star / 更新频率 / Stack Overflow 覆盖度全部最高。
4. **升级路径清晰**：等你真的碰到 SQLite 单写瓶颈（对像素社交空间基本要几千 DAU 才会），把 `sqlx` 的 `sqlite` feature 换成 `postgres`、迁移文件跑一遍就能上 Neon，业务代码几乎不动。
5. **session > JWT**：一人后端无跨服务需求，session 表让"改密码即踢下线"这类朴素需求零成本，AI Coding 时也不用处理 refresh token 生命周期。

不推荐 loco-rs / rauthy / Turso 的原因：**都在你还没到那个体量之前，先增加了架构复杂度或多进程运维成本**，与"一人 vibe coding"的定位不匹配。