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
use axum::extract::{DefaultBodyLimit, Multipart, Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post, put};
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

/// 校验 slot ∈ {back, hand, hat, face}（accessories 四槽位；hat/face=帽/眼镜面饰）。
fn validate_slot(slot: &str) -> Result<(), ApiError> {
    if !matches!(slot, "back" | "hand" | "hat" | "face") {
        return Err(ApiError(StatusCode::BAD_REQUEST, "slot must be 'back', 'hand', 'hat', or 'face'".into()));
    }
    Ok(())
}

/// 校验 equipped 数组结构：每项 slot ∈ {back, hand, hat, face}；至多 4 项（每 slot 一条）。
/// 用于 put_avatar 透传（不剥离，像样式字段一样）。equipped 由 equip 端点写入，
/// 这里只做结构校验防注入任意大 JSON。
fn validate_equipped(equipped: &serde_json::Value) -> Result<serde_json::Value, ApiError> {
    let arr = equipped
        .as_array()
        .ok_or_else(|| ApiError(StatusCode::BAD_REQUEST, "equipped must be an array".into()))?;
    if arr.len() > 4 {
        return Err(ApiError(StatusCode::BAD_REQUEST, "equipped has at most 4 items".into()));
    }
    for item in arr {
        let slot = item
            .get("slot")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ApiError(StatusCode::BAD_REQUEST, "equipped item missing slot".into()))?;
        validate_slot(slot)?;
    }
    Ok(equipped.clone())
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
    // Preserve equipped field (like style fields; validated structure).
    if let Some(equipped) = cfg.get("equipped") {
        n.insert("equipped".into(), validate_equipped(equipped)?);
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

// ---------- accessories（equip/unequip）----------

/// POST /api/avatar/equip {slot, asset_id} → 200 {avatar: config_json}。
/// 校验 slot ∈ {back,hand,hat,face} + asset 存在且 owner_id=自己 → 查 storage_key 作
/// asset_key → equipped 里替换同 slot（或追加）→ UPDATE config_json。
/// kind=modular 或 generated 都允许（D4，不拒绝 generated）。
#[derive(Deserialize)]
pub struct EquipRequest {
    pub slot: String,
    pub asset_id: String,
}

/// POST /api/avatar/unequip {slot} → 200 {avatar: config_json}。
#[derive(Deserialize)]
pub struct UnequipRequest {
    pub slot: String,
}

pub async fn avatar_equip(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<EquipRequest>,
) -> Result<Response, ApiError> {
    let (id, _, _) = current_user(&state, &headers)
        .await
        .ok_or(ApiError(StatusCode::UNAUTHORIZED, "not logged in".into()))?;
    validate_slot(&body.slot)?;

    // asset 存在且 owner_id = 自己
    let asset_row: Option<(String, i64)> =
        sqlx::query_as("SELECT storage_key, owner_id FROM assets WHERE id = ?")
            .bind(&body.asset_id)
            .fetch_optional(&state.db)
            .await?;
    let (storage_key, owner_id) = asset_row
        .ok_or(ApiError(StatusCode::NOT_FOUND, "asset not found".into()))?;
    if owner_id != id {
        return Err(ApiError(StatusCode::FORBIDDEN, "not asset owner".into()));
    }

    // 读当前 config_json
    let (cfg_str,): (String,) =
        sqlx::query_as("SELECT config_json FROM avatars WHERE user_id = ?")
            .bind(id)
            .fetch_optional(&state.db)
            .await?
            .ok_or(ApiError(StatusCode::NOT_FOUND, "no avatar; save one first".into()))?;
    let mut cfg: serde_json::Value = serde_json::from_str(&cfg_str)
        .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR, "bad config_json".into()))?;

    // 替换同 slot 或追加（每 slot 至多一条）
    let entry = json!({
        "slot": body.slot.as_str(),
        "asset_id": body.asset_id.as_str(),
        "asset_key": storage_key.as_str(),
    });
    match cfg.get_mut("equipped").and_then(|v| v.as_array_mut()) {
        Some(arr) => {
            let slot_str = body.slot.as_str();
            let idx = arr
                .iter()
                .position(|item| item.get("slot").and_then(|v| v.as_str()) == Some(slot_str));
            match idx {
                Some(i) => arr[i] = entry,
                None => arr.push(entry),
            }
        }
        None => {
            cfg["equipped"] = serde_json::Value::Array(vec![entry]);
        }
    }

    sqlx::query("UPDATE avatars SET config_json = ?, updated_at = ? WHERE user_id = ?")
        .bind(cfg.to_string())
        .bind(now_ts())
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok((StatusCode::OK, Json(json!({ "avatar": cfg }))).into_response())
}

