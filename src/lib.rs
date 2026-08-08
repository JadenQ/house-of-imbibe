//! Shared library for the `house-of-imbibe` package.
//!
//! Exposes `pixelart` (PixelLab + MiniMax vision wrapper), `realtime`
//! (WS protocol contracts; backend impl agent fills in the rest), and the
//! HTTP app: `AppState`, `build_router`, handlers, and session helpers.
//! The binary (`src/main.rs`) and integration tests (`tests/`) both
//! consume this. NO business code outside the package should reach into
//! `reqwest` directly — keep provider swaps possible.

pub mod pixelart;
pub mod realtime;

use crate::pixelart as px;
use crate::pixelart::Proportions;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use argon2::password_hash::rand_core::{OsRng, RngCore};
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use axum::extract::{DefaultBodyLimit, Multipart, Path, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::SqlitePool;
use tokio::sync::RwLock;
use tower_http::services::{ServeDir, ServeFile};
use tracing::{info, warn};

pub struct AppState {
    pub db: SqlitePool,
    pub pixellab_key: Option<String>,
    pub minimax_key: Option<String>,
    pub http: reqwest::Client,
    pub jobs: Arc<RwLock<HashMap<String, AvatarJob>>>,
    pub rt: Arc<crate::realtime::RealtimeState>,
}

// ───── avatar generation job ─────

#[derive(Clone, Debug)]
pub struct AvatarJob {
    pub status: String,
    pub user_id: i64,
    #[allow(dead_code)]
    pub description: String,
    pub error: Option<String>,
}

pub fn now_ts() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64
}

// ---------- 错误 ----------

pub struct ApiError(pub StatusCode, pub String);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(serde_json::json!({ "error": self.1 }))).into_response()
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(e: sqlx::Error) -> Self {
        tracing::error!(?e, "db error");
        ApiError(StatusCode::INTERNAL_SERVER_ERROR, "internal".into())
    }
}

// ---------- 模型 ----------

#[derive(Deserialize)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct MeResponse {
    pub id: i64,
    pub username: String,
    pub is_admin: bool,
    pub avatar: Option<serde_json::Value>,
}

#[derive(Deserialize)]
pub struct AvatarPut {
    pub config: serde_json::Value,
}

// ---------- session ----------

pub const COOKIE_NAME: &str = "hoi_session";

pub fn session_token(headers: &HeaderMap) -> Option<String> {
    let raw = headers.get(header::COOKIE)?.to_str().ok()?;
    raw.split(';')
        .filter_map(|p| p.trim().split_once('='))
        .find(|(k, _)| *k == COOKIE_NAME)
        .map(|(_, v)| v.to_string())
}

pub async fn current_user(state: &AppState, headers: &HeaderMap) -> Option<(i64, String, bool)> {
    let token = session_token(headers)?;
    sqlx::query_as::<_, (i64, String, bool)>(
        "SELECT u.id, u.username, u.is_admin FROM sessions s JOIN users u ON u.id = s.user_id WHERE s.token = ?",
    )
    .bind(&token)
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten()
}

pub fn make_session_cookie(token: &str) -> String {
    format!("{COOKIE_NAME}={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age=2592000")
}

pub async fn create_session(state: &AppState, user_id: i64) -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    let token = hex::encode(bytes);
    sqlx::query("INSERT INTO sessions (token, user_id, created_at) VALUES (?, ?, ?)")
        .bind(&token)
        .bind(user_id)
        .bind(now_ts())
        .execute(&state.db)
        .await
        .expect("insert session");
    token
}

// ---------- handlers ----------

pub fn validate_credentials(c: &Credentials) -> Result<(), ApiError> {
    let u = c.username.trim();
    if u.len() < 2 || u.len() > 20 || !u.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
        return Err(ApiError(StatusCode::BAD_REQUEST, "username must be 2-20 chars [a-z0-9_-]".into()));
    }
    if c.password.len() < 6 || c.password.len() > 128 {
        return Err(ApiError(StatusCode::BAD_REQUEST, "password must be 6-128 chars".into()));
    }
    Ok(())
}

