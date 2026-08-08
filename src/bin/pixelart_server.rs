//! `pixelart_server` — minimal HTTP front-end for image → text → PixelLab.
//!
//! Endpoints:
//!   GET  /                      → pixel-art HTML test page
//!   POST /api/vision            multipart {image?,description?,size} → { description, png_b64 }
//!   POST /api/avatar            multipart {image?,description?,size,template,proportions} → { job_id }
//!   GET  /api/avatar/{id}       → { status, description, rotations?, error? }
//!   POST /api/vision-text       JSON {description,size} → { png_b64 }
//!   POST /api/avatar-text       JSON {description,size,template,proportions} → { job_id }
//!   GET  /api/balance           → raw PixelLab balance
//!   GET  /api/health            → "ok"

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context, Result};
use axum::extract::{DefaultBodyLimit, Multipart, Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use house_of_imbibe::pixelart as px;
use house_of_imbibe::pixelart::Proportions;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::RwLock;
use tracing::{info, warn};

// ───── in-memory job store ─────

#[derive(Clone, Debug)]
struct AvatarJob {
    status: String,
    description: String,
    rotations: Vec<(String, String)>,
    error: Option<String>,
}

#[derive(Clone)]
struct AppState {
    pixellab_key: String,
    minimax_key: String,
    http: reqwest::Client,
    jobs: Arc<RwLock<HashMap<String, AvatarJob>>>,
}

// ───── error ─────

struct ApiError(StatusCode, String);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(json!({ "error": self.1 }))).into_response()
    }
}

impl<E: std::fmt::Display> From<E> for ApiError {
    fn from(e: E) -> Self {
        ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    }
}

// ───── HTML ─────

const INDEX_HTML: &str = include_str!("../../web/test-pixelart.html");

async fn index_handler() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "text/html; charset=utf-8")], INDEX_HTML)
}

// ───── helpers ─────

/// Parse multipart: extract `image` (bytes + mime), `description` (optional text),
/// `size` (u32), `template` (string), `proportions` (string).
/// Returns (image_bytes, mime, description, size, template, proportions).
async fn parse_multipart(
    mut form: Multipart,
    default_size: u32,
) -> Result<(Option<Vec<u8>>, Option<String>, Option<String>, u32, String, String), ApiError> {
    let mut image_bytes: Option<Vec<u8>> = None;
    let mut mime: Option<String> = None;
    let mut description: Option<String> = None;
    let mut size = default_size;
    let mut template = "mannequin".to_string();
    let mut proportions = "chibi".to_string();

    while let Some(field) = form.next_field().await.map_err(|e| ApiError(StatusCode::BAD_REQUEST, e.to_string()))? {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "image" => {
                let ct = field.content_type().map(|m| m.to_string());
                let data = field.bytes().await.map_err(|e| ApiError(StatusCode::BAD_REQUEST, e.to_string()))?;
                if !data.is_empty() {
                    if let Some(ct) = ct { mime = Some(ct); }
                    image_bytes = Some(data.to_vec());
                }
            }
            "description" => {
                let t = field.text().await.unwrap_or_default();
                if !t.is_empty() { description = Some(t); }
            }
            "size" => {
                if let Ok(n) = field.text().await.unwrap_or_default().parse::<u32>() { size = n; }
            }
            "template" => template = field.text().await.unwrap_or_default(),
            "proportions" => proportions = field.text().await.unwrap_or_default(),
            _ => {}
        }
    }

    Ok((image_bytes, mime, description, size, template, proportions))
}

fn parse_proportions(s: &str) -> Option<Proportions> {
    match s {
        "chibi" => Some(Proportions::Chibi),
        "cartoon" => Some(Proportions::Cartoon),
        "stylized" => Some(Proportions::Stylized),
        "default" | "" => None,
        _ => Some(Proportions::Chibi),
    }
}

// ───── /api/health ─────

async fn health() -> &'static str { "ok" }

// ───── /api/balance ─────

async fn balance(State(s): State<AppState>) -> Result<Json<serde_json::Value>, ApiError> {
    let v = px::pixellab_get_balance(&s.http, &s.pixellab_key).await.map_err(ApiError::from)?;
    Ok(Json(v))
}

