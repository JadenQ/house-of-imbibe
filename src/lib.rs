//! Shared library for the `house-of-imbibe` package.
//!
//! Exposes `pixelart` (PixelLab + MiniMax vision wrapper), `realtime`
//! (WS protocol contracts; backend impl agent fills in the rest), `assets`
//! (binary asset storage trait + local impl), and the HTTP app: `AppState`,
//! `build_router`, handlers, and session helpers. The binary (`src/main.rs`)
//! and integration tests (`tests/`) both consume this. NO business code
//! outside the package should reach into `reqwest` directly — keep provider
//! swaps possible.

pub mod assets;
pub mod pixelart;
pub mod realtime;

use crate::assets::{content_type_for_ext, AssetStore};
use crate::pixelart as px;
use crate::pixelart::Proportions;

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use argon2::password_hash::rand_core::{OsRng, RngCore};
use argon2::password_hash::{PasswordHash, PasswordHasher, SaltString, PasswordVerifier};
use argon2::Argon2;
use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, Multipart, Path, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::SqlitePool;
use tower_http::services::{ServeDir, ServeFile};
use tracing::{info, warn};

pub struct AppState {
    pub db: SqlitePool,
    pub pixellab_key: Option<String>,
    pub minimax_key: Option<String>,
    pub http: reqwest::Client,
    pub assets: Arc<dyn AssetStore + Send + Sync>,
    pub rt: Arc<crate::realtime::RealtimeState>,
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

/// POST /api/avatar/generate-text 的请求体（2c 文字入口）。
#[derive(Deserialize)]
pub struct TextGenRequest {
    pub description: String,
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

pub async fn create_session(state: &AppState, user_id: i64) -> Result<String, ApiError> {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    let token = hex::encode(bytes);
    sqlx::query("INSERT INTO sessions (token, user_id, created_at) VALUES (?, ?, ?)")
        .bind(&token)
        .bind(user_id)
        .bind(now_ts())
        .execute(&state.db)
        .await?;
    Ok(token)
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

    // 首个注册用户自动成为 admin（bootstrap 规则）。
    // 单语句原子决定 is_admin = (SELECT COUNT(*) FROM users) = 0，消除并发竞态。
    let res = sqlx::query(
        "INSERT INTO users (username, password_hash, is_admin, created_at)
         SELECT ?, ?, (SELECT COUNT(*) FROM users) = 0, ?",
    )
    .bind(&username)
    .bind(&hash)
    .bind(now_ts())
    .execute(&state.db)
    .await;
    let user_id = match res {
        Ok(r) => r.last_insert_rowid(),
        Err(_) => return Err(ApiError(StatusCode::CONFLICT, "username taken".into())),
    };

    // 读回 is_admin（INSERT...SELECT 的结果不在 last_insert_rowid 里）
    let (is_admin,): (bool,) = sqlx::query_as("SELECT is_admin FROM users WHERE id = ?")
        .bind(user_id)
        .fetch_one(&state.db)
        .await?;

    let token = create_session(&state, user_id).await?;
    Ok((
        StatusCode::CREATED,
        [(header::SET_COOKIE, make_session_cookie(&token))],
        Json(serde_json::json!({ "id": user_id, "username": username, "is_admin": is_admin })),
    )
        .into_response())
}

pub async fn login(State(state): State<Arc<AppState>>, Json(c): Json<Credentials>) -> Result<Response, ApiError> {
    let row = sqlx::query_as::<_, (i64, String, String, bool, bool)>(
        "SELECT id, username, password_hash, is_admin, banned FROM users WHERE username = ?",
    )
    .bind(c.username.trim())
    .fetch_optional(&state.db)
    .await?;
    let (id, username, hash, is_admin, banned) =
        row.ok_or(ApiError(StatusCode::UNAUTHORIZED, "invalid credentials".into()))?;

    let parsed = PasswordHash::new(&hash).map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR, "bad hash".into()))?;
    Argon2::default()
        .verify_password(c.password.as_bytes(), &parsed)
        .map_err(|_| ApiError(StatusCode::UNAUTHORIZED, "invalid credentials".into()))?;

