---
id: 8
title: "Slice 7 - Deploy + ops foundation"
status: ready-for-agent
labels: [ready-for-agent, slice]
feature: house-of-imbibe
blocked_by: [7]
---

## Parent

#1 - House of Imbibe - PRD

## What to build

Productionize the single binary on a Hetzner VPS behind Caddy (auto TLS) under systemd, with the backend serving the built frontend. Env-based config, structured logging (tracing), and a SQLite backup routine. Not visual polish (CRT/BGM are a later phase).

## Acceptance criteria

- [ ] `cargo build --release` produces one binary; the binary serves `./dist` (frontend) + API + WS on one port
- [ ] Caddyfile + systemd unit provided; Caddy terminates TLS and reverse-proxies to the binary; WebSocket upgraded correctly
- [ ] Config via env vars (DATABASE_URL, PUBLIC_BASE_URL, SESSION_KEY, PIXELLAB_KEY, ADMIN_USERNAME, etc.)
- [ ] `tracing` structured logs to stdout with basic request/error spans
- [ ] SQLite backup script (periodic `.backup` to local + optional object_store upload)
- [ ] Deployment runbook (provision + deploy commands) committed to the repo

## Blocked by

- #7 - Slice 6 (full feature set to productionize)