pub async fn avatar_unequip(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<UnequipRequest>,
) -> Result<Response, ApiError> {
    let (id, _, _) = current_user(&state, &headers)
        .await
        .ok_or(ApiError(StatusCode::UNAUTHORIZED, "not logged in".into()))?;
    validate_slot(&body.slot)?;

    let (cfg_str,): (String,) =
        sqlx::query_as("SELECT config_json FROM avatars WHERE user_id = ?")
            .bind(id)
            .fetch_optional(&state.db)
            .await?
            .ok_or(ApiError(StatusCode::NOT_FOUND, "no avatar; save one first".into()))?;
    let mut cfg: serde_json::Value = serde_json::from_str(&cfg_str)
        .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR, "bad config_json".into()))?;

    // 移除该 slot（equipped 不存在 → no-op）
    if let Some(arr) = cfg.get_mut("equipped").and_then(|v| v.as_array_mut()) {
        let slot_str = body.slot.as_str();
        arr.retain(|item| item.get("slot").and_then(|v| v.as_str()) != Some(slot_str));
    }

    sqlx::query("UPDATE avatars SET config_json = ?, updated_at = ? WHERE user_id = ?")
        .bind(cfg.to_string())
        .bind(now_ts())
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok((StatusCode::OK, Json(json!({ "avatar": cfg }))).into_response())
}

// ---------- 形象生成（照片 → PixelLab 4方向） ----------

const AVATAR_SIZE: u32 = 48;
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
        if let Err(e) = run_generation_job(&state2, &jid).await {
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
        if let Err(e) = run_generation_job(&state2, &jid).await {
            warn!("avatar text job {jid} worker error: {e}");
        }
    });
    Ok((StatusCode::ACCEPTED, Json(json!({ "job_id": job_id }))).into_response())
}

/// 后台 worker：认领 pending job → 按 kind 分发 → 标记 done/failed。
/// 无 API key → 标记 failed("not configured")，绝不 panic。
/// kind='avatar' 走 avatar 管线（photo/text 双模式）；kind='map_bg' 走地图背景管线。
pub async fn run_generation_job(state: &AppState, job_id: &str) -> anyhow::Result<()> {
    // 原子认领：UPDATE ... WHERE status='pending' RETURNING *
    let claimed: Option<(String, i64, String, String, Option<String>)> = sqlx::query_as(
        "UPDATE generation_jobs SET status='running' WHERE id=? AND status='pending'
         RETURNING id, owner_id, kind, status, params_json",
    )
    .bind(job_id)
    .fetch_optional(&state.db)
    .await?;
    let (_, owner_id, kind, _, params_json) = match claimed {
        None => return Ok(()), // 已被其他 worker 认领或不存在
        Some(c) => c,
    };

    let params: serde_json::Value = params_json
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or(json!({}));

    match kind.as_str() {
        "avatar" => run_avatar_body(state, job_id, owner_id, params).await,
        "map_bg" => run_map_bg_body(state, job_id, owner_id, params).await,
        "map_tileset" => run_map_tileset_body(state, job_id, owner_id, params).await,
        other => {
            finish_job_failed(state, job_id, None, &format!("unknown kind: {other}")).await;
        }
    }
    Ok(())
}

/// avatar job body：认领后的 avatar 生成逻辑（photo/text 双模式）。
async fn run_avatar_body(
    state: &AppState,
    job_id: &str,
    owner_id: i64,
    params: serde_json::Value,
) {
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
            return;
        };
        if description.trim().is_empty() {
            finish_job_failed(state, job_id, None, "empty description").await;
            return;
        }
        generate_from_description(state, owner_id, &description, key).await
    } else {
        let (Some(pk), Some(mk)) = (state.pixellab_key.as_deref(), state.minimax_key.as_deref()) else {
            finish_job_failed(state, job_id, photo_key.as_deref(), "not configured").await;
            return;
        };
        let mime = params["mime"].as_str().unwrap_or("image/png").to_string();
        let pkey = photo_key.clone().unwrap_or_default();
        match state.assets.get(&pkey).await {
            Ok(Some(bytes)) => avatar_pipeline(state, owner_id, &bytes, &mime, pk, mk).await,
            Ok(None) => Err(anyhow::anyhow!("photo bytes not found for key {pkey}")),
            Err(e) => Err(anyhow::anyhow!("asset get photo: {e}")),
        }
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
            .await
            .ok();
            if let Some(pk) = &photo_key {
                let _ = state.assets.delete(pk).await; // 隐私：删临时照片
            }
            info!("avatar job {job_id} done, asset_id={asset_id}");
        }
        Err(e) => {
            finish_job_failed(state, job_id, photo_key.as_deref(), &e.to_string()).await;
        }
    }
}

/// map_bg job body：认领后的地图背景图生成逻辑。
async fn run_map_bg_body(
    state: &AppState,
    job_id: &str,
    owner_id: i64,
    params: serde_json::Value,
) {
    let scene = params["scene"].as_str().unwrap_or("bar").to_string();
    let prompt = params["prompt"].as_str().unwrap_or("").to_string();

    let Some(key) = state.pixellab_key.as_deref() else {
        finish_job_failed(state, job_id, None, "not configured").await;
        return;
    };
    if prompt.trim().is_empty() {
        finish_job_failed(state, job_id, None, "empty prompt").await;
        return;
    }

    match map_bg_pipeline(state, job_id, owner_id, &scene, &prompt, key).await {
        Ok(asset_id) => {
            sqlx::query(
                "UPDATE generation_jobs SET status='done', result_asset_id=?, completed_at=? WHERE id=?",
            )
            .bind(&asset_id)
            .bind(now_ts())
            .bind(job_id)
            .execute(&state.db)
            .await
            .ok();
            info!("map_bg job {job_id} done, asset_id={asset_id}");
        }
        Err(e) => {
            finish_job_failed(state, job_id, None, &e.to_string()).await;
        }
    }
}

