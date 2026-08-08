---
id: 7
title: "Slice 6 - Admin: live decoration editing + member management"
status: ready-for-agent
labels: [ready-for-agent, slice]
feature: house-of-imbibe
blocked_by: [4, 6]
---

## Parent

#1 - House of Imbibe - PRD

## What to build

Admin role gating + runtime decoration editing on scenes + member management. An admin enters an edit mode, places/removes decoration objects (from the admin's generated asset library or a curated set) onto the bar/yard at tile coords; changes persist to DB and broadcast live to online clients. Admin can list members and promote/demote/ban.

## Acceptance criteria

- [ ] `decorations(id, scene, tile_x, tile_y, asset_id, z_layer, placed_by, created_at)` table; admin-only REST CRUD
- [ ] On decoration add/remove, the server broadcasts {decoration_added} / {decoration_removed} so online clients update without reload
- [ ] Admin edit-mode UI: pick a decoration asset, click a tile to place, click a placed decoration to remove; live for everyone
- [ ] Member management: GET /api/admin/members; POST promote/demote; POST ban (login disabled)
- [ ] All admin endpoints reject non-admins (403)
- [ ] Integration test: as admin, place a decoration via REST -> open a second WS client -> assert it receives `decoration_added` -> remove -> assert `decoration_removed`; as a member, assert admin endpoints return 403

## Blocked by

- #4 - Slice 3 (generation + library; decoration assets come from here)
- #6 - Slice 5 (scenes; decorations are placed on bar/yard)
