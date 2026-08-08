# Rust 后端部署方案对比（面向一人 vibe coding + WebSocket 长连接游戏）

## 前置：WebSocket 是硬指标

游戏后端 = 玩家位置广播 + 聊天 = **长连接 WebSocket**（tokio-tungstenite / axum::ws）。任何 request/response 模型的 serverless（Lambda、Vercel、传统 Cloudflare Workers）都要谨慎——它们要么不支持 WS，要么有连接时长上限（30s ~ 15min）。这决定了下面很多方案的生死。

配套推荐 crate（不管选哪个平台都用得上）：
- `axum = "0.7"` — https://docs.rs/axum/0.7 （HTTP + WebSocket 一体，Tokio 官方生态）
- `tokio = "1.40"` — https://docs.rs/tokio/1.40
- `tracing = "0.1"` + `tracing-subscriber = "0.3"` — https://docs.rs/tracing/0.1 / https://docs.rs/tracing-subscriber/0.3
- `tokio-console = "0.1"` — https://docs.rs/tokio-console/0.1 （调试异步任务/连接泄漏神器）
- `sentry = "0.34"` + `sentry-tower = "0.34"` — https://docs.rs/sentry/0.34
- `sqlx = "0.8"` — https://docs.rs/sqlx/0.8

---

## 方案 1：单 VPS（Hetzner / DigitalOcean / 腾讯云轻量）+ systemd + Caddy

- **首月成本**：Hetzner CX22 约 €4.5/月（2 vCPU / 4GB / 40GB SSD，欧美延迟对国内一般），DO $6/月，腾讯云轻量 24 元/月。
- **长期成本**：几乎线性，加内存也就 €10 左右封顶原型阶段。
- **部署命令数**：不是"一条"，但可以脚本化到 3 条：`scp target/release/game-server user@host:/opt/`、`systemctl restart game`、Caddy 一行 `game.example.com { reverse_proxy localhost:8080 }` 自动申请 TLS。
- **WebSocket**：**原生 100% 支持**，长连接想开多久开多久，没有平台超时。
- **域名 + TLS**：Caddy 自动 Let's Encrypt，零配置。或 `caddy = "2.8"` 二进制单独跑。
- **日志/监控**：`journalctl -u game -f` 直接看 `tracing` 输出；Sentry SDK 直接上；tokio-console 需要 unix socket 端口转发，本地能连。
- **AI 友好度**：★★★★★（AI 写 systemd unit / Caddyfile 训练数据海量）
- **二进制大小**：release + strip 后 axum 服务大约 8–15MB；运行时 30–60MB 内存起步。
- **坑**：备份、防火墙、fail2ban、SSH key 得自己搞；但一次搞完之后 6 个月不用碰。

## 方案 2：Fly.io（Rust 一等公民）

- **首月成本**：免费额度已被砍掉，但一台 `shared-cpu-1x@256mb` 约 $1.94/月 + 出流量 $0.02/GB。Postgres 附加（Fly Postgres 已停售新集群，改推荐 Supabase / Neon）。原型总成本 $3–8/月。
- **部署命令**：真的一条 `fly deploy`（配一个 20 行 `fly.toml` + Dockerfile 或 `[build] builder = "paketobuildpacks/builder:base"`）。
- **WebSocket**：**原生支持**，Fly 是 Anycast TCP proxy，长连接不掐。有全球边缘节点，可以把 game server 放在离玩家近的 region。
- **长连接超时**：默认 60s idle 会踢，但 axum 里加 ping/pong 或 `TCP_KEEPALIVE` 就永久。
- **域名 + TLS**：`fly certs add game.example.com`，DNS 加一条 CNAME 即可。
- **日志/监控**：`fly logs` 直连 tracing 输出；Grafana Cloud 免费层集成很顺；tokio-console 需要开 private network wireguard 才能连。
- **AI 友好度**：★★★★（fly.toml 文档结构化，Claude Code 写得很顺；Dockerfile 部分标准）
- **坑**：机器会 auto-stop（省钱），第一次冷启 300ms–1s，游戏服要把 `auto_stop_machines = false, min_machines_running = 1` 关掉。

