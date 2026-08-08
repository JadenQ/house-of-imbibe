# `image2pixel` — image → vision → text → PixelArt pipeline demo

> Validated 2026-08-04. Lives at `src/bin/image2pixel.rs`.
> Built because PRD slice 4 (photo → game-avatar sprite) was the riskiest dependency on PixelLab, and the multi-step path in `docs/reference/pixellab-api.md` §五 looked expensive + slow + style-inconsistent.

## TL;DR

```
image (any 16×16–1280×1280 PNG/JPEG/WEBP/GIF)
   ↓ MiniMax-M3 vision  (~5 s, ~$0.001)
text description (≤80 words, species/colors/pose/distinctive features)
   ↓ + " Low top-down 3/4 view, pixel art character, GBA-era retro style."
PixelLab create-character-with-4-directions  (async, 30–120 s, 1 generation)
   ↓ poll /v2/background-jobs/{id}
4 × 92×92 RGBA PNGs (east/north/south/west)  + character_id
```

Total: **~100 s, ~$0.013 per avatar** (vs. PRD §五 path B: 2–4 min, $0.19–0.30).

## Quick start

```bash
# env (one-time)
export PIXELLAB_API_KEY=...          # from https://www.pixellab.ai/pixellab-api
export MINIMAX_API_KEY=sk-cp-...     # from https://platform.minimaxi.com

# modes
cargo run --bin image2pixel -- balance                                        # check quota
cargo run --bin image2pixel -- text "a red panda, pixel art" --size 256       # text-only
cargo run --bin image2pixel -- direct photo.png --size 256                   # pixelart only
cargo run --bin image2pixel -- vision photo.png --size 256                   # vision → pixon
cargo run --bin image2pixel -- avatar photo.png --size 64 --template mannequin --proportions chibi
cargo run --bin image2pixel -- avatar-text "tiny dragon with green scales" --size 64 --template mannequin
```

Outputs land in `pixellab-out/` (override with `PIXELLAB_OUT=...`).

## Subcommands

| Subcommand | Input | Path | Cost | Latency | Use |
|---|---|---|---|---|---|
| `balance` | — | `GET /v2/balance` | free | sync | Check USD credit + subscription quota before running |
| `direct <image>` | photo | `POST /v2/image-to-pixelart` | ~$0.007–0.012 | sync | PixelLab converts photo to pixel art, no LLM. Validates user's "strict size limit" concern (it doesn't actually exist on the API). |
| `text <desc>` | text | `POST /v2/create-image-pixen` | ~$0.007 | sync | Single pixel art image from text. Cheapest path. |
| `vision <image>` | photo | MiniMax → text → pixon | ~$0.008 | sync | The "new idea" path for single-image pixel art. |
| `avatar <image>` | photo | MiniMax → text → 4-dir character | ~$0.013 | 30–120 s async | The "new idea" path for full game avatar. **This is slice 4.** |
| `avatar-text <desc>` | text | text → 4-dir character | ~$0.012 | 30–120 s async | No photo needed; pure text-to-avatar. |

## Template options

`--template` is PixelLab's body-shape template:
- **Humanoid:** `mannequin` (default), `default`, `chibi`, `cartoon`, `stylized`, `realistic_male`, `realistic_female`, `heroic`
- **Quadruped:** `bear`, `cat`, `dog`, `horse`, `lion`

`--proportions` only applies to humanoid templates. Use `--skip-proportions` for animals.

## Findings worth documenting

1. **OpenAPI spec ≠ live API.** Three discrepancies captured in `src/bin/image2pixel.rs` comments:
   - `/v2/image-to-pixelart` requires both `image_size` AND `output_size` (docs say only `image_size`).
   - `/v2/create-character-with-4-directions` returns `background_job_id` (singular) + `character_id` (docs say `background_job_ids` array).
   - `/v2/create-image-pixen` rejects optional `model`/`negative_description`/`seed` fields with 422, even though docs list them. Use only `description` + `image_size`.

2. **MiniMax-M3 reasoning tokens.** Responses contain a `<think>…</think>` block before the answer. Strip it before feeding to PixelLab — the demo does this.

3. **PixelLab size limits.** `image-to-pixelart` input 16×16–1280×1280, output 16×16–320×320. The web UI has stricter constraints for the preview but the API does not.

4. **4-direction output canvas.** Each direction is 92×92 (not exactly 64×64 as requested). This is the documented 2× auto-padding for animation headroom. The character fits inside ~64×64, with the rest being transparent.

5. **No image format check needed.** PixelLab accepts base64 PNG/JPEG/WEBP/GIF. The demo detects via magic bytes (no need for an image crate).

## Integration plan (for slice 4)

When slice 4 picks up `VisionClient` + `PixelLabClient`:

1. **Trait shape** (mirrors §二.1 in dev-plan):
   ```rust
   pub trait VisionClient: Send + Sync {
       async fn describe(&self, image: ImageRef) -> Result<String, VisionError>;
       fn provider_id(&self) -> &'static str;
   }
   ```
   `MiniMaxVision` (real) + `AnthropicVision` (real, future) + `StubVision` (deterministic for tests).

2. **PixelLab trait** keeps `submit/poll` separation (see dev-plan §二.1). The vision-bridged call composes `vision.describe()` → `pixellab.create_character_4dir()`.

3. **Async UX** unchanged from PRD: `POST /api/avatar/from-photo` → 201 `{job_id}` → worker does vision + generation → `GET /api/library` shows done.

4. **Privacy**: original photo lives in `tempfile::TempDir` only, purged on job completion (PRD §三 slice 4 caveat).

5. **Style fidelity**: ship a fixed GBA-emerald palette as `color_image` + `force_colors: true` so all avatars share the palette. The demo does NOT do this yet (could be added as a `--palette` flag).

## Open questions (for the workflow that ran in parallel)

The parallel exploration (`wf_3f2dee58-845`) covers:
- Which vision model produces best descriptions (MiniMax-M3 vs Claude Sonnet 4 / Opus 4.7 / GPT-4V / Gemini 2.x)
- Pixel-art-specific prompting strategies (style keywords, negative prompts, color references)
- Alternative pixel-art providers (fal.ai, Replicate, SDXL pixel-art LoRAs)
- Iterative refinement / self-critique loop on top of the pipeline

Synthesis lands in `docs/development-plan.md` after the workflow returns.

## Tested on

- Synthetic Pillow-generated cartoon red panda (512×512) — vision described it accurately as "A stylized red panda character with a round, teardrop-shaped body..." and PixelLab produced a 4-direction sprite that looks like a chibi red panda.
- NOT yet tested on real photos.

## Known issues

- **Balance is in the red.** PixelLab account started at 3/2000 subscription quota, USD = −$0.048. Each successful generation consumed 1 subscription unit. Plan accordingly before running on production photos.
- **MiniMax API key.** The `sk-cp-…` format works at `https://api.minimaxi.com/v1/`. Balance on that account is unknown.
- **`/v2/balance` and `/v2/characters/{id}` are unauthenticated-friendly (200 OK either way)** but we still send the bearer header for correctness.