//! `image2pixel` — image → (LLM vision) → text → PixelLab pixel-art pipeline demo (CLI).
//!
//! Modes
//! ──────
//!   balance                          query PixelLab balance & subscription quota
//!   direct    <image>                POST /v2/image-to-pixelart          (no LLM)
//!   text      <description>          POST /v2/create-image-pixen         (text only)
//!   vision    <image>                MiniMax-M3 → text → pixon           (NEW idea)
//!   avatar    <image>                MiniMax-M3 → text → 4-dir character (NEW idea, async)
//!   avatar-text <description>        text → 4-dir character              (async)
//!   map      <prompt>                text → map background / tileset      (sync)
//!   animate  <character_id>         4-dir walk animation for existing char (async)
//!
//! Env
//! ───
//!   PIXELLAB_API_KEY   required
//!   MINIMAX_API_KEY    required for `vision` / `avatar`
//!   PIXELLAB_OUT       output dir, default ./pixellab-out
//!
//! Cost notes (verified via docs/reference/pixellab-api.md, 2026-08-01):
//!   direct         $0.006–0.012
//!   text / vision  $0.007 (64×64) – 0.017 (256×256) — synchronous
//!   avatar*        $0.011 (48×48) – 0.012 (64×64) per character + per-dir animation later
//!                  NOTE: 4-dir character is async (30–80 s typical).
//!
//! API keys are NEVER logged; the tool only reads them from env.
//!
//! Shared library code lives in `crate::pixelart` — same module backs
//! `pixelart_server` (HTTP demo).

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use house_of_imbibe::pixelart as px;
use house_of_imbibe::pixelart::Proportions;
use tracing::{info, warn};

// ───── CLI ─────

#[derive(Parser, Debug)]
#[command(name = "image2pixel", about = "image → text → PixelLab pixel art demo")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Show PixelLab account balance + subscription quota.
    Balance,
    /// POST /v2/image-to-pixelart — convert photo to pixel art, no LLM in the loop.
    /// Validates whether PixelLab's size limits are actually a blocker (user claim).
    Direct {
        image: PathBuf,
        #[arg(long, default_value_t = 256)]
        size: u32,
    },
    /// POST /v2/create-image-pixen — single pixel-art image from text prompt only.
    Text {
        description: String,
        #[arg(long, default_value_t = 256)]
        size: u32,
    },
    /// NEW IDEA: image → MiniMax-M3 vision → text prompt → pixon.
    /// The whole pipeline runs in one command.
    Vision {
        image: PathBuf,
        #[arg(long, default_value_t = 256)]
        size: u32,
    },
    /// NEW IDEA, game-avatar version: image → vision → 4-direction walkable character.
    /// Async (30–80 s typical). Polls background job until done.
    Avatar {
        image: PathBuf,
        #[arg(long, default_value_t = 48)]
        size: u32,
        #[arg(long, default_value = "mannequin", value_parser = px::parse_template)]
        template: String,
        #[arg(long, value_enum, default_value_t = Proportions::Chibi)]
        proportions: Proportions,
        #[arg(long, default_value_t = false)]
        skip_proportions: bool,
    },
    /// Text-only avatar (no image): 4-direction character from a prompt.
    AvatarText {
        description: String,
        #[arg(long, default_value_t = 48)]
        size: u32,
        #[arg(long, default_value = "mannequin", value_parser = px::parse_template)]
        template: String,
        #[arg(long, value_enum, default_value_t = Proportions::Chibi)]
        proportions: Proportions,
        #[arg(long, default_value_t = false)]
        skip_proportions: bool,
    },
    /// Generate a map background (standard: create-image-pixen 256×256) or
    /// tileset (create-tileset, REST endpoint unverified — falls back to pixen on 422).
    Map {
        prompt: String,
        #[arg(long, default_value = "standard")]
        kind: String,
        #[arg(long, default_value_t = 256)]
        size: u32,
    },
    /// Animate an existing character's walk cycle (4 directions, v3 mode).
    /// Requires a character_id from a prior `avatar` / `avatar-text` run.
    Animate {
        character_id: String,
        #[arg(long, default_value_t = 4)]
        frame_count: u32,
        #[arg(long, default_value = "walk")]
        action: String,
    },
}

