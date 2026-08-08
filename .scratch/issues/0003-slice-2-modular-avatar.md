---
id: 3
title: "Slice 2 - Avatar model + Modular avatar"
status: ready-for-agent
labels: [ready-for-agent, slice]
feature: house-of-imbibe
blocked_by: [2]
---

## Parent

#1 - House of Imbibe - PRD

## What to build

The avatar data model with a `kind` discriminator, and the **Modular** kind end-to-end. Members choose a default avatar on first entry, then build a modular avatar from curated preset parts (hair, outfit, accessory silhouettes) shipped as PNG assets, with recoloring. The active avatar's layered composite renders in the room and is broadcast to others. No PixelLab in this slice - presets only.

## Acceptance criteria

- [ ] Schema: `avatars(id, owner_id, kind, layers_json NULL, sprite_asset_id NULL, is_active, created_at)`; migrations run on boot
- [ ] GET /api/avatar returns the member's active avatar (or assigns/returns a default on first call); PUT /api/avatar updates a modular avatar's `layers_json`; presets listed via GET /api/avatar/presets
- [ ] Modular avatar renders as a layered PNG composite (body/hair/outfit/accessory) conforming to the canonical 8-direction sprite-sheet contract; recoloring via HSL shift on recolorable layers
- [ ] The active avatar snapshot is part of the realtime player state and broadcast on change; other clients render the composite
- [ ] Avatar builder UI: pick hair/outfit/accessory + recolor, live preview, save -> sets avatar active
- [ ] Integration test: register -> create a modular avatar -> connect WS -> assert avatar snapshot present in the snapshot frame -> update avatar -> assert a delta reflects the change

## Blocked by

- #2 - Slice 1 (walking skeleton; needs auth, realtime, test harness)
