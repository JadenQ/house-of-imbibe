# House of Imbibe — 像素社交酒吧

> GBA/宝可梦绿宝石画风的 Web 像素社交酒吧。Rust 单二进制 + Phaser 前端，逻辑分辨率 240×160 整数缩放。
> 设计权威：[PRD](.scratch/issues/0001-house-of-imbibe-prd.md) · [开发计划](docs/development-plan.md) · [PixelLab API 实测](docs/reference/pixellab-api.md)

---

## 一、功能一览

| 模块 | 说明 |
|---|---|
| **注册/登录** | 用户名 + 密码（argon2id），无邮箱无第三方登录。**管理员密钥制**：注册时填 `i am jaden` → 管理员；留空 → 普通用户。首用户自动 admin 兜底。 |
| **实时房间** | WebSocket 多人移动 + 聊天。聊天是内存 ring buffer 50 条（**绝不落库**），头顶气泡 7s TTL，ping 时钟同步 + 本地预测 + 服务端纠正。 |
| **菜单** | 只读看单（不下单）。`menu_items` 表，管理员可 CRUD，公开接口只返回上架项。 |
| **地图三层架构** | ① 视觉背景层（D1：PixelLab 生成 256×256 图，前端缩放到 240×160）② 可走/碰撞网格层（admin 笔刷标 walkable/blocked，存 `maps.walkable` JSON 2D 数组）③ 装饰对象层（`decorations` 表，admin 网格点选放置）。三层解耦。 |
| **形象（avatar）** | **modular 捏脸**：发型/上衣/下装/鞋子 4 样式 × 配色，代码手绘。**accessories**：hand/back/hat/face 4 槽 + 手绘预设库（cap/wizard_hat/crown、glasses/mask/shades、sword/mug/staff、cape/wings/quiver）+ 装备 UI，generated 形象也走同一 overlay（D4）。**generated**：图片或文字 → 异步生成 4 方向 48px + walk 动画（4 方向 × 4 帧），非阻塞入场。 |
| **生成中心** | HUD「生成」入口，列当前用户所有生成 job（pending/running/done/failed 徽章 + 耗时 + 4 方向预览 + 应用）。非阻塞：提交后即可游玩，完成时通知。 |
| **管理台** | is_admin 才显示「管理」入口 → 全屏 DOM 控制台（非 Phaser）。tabs：成员（提升/降级/封禁/解封）、地图（walkable 笔刷 + 装饰点选 + 重生成背景）、酒单（CRUD）。 |
| **移动端横屏** | 设计第一原则：左虚拟摇杆 + 右动作键（DOM/CSS 实现，不进 Phaser 渲染层）+ 整数缩放 + `imageSmoothingEnabled=false`。桌面 WASD/方向键 + E 为回退。 |

---

## 二、技术栈

- **后端**：Rust + Axum 0.8 + SQLite（sqlx 0.8，WAL 模式）单二进制。argon2id 密码哈希。tokio 异步。
- **前端**：Phaser 4 + TypeScript + Vite。分层：`net/` `game/` `protocol/` 禁止 import phaser；`scene/` 只读状态、只调 net。
- **生成**：PixelLab v2 REST（角色 4 方向 + animate-character walk）+ MiniMax-M3 vision。token 只存后端。

---

## 三、快速开始

### 依赖

| 依赖 | 必需 | 备注 |
|---|---|---|
| Rust stable + cargo | ✅ | 编译后端单二进制 |
| Node.js ≥20 + npm | ✅ | 构建前端 `web/dist` |
| C 编译器 + make | ✅ | mac 自带 clang；Linux 装 `gcc gcc-c++ make`（给 libsqlite3-sys bundle 编 C） |
| `just`（或 make） | 可选 | 便捷启停；无则用原生命令 |

**不需要装**：OpenSSL（reqwest 用 rustls-tls）、SQLite/sqlite-devel（sqlx bundle libsqlite3-sys）、sqlx-cli（main.rs 启动自动跑 migration）、外部数据库服务（内嵌 SQLite）。

### 启动

```bash
# 1. 配置环境变量（生成功能需要，不配则生成禁用但其他功能正常）
export PIXELLAB_API_KEY=你的key      # https://www.pixellab.ai/pixellab-api
export MINIMAX_API_KEY=你的key       # https://platform.minimaxi.com（vision 用）

# 2a. 开发模式（HMR，功能最全）—— 两个终端
cd web && npm run dev                # vite :5173，/api 与 /ws 代理到 :8080
cargo run                            # 后端 :8080
# 浏览器开 http://localhost:5173

# 2b. 生产模式（单二进制，最接近部署态）
cd web && npm run build && cd ..
cargo run --release                  # 首次 LTO 编译约 3-5 分钟
# 浏览器开 http://localhost:8080
```