/// 地图背景图生成：文字 → create-image-pixen（同步）→ PNG bytes → AssetStore
/// → INSERT assets → UPDATE maps.bg_key。返回 asset_id（generation_jobs.result_asset_id）。
/// 256×256 标准尺寸档（240×160 非标准导致质量差，见 docs/reference/pixellab-api.md §七）；
/// 前端 BarScene.loadBgImage 已 setDisplaySize 到 240×160 拉伸显示，行走网格（grid.rs）不变。
async fn map_bg_pipeline(
    state: &AppState,
    job_id: &str,
    owner_id: i64,
    scene: &str,
    prompt: &str,
    pixellab_key: &str,
) -> anyhow::Result<String> {
    // 1. 调 create-image-pixen（同步 ~30-120s，256×256 标准档；GBA 锁风格）
    let full = format!(
        "{prompt}. GBA Emerald-era 16-bit pixel art, retro game bar interior, wooden floor \
         and bar counter with shelves of bottles, stools and tables scattered around, warm \
         dim ambient lighting, limited color palette, top-down 3/4 view, no characters."
    );
    let png_bytes = px::pixellab_create_image_pixen_wh(
        &state.http, pixellab_key, &full, 256, 256,
    )
    .await?;

    // 2. 存储 PNG 到 AssetStore（存 key 不存 URL）
    let storage_key = format!("map/{scene}/{job_id}.png");
    state
        .assets
        .put(&storage_key, &png_bytes)
        .await
        .map_err(|e| anyhow::anyhow!("asset put {storage_key}: {e}"))?;

    // 3. INSERT assets 行（kind 'map_bg'）
    let asset_id = crate::assets::random_asset_id();
    let meta = json!({ "scene": scene, "prompt": prompt });
    sqlx::query(
        "INSERT INTO assets (id, owner_id, kind, storage_key, meta_json, created_at)
         VALUES (?, ?, 'map_bg', ?, ?, ?)",
    )
    .bind(&asset_id)
    .bind(owner_id)
    .bind(&storage_key)
    .bind(meta.to_string())
    .bind(now_ts())
    .execute(&state.db)
    .await?;

    // 4. UPDATE maps.bg_key（视觉背景层切换；行走网格层不变）
    sqlx::query("UPDATE maps SET bg_key = ?, updated_at = ? WHERE scene = ?")
        .bind(&storage_key)
        .bind(now_ts())
        .bind(scene)
        .execute(&state.db)
        .await?;

    info!("map_bg pipeline done: scene={scene}, asset_id={asset_id}, key={storage_key}");
    Ok(asset_id)
}

/// map_tileset job body：认领后的 tileset 生成逻辑。
/// ⚠️ REST /v2/create-tileset 端点未实测；422/失败 → job failed 但不崩服务。
async fn run_map_tileset_body(
    state: &AppState,
    job_id: &str,
    owner_id: i64,
    params: serde_json::Value,
) {
    let prompt = params["prompt"].as_str().unwrap_or("").to_string();
    let tile_size = params["tile_size"].as_u64().unwrap_or(32) as u32;

    let Some(key) = state.pixellab_key.as_deref() else {
        finish_job_failed(state, job_id, None, "not configured").await;
        return;
    };
    if prompt.trim().is_empty() {
        finish_job_failed(state, job_id, None, "empty prompt").await;
        return;
    }

    match map_tileset_pipeline(state, job_id, owner_id, &prompt, tile_size, key).await {
        Ok(asset_id) => {
            sqlx::query(
                "UPDATE generation_jobs SET status='done', result_asset_id=?, completed_at=? WHERE id=?",
            )
            .bind(&asset_id)
            .bind(now_ts())
            .bind(job_id)
            .execute(&state.db)
            .await
            .ok();
            info!("map_tileset job {job_id} done, asset_id={asset_id}");
        }
        Err(e) => {
            finish_job_failed(state, job_id, None, &e.to_string()).await;
        }
    }
}

