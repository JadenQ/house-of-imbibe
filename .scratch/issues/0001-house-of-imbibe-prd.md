---
id: 1
title: "House of Imbibe - Product Requirements"
status: ready-for-agent
labels: [ready-for-agent, prd]
feature: house-of-imbibe
blocked_by: []
---

# House of Imbibe - PRD

## Problem Statement

I want a shared, social space on the web where friends drop in as little
pixel characters, wander a cozy bar, and chat - the way *Pokémon Emerald*
felt on a GBA. Existing social-game tools are either too heavy to self-host,
lock me into someone else's art style, or don't let me turn real photos into
in-world pixel sprites. As a solo builder with limited art and time, I need a
lightweight foundation I can vibe-code with AI assistance, where members make
their own pixel avatars (from presets or their own photos), the host can
redecorate the room live, and a bartender NPC hands them a drink menu - all in
one small, self-contained web service I fully control.

## Solution

A single-binary web service: a Rust (Axum) backend serving a TypeScript +
Phaser 4 frontend, talking over REST (auth, library, admin, generation) and
WebSocket (realtime room). One bar scene (interior + yard) hosts 30-50
concurrent members. Members register with a username + password, build an
avatar through one of two independent pipelines - **Modular** (preset
hair/outfit/accessory parts) or **Generated** (a single image from an uploaded
photo via PixelLab) - both conforming to the same sprite-sheet animation
contract so the renderer treats them alike. PixelLab generation runs
asynchronously into a per-member asset library, so members are never blocked
waiting. An admin can add/remove decoration objects onto a static Tiled base
map at runtime, with changes streaming live to everyone online. Two scripted
bartender NPCs behind the bar hand out a (placeholder) drink menu on
interaction. Mobile is the primary target (landscape, left joystick + right
button); desktop keyboard is a fallback.

## User Stories

1. As a visitor, I want to register with just a username and password, so that I can get in without handing over an email.
2. As a member, I want to log in and stay logged in across browser sessions, so that I don't have to re-enter credentials constantly.
3. As a member, I want my password stored securely, so that a database leak doesn't expose it.
4. As a member, I want to log out from any device, so that I can end a shared-computer session.
5. As a member, I want to pick a default placeholder avatar immediately on first login, so that I can enter the room before customizing anything.
6. As a member, I want to build a Modular avatar by choosing base hair, outfit, and accessories from preset pixel parts, so that I look the way I want without drawing.
7. As a member, I want to recolor preset parts (hair, outfit), so that two people with the same silhouette still look distinct.
8. As a member, I want to upload a photo and have it turned into a Generated avatar via PixelLab, so that my in-world character actually resembles me.
9. As a member, I want generation to happen in the background, so that I can leave the screen and come back later instead of staring at a spinner.
10. As a member, I want a personal asset library showing my generated avatars and accessories with their status (pending/done/failed), so that I can find and use them later.
11. As a member, I want to generate an accessory (e.g. a backpack, a mug) via PixelLab, so that I can wear custom items.
12. As a member, I want to equip an accessory onto my Modular avatar's back or hand, so that the item shows on my character in the room.
13. As a member, I want my chosen avatar and equipped accessories shown to other members in real time, so that everyone sees my current look.
14. As a member, I want to enter the bar interior scene after login, so that I'm immediately in the social space.
15. As a member, I want to walk out to the yard outside the bar, so that I can hang out in a different area.
16. As a member on mobile, I want a left-side virtual joystick to move and a right-side action button, so that I can play one-handed in landscape.
17. As a member on desktop, I want to move with arrow keys or WASD and trigger actions with a key, so that I'm not forced onto touch.
18. As a member, I want my character to face 8 directions and animate while walking, so that movement reads as natural, GBA-style.
19. As a member, I want to see other members' characters move smoothly in real time, so that the room feels alive even under 30-50 concurrency.
20. As a member, I want to send a chat message that appears as a bubble over my head, so that I can talk to people near me.
21. As a member, I want a side panel showing the most recent ~50 chat lines, so that I can scroll back what I just missed (no permanent history required).
22. As a member, I want to walk up to the bar and tap a bartender NPC, so that I can interact with the counter.
23. As a member talking to a bartender, I want to see the drink menu, so that I know what's on offer (menu content/design TBD; placeholder data for now).
24. As a member, I want bartender dialogue to feel responsive and instant, so that I'm not waiting on an LLM (scripted responses only for MVP).
25. As an admin, I want to enter an "edit mode" and place decoration objects (chairs, plants, signs) onto the map, so that I can redecorate the room live.
26. As an admin, I want to remove decorations I or others placed, so that I can tidy up.
27. As an admin, I want my decoration changes to appear instantly for everyone online, so that redecoration is a shared, live experience.
28. As an admin, I want to manage members (view list, promote/demote admin, ban), so that I can moderate the space.
29. As a builder, I want the whole service to be one deployable binary plus a SQLite file, so that hosting costs ~€5/month and ops are trivial.
30. As a builder, I want the PixelLab dependency isolated behind one interface, so that I can stub it in tests and swap providers without touching game logic.
31. As a builder, I want the asset store abstracted (local disk now, R2 later), so that I can migrate storage without changing business code.
32. As a builder, I want the public HTTP + WebSocket contract to be the primary tested boundary, so that I can verify end-to-end behavior with one seam.
33. As a builder, I want the frontend networking and game-state in plain, testable TS modules decoupled from Phaser, so that logic is verifiable without a canvas.

