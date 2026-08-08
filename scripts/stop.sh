#!/usr/bin/env bash
# 生产停止：按 pidfile kill。
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ -f data/hoi.pid ]]; then
  PID="$(cat data/hoi.pid)"
  kill "$PID" 2>/dev/null || true
  rm -f data/hoi.pid
  echo "stopped (pid $PID)"
else
  echo "no pidfile (data/hoi.pid); nothing to stop"
fi
