---
id: 6
title: "Slice 5 - Bar + yard scenes + scripted bartender NPC + menu interface"
status: ready-for-agent
labels: [ready-for-agent, slice]
feature: house-of-imbibe
blocked_by: [2]
---

## Parent

#1 - House of Imbibe - PRD

## What to build

Replace the placeholder room with the real bar interior scene (Tiled base map with collision/walkable layers) plus a yard scene outside the bar, with transitions. Place two bartender NPCs at fixed positions behind the bar. Walking near a bartender and pressing action opens a scripted dialogue (JSON dialogue tree); a dialogue node can open the drink menu (data-driven JSON payload; concrete menu design is TBD - placeholder data only).

## Acceptance criteria

- [ ] Static Tiled `.tmj` for bar interior + yard, shipped as assets, with collision/walkable layers loaded by both server (movement clamp) and client (rendering)
- [ ] Scene transitions: walking through the bar door moves the player between bar and yard scenes
- [ ] Two bartender NPCs at fixed bar positions; `npc_def` dialogue trees as JSON
- [ ] {interact, npc_id} returns the current dialogue node; a node may carry a menu payload
- [ ] Menu is an interface + placeholder JSON data; UI renders the placeholder menu (final design deferred)
- [ ] Integration test: load bar scene -> assert walkable tiles respected -> move near NPC -> interact -> assert dialogue node returned -> follow a node that opens the menu -> assert menu payload present

## Blocked by

- #2 - Slice 1 (walking skeleton; needs room + movement + WS)
