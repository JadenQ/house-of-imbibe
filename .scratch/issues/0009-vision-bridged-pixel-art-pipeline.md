---
id: 9
title: "Vision-bridged pixel-art pipeline (image → LLM → text → PixelLab)"
status: validated-by-demo
labels: [validated, spike, pixel-art, slice-4-input]
feature: house-of-imbibe
blocked_by: []
---

## TL;DR

Validated end-to-end on 2026-08-04: **`image → MiniMax-M3 vision → text description → PixelLab create-character-with-4-directions` produces a usable 4-direction character sprite in ~100 s for 1 generation (~$0.012).** This is dramatically simpler and cheaper than the multi-step path in `docs/reference/pixellab-api.md` §五 (portrait-pro + v3 + animate, 3 paid calls, 2–4 min, ~$0.19–0.30) and the quality is good enough to short-circuit the original "Spike-0" risk flagged in `docs/development-plan.md` §1.3.

A reusable CLI demo lives at `src/bin/image2pixel.rs` (`cargo run --bin image2pixel -- <subcommand>`). All 4 directions + supporting outputs in `pixellab-out/`.

## Parent

#1 - PRD (specifically slices #4 and #5)

## What this validates

1. **PixelLab `image-to-pixelart` is NOT actually restricted.** Live API accepts 512×512 input → 256×256 output fine (47 KB PNG). The "strict size requirements" the user noticed on the PixelLab web UI are a UI constraint, not an API constraint. Documented in `docs/reference/pixellab-api.md` §八 but worth re-confirming: input 16×16–1280×1280, output 16×16–320×320.

2. **Vision-bridged text-prompted generation works.** The new pipeline:
   ```
   image (512×512 PNG, 5 KB)
     → MiniMax-M3 vision (OpenAI-compatible /v1/chat/completions, ~5 s)
     → 1-paragraph visual description (≤80 words, focused on species/colors/pose)
     → + " Low top-down 3/4 view, pixel art character, GBA-era retro style."
     → PixelLab create-character-with-4-directions (async, 105 s in our test)
     → 4 × 92×92 RGBA PNGs (east/north/south/west)
   ```
   Total wall-clock 100–120 s, 1 generation.

3. **OpenAPI spec vs live API discrepancies** (worth updating `docs/reference/pixellab-api.md`):
   - `/v2/image-to-pixelart`: docs say `image_size`; live requires BOTH `image_size` AND `output_size`.
   - `/v2/create-character-with-4-directions`: docs say `background_job_ids` (array); live returns `background_job_id` (singular) + `character_id`.
   - `/v2/create-image-pixen`: docs list `model`, `negative_description`, `seed` as optional fields; live API rejects these with 422 even though the OpenAPI spec says they're allowed. Strip to just `description` + `image_size`.

4. **MiniMax-M3 emits `<think>…</think>` reasoning** before its answer. The demo strips these. For prod usage the same `strip_think()` logic should live in any wrapper around MiniMax.

## Why this matters for slice 4

The original slice 4 plan (from PRD + dev-plan §1.3) anticipated:
- `portrait-character-pro` ($0.095) → `create-character-v3` reference mode ($0.041) → 4 × `animate-character` v3 ($0.052) ≈ **$0.19–0.30 + 2–4 min + style-consistency risk**

The vision-bridged path costs:
- 1 × MiniMax-M3 vision (~$0.001–0.003, depends on plan)
- 1 × `create-character-with-4-directions` ($0.012 at 64×64) ≈ **$0.013–0.015 + ~100 s**

**Cost drop ~20×, time drop ~3×, no multi-step style-consistency risk** because the vision model can describe colors/proportions in one shot that the character model can render in one shot. Style consistency is bounded by the prompt + the `view`/`outline`/`shading`/`detail`/`proportions`/`template_id` params.

Animation can still be a separate `animate-character` call later (PRD §二.4 contract still applies), but that's a slice-3b concern, not a slice-4 blocker.

## Acceptance criteria (already met by the demo)

- [x] End-to-end pipeline runs from a single CLI invocation (`image2pixel avatar <image>`)
- [x] MiniMax-M3 produces usable descriptions (validated on synthetic red-panda image; output was descriptive, focused, and well within 80 words)
- [x] PixelLab character has 4 directions in `rotation_urls` (east/north/south/west)
- [x] Output PNGs are RGBA, transparent background, character fits inside ~64×64 area with padding to 92×92 (matches docs §五 footnote about automatic 2× padding)
- [x] Async polling works (`/v2/background-jobs/{id}` returns `processing` then `completed`)
- [x] Cost: 1 generation per character (within target)
- [x] Latency: ~100 s end-to-end (within PRD §二.6 "5–9 minute" envelope)

## Caveats / what we didn't validate

- **No real photo tested.** All runs were on a synthetic Pillow-generated cartoon red panda (512×512). Real photos may produce different-quality descriptions. Worth re-running on real human/animal photos before locking the slice 4 design.
- **MiniMax-M3 balance is unknown.** The vision API key works but we didn't load it with credits — the demo charges unknown amount per call (likely <$0.01).
- **Style fidelity to GBA emerald aesthetic is qualitative.** The output looks like a chibi character sprite, but visual A/B against a Pokemon Emerald sprite sheet is needed to confirm "GBA-style". Consider adding a `color_image` palette reference (GBA emerald palette) to lock colors.
- **Each direction is a single frame.** The 4-direction output has no walk animation yet. Slice 3b's `animate-character` step still applies; vision-bridged path doesn't replace it.
- **English-only descriptions tested.** MiniMax-M3 supports Chinese; if users upload Chinese-character-name photos the description quality is unverified.

## Files added

- `src/bin/image2pixel.rs` — CLI demo binary (Rust + reqwest + clap + base64). Modes: `balance`, `direct`, `text`, `vision`, `avatar`, `avatar-text`.
- `pixellab-out/pixon-256px.png` — text-mode test (orange tabby cat)
- `pixellab-out/direct-256px.png` — image-to-pixelart test (red panda → pixel art)
- `pixellab-out/avatar-test-red-panda-64px/{east,north,south,west}.png` — full pipeline output
- `docs/image2pixel-demo.md` — usage instructions, cost notes, integration plan

## Recommended next actions

1. **Slice 4 design change.** Adopt vision-bridged pipeline as the default for slice 4 photo avatars. Keep `portrait-character-pro` + `create-character-v3` as a documented fallback for users who specifically want identity preservation (and are willing to pay 20×).
2. **Add `color_image` reference.** Ship a fixed GBA-emerald palette PNG (~1 KB) and pass it as `color_image` with `force_colors: true` to every `create-character-with-4-directions` call. Likely improves visual consistency at zero ongoing cost.
3. **Validate on real photos.** Run `image2pixel avatar` against 5–10 real human + animal photos before freezing slice 4 design.
4. **Vision provider abstraction.** Same `submit/poll` trait pattern as `PixelLabClient` (§二.1 in dev-plan) — `VisionClient` trait with `MiniMaxVision` + `AnthropicVision` + stub impls. Don't bake MiniMax into the trait.
5. **Cost guard.** Per-user daily quota for vision calls (cheap but not free). Same `generation_jobs` table can host vision-call jobs since the async UX is identical.

## Blocked by

Nothing — the spike is done. Slice 4 (#5) can pick this up directly.

## Related

- `#5 Slice 4 - Generated (photo) avatar`
- `docs/reference/pixellab-api.md` §五 (path B — the multi-step approach this supersedes for most cases)
- `docs/development-plan.md` §1.3 (Spike-0, now satisfied)
- `docs/image2pixel-demo.md` (CLI usage)