# Makefile — just 的回退（同目标）。无 just 时用 `make dev` / `make test` 等。
.PHONY: dev build run test check clippy migrate

dev:
	cd web && npm run dev &
	trap 'kill %1 2>/dev/null' EXIT; cargo run

build:
	cd web && npm run build
	cargo build --release

run:
	cargo run --release

test:
	cargo test --all-targets
	cd web && npm run test

check:
	cargo check --all-targets
	cd web && npx tsc --noEmit

clippy:
	cargo clippy --all-targets -- -D warnings

migrate:
	DATABASE_URL=sqlite:data/hoi.db sqlx migrate run
