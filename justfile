# House of Imbibe — 简洁启停 / 构建 / 测试（just）。dev-plan §六 D 要求固化 migrate 的 DATABASE_URL 前置。
# 无 just 时可 `make` 同名目标（见 Makefile），或直接看下面的命令。

# 默认：开发模式 — vite dev（:5173，HMR，/api 与 /ws 代理到 :8080）+ cargo run（:8080，API+WS+web/dist）
dev:
    cd web && npm run dev &
    trap 'kill %1 2>/dev/null' EXIT; cargo run

# 生产构建：前端 build → release 二进制
build:
    cd web && npm run build
    cargo build --release

# 生产运行：单二进制服务 :8080（API + WS + web/dist）
run:
    cargo run --release

# 全量测试（离线，无 API key）— 集成 harness 是切片1 主交付物
test:
    cargo test --all-targets
    cd web && npm run test

# 类型/编译检查（快速）
check:
    cargo check --all-targets
    cd web && npx tsc --noEmit

# lint
clippy:
    cargo clippy --all-targets -- -D warnings

# 显式数据库迁移（v2 文档标记的头号踩坑：必须先有 DATABASE_URL）
migrate:
    DATABASE_URL=sqlite:data/hoi.db sqlx migrate run