## 方案 3：Railway / Render

- **首月成本**：Railway $5 起（含 $5 usage），Render 免费实例会 sleep（不适合 WS），付费 $7/月。
- **部署命令**：Git push → 自动构建部署，零命令。
- **WebSocket**：Railway 原生支持，Render 付费实例支持。都可以长连。
- **长连接超时**：Railway 无固定上限，Render 付费无限。
- **域名 + TLS**：面板点两下，自动 TLS。
- **日志**：面板内 tail，tracing 直接输出；集成 Sentry 需要自己接。
- **AI 友好度**：★★★★（几乎没配置要写）
- **坑**：Rust 冷编译慢（Railway 单次构建 3–8 分钟），Nixpacks 对 Rust 支持时好时坏；出流量比 VPS 贵；黑盒程度高，`tokio-console` 基本没法用。

## 方案 4：Shuttle（专为 Rust）

- **首月成本**：Community 层免费 500 小时/月 + Postgres 512MB；Pro $20/月。
- **部署命令**：`cargo shuttle deploy` 一条完事。代码里用宏声明依赖，Shuttle 自动 provision Postgres / Redis。
- **WebSocket**：**支持** axum 的 `WebSocketUpgrade`，官方有 examples/websocket。
- **长连接超时**：目前没有硬性上限文档，社区反馈稳定跑 chat 型服务 OK；但 Community 层机器会因为不活跃回收。
- **域名 + TLS**：`*.shuttleapp.rs` 自带 TLS；自定义域名 Pro 层才行。
- **日志**：`cargo shuttle logs`，tracing 直出；Sentry 需自己接；tokio-console 用不了。
- **相关 crate**：
  - `shuttle-runtime = "0.48"` — https://docs.rs/shuttle-runtime/0.48
  - `shuttle-axum = "0.48"` — https://docs.rs/shuttle-axum/0.48
  - `shuttle-shared-db = "0.48"` — https://docs.rs/shuttle-shared-db/0.48
- **AI 友好度**：★★★★★（宏注解模式极简，Claude Code 训练语料充足）
- **坑（重要）**：2024 底 Shuttle 做了大改版（Shuttle Console + 定价上调），Community 免费额度收紧；作为一个初创平台有跑路风险，**别把生产托付给它**，原型/demo 完美。

## 方案 5：Cloudflare Workers + workers-rs

- **首月成本**：免费 100k requests/day，Durable Objects 收费。
- **部署命令**：`npx wrangler deploy` 一条。
- **WebSocket**：**支持但受限** —— 必须走 Durable Objects（`WebSocketPair` API），每个 DO 单线程、有 CPU 时间片限制、hibernation API 复杂。
- **长连接超时**：WS Hibernation 后可以"睡"很久，但状态管理心智负担大。
- **相关 crate**：`worker = "0.4"` — https://docs.rs/worker/0.4
- **AI 友好度**：★★（Rust on Workers 的 patterns 训练数据少，Claude 容易写出 native Rust 惯用法但在 Workers 上跑不了：没有 `tokio`、没有本地文件、`std::time` 受限）
- **坑**：不能用 tokio、sqlx、大部分 crate；调试链路长；游戏服这种有共享状态 + 广播的场景在 DO 里写起来非常拧巴。**不推荐**。

## 方案 6：Docker + 自选云（GCP Cloud Run / AWS ECS / 阿里云）

- **首月成本**：Cloud Run 免费额度可能覆盖原型，但 **Cloud Run 有 60min 请求超时**（改造后 WS 可用但要处理断线）；ECS Fargate $10+/月起。
- **部署命令**：需要写 Dockerfile + Terraform/CDK 或 gcloud 命令，3–10 条。
- **WebSocket**：Cloud Run 支持（HTTP/2），ECS 完全支持。
- **AI 友好度**：★★★（Dockerfile 熟，但 IaC 部分 Claude 写起来啰嗦、易错，需要人 review）
- **坑**：过度工程，一人开发不值得。

## 方案 7：家里 mini PC / 树莓派自架