    // 封禁用户拒绝登录（在密码验证之后，避免泄露 banned 状态）。
    if banned {
        return Err(ApiError(StatusCode::FORBIDDEN, "banned".into()));
    }

    let token = create_session(&state, id).await?;
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

/// 配色校验：#rrggbb。
fn is_color(c: &str) -> bool {
    c.len() == 7 && c.starts_with('#') && c[1..].chars().all(|ch| ch.is_ascii_hexdigit())
}

/// 把可选样式字符串约束到白名单；未知/缺失 → fallback。与前端 game/character.ts 一致。
fn whitelist(v: Option<&str>, allowed: &[&'static str], fallback: &'static str) -> &'static str {
    let s = v.unwrap_or("");
    for a in allowed.iter().copied() {
        if a == s {
            return a;
        }
    }
    fallback
}

pub async fn put_avatar(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<AvatarPut>,
) -> Result<StatusCode, ApiError> {
    let (id, _, _) =
        current_user(&state, &headers).await.ok_or(ApiError(StatusCode::UNAUTHORIZED, "not logged in".into()))?;
    // 校验：配色 #rrggbb + 样式白名单（与前端 game/character.ts 一致），防止塞入任意大 JSON。
    // 样式字段必须完整持久化 + WS 快照透传，否则远端玩家只看到默认样式。
    let cfg = &body.config;
    let skin = cfg.get("skin").and_then(|v| v.as_str()).unwrap_or("#f0c8a0");
    let hair = cfg.get("hair").and_then(|v| v.as_str()).unwrap_or("#503018");
    let shirt = cfg.get("shirt").and_then(|v| v.as_str()).unwrap_or("#3868b0");
    let pants = cfg.get("pants").and_then(|v| v.as_str()).unwrap_or("#404048");
    let shoes_opt = cfg.get("shoes").and_then(|v| v.as_str());
    for c in [skin, hair, shirt, pants] {
        if !is_color(c) {
            return Err(ApiError(StatusCode::BAD_REQUEST, "colors must be #rrggbb".into()));
        }
    }
    if let Some(s) = shoes_opt {
        if !is_color(s) {
            return Err(ApiError(StatusCode::BAD_REQUEST, "shoes must be #rrggbb".into()));
        }
    }
    let hair_style = whitelist(cfg.get("hairStyle").and_then(|v| v.as_str()), &["short", "long", "bald", "cap"], "short");
    let top_style = whitelist(cfg.get("topStyle").and_then(|v| v.as_str()), &["tshirt", "longsleeve", "vest"], "tshirt");
    let bottom_style = whitelist(cfg.get("bottomStyle").and_then(|v| v.as_str()), &["pants", "shorts", "skirt"], "pants");
    let shoe_style = whitelist(cfg.get("shoeStyle").and_then(|v| v.as_str()), &["boots", "sneakers", "sandals"], "boots");
    let mut n = serde_json::Map::new();
    n.insert("kind".into(), serde_json::Value::from("modular"));
    n.insert("skin".into(), serde_json::Value::from(skin));
    n.insert("hair".into(), serde_json::Value::from(hair));
    n.insert("shirt".into(), serde_json::Value::from(shirt));
    n.insert("pants".into(), serde_json::Value::from(pants));
    n.insert("hairStyle".into(), serde_json::Value::from(hair_style));
    n.insert("topStyle".into(), serde_json::Value::from(top_style));
    n.insert("bottomStyle".into(), serde_json::Value::from(bottom_style));
    n.insert("shoeStyle".into(), serde_json::Value::from(shoe_style));
    if let Some(s) = shoes_opt {
        n.insert("shoes".into(), serde_json::Value::from(s));
    }
    let normalized = serde_json::Value::Object(n);
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
///
/// 禁令#1 修复：handler 内绝不 await 任何 LLM/生成调用。
/// 流程：auth → 读 multipart image → put 照片字节到 AssetStore 临时 key
/// → INSERT generation_jobs(status pending) → 立即返回 {job_id}。
/// 后台 worker 认领 pending job 跑管线。
pub async fn avatar_generate_submit(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    mut form: Multipart,
) -> Result<Response, ApiError> {
    let (id, _, _) = current_user(&state, &headers).await
        .ok_or(ApiError(StatusCode::UNAUTHORIZED, "not logged in".into()))?;

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

    // 生成 job id + 临时照片 key
    let job_id = crate::assets::random_asset_id();
    let ext = crate::pixelart::mime_to_ext(&m);
    let photo_key = format!("tmp/gen/{job_id}.{ext}");

    // 照片字节落地（worker 后续读取）
    state.assets.put(&photo_key, &bytes).await
        .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR, "asset store failed".into()))?;