pub async fn register(State(state): State<Arc<AppState>>, Json(c): Json<Credentials>) -> Result<Response, ApiError> {
    validate_credentials(&c)?;
    let username = c.username.trim().to_string();
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(c.password.as_bytes(), &salt)
        .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR, "hash failed".into()))?
        .to_string();

    // 首个注册用户自动成为 admin（bootstrap 规则）
    let user_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users").fetch_one(&state.db).await?;
    let is_admin = user_count.0 == 0;

    let res = sqlx::query("INSERT INTO users (username, password_hash, is_admin, created_at) VALUES (?, ?, ?, ?)")
        .bind(&username)
        .bind(&hash)
        .bind(is_admin)
        .bind(now_ts())
        .execute(&state.db)
        .await;
    let user_id = match res {
        Ok(r) => r.last_insert_rowid(),
        Err(_) => return Err(ApiError(StatusCode::CONFLICT, "username taken".into())),
    };

    let token = create_session(&state, user_id).await;
    Ok((
        StatusCode::CREATED,
        [(header::SET_COOKIE, make_session_cookie(&token))],
        Json(serde_json::json!({ "id": user_id, "username": username, "is_admin": is_admin })),
    )
        .into_response())
}

pub async fn login(State(state): State<Arc<AppState>>, Json(c): Json<Credentials>) -> Result<Response, ApiError> {
    let row = sqlx::query_as::<_, (i64, String, String, bool)>(
        "SELECT id, username, password_hash, is_admin FROM users WHERE username = ?",
    )
    .bind(c.username.trim())
    .fetch_optional(&state.db)
    .await?;
    let (id, username, hash, is_admin) =
        row.ok_or(ApiError(StatusCode::UNAUTHORIZED, "invalid credentials".into()))?;

    let parsed = PasswordHash::new(&hash).map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR, "bad hash".into()))?;
    Argon2::default()
        .verify_password(c.password.as_bytes(), &parsed)
        .map_err(|_| ApiError(StatusCode::UNAUTHORIZED, "invalid credentials".into()))?;

    let token = create_session(&state, id).await;
    Ok((
        [(header::SET_COOKIE, make_session_cookie(&token))],
        Json(serde_json::json!({ "id": id, "username": username, "is_admin": is_admin })),
    )
        .into_response())
}

pub async fn logout(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    if let Some(token) = session_token(&headers) {
        let _ = sqlx::query("DELETE FROM sessions WHERE token = ?").bind(&token).execute(&state.db).await;
    }
    (
        [(header::SET_COOKIE, format!("{COOKIE_NAME}=; Path=/; HttpOnly; Max-Age=0"))],
        StatusCode::NO_CONTENT,
    )
        .into_response()
}