## Implementation Decisions

### Stack (locked)

- **Frontend**: Phaser 4 + TypeScript + Vite. Logical resolution 240×160, integer-scaled, `imageSmoothingEnabled=false`.
- **Backend**: Rust, Axum 0.8, SQLite in WAL mode, sqlx 0.8 (compile-time-checked queries). Single binary serves API + WS + the built frontend (`tower_http::services::ServeDir` on `./dist`).
- **Realtime**: Axum WebSocket upgrade + `tokio::sync::broadcast` per room + `DashMap` for room/player state. **Server-authoritative position clamp**; clients send movement intent, server ticks a 10 Hz delta snapshot, clients interpolate (~100-200 ms).
- **Auth**: username + password only (no email, no email verification). `argon2id` hashing (OWASP params). `tower-sessions` cookie sessions. A `role` column on `users` (`member` | `admin`); first registered user is seeded admin, or an env-flag bootstrap admin.
- **Generation**: PixelLab.ai via its official MCP/API (`create_character`, `create_image_pixflux`, `create_map_object`, `animate_character`). Called from a background tokio task; never on the request path.
- **Storage**: `object_store` crate behind a trait; `LocalFileSystem` at MVP, swappable to Cloudflare R2 with no business-code change.
- **Deploy**: single static binary + SQLite file on a Hetzner VPS, behind Caddy (auto TLS), managed by systemd. ~€5/month.

### Avatar dual pipeline (core data model)

Avatars have a discriminator `kind`:

- **`modular`**: stores a `layers` JSON describing preset base parts (hair id + color, outfit id + color, body, etc.) **and** an `equipped` list of accessory references with slots. Rendered by compositing layered PNGs at runtime.
- **`generated`**: stores a single `sprite_asset_id` (one PNG sprite sheet produced by PixelLab from an uploaded photo). No preset parts, no per-part editing.

Both `modular` and `generated` avatars conform to the **same canonical sprite-sheet contract** - 8 directions × the same animation set (idle, walk, …) on the same grid layout - so the Phaser renderer treats them identically. **Modular and generated avatars do not interoperate**: a generated avatar cannot have preset parts swapped, and accessories attach only to modular avatars. This keeps each pipeline simple rather than forcing a brittle unification.

Accessory equipping (back / hand slots) is a runtime overlay composition: the accessory sprite is a transparent PNG drawn at a defined anchor per slot, on top of the modular base, per animation frame. Accessories are **not** equipable onto generated avatars (per the non-interoperability decision).

### Realtime room model