    // params_json 记录 photo_key + mime，供 worker 复原
    let params = json!({ "photo_key": photo_key, "mime": m });
    sqlx::query(
        "INSERT INTO generation_jobs (id, owner_id, kind, status, params_json, created_at)
         VALUES (?, ?, 'avatar', 'pending', ?, ?)",
    )
    .bind(&job_id)
    .bind(id)
    .bind(params.to_string())
    .bind(now_ts())
    .execute(&state.db)
    .await?;

    // 启动后台 worker 处理这一个 job（spawn-per-job）。
    // worker 无 key 时会把 job 标记 failed("not configured")，不影响离线测试。
    let state2 = state.clone();
    let jid = job_id.clone();
    tokio::spawn(async move {
        if let Err(e) = run_avatar_job(&state2, &jid).await {
            warn!("avatar job {jid} worker error: {e}");
        }
    });

    Ok((StatusCode::ACCEPTED, Json(json!({ "job_id": job_id }))).into_response())
}

/// POST /api/avatar/generate-text — JSON {description} → 异步 job → { job_id }（2c 文字入口）。
/// 禁令#1：handler 内绝不 await 生成调用。worker 的 text 分支复用 generate_from_description。
pub async fn avatar_generate_text(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<TextGenRequest>,
) -> Result<Response, ApiError> {
    let (id, _, _) = current_user(&state, &headers).await
        .ok_or(ApiError(StatusCode::UNAUTHORIZED, "not logged in".into()))?;
    let desc = req.description.trim();
    if desc.is_empty() {
        return Err(ApiError(StatusCode::BAD_REQUEST, "description required".into()));
    }
    if desc.chars().count() > 2000 {
        return Err(ApiError(StatusCode::BAD_REQUEST, "description too long (max 2000 chars)".into()));
    }
    let job_id = crate::assets::random_asset_id();
    let params = json!({ "mode": "text", "description": desc });
    sqlx::query(
        "INSERT INTO generation_jobs (id, owner_id, kind, status, params_json, created_at)
         VALUES (?, ?, 'avatar', 'pending', ?, ?)",
    )
    .bind(&job_id)
    .bind(id)
    .bind(params.to_string())
    .bind(now_ts())
    .execute(&state.db)
    .await?;
    let state2 = state.clone();
    let jid = job_id.clone();
    tokio::spawn(async move {
        if let Err(e) = run_avatar_job(&state2, &jid).await {
            warn!("avatar text job {jid} worker error: {e}");
        }
    });
    Ok((StatusCode::ACCEPTED, Json(json!({ "job_id": job_id }))).into_response())
}

