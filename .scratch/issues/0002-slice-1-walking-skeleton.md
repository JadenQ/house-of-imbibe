---
id: 2
title: "Slice 1 - Walking skeleton + realtime spine"
status: ready-for-agent
labels: [ready-for-agent, slice]
feature: house-of-imbibe
blocked_by: []
---

## Parent

#1 - House of Imbibe - PRD

## What to build

A single Rust binary (Axum) serving a Vite + TypeScript/Phaser frontend that lets a visitor register with username + password, log in, and enter an empty placeholder room where they move a block avatar using a mobile left joystick or desktop arrows/WASD. Other online players appear and move with interpolation. Chat messages show as head bubbles and in a side panel (last ~50, in-memory). The integration-test harness (real Axum + temp SQLite + WS client) is established here and reused by every later slice. No PixelLab, no real avatar art - this is the tracer bullet that proves every layer.

## Acceptance criteria

- [ ] `cargo run` starts one binary serving API + WS + the built frontend on one port
- [ ] POST /api/auth/register {username, password} creates a user (argon2id hash); POST /api/auth/login sets a cookie session (tower-sessions); POST /api/auth/logout clears it; GET /api/me returns current user or 401
- [ ] First registered user (or an `ADMIN_USERNAME` env bootstrap) is promoted to admin role
- [ ] WS /ws/room: on connect the server sends a full snapshot; client sends {move, tx, ty} intents; server clamps to a walkable area and broadcasts a 10 Hz delta snapshot; client interpolates other players ~100-200 ms
- [ ] Chat: client sends {chat, text}; server rebroadcasts + keeps an in-memory ring buffer (last 50) per room; new joiners receive the buffer; nothing is persisted to the DB
- [ ] Frontend: Phaser boots in landscape, integer-scaled, `imageSmoothingEnabled=false`; left virtual joystick + right action button on touch; arrows/WASD + a key on desktop
- [ ] Integration test harness (established here, reused later): spins up the backend on a random port with a temp SQLite file, registers + logs in via HTTP, opens a WS, sends a move, asserts a delta snapshot frame is received, asserts chat is rebroadcast, asserts no chat rows exist in the DB
- [ ] No request blocks on external network; tests run fully offline

## Blocked by

None - can start immediately.