- One room per server (MVP). State: `players: Map<id, {x, y, dir, avatar_snapshot, name}>`.
- Inbound client messages: `{type:"move", tx, ty}` (target tile intent), `{type:"chat", text}`, `{type:"interact", target}`.
- Server clamps movement to walkable tiles (from the Tiled collision layer), updates state, broadcasts a **delta snapshot** at 10 Hz (only changed players). New connections get a full snapshot on join.
- Chat is fire-and-forget: server rebroadcasts to the room and keeps an in-memory ring buffer (last 50) per room for late-joiners and the side panel. **Chat is never persisted to the DB.**

### Map & decorations

- **Base map**: a static Tiled `.tmj` (JSON) exported offline, shipped as an asset. Includes collision/walkable layers.
- **Decorations**: runtime, DB-backed. `decorations` rows carry `{scene, tile_x, tile_y, asset_id, z_layer, placed_by}`. Admin mutates via REST (`POST`/`DELETE`); the server broadcasts `{type:"decoration_added"|"decoration_removed"}` over WS so online clients update live. Decoration assets come from the admin's own asset library (generated via PixelLab `create_map_object`) or a curated set.

### Bartender NPCs

- Two NPCs at fixed positions behind the bar, defined by an `npc_def` (id, sprite, dialogue tree as JSON). Scripted state machine only - no LLM in MVP.
- Interaction: proximity + action button -> client sends `{type:"interact", target: npc_id}` -> server returns the current dialogue node. A node may open the **drink menu** (a data-driven JSON payload; concrete menu design is TBD and out of scope here - the PRD delivers the interface and placeholder data).

### Generation async UX

- `POST /api/generate` with `{kind: avatar_generated | accessory, params}` creates a `generation_jobs` row (`status: pending`), returns `{job_id}` immediately.
- A background worker (same process) picks pending jobs, calls PixelLab, stores the result as an `assets` row, marks the job `done` (or `failed`).
- Client reads status via `GET /api/jobs/:id` and lists owned assets via `GET /api/library`. The library page is the "come back later" surface - no blocking spinner.

### Schema (decision-rich sketch; from v2 research, adapted: no email)

```sql
users(id TEXT PK, username TEXT UNIQUE, password_hash TEXT, role TEXT, created_at TEXT)
avatars(id TEXT PK, owner_id TEXT FK, kind TEXT,        -- 'modular' | 'generated'
        layers_json TEXT NULL,                          -- modular only
        sprite_asset_id TEXT NULL,                      -- generated only
        is_active INTEGER, created_at TEXT)
assets(id TEXT PK, owner_id TEXT FK, kind TEXT,          -- 'sprite_sheet'|'accessory'|'decoration'
       storage_key TEXT, meta_json TEXT, created_at TEXT)
generation_jobs(id TEXT PK, owner_id TEXT FK, kind TEXT, status TEXT,
                params_json TEXT, result_asset_id TEXT NULL, error TEXT NULL,
                created_at TEXT, completed_at TEXT NULL)
decorations(id TEXT PK, scene TEXT, tile_x INT, tile_y INT, asset_id TEXT FK,
            z_layer INT, placed_by TEXT FK, created_at TEXT)
npcs(id TEXT PK, scene TEXT, npc_def_id TEXT, x INT, y INT)
-- chat is intentionally NOT persisted; held in-memory ring buffer per room.
```

`tower-sessions` manages its own session store table.

### Modules to build (seams respected)

- `auth` - registration, login, logout, session, role gating.
- `assets` - the `object_store`-backed asset blob store + metadata; `library` queries.
- `generation` - the `PixelLabClient` trait (real + stub impls), the job worker, the `generation_jobs` lifecycle.
- `avatars` - create/update/activate, dual-kind validation, equipped-accessory composition config.
- `realtime` - WS handler, room state, 10 Hz delta tick, chat ring buffer, decoration live-sync.
- `maps` - load static `.tmj`, serve walkable/collision data, decoration CRUD + broadcast.
- `npcs` - scripted dialogue tree engine + menu payload interface.
- `admin` - member management + decoration edit endpoints, role-gated.
- Frontend: `net` (WS + REST client), `game-state` (room state mirror + interpolation), `phaser-scene` (rendering + input), `ui` (joystick, buttons, chat panel, library, avatar builder).