/// 后台 worker：认领 pending job → 跑管线 → 下载 PNG 进 AssetStore → 存 key → 标记 done。
/// 无 API key → 标记 failed("not configured")，绝不 panic。
pub async fn run_avatar_job(state: &AppState, job_id: &str) -> anyhow::Result<()> {
    // 原子认领：UPDATE ... WHERE status='pending' RETURNING *
    let claimed: Option<(String, i64, String, String, Option<String>)> = sqlx::query_as(
        "UPDATE generation_jobs SET status='running' WHERE id=? AND status='pending'
         RETURNING id, owner_id, kind, status, params_json",
    )
    .bind(job_id)
    .fetch_optional(&state.db)
    .await?;
    let (_, owner_id, _, _, params_json) = match claimed {
        None => return Ok(()), // 已被其他 worker 认领或不存在
        Some(c) => c,
    };

    let params: serde_json::Value = params_json
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or(json!({}));

    // 分支：text 模式（description，只需 pixellab_key）vs photo 模式（photo_key，需 pixellab+minimax）
    let mode = params["mode"].as_str().unwrap_or("photo");
    let photo_key: Option<String> = if mode == "text" {
        None
    } else {
        params["photo_key"].as_str().map(|s| s.to_string())
    };

    let result: anyhow::Result<String> = if mode == "text" {
        let description = params["description"].as_str().unwrap_or("").to_string();
        let Some(key) = state.pixellab_key.as_deref() else {
            finish_job_failed(state, job_id, None, "not configured").await;
            return Ok(());
        };
        if description.trim().is_empty() {
            finish_job_failed(state, job_id, None, "empty description").await;
            return Ok(());
        }
        generate_from_description(state, owner_id, &description, key).await
    } else {
        let (Some(pk), Some(mk)) = (state.pixellab_key.as_deref(), state.minimax_key.as_deref()) else {
            finish_job_failed(state, job_id, photo_key.as_deref(), "not configured").await;
            return Ok(());
        };
        let mime = params["mime"].as_str().unwrap_or("image/png").to_string();
        let pkey = photo_key.clone().unwrap_or_default();
        let photo_bytes = state.assets.get(&pkey).await
            .map_err(|e| anyhow::anyhow!("asset get photo: {e}"))?
            .ok_or_else(|| anyhow::anyhow!("photo bytes not found for key {pkey}"))?;
        avatar_pipeline(state, owner_id, &photo_bytes, &mime, pk, mk).await
    };

    match result {
        Ok(asset_id) => {
            sqlx::query(
                "UPDATE generation_jobs SET status='done', result_asset_id=?, completed_at=? WHERE id=?",
            )
            .bind(&asset_id)
            .bind(now_ts())
            .bind(job_id)
            .execute(&state.db)
            .await?;
            if let Some(pk) = &photo_key {
                let _ = state.assets.delete(pk).await; // 隐私：删临时照片
            }
            info!("avatar job {job_id} done, asset_id={asset_id}");
        }
        Err(e) => {
            finish_job_failed(state, job_id, photo_key.as_deref(), &e.to_string()).await;
        }
    }
    Ok(())
}

/// 标记 job failed + 删除临时照片（如果提供）。
async fn finish_job_failed(state: &AppState, job_id: &str, photo_key: Option<&str>, error: &str) {
    if let Some(pk) = photo_key {
        let _ = state.assets.delete(pk).await;
    }
    let _ = sqlx::query(
        "UPDATE generation_jobs SET status='failed', error=?, completed_at=? WHERE id=?",
    )
    .bind(error)
    .bind(now_ts())
    .bind(job_id)
    .execute(&state.db)
    .await;
    warn!("avatar job {job_id} failed: {error}");
}

/// 照片→形象：MiniMax vision 描述文字 → generate_from_description。
async fn avatar_pipeline(
    state: &AppState,
    owner_id: i64,
    photo_bytes: &[u8],
    mime: &str,
    pixellab_key: &str,
    minimax_key: &str,
) -> anyhow::Result<String> {
    let desc = px::minimax_describe_image(&state.http, minimax_key, photo_bytes, mime).await?;
    info!("  vision description: {desc}");
    generate_from_description(state, owner_id, &desc, pixellab_key).await
}

