//! House of Imbibe — 二进制入口。
//! 所有应用逻辑（AppState / handlers / build_router）在 `house_of_imbibe` lib，
//! 供集成测试复用。本文件只负责：DB 连接 + migrations + 装配 + serve。

use std::net::SocketAddr;
use std::sync::Arc;

use house_of_imbibe::assets::LocalAssetStore;
use sqlx::SqlitePool;
use tracing::info;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:data/hoi.db".into());
    let db_path = db_url
        .strip_prefix("sqlite:")
        .unwrap_or(&db_url)
        .trim_start_matches('/')
        .to_string();
    if let Some(dir) = std::path::Path::new(&db_path).parent() {
        std::fs::create_dir_all(dir).ok();
    }
    let db = SqlitePool::connect_with(
        sqlx::sqlite::SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .busy_timeout(std::time::Duration::from_secs(5))
            .foreign_keys(true),
    )
    .await
    .expect("connect sqlite");
    sqlx::migrate!("./migrations").run(&db).await.expect("migrations");

    let asset_root = std::env::var("ASSET_DIR").unwrap_or_else(|_| "data/assets".into());
    std::fs::create_dir_all(&asset_root).ok();

    let state = Arc::new(house_of_imbibe::AppState {
        db,
        pixellab_key: std::env::var("PIXELLAB_API_KEY").ok(),
        minimax_key: std::env::var("MINIMAX_API_KEY").ok(),
        http: house_of_imbibe::pixelart::http_client(),
        assets: Arc::new(LocalAssetStore::new(asset_root)),
        rt: Arc::new(house_of_imbibe::realtime::RealtimeState::new()),
    });

    if state.pixellab_key.is_some() {
        info!("PixelLab avatar generation: enabled");
    } else {
        info!("PixelLab avatar generation: disabled (set PIXELLAB_API_KEY to enable)");
    }

    let app = house_of_imbibe::build_router(state);

    let port: u16 = std::env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(8080);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!(%addr, "house-of-imbibe listening");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