/// tileset 生成：文字 → create-tileset（同步）→ PNG bytes → AssetStore → INSERT assets。
/// 返回 asset_id。⚠️ REST 端点未实测；若 422 返回 Err，job 标记 failed。
async fn map_tileset_pipeline(
    state: &AppState,
    job_id: &str,
    owner_id: i64,
    prompt: &str,
    tile_size: u32,
    pixellab_key: &str,
) -> anyhow::Result<String> {
    let full = format!(
        "{prompt}. GBA Emerald-era 16-bit pixel art, top-down tileable floor and wall tiles."
    );
    let png_bytes = px::pixellab_create_tileset(
        &state.http, pixellab_key, &full, tile_size,
    )
    .await?;

    // 存储 PNG 到 AssetStore（存 key 不存 URL）
    let storage_key = format!("map/tileset/{job_id}.png");
    state
        .assets
        .put(&storage_key, &png_bytes)
        .await
        .map_err(|e| anyhow::anyhow!("asset put {storage_key}: {e}"))?;

    let asset_id = crate::assets::random_asset_id();
    let meta = json!({ "prompt": prompt, "tile_size": tile_size });
    sqlx::query(
        "INSERT INTO assets (id, owner_id, kind, storage_key, meta_json, created_at)
         VALUES (?, ?, 'map_tileset', ?, ?, ?)",
    )
    .bind(&asset_id)
    .bind(owner_id)
    .bind(&storage_key)
    .bind(meta.to_string())
    .bind(now_ts())
    .execute(&state.db)
    .await?;

    info!("map_tileset pipeline done: asset_id={asset_id}, key={storage_key}");
    Ok(asset_id)
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
/// 下载 rotation PNG → animate-character（4 方向行走动画）→ 下载动画帧 →
/// 存 frames 契约。返回 asset_id。
/// 照片路径（vision 描述后）与文字路径（直接描述）共用此函数。
/// animate 失败时 fallback 到单帧静站（frames:{dir:[key]}），不阻断形象创建。
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

    // 4. 下载每个 rotation PNG → AssetStore → 单帧 key（animate 失败时的 fallback）
    //    每方向先存 1 张静站帧；animate 步骤（下一步）成功后覆盖为多帧。
    let mut single_keys: Vec<(String, String)> = Vec::new();
    for (dir, url) in &urls {
        let png_bytes = state.http.get(url).send().await?.error_for_status()?.bytes().await?;
        let key = format!("avatar/{owner_id}/{dir}.png");
        state.assets.put(&key, &png_bytes).await
            .map_err(|e| anyhow::anyhow!("asset put {key}: {e}"))?;
        single_keys.push((dir.clone(), key));
    }

    // 5. animate-character：4 方向行走动画（v3, 4 帧/方向, keep_first_frame=false）
    //    成功 → frames:{dir:[key×4]}（帧0=静站, 帧1-3=行走）。
    //    失败（422/余额不足/超时）→ fallback 到单帧 frames:{dir:[key]}，不阻断形象创建。
    let anim_result = animate_walk_frames(state, owner_id, &character_id, pixellab_key).await;
    let mut frames: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();
    match anim_result {
        Ok(multi) if !multi.is_empty() => {
            for (dir, keys) in &multi {
                frames.insert(dir.clone(), json!(keys));
            }
        }
        _ => {
            // fallback: 单帧静站（rotation PNG 已存好）
            for (dir, key) in &single_keys {
                frames.insert(dir.clone(), json!([key]));
            }
        }
    }
    let frames_val = serde_json::Value::Object(frames);

    // 6. asset 记录（result_asset_id 指向它）
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

    // 7. avatars.config_json（存 key 不存 URL；契约 frames:{dir:[key…]}）
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

/// Animate a character's walk cycle (4 directions, 4 frames each) and store
/// the animation frames in AssetStore. Returns `[(dir, [key, ...])]` — empty on failure.
///
/// v3 mode, frame_count=4, keep_first_frame=false → exactly 4 frames per direction.
/// Cost ≈ $0.0129/direction × 4 ≈ $0.052. If animate fails (422, insufficient
/// balance, timeout, etc.), returns empty Vec — caller falls back to single-frame
/// rotation_urls (already stored in `generate_from_description` step 4).
///
/// Frame keys: `avatar/{owner_id}/{dir}_{i}.png` (i = 0..3).
/// frames 契约: frames:{dir:[key0,key1,key2,key3]} — 帧0=静站, 帧1-3=行走。
async fn animate_walk_frames(
    state: &AppState,
    owner_id: i64,
    character_id: &str,
    pixellab_key: &str,
) -> anyhow::Result<Vec<(String, Vec<String>)>> {
    // 1. 提交 animate jobs（每方向一个 background job）
    let dir_jobs = px::pixellab_animate_character(
        &state.http, pixellab_key, character_id,
        &["south", "north", "east", "west"], 4, "walk",
    ).await?;
    info!("  animate submitted: {} direction jobs for character {character_id}", dir_jobs.len());

    // 2. 轮询每个方向 job 到完成（复用 poll_character，5s 间隔，最多 60 次=5min）
    for (dir, job_id) in &dir_jobs {
        let _ = px::poll_character(&state.http, pixellab_key, job_id, 60).await
            .map_err(|e| anyhow::anyhow!("animate poll {dir}: {e}"))?;
    }
    info!("  animate jobs done for character {character_id}");

    // 3. GET /v2/characters/{id} → animations（每方向动画帧 URL 列表）
    let anims = px::pixellab_character_animations(
        &state.http, pixellab_key, character_id,
    ).await?;
    if anims.is_empty() {
        anyhow::bail!("no animations returned for character {character_id}");
    }

    // 4. 下载每帧 PNG → AssetStore → 收集 key
    let mut out = Vec::new();
    for (dir, urls) in &anims {
        let mut keys = Vec::new();
        for (i, url) in urls.iter().enumerate() {
            let png_bytes = state.http.get(url).send().await?
                .error_for_status()?.bytes().await?;
            let key = format!("avatar/{owner_id}/{dir}_{i}.png");
            state.assets.put(&key, &png_bytes).await
                .map_err(|e| anyhow::anyhow!("asset put {key}: {e}"))?;
            keys.push(key);
        }
        if !keys.is_empty() {
            out.push((dir.clone(), keys));
        }
    }
    Ok(out)
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

// ---------- 形象 job 列表 ----------

/// GET /api/avatar/jobs → 200 [{id, kind, status, params_json?, created_at}]
/// 列当前用户自己的形象生成 job（按 created_at DESC）。status 对齐 DB 枚举
/// （pending/running/done/failed）。params_json 原样解析回 JSON（可能含 photo_key
/// 等临时信息，仅 owner 可见）。
pub async fn avatar_list_jobs(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    let (id, _, _) = current_user(&state, &headers)
        .await
        .ok_or(ApiError(StatusCode::UNAUTHORIZED, "not logged in".into()))?;
    let rows = sqlx::query_as::<_, (String, String, String, Option<String>, i64)>(
        "SELECT id, kind, status, params_json, created_at FROM generation_jobs
         WHERE owner_id = ? ORDER BY created_at DESC",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;
    let jobs = rows
        .iter()
        .map(|(id, kind, status, params_json, created_at)| {
            let params: Option<serde_json::Value> = params_json
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok());
            json!({
                "id": id,
                "kind": kind,
                "status": status,
                "params_json": params,
                "created_at": created_at,
            })
        })
        .collect();
    Ok(Json(jobs))
}

// ---------- map（视觉背景层）----------

/// GET /api/map?scene=bar 和 GET /api/admin/map?scene=bar 的查询参数。
#[derive(Deserialize)]
pub struct MapQuery {
    pub scene: Option<String>,
}

/// POST /api/admin/map/regenerate 的请求体。
#[derive(Deserialize)]
pub struct MapRegenerateRequest {
    pub prompt: String,
    pub scene: Option<String>,
}

/// 查地图行 → {scene, width, height, bg_key, walkable}。不存在 → 404。
/// walkable = JSON 2D 数组（0=可走,1=阻挡）；NULL → null（前端用静态 BAR_MAP 兜底）。
async fn fetch_map(state: &AppState, scene: &str) -> Result<serde_json::Value, ApiError> {
    let row = sqlx::query_as::<_, (String, i64, i64, Option<String>, Option<String>)>(
        "SELECT scene, width, height, bg_key, walkable FROM maps WHERE scene = ?",
    )
    .bind(scene)
    .fetch_optional(&state.db)
    .await?;
    let (scene, width, height, bg_key, walkable_str) = row
        .ok_or(ApiError(StatusCode::NOT_FOUND, "scene not found".into()))?;
    let walkable: Option<serde_json::Value> = walkable_str
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok());
    Ok(json!({
        "scene": scene,
        "width": width,
        "height": height,
        "bg_key": bg_key,
        "walkable": walkable,
    }))
}

/// GET /api/map?scene=bar → 200 {scene, width, height, bg_key}（公开，前端 BarScene 拉 bg_key）。
pub async fn get_map(
    State(state): State<Arc<AppState>>,
    Query(q): Query<MapQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let scene = q.scene.as_deref().unwrap_or("bar");
    Ok(Json(fetch_map(&state, scene).await?))
}

/// GET /api/admin/map?scene=bar → 200 {scene, width, height, bg_key}（admin 403 门禁）。
pub async fn admin_get_map(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<MapQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_admin(&state, &headers).await?;
    let scene = q.scene.as_deref().unwrap_or("bar");
    Ok(Json(fetch_map(&state, scene).await?))
}

/// POST /api/admin/map/regenerate {prompt, scene?} → 202 {job_id}（admin 403 门禁）。
/// 禁令#1：handler 内不 await 生成调用。INSERT generation_jobs(kind='map_bg', pending) → spawn worker。
pub async fn admin_map_regenerate(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<MapRegenerateRequest>,
) -> Result<Response, ApiError> {
    let admin_id = require_admin(&state, &headers).await?;
    let prompt = body.prompt.trim();
    if prompt.is_empty() {
        return Err(ApiError(StatusCode::BAD_REQUEST, "prompt required".into()));
    }
    if prompt.chars().count() > 2000 {
        return Err(ApiError(StatusCode::BAD_REQUEST, "prompt too long (max 2000 chars)".into()));
    }
    let scene = body.scene.as_deref().unwrap_or("bar");
    let job_id = crate::assets::random_asset_id();
    let params = json!({ "scene": scene, "prompt": prompt });
    sqlx::query(
        "INSERT INTO generation_jobs (id, owner_id, kind, status, params_json, created_at)
         VALUES (?, ?, 'map_bg', 'pending', ?, ?)",
    )
    .bind(&job_id)
    .bind(admin_id)
    .bind(params.to_string())
    .bind(now_ts())
    .execute(&state.db)
    .await?;

    let state2 = state.clone();
    let jid = job_id.clone();
    tokio::spawn(async move {
        if let Err(e) = run_generation_job(&state2, &jid).await {
            warn!("map_bg job {jid} worker error: {e}");
        }
    });

    Ok((StatusCode::ACCEPTED, Json(json!({ "job_id": job_id }))).into_response())
}

/// POST /api/admin/map/tileset {prompt, tile_size?} → 202 {job_id}（admin 403 门禁）。
/// 生成 top-down 可拼接 tileset → 存 assets。复用 generation_jobs 异步。
/// ⚠️ REST /v2/create-tileset 端点未实测确认；若 API 返回 422 job 会失败但不崩。
#[derive(Deserialize)]
pub struct MapTilesetRequest {
    pub prompt: String,
    #[serde(default = "default_tile_size")]
    pub tile_size: u32,
}

fn default_tile_size() -> u32 {
    32
}

pub async fn admin_map_tileset(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<MapTilesetRequest>,
) -> Result<Response, ApiError> {
    let admin_id = require_admin(&state, &headers).await?;
    let prompt = body.prompt.trim();
    if prompt.is_empty() {
        return Err(ApiError(StatusCode::BAD_REQUEST, "prompt required".into()));
    }
    if prompt.chars().count() > 2000 {
        return Err(ApiError(StatusCode::BAD_REQUEST, "prompt too long (max 2000 chars)".into()));
    }
    // tile_size 只接受 16/32（docs/reference/pixellab-api.md §七 cost table）
    let tile_size = match body.tile_size {
        16 | 32 => body.tile_size,
        _ => 32, // 非法值 → 默认 32
    };
    let job_id = crate::assets::random_asset_id();
    let params = json!({ "prompt": prompt, "tile_size": tile_size });
    sqlx::query(
        "INSERT INTO generation_jobs (id, owner_id, kind, status, params_json, created_at)
         VALUES (?, ?, 'map_tileset', 'pending', ?, ?)",
    )
    .bind(&job_id)
    .bind(admin_id)
    .bind(params.to_string())
    .bind(now_ts())
    .execute(&state.db)
    .await?;

    let state2 = state.clone();
    let jid = job_id.clone();
    tokio::spawn(async move {
        if let Err(e) = run_generation_job(&state2, &jid).await {
            warn!("map_tileset job {jid} worker error: {e}");
        }
    });

    Ok((StatusCode::ACCEPTED, Json(json!({ "job_id": job_id }))).into_response())
}

/// PUT /api/admin/map/walkable {scene, walkable} → 204。
/// walkable = 2D 数组（0=可走,1=阻挡）。校验结构后存 JSON 文本到 maps.walkable。
/// 场景不存在 → 404。
#[derive(Deserialize)]
pub struct WalkableRequest {
    pub scene: String,
    pub walkable: serde_json::Value,
}

pub async fn admin_set_walkable(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<WalkableRequest>,
) -> Result<StatusCode, ApiError> {
    require_admin(&state, &headers).await?;
    // walkable 必须是 2D 数组，每格 0 或 1（防注入任意大 JSON）。
    let grid = body.walkable.as_array()
        .ok_or_else(|| ApiError(StatusCode::BAD_REQUEST, "walkable must be a 2D array".into()))?;
    for row in grid {
        let r = row.as_array()
            .ok_or_else(|| ApiError(StatusCode::BAD_REQUEST, "walkable rows must be arrays".into()))?;
        for cell in r {
            let n = cell.as_i64()
                .ok_or_else(|| ApiError(StatusCode::BAD_REQUEST, "walkable cells must be 0 or 1".into()))?;
            if n != 0 && n != 1 {
                return Err(ApiError(StatusCode::BAD_REQUEST, "walkable cells must be 0 or 1".into()));
            }
        }
    }
    let res = sqlx::query("UPDATE maps SET walkable = ?, updated_at = ? WHERE scene = ?")
        .bind(body.walkable.to_string())
        .bind(now_ts())
        .bind(&body.scene)
        .execute(&state.db)
        .await?;
    if res.rows_affected() == 0 {
        return Err(ApiError(StatusCode::NOT_FOUND, "scene not found".into()));
    }
    Ok(StatusCode::NO_CONTENT)
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

/// GET /api/menu → 200 {id, sections:[{title, items:[{name, desc, price}]}]}
/// 从 menu_items 读 visible=1，按 section, sort_order 组装。保持前端 MenuData 契约不变
/// （只读看单，不下单，见 design 定稿）。
pub async fn get_menu(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let rows = sqlx::query_as::<_, (String, String, String, i64, i64)>(
        "SELECT section, name, description, price, sort_order FROM menu_items
         WHERE visible = 1 ORDER BY section, sort_order",
    )
    .fetch_all(&state.db)
    .await?;
    // 按 section 分组（ORDER BY section 已排序，首遇新 section 即开新组）。
    let mut sections: Vec<serde_json::Value> = Vec::new();
    let mut cur_title: Option<String> = None;
    let mut cur_items: Vec<serde_json::Value> = Vec::new();
    for (section, name, desc, price, _sort) in rows {
        if cur_title.as_deref() != Some(section.as_str()) {
            if let Some(t) = cur_title.take() {
                sections.push(json!({ "title": t, "items": cur_items }));
                cur_items = Vec::new();
            }
            cur_title = Some(section);
        }
        cur_items.push(json!({ "name": name, "desc": desc, "price": price }));
    }
    if let Some(t) = cur_title.take() {
        sections.push(json!({ "title": t, "items": cur_items }));
    }
    Ok(Json(json!({ "id": "house-menu", "sections": sections })))
}

// ---------- 酒单 CRUD（admin）----------

/// POST/PUT /api/admin/menu 请求体。id 由服务端生成（random_asset_id）；
/// description/price/sort_order/visible 可选，缺失走 DB 默认。
#[derive(Deserialize)]
pub struct MenuInput {
    pub section: String,
    pub name: String,
    pub description: Option<String>,
    pub price: Option<i64>,
    pub sort_order: Option<i64>,
    pub visible: Option<i64>,
}

/// GET /api/admin/menu → 200 [MenuItem 全量含 visible=0]（按 section, sort_order）。
pub async fn admin_list_menu(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    require_admin(&state, &headers).await?;
    let rows = sqlx::query_as::<_, (String, String, String, String, i64, i64, i64, i64)>(
        "SELECT id, section, name, description, price, sort_order, visible, created_at
         FROM menu_items ORDER BY section, sort_order",
    )
    .fetch_all(&state.db)
    .await?;
    let items = rows
        .iter()
        .map(|(id, section, name, desc, price, sort, visible, created)| {
            json!({
                "id": id,
                "section": section,
                "name": name,
                "description": desc,
                "price": price,
                "sort_order": sort,
                "visible": visible,
                "created_at": created,
            })
        })
        .collect();
    Ok(Json(items))
}

/// POST /api/admin/menu {section, name, ...} → 201 {MenuItem}。id 用 random_asset_id。
pub async fn admin_create_menu(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<MenuInput>,
) -> Result<Response, ApiError> {
    require_admin(&state, &headers).await?;
    let section = body.section.trim();
    let name = body.name.trim();
    if section.is_empty() || name.is_empty() {
        return Err(ApiError(StatusCode::BAD_REQUEST, "section and name required".into()));
    }
    let description = body.description.unwrap_or_default();
    let price = body.price.unwrap_or(0);
    let sort_order = body.sort_order.unwrap_or(0);
    let visible = body.visible.unwrap_or(1);
    let created_at = now_ts();
    let id = crate::assets::random_asset_id();
    sqlx::query(
        "INSERT INTO menu_items (id, section, name, description, price, sort_order, visible, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(section)
    .bind(name)
    .bind(&description)
    .bind(price)
    .bind(sort_order)
    .bind(visible)
    .bind(created_at)
    .execute(&state.db)
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({
            "id": id,
            "section": section,
            "name": name,
            "description": description,
            "price": price,
            "sort_order": sort_order,
            "visible": visible,
            "created_at": created_at,
        })),
    )
        .into_response())
}