- **首月成本**：电费 ≈ $2/月，机器一次性 $200–500。
- **部署命令**：`cargo build --release` + `systemctl restart`；外网需要 Cloudflare Tunnel 或 Tailscale Funnel。
- **WebSocket**：完全支持。
- **域名 + TLS**：Cloudflare Tunnel 顺带做，或 Caddy + DDNS。
- **AI 友好度**：★★★★
- **坑**：**动态 IP、家宽上行带宽、断电、机器挂了没人重启**。可以做备用/内测，不适合真的开放注册。

---

## 快速对比表

| 方案 | 一键部署 | 原生 WS | 长连稳定 | 首月成本 | AI 友好 | 供应商风险 |
|------|--------|--------|--------|--------|--------|-----------|
| Hetzner VPS + Caddy | 半 | ✅ | ✅✅ | €4.5 | ★★★★★ | 极低 |
| Fly.io | ✅ | ✅ | ✅ (需关 auto-stop) | $3–8 | ★★★★ | 中 |
| Railway | ✅ | ✅ | ✅ | $5 | ★★★★ | 中 |
| Shuttle | ✅ | ✅ | ○ | $0 (免费层) | ★★★★★ | 高 |
| CF Workers | ✅ | △ | △ | $0 | ★★ | 低 |
| Cloud Run | 半 | ○ | △ | $0–10 | ★★★ | 低 |
| 家里 mini PC | 半 | ✅ | △ | $2 电费 | ★★★★ | 你自己 |

---

## 我的首选：两阶段路线 + 一条小调整

**原型阶段（0 – 100 DAU）：直接上 Fly.io，不走 Shuttle。**

理由：
1. Shuttle 虽然对 Rust 最丝滑，但 2024 年商业化转型 + 免费层收紧后长期不确定性偏高。一人开发最怕平台跑路——你正在把注册/登录系统自建化的时候，再让部署平台成为单点是反的。
2. Fly.io 是 Rust 长连接游戏后端事实标准（Turbo Puffer、Discord bot 生态大量在用），Anycast TCP + WS 原生 + 一条命令部署 + 全球 region，几乎是原型阶段的最优解。付一点点钱（$3–8/月）换掉"免费但不稳"的心智负担。
3. Postgres 用 Neon 免费层或 Supabase Postgres（**只用它的 Postgres，不用它的 Auth/Realtime**，跟本轮"自建注册登录"路线一致）通过外网连接到 Fly 服务；连接池用 `sqlx` + PgBouncer 侧车或 Neon 自带 pooler。

**生产阶段（DAU 上百、有付费/敏感数据后）：Hetzner CX22/CX32 + Caddy + systemd。**

理由：
1. 一台 €4.5–€10 的 Hetzner 机器 4–8GB 内存对你 <200MB 的目标绰绰有余，可以同时跑 game server + Postgres + Caddy 三个进程，成本比 Fly + Neon + Sentry 三家加起来还便宜。
2. WebSocket 长连接在 VPS 上是"就那样跑着"，没有 idle timeout、没有 auto-stop、没有平台重启，游戏在线率直接抬一档。
3. 自建注册系统意味着你要放密码 hash / session / 手机号，多一层平台就多一层泄露面。裸 VPS + 全盘加密 + `argon2` 密码哈希是最简干净的组合。
4. AI 写 systemd unit / Caddyfile / `ufw` 命令是训练数据密度最高的运维场景，Claude Code 出错率极低。

**对你原提议的一处修正**：跳过 Shuttle，从 Fly.io 直接开始。原因是"从 Shuttle 迁到 Hetzner"要重写 `#[shuttle_runtime::main]` 宏、重接资源；而"从 Fly 迁到 Hetzner"就是同一个 Docker 镜像换个跑法，代码零改动。少一次迁移 = 少一次断更周末。

**监控栈无论哪阶段都固定**：`tracing` + `tracing-subscriber` 结构化 JSON 输出到 stdout，`sentry-rust` 抓 panic 和 error，`tokio-console` 本地开发和线上偶发排查用（生产默认关，需要时开 feature flag）。这套是 Rust 后端目前 AI 最会写的一套，闭眼上没问题。