## Testing Decisions

- **Good test = tests external behavior through the public contract, never internals.** No mocking of our own modules; we drive the real Axum app on a random port and assert on JSON / WS frames.
- **Primary seam: the HTTP + WebSocket contract.** One integration test harness spins up the whole backend (real SQLite on a temp file, real Axum router, real WS) and exercises auth, library, generation (via the **stub** PixelLab client), avatars, decorations, NPC dialogue, and room sync end-to-end. This is the seam `to-issues` acceptance criteria target.
- **PixelLab is isolated behind a `PixelLabClient` trait.** Tests inject a deterministic stub that returns canned sprite bytes and resolves jobs synchronously, so generation paths are tested without network or cost. The real impl gets a manual smoke test only.
- **Frontend `net` + `game-state` modules are plain TS, unit-tested** against the same message contracts (no Phaser, no DOM). Phaser rendering is not unit-tested; visual correctness is covered by Playwright screenshots + the art-director review loop (per the v2 workflow), not by automated assertion.
- Prior art in this repo: none yet (greenfield). The harness established by the first slice becomes the pattern all later slices reuse.

## Out of Scope

- Email verification, password reset via email, third-party/OAuth login.
- Persisting chat history to the database (in-memory ring buffer only).
- LLM-driven NPC dialogue (scripted state machine only for MVP).
- Multiple sharded rooms / room selection (one room per server).
- Fully runtime-editable terrain tiles (only DB-backed decoration objects on a static base map).
- Equipping accessories onto *generated* avatars, or converting between modular and generated avatars.
- The final drink-menu visual design and content (interface + placeholder data delivered; content TBD).
- CRT/LCD post-processing shaders, BGM/SFX, and polish (later phase).
- Native mobile apps (web only, mobile-first responsive).

## Further Notes

- **Sustainability guardrails**: every slice must keep the PixelLab dependency behind the trait, the asset store behind `object_store`, and the frontend logic out of Phaser - so later slices (R2 migration, a second scene, LLM NPC upgrade, post-processing) are additive, not rewrites.
- **Lightweight mandate**: no ORMs beyond sqlx, no Redis, no separate worker process, no message queue - one binary, one SQLite file, one background tokio task for generation. If a proposed slice needs more, it's out of scope for MVP.
- **Generation UX**: the async job + library is the explicit answer to "avoid long waits"; no slice may introduce a blocking generation request on the HTTP path.
- **Bootstrap admin**: to avoid a locked-out first deploy, the first registered user (or an `ADMIN_USERNAME` env bootstrap) is promoted to admin; documented in the auth slice.
- Prior research: `docs/pixel-mosaic-game-workflow-v2-rust.md` (full stack), `docs/物理世界转像素游戏素材-调研报告.md` (PixelLab), `docs/research/09-13` (Rust backend).

## Child issues

- #2 - Slice 1 - Walking skeleton + realtime spine (blocked by: none)
- #3 - Slice 2 - Avatar model + Modular avatar (blocked by: #2)
- #4 - Slice 3 - PixelLab generation + asset library + accessories (blocked by: #3)
- #5 - Slice 4 - Generated (photo) avatar (blocked by: #3, #4)
- #6 - Slice 5 - Bar + yard scenes + scripted bartender NPC + menu interface (blocked by: #2)
- #7 - Slice 6 - Admin: live decoration editing + member management (blocked by: #4, #6)
- #8 - Slice 7 - Deploy + ops foundation (blocked by: #7)

Dependency graph: `1(skeleton) -> {2(modular), 5(scenes)}` parallel; `2 -> 3(generation) -> 4(generated-avatar)`; `3 + 5 -> 6(admin) -> 7(deploy)`.