/// PUT /api/admin/menu/{id} {section, name, ...} → 200 {MenuItem}。不存在 → 404。
pub async fn admin_update_menu(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<MenuInput>,
) -> Result<Response, ApiError> {
    require_admin(&state, &headers).await?;
    let section = body.section.trim();
    let name = body.name.trim();
    if section.is_empty() || name.is_empty() {
        return Err(ApiError(StatusCode::BAD_REQUEST, "section and name required".into()));
    }
    let description = body.description.unwrap_or_default();
    let price = body.price.unwrap_or(0);
    let sort_order = body.sort_order.unwrap_or(0);
    let visible = body.visible.unwrap_or(1);
    let res = sqlx::query(
        "UPDATE menu_items SET section=?, name=?, description=?, price=?, sort_order=?, visible=?
         WHERE id=?",
    )
    .bind(section)
    .bind(name)
    .bind(&description)
    .bind(price)
    .bind(sort_order)
    .bind(visible)
    .bind(&id)
    .execute(&state.db)
    .await?;
    if res.rows_affected() == 0 {
        return Err(ApiError(StatusCode::NOT_FOUND, "menu item not found".into()));
    }
    Ok((StatusCode::OK, Json(json!({
        "id": id,
        "section": section,
        "name": name,
        "description": description,
        "price": price,
        "sort_order": sort_order,
        "visible": visible,
    })))
        .into_response())
}

