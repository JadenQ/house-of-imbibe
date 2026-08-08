---
id: 5
title: "Slice 4 - Generated (photo) avatar"
status: ready-for-agent
labels: [ready-for-agent, slice]
feature: house-of-imbibe
blocked_by: [3, 4]
---

## Parent

#1 - House of Imbibe - PRD

## What to build

Extend the generation pipeline to produce a full avatar from an uploaded photo, conforming to the same canonical 8-direction sprite-sheet contract as modular avatars. The member activates the generated avatar; it renders in the room. Generated avatars cannot equip accessories - non-interoperability is enforced at the API.

## Acceptance criteria

- [ ] POST /api/generate {kind: avatar_generated, photo, params} enqueues a job using PixelLab `create_character` / image tools to produce a sprite sheet conforming to the canonical contract
- [ ] On completion, an `avatars` row of `kind='generated'` with `sprite_asset_id` is created in the member's library
- [ ] Member can set a generated avatar active; it renders in the room via the same renderer path as a modular avatar
- [ ] Equipping accessories onto a generated avatar is rejected (400) - non-interoperability enforced
- [ ] The uploaded photo is used only for generation; the original photo is not persisted beyond the job (privacy)
- [ ] Integration test (stub PixelLab): upload photo -> job done -> generated avatar in library -> activate -> assert rendered; assert equip-on-generated returns 400

## Blocked by

- #3 - Slice 2 (avatar model)
- #4 - Slice 3 (generation + library infrastructure)
