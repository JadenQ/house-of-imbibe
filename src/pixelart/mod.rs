//! PixelLab REST client + MiniMax-M3 vision wrapper.
//!
//! ## Pipeline strategy (locked 2026-08-04)
//!
//! **Primary path** — image → PixelLab direct:
//!   `pixellab_image_to_pixelart`  (sync, ~$0.007, works for any image ≤1280×1280)
//!
//! **Fallback path** — image too large or direct API fails → vision → text → PixelLab:
//!   `minimax_describe_image` → `pixellab_create_image_pixen` / `pixellab_create_character_4dir`
//!   (async, ~$0.013, 30–120 s)
//!
//! The fallback is NOT an alternative — it's a **recovery** for when the primary path
//! can't serve (image-to-pixelart 422/413, network timeout, etc.). In production the
//! caller should try `image_to_pixelart` first, catch the error, then fall back to
//! vision→text. The MiniMax vision API key may also be rate-limited (429), in which
//! case the user must provide a manual description.
//!
//! ## Endpoints
//!
//! - `minimax_describe_image` — turn a photo into a ≤80-word pixel-art-ready prompt.
//! - `pixellab_image_to_pixelart` — photo → single pixel-art image (sync, PRIMARY).
//! - `pixellab_create_image_pixen` — text → single pixel-art image (sync, FALLBACK step 2).
//! - `pixellab_create_character_4dir` — text → 4-direction game character (async; submit).
//! - `poll_character` — poll an async PixelLab job to completion.
//! - `pixellab_character_rotation_urls` — fetch the 4 direction URLs after completion.
//!
//! Live API vs OpenAPI spec discrepancies (logged 2026-08-04) — see source comments:
//!   1. `/v2/image-to-pixelart` requires BOTH `image_size` AND `output_size`.
//!   2. `/v2/create-character-with-4-directions` returns `background_job_id` (singular)
//!      + `character_id`, not the array shape the spec advertises.
//!   3. `/v2/create-image-pixen` rejects the optional `model` / `negative_description`
//!      / `seed` fields with 422 even though the spec lists them.
//!
//! NO business logic lives here. This module is a thin transport layer.

use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use clap::ValueEnum;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use tracing::{info, warn};

// ───── constants ─────

pub const PIXELLAB_BASE: &str = "https://api.pixellab.ai/v2";
pub const MINIMAX_URL: &str = "https://api.minimaxi.com/v1/chat/completions";

/// GBA Emerald palette reference image for `color_image` + `force_colors`.
/// 16×16 PNG with 256 representative GBA-era colors (skin, hair, clothing,
/// environment, accent). Shipped as `include_bytes!` so no runtime file needed.
pub const GBA_PALETTE_PNG: &[u8] = include_bytes!("gba-palette.png");

pub fn http_client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(180))
        .connect_timeout(Duration::from_secs(15))
        .build()
        .expect("reqwest client")
}

// ───── enum(s) ─────

/// Body-shape / proportions presets PixelLab accepts on character endpoints.
/// `chibi` is the default for humanoid avatars (GBA Pokémon-style).
/// `Default` is a sentinel meaning "don't send `proportions` at all".
#[derive(Copy, Clone, Debug, ValueEnum, PartialEq, Eq)]
pub enum Proportions {
    Chibi,
    Cartoon,
    Stylized,
    Default,
}

pub fn prop_name(p: Proportions) -> &'static str {
    match p {
        Proportions::Chibi => "chibi",
        Proportions::Cartoon => "cartoon",
        Proportions::Stylized => "stylized",
        Proportions::Default => "default",
    }
}

/// Templates PixelLab accepts. The split is humanoid vs four-legged animal —
/// passing a humanoid template with a non-humanoid prompt is silently wrong.
pub const TEMPLATES: &[&str] = &[
    "mannequin", "default", "chibi", "cartoon", "stylized",
    "realistic_male", "realistic_female", "heroic",
    "bear", "cat", "dog", "horse", "lion",
];

pub fn parse_template(s: &str) -> Result<String, String> {
    if TEMPLATES.contains(&s) {
        Ok(s.to_string())
    } else {
        Err(format!("unknown template `{s}`; valid: {}", TEMPLATES.join(", ")))
    }
}

// ───── MiniMax-M3 vision ─────