或有 `just` / `make`：`just dev` / `just build` / `just run` / `just test` / `just migrate`。

---

## 四、环境变量

| 变量 | 必需 | 默认 | 说明 |
|---|---|---|---|
| `PIXELLAB_API_KEY` | 可选 | — | 形象/地图生成；不设则生成禁用 |
| `MINIMAX_API_KEY` | 可选 | — | vision 图片→文字描述；不设则图片入口降级为手填文字 |
| `PORT` | 可选 | `8080` | 服务端口 |
| `DATABASE_URL` | 可选 | `sqlite:data/hoi.db` | SQLite 路径 |
| `ASSET_DIR` | 可选 | `data/assets` | 生成资源存储目录 |
| `STATIC_DIR` | 可选 | `web/dist` | 前端静态文件目录 |
| `ADMIN_KEY` | 可选 | `i am jaden` | 管理员注册密钥（代码常量，可改） |

> ⚠️ `.env` 不会被自动加载（`main.rs` 用 `std::env::var`，无 dotenv）。本地必须 `source .env` 或 export。生产用 systemd `EnvironmentFile`。

---

## 五、管理员

注册时填密钥 `i am jaden` → 该账号 `is_admin=1`，HUD 出现「管理」按钮 → 全屏管理台（成员/地图/酒单）。留空或填错 → 普通用户。

密钥只在后端常量比对（`src/lib.rs` `ADMIN_KEY`），不进前端代码。要换密钥改常量。首用户 fallback 保留：DB 为空时第一个注册的账号自动 admin（bootstrap）。

---

## 六、PixelLab 生成管线

**角色生成**（CLAUDE.md 主路径，端到端约 $0.062/角色）：

```
图片 → MiniMax-M3 vision 描述文字 → create-character-with-4-directions (48×48, 4 方向)
     → animate-character v3 (4 方向 × 4 帧 walk, action="walk")
     → 下载 PNG 进 AssetStore → frames:{dir:[key×4]} 契约
```

- **a 文字直接生成**：文字 → 4dir 48px + walk（`avatar-text` 入口）
- **b 图片生成**（默认）：图片 → MiniMax vision → text → 4dir 48px + walk（`avatar` 入口，主路径）
- 回退：MiniMax 429 → 用户手填文字 → 走 a

**地图背景生成**（D1，约 $0.009/张）：文字 → `create-image-pixen` 256×256 标准档（非标准档如 240×160 质量差）→ 前端缩放到 240×160。admin 地图 tab「重新生成背景」。

> 实测：PixelLab `create-tileset` REST 端点当前不存在（4xx），地图背景用 `create-image-pixen` 256 即可。可走网格是单独的 admin 笔刷层，不靠生成图自带碰撞。

---

## 七、项目结构

```
src/
  main.rs                  主服务入口（DB + migrate + serve :8080）
  lib.rs                   全部业务逻辑（handlers + router + AppState）
  pixelart/mod.rs          PixelLab REST 客户端 + MiniMax vision 包装
  assets.rs                AssetStore（存 storage_key 不存 URL）
  realtime/                WS 房间状态 + session
  bin/
    image2pixel.rs         CLI demo（balance/text/direct/vision/avatar/avatar-text/map/animate）
    pixelart_server.rs     旧测试 UI（:8081）

web/
  index.html               HUD + DOM overlay 样式
  src/
    main.ts                启动流程（登录 → 形象 → 游戏）
    scene/BarScene.ts      Phaser 场景（只读状态，渲染 + 发意图）
    ui/                    DOM overlay（login/chat/menu/avatarBuilder/avatarCreate/admin/touch）
    game/character.ts      角色装载层（modular + generated 统一，accessory overlay）
    game-state/            纯状态机（room/joystick/types，无 phaser）
    net/                   api + ws + transport（无 phaser）
    protocol/              消息类型

migrations/                SQL 迁移（0001 auth+avatar ~ 0007 map_walkable）
scripts/                   run.sh / stop.sh / hoi.service（systemd 模板）
docs/                      开发计划 + PixelLab API 实测 + image2pixel demo
.scratch/issues/           issue 跟踪（本机无 gh）
```

---

## 八、CLI demo（image2pixel）

底层二进制，复用主服务同款 PixelLab 客户端，便于批量测试。

```bash
export PIXELLAB_API_KEY=... MINIMAX_API_KEY=...

cargo run --bin image2pixel -- balance                          # 查余额（免费）
cargo run --bin image2pixel -- text "cozy tavern interior..." --size 256   # 文字→地图 256
cargo run --bin image2pixel -- avatar-text "small knight, green armor" --size 48  # 文字→48px 4方向角色
cargo run --bin image2pixel -- avatar photo.png --size 48       # 图片→vision→48px 4方向角色
cargo run --bin image2pixel -- map "tavern" --kind standard    # 地图（standard 256）
cargo run --bin image2pixel -- animate <character_id>          # 给已有角色加 walk 动画
```

