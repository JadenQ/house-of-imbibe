#!/usr/bin/env bash
# 生产启动：单二进制，pidfile 记录便于 stop。日志写 data/hoi.log。
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p data
PORT="${PORT:-8080}"
DATABASE_URL="${DATABASE_URL:-sqlite:data/hoi.db}"
echo "starting house-of-imbibe on :$PORT (log: data/hoi.log)"
PORT="$PORT" DATABASE_URL="$DATABASE_URL" nohup cargo run --release > data/hoi.log 2>&1 &
echo $! > data/hoi.pid
echo "started (pid $(cat data/hoi.pid))"