/// 从文字描述生成 4 方向角色：create-character-with-4-directions → poll →
/// 下载 rotation PNG → AssetStore → 存 frames 契约。返回 asset_id。
/// 照片路径（vision 描述后）与文字路径（直接描述）共用此函数。
async fn generate_from_description(
    state: &AppState,
    owner_id: i64,
    description: &str,
    pixellab_key: &str,
) -> anyhow::Result<String> {
    // 1. 提交 PixelLab 4dir 生成（GBA 调色板锁色）
    let full = format!("{description}. Low top-down 3/4 view, pixel art character, GBA-era retro style.");
    let props = Some(Proportions::Chibi);
    let (character_id, pixellab_job_id) = px::pixellab_create_character_4dir(
        &state.http, pixellab_key, &full, AVATAR_SIZE, AVATAR_TEMPLATE, props,
        Some(px::GBA_PALETTE_PNG), true,
    ).await?;
    info!("  submitted character_id={character_id} job_id={pixellab_job_id}");

    // 2. 轮询完成
    let _last = px::poll_character(&state.http, pixellab_key, &pixellab_job_id, 60).await?;

    // 3. 取 rotation URL
    let urls = px::pixellab_character_rotation_urls(&state.http, pixellab_key, &character_id).await?;
    info!("character {character_id} done, {} directions", urls.len());

    // 4. 下载每个 rotation PNG → AssetStore → frames 契约
    //    每方向帧 key 数组（1 帧=静站，3 帧=行走。当前只出 1 静态帧；
    //    animate-character 步骤将补 3 行走帧——见 issue #0010 / CLAUDE.md。）
    let mut frames: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();
    for (dir, url) in &urls {
        let png_bytes = state.http.get(url).send().await?.error_for_status()?.bytes().await?;
        let key = format!("avatar/{owner_id}/{dir}.png");
        state.assets.put(&key, &png_bytes).await
            .map_err(|e| anyhow::anyhow!("asset put {key}: {e}"))?;
        frames.insert(dir.clone(), json!([key]));
    }
    let frames_val = serde_json::Value::Object(frames);

    // 5. asset 记录（result_asset_id 指向它）
    let asset_id = crate::assets::random_asset_id();
    let meta = json!({ "character_id": character_id, "frames": frames_val.clone() });
    sqlx::query(
        "INSERT INTO assets (id, owner_id, kind, storage_key, meta_json, created_at)
         VALUES (?, ?, 'avatar', '', ?, ?)",
    )
    .bind(&asset_id)
    .bind(owner_id)
    .bind(meta.to_string())
    .bind(now_ts())
    .execute(&state.db)
    .await?;

    // 6. avatars.config_json（存 key 不存 URL；契约 frames:{dir:[key…]}）
    let config = json!({
        "kind": "generated",
        "character_id": character_id,
        "frames": frames_val,
    });
    sqlx::query(
        "INSERT INTO avatars (user_id, kind, config_json, updated_at) VALUES (?, 'generated', ?, ?)
         ON CONFLICT(user_id) DO UPDATE SET kind = 'generated', config_json = excluded.config_json, updated_at = excluded.updated_at",
    )
    .bind(owner_id)
    .bind(config.to_string())
    .bind(now_ts())
    .execute(&state.db)
    .await?;

    Ok(asset_id)
}

/// GET /api/avatar/generate/{job_id} — 轮询 job 状态（读 generation_jobs 表，DB 为唯一真相源）
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
    let row: Option<(String, Option<String>)> = sqlx::query_as(
        "SELECT status, error FROM generation_jobs WHERE id = ?",
    )
    .bind(&job_id)
    .fetch_optional(&state.db)
    .await?;
    let (status, error) = row
        .ok_or(ApiError(StatusCode::NOT_FOUND, format!("no job {job_id}")))?;
    Ok(Json(AvatarJobStatus { status, error }))
}

// ---------- 资产服务 ----------

/// GET /api/assets/:key — 返回字节流 + 按扩展名 content-type。
/// 404 不存在、400 穿越尝试。
pub async fn serve_asset(
    State(state): State<Arc<AppState>>,
    Path(key): Path<String>,
) -> Result<Response, ApiError> {
    // 穿越检查在 AssetStore::get 内部，但提前拦截含 `..` 的返回 400。
    if key.contains("..") {
        return Err(ApiError(StatusCode::BAD_REQUEST, "invalid key".into()));
    }
    let bytes = state.assets.get(&key).await
        .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "invalid key".into()))?;
    let bytes = bytes
        .ok_or(ApiError(StatusCode::NOT_FOUND, "asset not found".into()))?;
    let ct = content_type_for_ext(&key);
    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, ct)],
        Bytes::from(bytes),
    ).into_response())
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