/// Vision → text. ≤80-word pixel-art-ready description.
/// Strip `<think>...</think>` reasoning (MiniMax-M3 emits it).
pub async fn minimax_describe_image(
    http: &Client,
    api_key: &str,
    image_bytes: &[u8],
    mime: &str,
) -> Result<String> {
    let b64 = B64.encode(image_bytes);
    let data_url = format!("data:{mime};base64,{b64}");

    let body = json!({
        "model": "MiniMax-M3",
        "max_tokens": 500,
        "messages": [
            {
                "role": "system",
                "content": "You caption images for a pixel-art character generator. Output exactly ONE concise paragraph (≤80 words). No preamble, no bullet lists, no labels, no JSON. Just the paragraph."
            },
            {
                "role": "user",
                "content": [
                    {"type": "text", "text": "Describe the main subject (person, animal, or object) so an artist can recreate it as a pixel-art character. Cover: what it is and approximate species/type, body proportions and pose, clothing or fur/marking pattern, 2-4 dominant colors by name (e.g. \"brown fur\", \"red shirt\"), and any distinctive visual feature. End the paragraph with one short sentence giving overall mood/vibe."},
                    {"type": "image_url", "image_url": {"url": data_url}}
                ]
            }
        ]
    });

    let resp = http
        .post(MINIMAX_URL)
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .context("minimax request")?
        .error_for_status()
        .context("minimax non-2xx")?;

    let v: serde_json::Value = resp.json().await?;
    let raw = v["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| anyhow!("minimax returned no content: {v}"))?
        .to_string();
    Ok(strip_think(&raw).trim().to_string())
}

pub fn strip_think(s: &str) -> String {
    if let Some(end) = s.find("</think>") {
        s[end + "</think>".len()..].to_string()
    } else if let Some(start) = s.find("<think>") {
        s[..start].to_string()
    } else {
        s.to_string()
    }
}

// ───── PixelLab REST ─────

pub async fn pixellab_get_balance(http: &Client, key: &str) -> Result<serde_json::Value> {
    let v: serde_json::Value = http
        .get(format!("{PIXELLAB_BASE}/balance"))
        .bearer_auth(key)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(v)
}

/// POST /v2/image-to-pixelart — synchronous. Returns PNG bytes.
/// NOTE: live API requires BOTH `image_size` (input) AND `output_size` (output target).
/// Docs only mention `image_size`. Also accepts up to 1280×1280 input / 320×320 output.
pub async fn pixellab_image_to_pixelart(
    http: &Client,
    key: &str,
    image: &[u8],
    mime: &str,
    out_size: u32,
) -> Result<Vec<u8>> {
    let b64 = B64.encode(image);
    let body = json!({
        "image": {"type": "base64", "base64": b64, "format": mime_to_ext(mime)},
        "image_size": {"width": out_size, "height": out_size},
        "output_size": {"width": out_size, "height": out_size},
    });
    let resp: PixelImageResponse = http
        .post(format!("{PIXELLAB_BASE}/image-to-pixelart"))
        .bearer_auth(key)
        .json(&body)
        .send()
        .await?
        .error_for_status()
        .context("image-to-pixelart 4xx/5xx")?
        .json()
        .await?;
    Ok(B64.decode(&resp.image.base64)?)
}

/// POST /v2/create-image-pixen — synchronous. Returns PNG bytes.
/// Minimal body: only `description` + `image_size` are accepted (live API).
pub async fn pixellab_create_image_pixen(
    http: &Client,
    key: &str,
    description: &str,
    out_size: u32,
) -> Result<Vec<u8>> {
    let body = json!({
        "description": description,
        "image_size": {"width": out_size, "height": out_size},
    });
    let resp: PixelImageResponse = http
        .post(format!("{PIXELLAB_BASE}/create-image-pixen"))
        .bearer_auth(key)
        .json(&body)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(B64.decode(&resp.image.base64)?)
}

/// POST /v2/create-character-with-4-directions — async.
///
/// Returns `(character_id, job_id)`. The live API actually returns
/// `background_job_id` (singular) and `character_id`, while the OpenAPI spec says
/// `background_job_ids` (array). We accept both shapes.
///
/// `color_image` — optional palette reference PNG bytes (e.g. `GBA_PALETTE_PNG`).
/// `force_colors` — when true, PixelLab restricts output to the palette in `color_image`.
pub async fn pixellab_create_character_4dir(
    http: &Client,
    key: &str,
    description: &str,
    size: u32,
    template: &str,
    proportions: Option<Proportions>,
    color_image: Option<&[u8]>,
    force_colors: bool,
) -> Result<(String, String)> {
    let props = match proportions {
        None => json!(null),
        Some(p) => json!({"type": "preset", "name": prop_name(p)}),
    };
    let mut body = json!({
        "description": description,
        "image_size": {"width": size, "height": size},
        "view": "low top-down",
        "outline": "single color black outline",
        "shading": "basic shading",
        "detail": "medium detail",
        "proportions": props,
        "template_id": template,
        "isometric": false,
        "async_mode": true,
        "force_colors": force_colors,
    });
    if let Some(palette) = color_image {
        body["color_image"] = json!({
            "type": "base64",
            "base64": B64.encode(palette),
            "format": "png",
        });
    }
    let resp: serde_json::Value = http
        .post(format!("{PIXELLAB_BASE}/create-character-with-4-directions"))
        .bearer_auth(key)
        .json(&body)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let character_id = resp["character_id"]
        .as_str()
        .ok_or_else(|| anyhow!("missing character_id: {resp}"))?
        .to_string();
    let job_id = if let Some(s) = resp["background_job_id"].as_str() {
        s.to_string()
    } else if let Some(arr) = resp["background_job_ids"].as_array() {
        arr.first()
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("background_job_ids empty: {resp}"))?
            .to_string()
    } else {
        return Err(anyhow!("no background_job_id or background_job_ids: {resp}"));
    };
    Ok((character_id, job_id))
}