pub async fn me(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Result<Json<MeResponse>, ApiError> {
    let (id, username, is_admin) =
        current_user(&state, &headers).await.ok_or(ApiError(StatusCode::UNAUTHORIZED, "not logged in".into()))?;
    let avatar: Option<(String,)> = sqlx::query_as("SELECT config_json FROM avatars WHERE user_id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await?;
    let avatar = avatar.and_then(|(j,)| serde_json::from_str(&j).ok());
    Ok(Json(MeResponse { id, username, is_admin, avatar }))
}

pub async fn put_avatar(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<AvatarPut>,
) -> Result<StatusCode, ApiError> {
    let (id, _, _) =
        current_user(&state, &headers).await.ok_or(ApiError(StatusCode::UNAUTHORIZED, "not logged in".into()))?;
    // 校验：只允许预期的配色字段，防止塞入任意大 JSON
    let cfg = body.config;
    let skin = cfg.get("skin").and_then(|v| v.as_str()).unwrap_or("#f0c8a0");
    let hair = cfg.get("hair").and_then(|v| v.as_str()).unwrap_or("#503018");
    let shirt = cfg.get("shirt").and_then(|v| v.as_str()).unwrap_or("#3868b0");
    let pants = cfg.get("pants").and_then(|v| v.as_str()).unwrap_or("#404048");
    for c in [skin, hair, shirt, pants] {
        if c.len() != 7 || !c.starts_with('#') || !c[1..].chars().all(|ch| ch.is_ascii_hexdigit()) {
            return Err(ApiError(StatusCode::BAD_REQUEST, "colors must be #rrggbb".into()));
        }
    }
    let normalized = serde_json::json!({ "kind": "modular", "skin": skin, "hair": hair, "shirt": shirt, "pants": pants });
    sqlx::query(
        "INSERT INTO avatars (user_id, kind, config_json, updated_at) VALUES (?, 'modular', ?, ?)
         ON CONFLICT(user_id) DO UPDATE SET config_json = excluded.config_json, updated_at = excluded.updated_at",
    )
    .bind(id)
    .bind(normalized.to_string())
    .bind(now_ts())
    .execute(&state.db)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---------- 形象生成（照片 → PixelLab 4方向） ----------

const AVATAR_SIZE: u32 = 64;
const AVATAR_TEMPLATE: &str = "mannequin";

/// POST /api/avatar/generate — multipart {image} → 异步 job → { job_id }
/// 固定 64×64, chibi proportions, mannequin template, GBA palette + force_colors.
pub async fn avatar_generate_submit(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    mut form: Multipart,
) -> Result<Response, ApiError> {
    let (id, _, _) = current_user(&state, &headers).await
        .ok_or(ApiError(StatusCode::UNAUTHORIZED, "not logged in".into()))?;

    let pixellab_key = state.pixellab_key.as_deref()
        .ok_or(ApiError(StatusCode::SERVICE_UNAVAILABLE, "avatar generation not configured".into()))?;
    let minimax_key = state.minimax_key.as_deref()
        .ok_or(ApiError(StatusCode::SERVICE_UNAVAILABLE, "vision API not configured".into()))?;

    // 解析 multipart，只取 image
    let mut image_bytes: Option<Vec<u8>> = None;
    let mut mime: Option<String> = None;
    while let Some(field) = form.next_field().await
        .map_err(|e| ApiError(StatusCode::BAD_REQUEST, e.to_string()))?
    {
        if field.name().unwrap_or("") == "image" {
            let ct = field.content_type().map(|m| m.to_string());
            let data = field.bytes().await
                .map_err(|e| ApiError(StatusCode::BAD_REQUEST, e.to_string()))?;
            if !data.is_empty() {
                if let Some(ct) = ct { mime = Some(ct); }
                image_bytes = Some(data.to_vec());
            }
        }
    }

    let bytes = image_bytes
        .ok_or(ApiError(StatusCode::BAD_REQUEST, "provide `image` field".into()))?;
    let m = mime.unwrap_or_else(|| px::detect_mime(&bytes).unwrap_or("image/png").to_string());
    info!("POST /api/avatar/generate: user={id}, {} bytes mime={m}", bytes.len());

    // MiniMax vision → text
    let desc = px::minimax_describe_image(&state.http, minimax_key, &bytes, &m).await
        .map_err(|e| ApiError(StatusCode::BAD_GATEWAY,
            format!("Vision API failed: {e}. Please try again later.")))?;
    info!("  vision description: {desc}");

    // 提交 PixelLab 4dir 生成
    let full = format!("{desc}. Low top-down 3/4 view, pixel art character, GBA-era retro style.");
    let props = Some(Proportions::Chibi);
    let (character_id, job_id) = px::pixellab_create_character_4dir(
        &state.http, pixellab_key, &full, AVATAR_SIZE, AVATAR_TEMPLATE, props,
        Some(px::GBA_PALETTE_PNG), true,
    ).await.map_err(|e| ApiError(StatusCode::BAD_GATEWAY, format!("pixellab submit: {e}")))?;
    info!("  submitted character_id={character_id} job_id={job_id}");

    // 存入内存 job store
    {
        let mut map = state.jobs.write().await;
        map.insert(job_id.clone(), AvatarJob {
            status: "processing".into(),
            user_id: id,
            description: desc,
            error: None,
        });
    }

    // 后台 poll
    let state2 = state.clone();
    let jid = job_id.clone();
    let cid = character_id.clone();
    tokio::spawn(async move {
        if let Err(e) = poll_and_store_avatar(&state2, &jid, &cid).await {
            warn!("job {jid} poll failed: {e}");
            let mut map = state2.jobs.write().await;
            if let Some(j) = map.get_mut(&jid) {
                j.status = "failed".into();
                j.error = Some(e.to_string());
            }
        }
    });

    Ok((StatusCode::ACCEPTED, Json(json!({ "job_id": job_id }))).into_response())
}

pub async fn poll_and_store_avatar(state: &AppState, job_id: &str, character_id: &str) -> anyhow::Result<()> {
    let pixellab_key = state.pixellab_key.as_deref()
        .ok_or_else(|| anyhow::anyhow!("no pixellab key"))?;

    let _last = px::poll_character(&state.http, pixellab_key, job_id, 60).await?;
    let urls = px::pixellab_character_rotation_urls(&state.http, pixellab_key, character_id).await?;
    info!("character {character_id} done, {} directions", urls.len());

    // 存储方案：将 4 个方向 URL 存入 config_json，前端加载时缩放。
    // 这样最简单，不需要后端合成 sprite sheet。
    let rotations_json: Vec<serde_json::Value> = urls.iter()
        .map(|(d, u)| json!({ "direction": d, "url": u }))
        .collect();

    // 更新 job 状态
    let user_id = {
        let map = state.jobs.read().await;
        map.get(job_id).map(|j| j.user_id).unwrap_or(0)
    };

    // 保存到数据库
    let config = json!({
        "kind": "generated",
        "character_id": character_id,
        "rotations": rotations_json,
    });
    sqlx::query(
        "INSERT INTO avatars (user_id, kind, config_json, updated_at) VALUES (?, 'generated', ?, ?)
         ON CONFLICT(user_id) DO UPDATE SET kind = 'generated', config_json = excluded.config_json, updated_at = excluded.updated_at",
    )
    .bind(user_id)
    .bind(config.to_string())
    .bind(now_ts())
    .execute(&state.db)
    .await?;

    // 更新 job 状态
    {
        let mut map = state.jobs.write().await;
        if let Some(j) = map.get_mut(job_id) {
            j.status = "completed".into();
        }
    }

    Ok(())
}

/// GET /api/avatar/generate/{job_id} — 轮询 job 状态
#[derive(Debug, Serialize)]
pub struct AvatarJobStatus {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub async fn avatar_generate_poll(
    State(state): State<Arc<AppState>>,
    Path(job_id): Path<String>,
) -> Result<Json<AvatarJobStatus>, ApiError> {
    let map = state.jobs.read().await;
    let job = map.get(&job_id)
        .ok_or(ApiError(StatusCode::NOT_FOUND, format!("no job {job_id}")))?;
    Ok(Json(AvatarJobStatus {
        status: job.status.clone(),
        error: job.error.clone(),
    }))
}

// ---------- 酒单 ----------
pub async fn get_menu() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "id": "house-menu",
        "sections": [
            { "title": "Signature Cocktails", "items": [
                { "name": "Imbibe Old Fashioned", "desc": "Bourbon, bitters, a whisper of smoke", "price": 12 },
                { "name": "Pixel Sour", "desc": "Gin, lemon, egg white, 8-bit cherry", "price": 11 },
                { "name": "Mosaic Mule", "desc": "Vodka, ginger beer, lime, copper mug", "price": 10 }
            ]},
            { "title": "Classics", "items": [
                { "name": "Negroni", "desc": "Gin, Campari, sweet vermouth", "price": 11 },
                { "name": "Margarita", "desc": "Tequila, lime, triple sec, salt rim", "price": 10 },
                { "name": "Espresso Martini", "desc": "Vodka, coffee liqueur, fresh espresso", "price": 12 }
            ]},
            { "title": "Zero Proof", "items": [
                { "name": "Garden Spritz", "desc": "Cucumber, mint, soda", "price": 7 },
                { "name": "Berry Fizz", "desc": "Mixed berries, lemon, tonic", "price": 7 }
            ]}
        ]
    }))
}

