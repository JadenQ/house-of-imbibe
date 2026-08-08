//! 切片1 集成测试 harness（dev-plan §三 切片1 "最重要的交付物"，后续切片复用）。
//! 起真 Axum（随机端口 + 临时 SQLite + 无 API key → 完全离线），带 cookie 的 WS 客户端。
//!
//! 放在 `tests/common/mod.rs`（而非 `tests/harness.rs`）以走 Rust 共享 helper 的
//! 习惯模式：子模块而非独立 test crate，避免重复编译。

#![allow(dead_code)]

use std::sync::Arc;

use house_of_imbibe::realtime::RealtimeState;
use house_of_imbibe::{build_router, AppState};
use sqlx::SqlitePool;
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::http::{Request, header};
use tokio_tungstenite::MaybeTlsStream;

pub struct TestApp {
    pub base_url: String,
    pub db: SqlitePool,
}

/// 起真 Axum（随机端口 + 临时 SQLite + 无 API key → 离线），返回 base_url + db 句柄。
pub async fn spawn_app() -> TestApp {
    // 临时 SQLite 文件；forget 保证测试期间不被删。
    let tmp = tempfile::NamedTempFile::new().expect("tempfile");
    let path = tmp.path().to_str().expect("path").to_string();
    let db = SqlitePool::connect_with(
        sqlx::sqlite::SqliteConnectOptions::new()
            .filename(&path)
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .busy_timeout(std::time::Duration::from_secs(5)),
    )
    .await
    .expect("connect sqlite");
    sqlx::migrate!("./migrations")
        .run(&db)
        .await
        .expect("migrate");

    let rt = Arc::new(RealtimeState::new());
    let state = Arc::new(AppState {
        db: db.clone(),
        pixellab_key: None,
        minimax_key: None,
        http: house_of_imbibe::pixelart::http_client(),
        jobs: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        rt,
    });
    let app = build_router(state);

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve");
    });
    std::mem::forget(tmp);
    TestApp { base_url: format!("http://{addr}"), db }
}

/// 注册（注册即登录、即发 cookie），返回 "hoi_session=<token>" 供 WS 握手用。
pub async fn register_and_login(base_url: &str, username: &str) -> String {
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{base_url}/api/register"))
        .json(&serde_json::json!({ "username": username, "password": "hunter2" }))
        .send()
        .await
        .expect("register");
    assert!(
        resp.status().is_success(),
        "register {} failed: {:?}",
        username,
        resp.status()
    );
    for hv in resp.headers().get_all(reqwest::header::SET_COOKIE).iter() {
        if let Ok(s) = hv.to_str() {
            if let Some(start) = s.find("hoi_session=") {
                let token = s[start..].split(';').next().expect("token");
                return token.to_string();
            }
        }
    }
    panic!("no hoi_session cookie in register response");
}

/// 带 cookie 打开 WS /ws/room。
pub async fn ws_connect(
    base_url: &str,
    cookie: &str,
) -> tokio_tungstenite::WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>> {
    let ws_url = base_url.replace("http://", "ws://") + "/ws/room";
    let host = ws_url
        .trim_start_matches("ws://")
        .split('/')
        .next()
        .unwrap_or("")
        .to_string();
    let req = Request::builder()
        .uri(&ws_url)
        .header(header::HOST, host)
        .header(header::UPGRADE, "websocket")
        .header(header::CONNECTION, "upgrade")
        .header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")
        .header("sec-websocket-version", "13")
        .header(header::COOKIE, cookie)
        .body(())
        .expect("ws request");
    tokio_tungstenite::connect_async(req)
        .await
        .expect("ws connect")
        .0
}
