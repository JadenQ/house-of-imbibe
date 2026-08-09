//! 装饰对象 CRUD + WS 广播测试（issue #7 三层架构第三层）。
//! 全程离线（无 PIXELLAB/MINIMAX key）。

mod common;
use common::{register_and_login, spawn_app, ws_connect};

use std::time::Duration;

use futures_util::StreamExt;
use house_of_imbibe::realtime::protocol::ServerMsg;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::Message;

type WS = tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

/// 收下一条 ServerMsg（跳过非文本/非 JSON 帧）；超时返回 None。
async fn recv(ws: &mut WS) -> Option<ServerMsg> {
    loop {
        match timeout(Duration::from_millis(1000), ws.next()).await {
            Ok(Some(Ok(Message::Text(t)))) => {
                if let Ok(m) = serde_json::from_str::<ServerMsg>(&t.to_string()) {
                    return Some(m);
                }
            }
            Ok(Some(Ok(_))) => continue,
            _ => return None,
        }
    }
}

/// 辅助：用 cookie 发请求，返回 (status, json)。
async fn req(
    base: &str,
    method: &str,
    path: &str,
    cookie: &str,
    body: Option<&serde_json::Value>,
) -> (reqwest::StatusCode, serde_json::Value) {
    let client = reqwest::Client::new();
    let mut builder = client
        .request(method.parse().expect("method"), format!("{base}{path}"))
        .header(reqwest::header::COOKIE, cookie);
    if let Some(b) = body {
        builder = builder.json(b);
    }
    let resp = builder.send().await.expect("req");
    let status = resp.status();
    let json: serde_json::Value = resp.json().await.unwrap_or(serde_json::Value::Null);
    (status, json)
}

/// admin 放置装饰（asset_id=null 占位）→ 201 + 两个 WS 客户端收到 DecorationAdded
/// → 移除 → 收到 DecorationRemoved。
#[tokio::test]
async fn decoration_crud_and_broadcast() {
    let app = spawn_app().await;
    let base = &app.base_url;

    let admin_cookie = register_and_login(base, "alice").await; // 首个 = admin
    let bob_cookie = register_and_login(base, "bob").await; // 非 admin

    // 连接 WS（admin）→ drain 初始消息
    let mut ws_a = ws_connect(base, &admin_cookie).await;
    let _ = recv(&mut ws_a).await; // welcome
    let _ = recv(&mut ws_a).await; // snapshot_full
    let _ = recv(&mut ws_a).await; // chat_backlog

    // 连接 WS（bob）→ drain 初始消息
    let mut ws_b = ws_connect(base, &bob_cookie).await;
    let _ = recv(&mut ws_b).await; // welcome
    let _ = recv(&mut ws_b).await; // snapshot_full
    let _ = recv(&mut ws_b).await; // chat_backlog

    // admin 放置装饰（asset_id=null 占位）
    let (status, json) = req(
        base,
        "POST",
        "/api/admin/decorations",
        &admin_cookie,
        Some(&serde_json::json!({
            "scene": "bar",
            "tile_x": 5,
            "tile_y": 3,
            "z_layer": 1,
        })),
    )
    .await;
    assert_eq!(
        status,
        reqwest::StatusCode::CREATED,
        "place decoration should be 201"
    );
    assert_eq!(json["scene"], "bar");
    assert_eq!(json["tile_x"], 5);
    assert_eq!(json["tile_y"], 3);
    assert!(json["asset_id"].is_null(), "asset_id should be null for placeholder");
    assert_eq!(json["z_layer"], 1);
    assert_eq!(json["placed_by"], 1); // admin is user 1
    let dec_id = json["id"].as_str().expect("id is string").to_string();

    // 列表 GET → 含刚放置的装饰
    let (status, list_json) = req(
        base,
        "GET",
        "/api/admin/decorations?scene=bar",
        &admin_cookie,
        None,
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK);
    let arr = list_json.as_array().expect("array");
    assert_eq!(arr.len(), 1, "one decoration");
    assert_eq!(arr[0]["id"], dec_id);

    // 两个 WS 客户端都应收到 DecorationAdded（跳过 tick 的 SnapshotDelta）
    let added_a = loop {
        match recv(&mut ws_a).await {
            Some(ServerMsg::DecorationAdded { decoration, .. }) => {
                assert_eq!(decoration["id"], dec_id);
                break true;
            }
            Some(_) => continue,
            None => break false,
        }
    };
    assert!(added_a, "ws_a did not receive DecorationAdded");

    let added_b = loop {
        match recv(&mut ws_b).await {
            Some(ServerMsg::DecorationAdded { decoration, .. }) => {
                assert_eq!(decoration["id"], dec_id);
                break true;
            }
            Some(_) => continue,
            None => break false,
        }
    };
    assert!(added_b, "ws_b did not receive DecorationAdded");

    // admin 移除装饰
    let (status, _) = req(
        base,
        "DELETE",
        &format!("/api/admin/decorations/{dec_id}"),
        &admin_cookie,
        None,
    )
    .await;
    assert_eq!(
        status,
        reqwest::StatusCode::NO_CONTENT,
        "remove should be 204"
    );

    // 两个 WS 客户端都应收到 DecorationRemoved
    let removed_a = loop {
        match recv(&mut ws_a).await {
            Some(ServerMsg::DecorationRemoved { id, .. }) => {
                assert_eq!(id, dec_id);
                break true;
            }
            Some(_) => continue,
            None => break false,
        }
    };
    assert!(removed_a, "ws_a did not receive DecorationRemoved");

    let removed_b = loop {
        match recv(&mut ws_b).await {
            Some(ServerMsg::DecorationRemoved { id, .. }) => {
                assert_eq!(id, dec_id);
                break true;
            }
            Some(_) => continue,
            None => break false,
        }
    };
    assert!(removed_b, "ws_b did not receive DecorationRemoved");

    // 列表 GET → 空
    let (_, list_json) = req(
        base,
        "GET",
        "/api/admin/decorations?scene=bar",
        &admin_cookie,
        None,
    )
    .await;
    let arr = list_json.as_array().expect("array");
    assert!(arr.is_empty(), "decorations should be empty after remove");
}