// ───── /api/describe (multipart, sync) ─────
// Vision-only — no PixelLab generation. Used by the "Describe Image" button
// to fill the editable textarea. Returns {description} only.

async fn describe_handler(
    State(s): State<AppState>,
    mut form: Multipart,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut image_bytes: Option<Vec<u8>> = None;
    let mut mime: Option<String> = None;

    while let Some(field) = form.next_field().await.map_err(|e| ApiError(StatusCode::BAD_REQUEST, e.to_string()))? {
        let name = field.name().unwrap_or("").to_string();
        if name == "image" {
            let ct = field.content_type().map(|m| m.to_string());
            let data = field.bytes().await.map_err(|e| ApiError(StatusCode::BAD_REQUEST, e.to_string()))?;
            if !data.is_empty() {
                if let Some(ct) = ct { mime = Some(ct); }
                image_bytes = Some(data.to_vec());
            }
        }
    }

    let bytes = image_bytes.ok_or_else(|| ApiError(StatusCode::BAD_REQUEST, "missing `image` field".into()))?;
    let m = mime.unwrap_or_else(|| px::detect_mime(&bytes).unwrap_or("image/png").to_string());
    info!("POST /api/describe: {} bytes mime={m}", bytes.len());

    let description = px::minimax_describe_image(&s.http, &s.minimax_key, &bytes, &m)
        .await
        .map_err(|e| ApiError(StatusCode::BAD_GATEWAY,
            format!("Vision API failed: {e}. Please type a description manually.")))?;

    info!("  description: {description}");
    Ok(Json(json!({ "description": description })))
}

// ───── /api/vision (multipart, sync) ─────
//
// Pipeline strategy (locked 2026-08-04):
//   1. If description is provided → skip both direct and vision, go straight to text→pixon.
//   2. If image is provided (no description):
//      a. Try `image-to-pixelart` first (PRIMARY — direct, fast, cheap).
//      b. If that fails (422/413/timeout) → fall back to MiniMax vision → text → pixon.
//   3. If neither → 400.

async fn vision_handler(
    State(s): State<AppState>,
    form: Multipart,
) -> Result<Json<serde_json::Value>, ApiError> {
    let (image_bytes, mime, description, size, _, _) = parse_multipart(form, 256).await?;

    // ── Branch 1: manual description → text generation ──
    if let Some(desc) = description {
        info!("POST /api/vision: manual description, size={size}");
        return generate_from_text(&s, &desc, size).await;
    }

    // ── Branch 2: image provided → try direct, fallback to vision ──
    if let Some(bytes) = image_bytes {
        let m = mime.unwrap_or_else(|| px::detect_mime(&bytes).unwrap_or("image/png").to_string());
        info!("POST /api/vision: {} bytes mime={m} size={size}", bytes.len());

        // Step 2a: try image-to-pixelart (PRIMARY)
        match px::pixellab_image_to_pixelart(&s.http, &s.pixellab_key, &bytes, &m, size).await {
            Ok(png) => {
                info!("  image-to-pixelart succeeded ({} bytes)", png.len());
                return Ok(Json(json!({
                    "method": "direct",
                    "description": "",
                    "full_prompt": "",
                    "png_b64": B64.encode(&png),
                    "size": size,
                })));
            }
            Err(e) => {
                info!("  image-to-pixelart failed: {e}, falling back to vision→text");
                // Step 2b: fallback to MiniMax vision → text → pixon
                let desc = match px::minimax_describe_image(&s.http, &s.minimax_key, &bytes, &m).await {
                    Ok(d) => d,
                    Err(ve) => {
                        return Err(ApiError(StatusCode::BAD_GATEWAY,
                            format!("Both direct and vision paths failed. Direct: {e}. Vision: {ve}. \
                                     Please provide a description manually in the text field.")));
                    }
                };
                return generate_from_text(&s, &desc, size).await;
            }
        }
    }

    // ── Branch 3: neither image nor description ──
    Err(ApiError(StatusCode::BAD_REQUEST, "provide `image` or `description`".into()))
}