输出落 `pixellab-out/`（`PIXELLAB_OUT` 覆盖）。成本：48px 角色 $0.0105 + walk 4×$0.0129 ≈ $0.062；地图 256 $0.0089。

---

## 九、部署（CentOS / Linux 长期运行）

```bash
# 1. 装依赖
sudo dnf install -y gcc gcc-c++ make git firewalld sqlite
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
curl -fsSL https://rpm.nodesource.com/setup_20.x | sudo bash - && sudo dnf install -y nodejs

# 2. 建专用用户 + 目录
sudo useradd -r -d /opt/hoi -s /usr/sbin/nologin hoi
sudo mkdir -p /opt/hoi/{data/assets,web/dist,backup} && sudo chown -R hoi:hoi /opt/hoi

# 3. 拉代码 + 构建
sudo -u hoi git clone <repo> /opt/hoi/src
cd /opt/hoi/src && sudo -u hoi bash -lc 'source ~/.cargo/env && cargo build --release'
cd web && sudo -u hoi npm install && sudo -u hoi npm run build && cd ..
sudo cp target/release/house-of-imbibe /opt/hoi/ && sudo cp -r web/dist /opt/hoi/web/dist

# 4. 密钥文件（systemd 格式，不带 export）
sudo tee /opt/hoi/hoi.env >/dev/null <<'EOF'
PIXELLAB_API_KEY=你的key
MINIMAX_API_KEY=你的key
EOF
sudo chmod 600 /opt/hoi/hoi.env && sudo chown hoi:hoi /opt/hoi/hoi.env

# 5. systemd unit（改进版见 scripts/hoi.service，补 ASSET_DIR + EnvironmentFile + hardening）
sudo cp scripts/hoi.service /etc/systemd/system/hoi.service  # 按实际改路径
sudo systemctl daemon-reload && sudo systemctl enable --now hoi
sudo firewall-cmd --add-port=8080/tcp --permanent && sudo firewall-cmd --reload
curl -sf http://localhost:8080/api/health && echo OK
```

运维：`journalctl -u hoi -f`（日志自动轮转）· SQLite 备份用 `sqlite3 ... ".backup '...'"` cron（WAL 模式不能直接 cp）· 升级 `git pull && build && cp 产物 && systemctl restart hoi`。

> 建议前面挂 nginx/caddy 做 TLS（WS 要透传 `Upgrade` 头），后端绑 `127.0.0.1:8080` 不直接暴露公网。

---

## 十、测试

```bash
just test                       # 或：cargo test --all-targets && cd web && npm run test
just check                      # cargo check + tsc --noEmit
just clippy                     # cargo clippy -D warnings
```

集成测试覆盖：WS 实时脊椎端到端、admin bootstrap（首用户 admin）、accessories equip/unequip、map walkable、decorations CRUD、menu、generation job。

---

## 十一、三条项目禁令

1. **禁止在 HTTP 请求路径上等待生成**（PixelLab 端到端 5-9 分钟，必须异步 job + 轮询）
2. **禁止把聊天写进任何表**（内存 ring buffer 50 条，绝不落库）
3. **禁止在 `scene/` 里出现 avatar `kind` 分支**（双管线渲染统一，只允许装载层分支）

---

## 十二、已知限制

- walk 动画帧 96×96、静态帧 68×68 canvas（PixelLab 自动 padding），`drawImage` 缩到 16×16 时 68→16 是 4.25 倍非整数，角色定位可能偏几像素。96→16 整数 6 倍 OK。
- 菜单 section 按字母序排（schema 只有 per-item `sort_order`，无 section 排序列）。
- `pixelart_server`（:8081）是早期测试 UI，已被主服务（:8080）管理台取代，保留仅作 demo。

---

## 十三、踩坑记录

- **migration 前必须 `DATABASE_URL`**：`just migrate` 或 `sqlx migrate run` 前要有该环境变量（头号踩坑）。但 `cargo run` 启动时 `main.rs` 自动跑 migration，不依赖 sqlx-cli。
- **`.env` 不自动加载**：`main.rs` 用 `std::env::var`，无 dotenv。本地 `source .env`，生产 systemd `EnvironmentFile`。
- **PixelLab live API 与 OpenAPI spec 三处不一致**（见 `src/pixelart/mod.rs` 注释）：image-to-pixelart 要 `image_size`+`output_size`；create-character 返回 `background_job_id` 单数；animate-character 字段是 `action_description` 不是 `animation_description`（后者 422）。
- **MiniMax-M3 emits `удш…` reasoning tokens**：`src/pixelart/mod.rs::strip_think()` 剥离。