/// 非 admin 调装饰端点 → 403。
#[tokio::test]
async fn decoration_non_admin_forbidden() {
    let app = spawn_app().await;
    let base = &app.base_url;

    let _admin_cookie = register_and_login(base, "alice").await; // admin
    let bob_cookie = register_and_login(base, "bob").await; // non-admin

    let (status, _) = req(
        base,
        "GET",
        "/api/admin/decorations?scene=bar",
        &bob_cookie,
        None,
    )
    .await;
    assert_eq!(
        status,
        reqwest::StatusCode::FORBIDDEN,
        "non-admin list must be 403"
    );

    let (status, _) = req(
        base,
        "POST",
        "/api/admin/decorations",
        &bob_cookie,
        Some(&serde_json::json!({ "scene": "bar", "tile_x": 0, "tile_y": 0 })),
    )
    .await;
    assert_eq!(
        status,
        reqwest::StatusCode::FORBIDDEN,
        "non-admin place must be 403"
    );

    let (status, _) = req(
        base,
        "DELETE",
        "/api/admin/decorations/fakeid",
        &bob_cookie,
        None,
    )
    .await;
    assert_eq!(
        status,
        reqwest::StatusCode::FORBIDDEN,
        "non-admin delete must be 403"
    );
}

/// DELETE 不存在的装饰 → 404。
#[tokio::test]
async fn decoration_delete_not_found() {
    let app = spawn_app().await;
    let base = &app.base_url;

    let admin_cookie = register_and_login(base, "alice").await; // admin

    let (status, _) = req(
        base,
        "DELETE",
        "/api/admin/decorations/nonexistent",
        &admin_cookie,
        None,
    )
    .await;
    assert_eq!(
        status,
        reqwest::StatusCode::NOT_FOUND,
        "delete nonexistent must be 404"
    );
}

/// 放置带真 asset 行的装饰 → POST + GET 返回 asset_key 非空（= LEFT JOIN assets.storage_key）。
#[tokio::test]
async fn decoration_with_asset_returns_asset_key() {
    let app = spawn_app().await;
    let base = &app.base_url;

    let admin_cookie = register_and_login(base, "alice").await; // admin (user id = 1)

    // 插入一行 asset（storage_key = "deco/test.png"）
    let asset_id = "test_asset_001";
    sqlx::query(
        "INSERT INTO assets (id, owner_id, kind, storage_key, meta_json, created_at)
         VALUES (?, 1, 'decoration', 'deco/test.png', NULL, ?)",
    )
    .bind(asset_id)
    .bind(house_of_imbibe::now_ts())
    .execute(&app.db)
    .await
    .expect("insert asset");

    // 放置装饰（asset_id 指向上面插入的行）
    let (status, json) = req(
        base,
        "POST",
        "/api/admin/decorations",
        &admin_cookie,
        Some(&serde_json::json!({
            "scene": "bar",
            "tile_x": 3,
            "tile_y": 4,
            "asset_id": asset_id,
            "z_layer": 1,
        })),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::CREATED);
    assert_eq!(json["asset_id"], asset_id);
    assert_eq!(
        json["asset_key"], "deco/test.png",
        "asset_key must be storage_key from LEFT JOIN assets"
    );

    // GET 列表 → asset_key 非空
    let (status, list_json) = req(
        base,
        "GET",
        "/api/admin/decorations?scene=bar",
        &admin_cookie,
        None,
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK);
    let arr = list_json.as_array().expect("array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["asset_key"], "deco/test.png");

    // 占位装饰（asset_id=null）→ asset_key 也 null
    let (status, json2) = req(
        base,
        "POST",
        "/api/admin/decorations",
        &admin_cookie,
        Some(&serde_json::json!({
            "scene": "bar",
            "tile_x": 0,
            "tile_y": 0,
        })),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::CREATED);
    assert!(json2["asset_key"].is_null(), "asset_key null for placeholder");
}