/// Poll background-jobs until terminal status. Returns the last response on success.
pub async fn poll_character(
    http: &Client,
    key: &str,
    job_id: &str,
    max_polls: u32,
) -> Result<serde_json::Value> {
    let url = format!("{PIXELLAB_BASE}/background-jobs/{job_id}");
    for i in 1..=max_polls {
        let v: serde_json::Value = http
            .get(&url)
            .bearer_auth(key)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let status = v["status"].as_str().unwrap_or("?");
        info!("  poll #{i}: status={status}");
        match status {
            "completed" => return Ok(v),
            "failed" => bail!("pixelab job failed: {v}"),
            _ => tokio::time::sleep(Duration::from_secs(5)).await,
        }
    }
    bail!("timed out after {max_polls} polls (5s each)")
}

/// GET /v2/characters/{id} → rotation_urls
pub async fn pixellab_character_rotation_urls(
    http: &Client,
    key: &str,
    character_id: &str,
) -> Result<Vec<(String, String)>> {
    let v: serde_json::Value = http
        .get(format!("{PIXELLAB_BASE}/characters/{character_id}"))
        .bearer_auth(key)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let rotations = v["rotation_urls"]
        .as_object()
        .ok_or_else(|| anyhow!("no rotation_urls in: {v}"))?;
    let mut out = Vec::new();
    for (dir_name, url) in rotations {
        if let Some(u) = url.as_str() {
            out.push((dir_name.clone(), u.to_string()));
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

// ───── types ─────

#[derive(Debug, Deserialize)]
pub struct PixelImageResponse {
    pub image: PixelImageInner,
    #[serde(default)]
    #[allow(dead_code)]
    pub usage: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct PixelImageInner {
    pub base64: String,
    #[allow(dead_code)]
    pub format: Option<String>,
}

// ───── file / mime ─────

pub fn read_image(path: &std::path::Path) -> Result<(Vec<u8>, String)> {
    let bytes = std::fs::read(path).with_context(|| format!("read {}", path.display()))?;
    let mime = detect_mime(&bytes).ok_or_else(|| {
        anyhow!(
            "could not detect image format from magic bytes (PNG/JPEG/WEBP/GIF only). \
             Use a converter if needed."
        )
    })?;
    Ok((bytes, mime.to_string()))
}

pub fn detect_mime(b: &[u8]) -> Option<&'static str> {
    if b.len() >= 8 && &b[..8] == b"\x89PNG\r\n\x1a\n" {
        Some("image/png")
    } else if b.len() >= 3 && &b[..3] == b"\xff\xd8\xff" {
        Some("image/jpeg")
    } else if b.len() >= 6 && &b[..4] == b"GIF8" {
        Some("image/gif")
    } else if b.len() >= 12 && &b[..4] == b"RIFF" && &b[8..12] == b"WEBP" {
        Some("image/webp")
    } else {
        None
    }
}

pub fn mime_to_ext(mime: &str) -> &'static str {
    match mime {
        "image/png" => "png",
        "image/jpeg" => "jpeg",
        "image/webp" => "webp",
        "image/gif" => "gif",
        other => {
            warn!("unknown mime {other}; passing through to pixelab as png");
            "png"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_mime_png() {
        let png = b"\x89PNG\r\n\x1a\nrest of file";
        assert_eq!(detect_mime(png), Some("image/png"));
    }

    #[test]
    fn detect_mime_jpeg() {
        let jpg = b"\xff\xd8\xff\xe0rest";
        assert_eq!(detect_mime(jpg), Some("image/jpeg"));
    }

    #[test]
    fn detect_mime_webp() {
        let webp = b"RIFF\x00\x00\x00\x00WEBPrest";
        assert_eq!(detect_mime(webp), Some("image/webp"));
    }

    #[test]
    fn detect_mime_unknown() {
        assert_eq!(detect_mime(b"not an image"), None);
    }

    #[test]
    fn strip_think_basic() {
        let s = "<think>reasoning</think>actual answer";
        assert_eq!(strip_think(s), "actual answer");
    }

    #[test]
    fn strip_think_unmatched_open() {
        let s = "blah<think>never closed";
        assert_eq!(strip_think(s), "blah");
    }

    #[test]
    fn strip_think_passthrough() {
        assert_eq!(strip_think("clean text"), "clean text");
    }
}