/// Shared: generate a single pixel-art image from a text description.
async fn generate_from_text(
    s: &AppState,
    desc: &str,
    size: u32,
) -> Result<Json<serde_json::Value>, ApiError> {
    info!("  generate_from_text: {desc}");
    let full = format!("{desc}. Pixel art, retro 16-bit character sprite.");
    let png = px::pixellab_create_image_pixen(&s.http, &s.pixellab_key, &full, size)
        .await
        .map_err(|e| ApiError(StatusCode::BAD_GATEWAY, format!("pixellab: {e}")))?;

    Ok(Json(json!({
        "method": "text",
        "description": desc,
        "full_prompt": full,
        "png_b64": B64.encode(&png),
        "size": size,
    })))
}

// ───── /api/avatar (multipart, async submit) ─────
//
// Pipeline strategy (locked 2026-08-04):
//   1. If description is provided → skip vision, go straight to text→character.
//   2. If image is provided (no description):
//      a. Try MiniMax vision → text → character (FALLBACK path for avatar).
//         Note: we don't try image-to-pixelart here because avatar needs
//         create-character-with-4-directions, which is text-driven.
//      b. If vision fails → return error asking for manual description.
//   3. If neither → 400.

async fn avatar_submit_handler(
    State(s): State<AppState>,
    form: Multipart,
) -> Result<Json<serde_json::Value>, ApiError> {
    let (image_bytes, mime, description, size, template, proportions) = parse_multipart(form, 64).await?;

    let desc = if let Some(d) = description {
        d
    } else if let Some(bytes) = image_bytes {
        let m = mime.unwrap_or_else(|| px::detect_mime(&bytes).unwrap_or("image/png").to_string());
        info!("POST /api/avatar: {} bytes mime={m} template={template}", bytes.len());
        match px::minimax_describe_image(&s.http, &s.minimax_key, &bytes, &m).await {
            Ok(d) => d,
            Err(e) => {
                return Err(ApiError(StatusCode::BAD_GATEWAY,
                    format!("Vision API failed: {e}. Please provide a description manually in the text field.")));
            }
        }
    } else {
        return Err(ApiError(StatusCode::BAD_REQUEST, "provide `image` or `description`".into()));
    };

    let props = parse_proportions(&proportions);
    let full = format!("{desc}. Low top-down 3/4 view, pixel art character, GBA-era retro style.");

    let (character_id, job_id) = px::pixellab_create_character_4dir(
        &s.http, &s.pixellab_key, &full, size, &template, props,
        Some(px::GBA_PALETTE_PNG), true,
    )
    .await
    .map_err(|e| ApiError(StatusCode::BAD_GATEWAY, format!("pixellab submit: {e}")))?;

    {
        let mut map = s.jobs.write().await;
        map.insert(job_id.clone(), AvatarJob {
            status: "processing".into(),
            description: desc,
            rotations: vec![],
            error: None,
        });
    }

    let state2 = s.clone();
    let jid = job_id.clone();
    let cid = character_id.clone();
    tokio::spawn(async move {
        if let Err(e) = poll_and_store(&state2, &jid, &cid).await {
            warn!("job {jid} poll failed: {e}");
            let mut map = state2.jobs.write().await;
            if let Some(j) = map.get_mut(&jid) {
                j.status = "failed".into();
                j.error = Some(e.to_string());
            }
        }
    });

    Ok(Json(json!({ "job_id": job_id, "character_id": character_id })))
}

async fn poll_and_store(s: &AppState, job_id: &str, character_id: &str) -> Result<()> {
    let _last = px::poll_character(&s.http, &s.pixellab_key, job_id, 60).await?;
    let urls = px::pixellab_character_rotation_urls(&s.http, &s.pixellab_key, character_id).await?;
    let mut map = s.jobs.write().await;
    if let Some(j) = map.get_mut(job_id) {
        j.status = "completed".into();
        j.rotations = urls;
    } else {
        warn!("job {job_id} disappeared from map before poll completion");
    }
    Ok(())
}

// ───── /api/avatar/{id} ─────

#[derive(Debug, Serialize)]
struct AvatarStatus {
    status: String,
    description: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    rotations: Vec<RotationEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct RotationEntry {
    direction: String,
    url: String,
}

async fn avatar_status_handler(
    State(s): State<AppState>,
    Path(job_id): Path<String>,
) -> Result<Json<AvatarStatus>, ApiError> {
    let map = s.jobs.read().await;
    let job = map.get(&job_id)
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, format!("no job {job_id}")))?;
    Ok(Json(AvatarStatus {
        status: job.status.clone(),
        description: job.description.clone(),
        rotations: job.rotations.iter().map(|(d, u)| RotationEntry {
            direction: d.clone(), url: u.clone(),
        }).collect(),
        error: job.error.clone(),
    }))
}