// ───── main ─────

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    let cli = Cli::parse();
    let pixellab_key = std::env::var("PIXELLAB_API_KEY")
        .context("PIXELLAB_API_KEY env var required (PixelLab bearer token)")?;
    let minimax_key = std::env::var("MINIMAX_API_KEY").ok();
    let out_dir = PathBuf::from(
        std::env::var("PIXELLAB_OUT").unwrap_or_else(|_| "pixellab-out".to_string()),
    );
    std::fs::create_dir_all(&out_dir)?;

    let http = px::http_client();

    match cli.cmd {
        Cmd::Balance => {
            let body = px::pixellab_get_balance(&http, &pixellab_key).await?;
            println!("{}", serde_json::to_string_pretty(&body)?);
        }
        Cmd::Direct { image, size } => {
            let (bytes, mime) = px::read_image(&image)?;
            info!("direct: {} bytes, mime={mime}, target={size}px", bytes.len());
            let png = px::pixellab_image_to_pixelart(&http, &pixellab_key, &bytes, &mime, size).await?;
            let out = out_dir.join(format!("direct-{}px.png", size));
            std::fs::write(&out, &png)?;
            println!("OK  saved {} ({} bytes)", out.display(), png.len());
        }
        Cmd::Text { description, size } => {
            info!("text → pixon: target={size}px, prompt={description:?}");
            let png = px::pixellab_create_image_pixen(&http, &pixellab_key, &description, size).await?;
            let out = out_dir.join(format!("pixon-{}px.png", size));
            std::fs::write(&out, &png)?;
            println!("OK  saved {} ({} bytes)", out.display(), png.len());
        }
        Cmd::Vision { image, size } => {
            let minimax = minimax_key
                .as_deref()
                .context("MINIMAX_API_KEY required for `vision`")?;
            let (bytes, mime) = px::read_image(&image)?;
            info!("vision: {} bytes, mime={mime}", bytes.len());
            let desc = px::minimax_describe_image(&http, minimax, &bytes, &mime).await?;
            info!("vision description: {desc}");
            let full = format!("{desc}. Pixel art, retro 16-bit character sprite.");
            info!("→ pixon prompt: {full}");
            let png = px::pixellab_create_image_pixen(&http, &pixellab_key, &full, size).await?;
            let stem = image.file_stem().and_then(|s| s.to_str()).unwrap_or("image");
            let out = out_dir.join(format!("vision-{stem}-{size}px.png"));
            std::fs::write(&out, &png)?;
            println!("OK  saved {} ({} bytes)", out.display(), png.len());
        }
        Cmd::Avatar { image, size, template, proportions, skip_proportions } => {
            let minimax = minimax_key
                .as_deref()
                .context("MINIMAX_API_KEY required for `avatar`")?;
            let (bytes, mime) = px::read_image(&image)?;
            info!("avatar: {} bytes, mime={mime}, size={size}, template={template}", bytes.len());
            let desc = px::minimax_describe_image(&http, minimax, &bytes, &mime).await?;
            info!("vision description: {desc}");
            run_avatar(
                &http,
                &pixellab_key,
                &desc,
                size,
                &template,
                proportions,
                skip_proportions,
                Some(&image),
                &out_dir,
            )
            .await?;
        }
        Cmd::AvatarText { description, size, template, proportions, skip_proportions } => {
            run_avatar(
                &http,
                &pixellab_key,
                &description,
                size,
                &template,
                proportions,
                skip_proportions,
                None,
                &out_dir,
            )
            .await?;
        }
        Cmd::Map { prompt, kind, size } => {
            match kind.as_str() {
                "standard" => {
                    info!("map standard: {size}×{size}, prompt={prompt:?}");
                    let png = px::pixellab_create_image_pixen(
                        &http, &pixellab_key, &prompt, size,
                    )
                    .await?;
                    let out = out_dir.join(format!("map-standard-{size}px.png"));
                    std::fs::write(&out, &png)?;
                    println!("OK  saved {} ({} bytes)", out.display(), png.len());
                }
                "tileset" => {
                    // tile_size only 16/32; if user passed a large `size`, clamp to 32.
                    let tile_size = if size <= 32 { size } else { 32 };
                    info!("map tileset: tile_size={tile_size}, prompt={prompt:?}");
                    match px::pixellab_create_tileset(
                        &http, &pixellab_key, &prompt, tile_size,
                    )
                    .await
                    {
                        Ok(png) => {
                            let out = out_dir.join(format!("map-tileset-{tile_size}px.png"));
                            std::fs::write(&out, &png)?;
                            println!("OK  saved {} ({} bytes)", out.display(), png.len());
                        }
                        Err(e) => {
                            // REST /v2/create-tileset may not exist (422) — fall back.
                            warn!("create-tileset failed: {e}");
                            warn!("falling back to create-image-pixen 256×256");
                            let png = px::pixellab_create_image_pixen(
                                &http, &pixellab_key, &prompt, 256,
                            )
                            .await?;
                            let out = out_dir.join("map-tileset-fallback-256px.png");
                            std::fs::write(&out, &png)?;
                            println!(
                                "OK  saved {} ({} bytes, fallback)",
                                out.display(),
                                png.len()
                            );
                        }
                    }
                }
                other => {
                    anyhow::bail!("unknown map kind `{other}` (use 'standard' or 'tileset')");
                }
            }
        }
        Cmd::Animate { character_id, frame_count, action } => {
            info!(
                "animate: character_id={character_id}, frame_count={frame_count}, action={action}"
            );
            let dir_jobs = px::pixellab_animate_character(
                &http,
                &pixellab_key,
                &character_id,
                &["south", "north", "east", "west"],
                frame_count,
                &action,
            )
            .await?;
            info!("submitted {} direction jobs", dir_jobs.len());
            for (dir, job_id) in &dir_jobs {
                info!("  polling {dir} job {job_id}...");
                let _ = px::poll_character(&http, &pixellab_key, job_id, 60).await?;
                info!("  {dir} done");
            }
            let anims =
                px::pixellab_character_animations(&http, &pixellab_key, &character_id).await?;
            let dir = out_dir.join(format!("animate-{character_id}"));
            std::fs::create_dir_all(&dir)?;
            for (dir_name, urls) in &anims {
                for (i, url) in urls.iter().enumerate() {
                    let png = http.get(url).send().await?.bytes().await?;
                    let out = dir.join(format!("{dir_name}_{i}.png"));
                    std::fs::write(&out, &png)?;
                    info!("  ↓ {} ({} bytes)", out.display(), png.len());
                }
            }
            println!(
                "OK  animate {character_id} saved to {} ({} directions)",
                dir.display(),
                anims.len()
            );
        }
    }
    Ok(())
}