/// DELETE /api/admin/menu/{id} → 204；不存在 → 404。
pub async fn admin_delete_menu(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    require_admin(&state, &headers).await?;
    let res = sqlx::query("DELETE FROM menu_items WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await?;
    if res.rows_affected() == 0 {
        return Err(ApiError(StatusCode::NOT_FOUND, "menu item not found".into()));
    }
    Ok(StatusCode::NO_CONTENT)
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

// ---------- admin（装饰管理）----------

/// GET /api/admin/decorations?scene=bar 的查询参数。
#[derive(Deserialize)]
pub struct DecorationSceneQuery {
    pub scene: Option<String>,
}

/// GET /api/admin/decorations?scene=bar → 200 [装饰 json]（按 created_at 排序）。
/// asset_key = LEFT JOIN assets.storage_key（无资产 → null）；不落库，join 出来。
pub async fn admin_list_decorations(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<DecorationSceneQuery>,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    require_admin(&state, &headers).await?;
    let scene = q.scene.as_deref().unwrap_or("bar");
    let rows = sqlx::query_as::<_, (String, String, i64, i64, Option<String>, i64, i64, Option<String>)>(
        "SELECT d.id, d.scene, d.tile_x, d.tile_y, d.asset_id, d.z_layer, d.placed_by, a.storage_key
         FROM decorations d
         LEFT JOIN assets a ON d.asset_id = a.id
         WHERE d.scene = ? ORDER BY d.created_at",
    )
    .bind(scene)
    .fetch_all(&state.db)
    .await?;
    let decorations = rows
        .iter()
        .map(|(id, scene, tx, ty, aid, z, pb, akey)| {
            json!({
                "id": id,
                "scene": scene,
                "tile_x": tx,
                "tile_y": ty,
                "asset_id": aid,
                "asset_key": akey,
                "z_layer": z,
                "placed_by": pb,
            })
        })
        .collect();
    Ok(Json(decorations))
}

/// POST /api/admin/decorations {scene, tile_x, tile_y, asset_id?, z_layer?} → 201 {装饰}。
/// INSERT + 广播 DecorationAdded。asset_id 可 null（占位装饰）。
#[derive(Deserialize)]
pub struct DecorationPut {
    pub scene: String,
    pub tile_x: i64,
    pub tile_y: i64,
    pub asset_id: Option<String>,
    pub z_layer: Option<i64>,
}

pub async fn admin_place_decoration(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<DecorationPut>,
) -> Result<Response, ApiError> {
    let admin_id = require_admin(&state, &headers).await?;
    let id = crate::assets::random_asset_id();
    let z_layer = body.z_layer.unwrap_or(0);
    let created_at = now_ts();
    sqlx::query(
        "INSERT INTO decorations (id, scene, tile_x, tile_y, asset_id, z_layer, placed_by, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&body.scene)
    .bind(body.tile_x)
    .bind(body.tile_y)
    .bind(&body.asset_id)
    .bind(z_layer)
    .bind(admin_id)
    .bind(created_at)
    .execute(&state.db)
    .await?;
    // 查 asset_key（LEFT JOIN 出来的字段；无资产 → null）。前端拼 /api/assets/{asset_key} 的唯一来源。
    let asset_key: Option<String> = match &body.asset_id {
        Some(aid) => sqlx::query_as::<_, (String,)>("SELECT storage_key FROM assets WHERE id = ?")
            .bind(aid)
            .fetch_optional(&state.db)
            .await?
            .map(|(s,)| s),
        None => None,
    };
    let decoration = json!({
        "id": id,
        "scene": body.scene,
        "tile_x": body.tile_x,
        "tile_y": body.tile_y,
        "asset_id": body.asset_id,
        "asset_key": asset_key,
        "z_layer": z_layer,
        "placed_by": admin_id,
    });
    // 广播 DecorationAdded（若房间存在且有在线玩家才到达；否则 DB 持有，下次 snapshot_full 查到）
    state.rt.add_decoration(&body.scene, decoration.clone());
    Ok((StatusCode::CREATED, Json(decoration)).into_response())
}

/// DELETE /api/admin/decorations/{id} → 204；广播 DecorationRemoved + DELETE。
pub async fn admin_remove_decoration(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    require_admin(&state, &headers).await?;
    // 先查 scene（广播需要），再 DELETE
    let row: Option<(String,)> = sqlx::query_as("SELECT scene FROM decorations WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await?;
    let scene = match row {
        Some((s,)) => s,
        None => return Err(ApiError(StatusCode::NOT_FOUND, "decoration not found".into())),
    };
    sqlx::query("DELETE FROM decorations WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await?;
    state.rt.remove_decoration(&scene, id);
    Ok(StatusCode::NO_CONTENT)
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
        .route("/members/{id}/unban", post(admin_unban))
        .route("/map", get(admin_get_map))
        .route("/map/regenerate", post(admin_map_regenerate))
        .route("/map/tileset", post(admin_map_tileset))
        .route("/map/walkable", put(admin_set_walkable))
        .route(
            "/menu",
            get(admin_list_menu).post(admin_create_menu),
        )
        .route("/menu/{id}", put(admin_update_menu).delete(admin_delete_menu))
        .route(
            "/decorations",
            get(admin_list_decorations).post(admin_place_decoration),
        )
        .route("/decorations/{id}", delete(admin_remove_decoration));

    let api = Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/me", get(me))
        .route("/avatar", put(put_avatar))
        .route("/avatar/equip", post(avatar_equip))
        .route("/avatar/unequip", post(avatar_unequip))
        .route("/avatar/generate", post(avatar_generate_submit))
        .route("/avatar/generate-text", post(avatar_generate_text))
        .route("/avatar/generate/{job_id}", get(avatar_generate_poll))
        .route("/avatar/jobs", get(avatar_list_jobs))
        .route("/assets/{key}", get(serve_asset))
        .route("/menu", get(get_menu))
        .route("/map", get(get_map))
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