pub async fn health() -> &'static str {
    "ok"
}

// ---------- router ----------

/// 构建 HTTP 路由（与原 main.rs 完全一致）。
/// `state` 在此应用，返回的 Router 状态类型为 `()`，可直接 `axum::serve`。
/// 不含 /ws 路由（backend impl agent 后续追加）。
pub fn build_router(state: Arc<AppState>) -> Router {
    let api = Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/me", get(me))
        .route("/avatar", put(put_avatar))
        .route("/avatar/generate", post(avatar_generate_submit))
        .route("/avatar/generate/{job_id}", get(avatar_generate_poll))
        .route("/menu", get(get_menu))
        .route("/health", get(health))
        .layer(DefaultBodyLimit::max(20 * 1024 * 1024));

    let static_dir = std::env::var("STATIC_DIR").unwrap_or_else(|_| "web/dist".into());
    Router::new()
        .route("/ws/room", get(crate::realtime::session::ws_room))
        .nest("/api", api)
        .fallback_service(
            ServeDir::new(&static_dir)
                .not_found_service(ServeFile::new(format!("{static_dir}/index.html"))),
        )
        .with_state(state)
}

// rand_core OsRng 同时实现了 rand 与 argon2 的 RngCore 需求
mod hex {
    pub fn encode(bytes: [u8; 32]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }
}