// ───── /api/vision-text (JSON, sync) ─────

#[derive(Debug, Deserialize)]
struct VisionTextBody {
    description: String,
    #[serde(default = "default_size")]
    size: u32,
}

fn default_size() -> u32 { 256 }

async fn vision_text_handler(
    State(s): State<AppState>,
    Json(body): Json<VisionTextBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    info!("POST /api/vision-text: size={} prompt={:?}", body.size, body.description);
    let full = format!("{}. Pixel art, retro 16-bit character sprite.", body.description);
    let png = px::pixellab_create_image_pixen(&s.http, &s.pixellab_key, &full, body.size)
        .await
        .map_err(|e| ApiError(StatusCode::BAD_GATEWAY, format!("pixellab: {e}")))?;
    Ok(Json(json!({
        "description": body.description,
        "full_prompt": full,
        "png_b64": B64.encode(&png),
        "size": body.size,
    })))
}

// ───── /api/avatar-text (JSON, async) ─────

#[derive(Debug, Deserialize)]
struct AvatarTextBody {
    description: String,
    #[serde(default = "default_avatar_size")]
    size: u32,
    #[serde(default = "default_template")]
    template: String,
    #[serde(default)]
    proportions: String,
}

fn default_avatar_size() -> u32 { 64 }
fn default_template() -> String { "mannequin".into() }

async fn avatar_text_handler(
    State(s): State<AppState>,
    Json(body): Json<AvatarTextBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    info!("POST /api/avatar-text: size={} template={} prompt={:?}", body.size, body.template, body.description);
    let props = parse_proportions(&body.proportions);
    let full = format!("{}. Low top-down 3/4 view, pixel art character, GBA-era retro style.", body.description);

    let (character_id, job_id) = px::pixellab_create_character_4dir(
        &s.http, &s.pixellab_key, &full, body.size, &body.template, props,
        Some(px::GBA_PALETTE_PNG), true,
    )
    .await
    .map_err(|e| ApiError(StatusCode::BAD_GATEWAY, format!("pixellab submit: {e}")))?;

    {
        let mut map = s.jobs.write().await;
        map.insert(job_id.clone(), AvatarJob {
            status: "processing".into(),
            description: body.description.clone(),
            rotations: vec![],
            error: None,
        });
    }

    let state2 = s.clone();
    let jid = job_id.clone();
    let cid = character_id.clone();
    tokio::spawn(async move {
        if let Err(e) = poll_and_store(&state2, &jid, &cid).await {
            warn!("job {jid} poll failed: {e}");
            let mut map = state2.jobs.write().await;
            if let Some(j) = map.get_mut(&jid) {
                j.status = "failed".into();
                j.error = Some(e.to_string());
            }
        }
    });

    Ok(Json(json!({ "job_id": job_id, "character_id": character_id })))
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

    let pixellab_key = std::env::var("PIXELLAB_API_KEY")
        .context("PIXELLAB_API_KEY env var required")?;
    let minimax_key = std::env::var("MINIMAX_API_KEY")
        .context("MINIMAX_API_KEY env var required for vision step")?;

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .or_else(|| {
            std::env::args().collect::<Vec<_>>().windows(2)
                .find(|w| w[0] == "--port")
                .and_then(|w| w[1].parse().ok())
        })
        .unwrap_or(8081);

    let state = AppState {
        pixellab_key,
        minimax_key,
        http: px::http_client(),
        jobs: Arc::new(RwLock::new(HashMap::new())),
    };

    let app = Router::new()
        .route("/", get(index_handler))
        .route("/api/health", get(health))
        .route("/api/balance", get(balance))
        .route("/api/describe", post(describe_handler))
        .route("/api/vision", post(vision_handler))
        .route("/api/vision-text", post(vision_text_handler))
        .route("/api/avatar", post(avatar_submit_handler))
        .route("/api/avatar-text", post(avatar_text_handler))
        .route("/api/avatar/{id}", get(avatar_status_handler))
        .layer(DefaultBodyLimit::max(20 * 1024 * 1024))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("pixelart_server listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}