// ───── avatar async wrapper (poll until done) ─────

#[allow(clippy::too_many_arguments)]
async fn run_avatar(
    http: &reqwest::Client,
    pixellab_key: &str,
    description: &str,
    size: u32,
    template: &str,
    proportions: Proportions,
    skip_proportions: bool,
    image_hint: Option<&Path>,
    out_dir: &Path,
) -> Result<()> {
    let props = match (skip_proportions, proportions) {
        (true, _) => None,
        (false, Proportions::Default) => None,
        (false, p) => Some(p),
    };

    let prompt = format!(
        "{description}. Low top-down 3/4 view, pixel art character, GBA-era retro style."
    );
    info!("→ avatar prompt: {prompt}");
    let (character_id, job_id) = px::pixellab_create_character_4dir(
        http,
        pixellab_key,
        &prompt,
        size,
        template,
        props,
        Some(px::GBA_PALETTE_PNG),
        true,
    )
    .await?;
    info!("submitted character_id={character_id} job_id={job_id}");
    let _last = px::poll_character(http, pixellab_key, &job_id, 60).await?;
    info!("character {character_id} done");

    let urls = px::pixellab_character_rotation_urls(http, pixellab_key, &character_id).await?;
    info!("rotation_urls: {urls:?}");

    let stem = image_hint
        .and_then(|p| p.file_stem())
        .and_then(|s| s.to_str())
        .unwrap_or("avatar");
    let dir = out_dir.join(format!("avatar-{stem}-{size}px"));
    std::fs::create_dir_all(&dir)?;

    for (dir_name, url) in &urls {
        let png = http.get(url).send().await?.bytes().await?;
        let out = dir.join(format!("{dir_name}.png"));
        std::fs::write(&out, &png)?;
        info!("  ↓ {} ({} bytes)", out.display(), png.len());
    }
    println!("OK  character {character_id} saved to {} ({} directions)", dir.display(), urls.len());
    Ok(())
}