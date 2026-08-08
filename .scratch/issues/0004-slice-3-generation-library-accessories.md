---
id: 4
title: "Slice 3 - PixelLab generation + asset library + accessories"
status: ready-for-agent
labels: [ready-for-agent, slice]
feature: house-of-imbibe
blocked_by: [3]
---

## Parent

#1 - House of Imbibe - PRD

## What to build

The generation + library infrastructure and the first generated artifact type: **accessories**. A `PixelLabClient` trait (real impl calling PixelLab MCP/API + a deterministic stub for tests), `object_store`-backed assets, a `generation_jobs` table, and a background worker. Members request an accessory, get a job id immediately, and later find the result in their personal library; they can equip an accessory onto their modular avatar's back or hand as a runtime overlay.

## Acceptance criteria

- [ ] `PixelLabClient` trait with real + stub impls; real impl behind an env-configured key; stub returns canned sprite bytes and resolves synchronously
- [ ] Schema: `assets(id, owner_id, kind, storage_key, meta_json, created_at)`, `generation_jobs(id, owner_id, kind, status, params_json, result_asset_id, error, created_at, completed_at)`; `object_store` `LocalFileSystem` at MVP
- [ ] POST /api/generate {kind: accessory, params} creates a pending job and returns {job_id} immediately (non-blocking); a background worker calls PixelLab, stores the result asset, marks the job done/failed
- [ ] GET /api/jobs/:id returns status; GET /api/library lists the member's assets + their job status (pending/done/failed)
- [ ] Accessory equipping: equip an owned accessory onto the active modular avatar in a back/hand slot; stored in the avatar's `layers_json` equipped list; rendered as a transparent overlay at the slot anchor per frame
- [ ] Library UI: lists assets with status; "come back later" pattern - no blocking spinner on the generate request; equip button for accessories
- [ ] Integration test using the stub PixelLab client: request accessory -> poll job -> done -> appears in library -> equip -> assert overlay present in the avatar composite
- [ ] Asset store behind the `object_store` trait so LocalFileSystem can be swapped to R2 with no business-code change (verified by a second store impl in tests)

## Blocked by

- #3 - Slice 2 (modular avatar; accessories equip onto it)