// ---------- admin（成员管理）----------

#[derive(Serialize)]
pub struct MemberRow {
    pub id: i64,
    pub username: String,
    pub is_admin: bool,
    pub banned: bool,
}

/// admin 门禁：返回当前用户 (id, is_admin)，非 admin → 403。
async fn require_admin(state: &AppState, headers: &HeaderMap) -> Result<i64, ApiError> {
    let (id, _, is_admin) = current_user(state, headers)
        .await
        .ok_or(ApiError(StatusCode::UNAUTHORIZED, "not logged in".into()))?;
    if !is_admin {
        return Err(ApiError(StatusCode::FORBIDDEN, "admin only".into()));
    }
    Ok(id)
}

/// GET /api/admin/members → 200 JSON [{id, username, is_admin, banned}]（按 id 排序）。
pub async fn admin_list_members(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<MemberRow>>, ApiError> {
    require_admin(&state, &headers).await?;
    let rows: Vec<(i64, String, bool, bool)> =
        sqlx::query_as("SELECT id, username, is_admin, banned FROM users ORDER BY id")
            .fetch_all(&state.db)
            .await?;
    let members = rows
        .into_iter()
        .map(|(id, username, is_admin, banned)| MemberRow { id, username, is_admin, banned })
        .collect();
    Ok(Json(members))
}

/// 通用：admin 修改成员标志位（promote/demote/ban/unban 共用）。
/// 不允许对自己操作（409 cannot modify self）；目标不存在 → 404。
async fn admin_update_member(
    state: &AppState,
    headers: &HeaderMap,
    target_id: i64,
    sql: &str,
) -> Result<StatusCode, ApiError> {
    let self_id = require_admin(state, headers).await?;
    if target_id == self_id {
        return Err(ApiError(StatusCode::CONFLICT, "cannot modify self".into()));
    }
    let res = sqlx::query(sql).bind(target_id).execute(&state.db).await?;
    if res.rows_affected() == 0 {
        return Err(ApiError(StatusCode::NOT_FOUND, "member not found".into()));
    }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn admin_promote(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(target_id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    admin_update_member(&state, &headers, target_id, "UPDATE users SET is_admin = 1 WHERE id = ?").await
}

pub async fn admin_demote(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(target_id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    admin_update_member(&state, &headers, target_id, "UPDATE users SET is_admin = 0 WHERE id = ?").await
}

pub async fn admin_ban(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(target_id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    admin_update_member(&state, &headers, target_id, "UPDATE users SET banned = 1 WHERE id = ?").await
}

pub async fn admin_unban(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(target_id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    admin_update_member(&state, &headers, target_id, "UPDATE users SET banned = 0 WHERE id = ?").await
}

// ---------- router ----------

/// 构建 HTTP 路由（与原 main.rs 完全一致）。
/// `state` 在此应用，返回的 Router 状态类型为 `()`，可直接 `axum::serve`。
/// 不含 /ws 路由（backend impl agent 后续追加）。
pub fn build_router(state: Arc<AppState>) -> Router {
    let admin = Router::new()
        .route("/members", get(admin_list_members))
        .route("/members/{id}/promote", post(admin_promote))
        .route("/members/{id}/demote", post(admin_demote))
        .route("/members/{id}/ban", post(admin_ban))
        .route("/members/{id}/unban", post(admin_unban));

    let api = Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/me", get(me))
        .route("/avatar", put(put_avatar))
        .route("/avatar/generate", post(avatar_generate_submit))
        .route("/avatar/generate-text", post(avatar_generate_text))
        .route("/avatar/generate/{job_id}", get(avatar_generate_poll))
        .route("/assets/{key}", get(serve_asset))
        .route("/menu", get(get_menu))
        .route("/health", get(health))
        .nest("/admin", admin)
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
