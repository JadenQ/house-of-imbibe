//! 切片4 Admin: 成员管理 + 控制台骨架测试。
//! 全程离线（无 PIXELLAB/MINIMAX key）。

mod common;
use common::{register_and_login, spawn_app};

/// 辅助：用 cookie 发请求，返回 (status, json)。
async fn req(
    base: &str,
    method: &str,
    path: &str,
    cookie: &str,
    body: Option<&serde_json::Value>,
) -> (reqwest::StatusCode, serde_json::Value) {
    let client = reqwest::Client::new();
    let mut builder = client.request(
        method.parse().expect("method"),
        format!("{base}{path}"),
    ).header(reqwest::header::COOKIE, cookie);
    if let Some(b) = body {
        builder = builder.json(b);
    }
    let resp = builder.send().await.expect("req");
    let status = resp.status();
    let json: serde_json::Value = resp.json().await.unwrap_or(serde_json::Value::Null);
    (status, json)
}

/// admin 列成员：首个用户是 admin，第二个是普通成员。
#[tokio::test]
async fn admin_lists_members() {
    let app = spawn_app().await;
    let base = &app.base_url;

    let admin_cookie = register_and_login(base, "alice").await; // 首个 = admin
    let _bob_cookie = register_and_login(base, "bob").await;

    let (status, json) = req(base, "GET", "/api/admin/members", &admin_cookie, None).await;
    assert_eq!(status, reqwest::StatusCode::OK, "admin list should be 200");
    let arr = json.as_array().expect("array");
    assert_eq!(arr.len(), 2, "two members");
    // 按 id 排序
    assert_eq!(arr[0]["username"], "alice");
    assert_eq!(arr[0]["is_admin"], true);
    assert_eq!(arr[0]["banned"], false);
    assert_eq!(arr[1]["username"], "bob");
    assert_eq!(arr[1]["is_admin"], false);
    assert_eq!(arr[1]["banned"], false);
}

/// admin promote/demote：升降级写入 DB。
#[tokio::test]
async fn admin_promote_demote() {
    let app = spawn_app().await;
    let base = &app.base_url;

    let admin_cookie = register_and_login(base, "alice").await; // admin
    let _bob_cookie = register_and_login(base, "bob").await;

    // promote bob
    let (status, _) = req(base, "POST", "/api/admin/members/2/promote", &admin_cookie, None).await;
    assert!(status.is_success(), "promote should succeed: {status}");

    let (status, json) = req(base, "GET", "/api/admin/members", &admin_cookie, None).await;
    assert!(status.is_success(), "list members should succeed: {status}");
    let arr = json.as_array().expect("array");
    assert_eq!(arr[1]["is_admin"], true, "bob promoted");

    // demote bob
    let (status, _) = req(base, "POST", "/api/admin/members/2/demote", &admin_cookie, None).await;
    assert!(status.is_success(), "demote should succeed: {status}");

    let (_, json) = req(base, "GET", "/api/admin/members", &admin_cookie, None).await;
    let arr = json.as_array().expect("array");
    assert_eq!(arr[1]["is_admin"], false, "bob demoted");
}

/// ban → 被封禁者 login 403。
#[tokio::test]
async fn ban_blocks_login() {
    let app = spawn_app().await;
    let base = &app.base_url;

    let admin_cookie = register_and_login(base, "alice").await; // admin
    let _bob_cookie = register_and_login(base, "bob").await;

    // ban bob
    let (status, _) = req(base, "POST", "/api/admin/members/2/ban", &admin_cookie, None).await;
    assert!(status.is_success(), "ban should succeed: {status}");

    // bob tries to login → 403
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{base}/api/login"))
        .json(&serde_json::json!({ "username": "bob", "password": "hunter2" }))
        .send()
        .await
        .expect("login");
    assert_eq!(resp.status(), reqwest::StatusCode::FORBIDDEN, "banned login must be 403");
    let body: serde_json::Value = resp.json().await.expect("json");
    assert_eq!(body["error"], "banned");

    // unban → login ok
    let (status, _) = req(base, "POST", "/api/admin/members/2/unban", &admin_cookie, None).await;
    assert!(status.is_success(), "unban should succeed: {status}");

    let resp = client
        .post(format!("{base}/api/login"))
        .json(&serde_json::json!({ "username": "bob", "password": "hunter2" }))
        .send()
        .await
        .expect("login 2");
    assert!(resp.status().is_success(), "login after unban should succeed: {}", resp.status());
}

/// 非 admin 调 admin 端点 → 403。
#[tokio::test]
async fn non_admin_forbidden() {
    let app = spawn_app().await;
    let base = &app.base_url;

    let _admin_cookie = register_and_login(base, "alice").await; // admin
    let bob_cookie = register_and_login(base, "bob").await; // non-admin

    let (status, _) = req(base, "GET", "/api/admin/members", &bob_cookie, None).await;
    assert_eq!(status, reqwest::StatusCode::FORBIDDEN, "non-admin list must be 403");

    let (status, _) = req(base, "POST", "/api/admin/members/1/promote", &bob_cookie, None).await;
    assert_eq!(status, reqwest::StatusCode::FORBIDDEN, "non-admin promote must be 403");

    let (status, _) = req(base, "POST", "/api/admin/members/1/ban", &bob_cookie, None).await;
    assert_eq!(status, reqwest::StatusCode::FORBIDDEN, "non-admin ban must be 403");
}

/// 不能 ban 自己（409 cannot modify self）。
#[tokio::test]
async fn cannot_ban_self() {
    let app = spawn_app().await;
    let base = &app.base_url;

    let admin_cookie = register_and_login(base, "alice").await; // admin, id=1

    let (status, json) = req(base, "POST", "/api/admin/members/1/ban", &admin_cookie, None).await;
    assert_eq!(status, reqwest::StatusCode::CONFLICT, "ban self must be 409");
    assert_eq!(json["error"], "cannot modify self");

    // promote/demote self 也拒
    let (status, _) = req(base, "POST", "/api/admin/members/1/promote", &admin_cookie, None).await;
    assert_eq!(status, reqwest::StatusCode::CONFLICT, "promote self must be 409");

    let (status, _) = req(base, "POST", "/api/admin/members/1/demote", &admin_cookie, None).await;
    assert_eq!(status, reqwest::StatusCode::CONFLICT, "demote self must be 409");
}

/// 不存在的成员 → 404。
#[tokio::test]
async fn modify_nonexistent_member_404() {
    let app = spawn_app().await;
    let base = &app.base_url;

    let admin_cookie = register_and_login(base, "alice").await; // admin

    let (status, _) = req(base, "POST", "/api/admin/members/999/ban", &admin_cookie, None).await;
    assert_eq!(status, reqwest::StatusCode::NOT_FOUND, "ban nonexistent must be 404");
}
