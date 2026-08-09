//! D1 地图背景图生成测试：admin regenerate → 202 + job 落表 → worker 无 key → failed。
//! GET /api/map 公开 + GET /api/admin/map admin 门禁。全程离线（无 PIXELLAB key）。

mod common;
use common::{register_and_login, spawn_app};

/// GET /api/map?scene=bar → 200 {scene:"bar", width:15, height:10, bg_key:null}（初始）。
#[tokio::test]
async fn get_map_public_initial() {
    let app = spawn_app().await;
    let base = &app.base_url;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{base}/api/map?scene=bar"))
        .send()
        .await
        .expect("get map");
    assert_eq!(resp.status().as_u16(), 200, "public map endpoint should 200");
    let body: serde_json::Value = resp.json().await.expect("json");
    assert_eq!(body["scene"], "bar");
    assert_eq!(body["width"], 15);
    assert_eq!(body["height"], 10);
    assert_eq!(body["bg_key"], serde_json::Value::Null, "initial bg_key must be null");
}

/// GET /api/map?scene=unknown → 404。
#[tokio::test]
async fn get_map_unknown_scene_404() {
    let app = spawn_app().await;
    let base = &app.base_url;
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{base}/api/map?scene=unknown"))
        .send()
        .await
        .expect("get unknown scene");
    assert_eq!(resp.status().as_u16(), 404, "unknown scene should 404");
}

/// GET /api/admin/map?scene=bar → 200（admin）；非 admin → 403。
#[tokio::test]
async fn admin_get_map_403_for_non_admin() {
    let app = spawn_app().await;
    let base = &app.base_url;
    let client = reqwest::Client::new();

    // 非登录 → 401
    let resp = client
        .get(format!("{base}/api/admin/map?scene=bar"))
        .send()
        .await
        .expect("admin map no auth");
    assert_eq!(resp.status().as_u16(), 401, "not logged in → 401");

    // admin → 200
    let cookie_admin = register_and_login(base, "alice").await;
    let resp = client
        .get(format!("{base}/api/admin/map?scene=bar"))
        .header(reqwest::header::COOKIE, &cookie_admin)
        .send()
        .await
        .expect("admin map");
    assert_eq!(resp.status().as_u16(), 200, "admin should 200");
    let body: serde_json::Value = resp.json().await.expect("json");
    assert_eq!(body["scene"], "bar");
    assert_eq!(body["bg_key"], serde_json::Value::Null);

    // 非 admin → 403
    let cookie_bob = register_and_login(base, "bob").await;
    let resp = client
        .get(format!("{base}/api/admin/map?scene=bar"))
        .header(reqwest::header::COOKIE, &cookie_bob)
        .send()
        .await
        .expect("non-admin map");
    assert_eq!(resp.status().as_u16(), 403, "non-admin → 403");
}

/// POST /api/admin/map/regenerate {prompt} → 202 + job_id + 落表 pending(kind 'map_bg')
/// → worker 无 key → failed。非 admin → 403。空 prompt → 400。
#[tokio::test]
async fn admin_map_regenerate_lands_job_then_failed() {
    let app = spawn_app().await;
    let base = &app.base_url;
    let client = reqwest::Client::new();

    // alice = admin（首个注册），bob = 非 admin
    let cookie_admin = register_and_login(base, "alice").await;
    let cookie_bob = register_and_login(base, "bob").await;

    // 非 admin → 403
    let resp = client
        .post(format!("{base}/api/admin/map/regenerate"))
        .header(reqwest::header::COOKIE, &cookie_bob)
        .json(&serde_json::json!({ "prompt": "cozy tavern" }))
        .send()
        .await
        .expect("non-admin regenerate");
    assert_eq!(resp.status().as_u16(), 403, "non-admin → 403");

    // 未登录 → 401
    let resp = client
        .post(format!("{base}/api/admin/map/regenerate"))
        .json(&serde_json::json!({ "prompt": "cozy tavern" }))
        .send()
        .await
        .expect("no auth regenerate");
    assert_eq!(resp.status().as_u16(), 401, "not logged in → 401");

    // admin 提交 → 202
    let resp = client
        .post(format!("{base}/api/admin/map/regenerate"))
        .header(reqwest::header::COOKIE, &cookie_admin)
        .json(&serde_json::json!({ "prompt": "cozy tavern interior with wooden bar" }))
        .send()
        .await
        .expect("admin regenerate");
    assert!(
        resp.status().is_success(),
        "admin regenerate should succeed: {}",
        resp.status()
    );
    assert_eq!(resp.status().as_u16(), 202, "should be 202 Accepted");
    let body: serde_json::Value = resp.json().await.expect("json");
    let job_id = body["job_id"].as_str().expect("job_id").to_string();
    assert!(!job_id.is_empty());

    // 确认落表 kind='map_bg'
    let row: Option<(String, String)> =
        sqlx::query_as("SELECT kind, status FROM generation_jobs WHERE id = ?")
            .bind(&job_id)
            .fetch_optional(&app.db)
            .await
            .expect("query job");
    assert!(row.is_some(), "job must be in DB");
    let (kind, status) = row.unwrap();
    assert_eq!(kind, "map_bg", "kind must be map_bg");
    assert!(
        status == "pending" || status == "running" || status == "failed",
        "unexpected status: {status}"
    );

    // 等待 worker（无 key）→ failed
    for _ in 0..20 {
        let row: (String, Option<String>) =
            sqlx::query_as("SELECT status, error FROM generation_jobs WHERE id = ?")
                .bind(&job_id)
                .fetch_one(&app.db)
                .await
                .expect("poll map_bg job");
        if row.0 == "failed" {
            assert!(row.1.is_some(), "failed map_bg job must have error");
            assert!(
                row.1.as_deref().unwrap_or("").contains("not configured"),
                "error should mention not configured: {:?}",
                row.1
            );
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    panic!("map_bg job did not reach failed (no API key → should fail fast)");
}

/// POST /api/admin/map/regenerate 空 prompt → 400（不落 job）。
#[tokio::test]
async fn admin_map_regenerate_empty_prompt_400() {
    let app = spawn_app().await;
    let base = &app.base_url;
    let cookie = register_and_login(base, "alice").await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/api/admin/map/regenerate"))
        .header(reqwest::header::COOKIE, &cookie)
        .json(&serde_json::json!({ "prompt": "   " }))
        .send()
        .await
        .expect("empty prompt");
    assert_eq!(resp.status().as_u16(), 400, "empty prompt should 400");

    // 不应落 job
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM generation_jobs")
        .fetch_one(&app.db)
        .await
        .expect("count jobs");
    assert_eq!(count.0, 0, "empty-prompt 400 must not create a job");
}

/// GET /api/map?scene=bar 无 scene 参数 → 默认 bar → 200。
#[tokio::test]
async fn get_map_default_scene_bar() {
    let app = spawn_app().await;
    let base = &app.base_url;
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{base}/api/map"))
        .send()
        .await
        .expect("get map no scene");
    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value = resp.json().await.expect("json");
    assert_eq!(body["scene"], "bar", "default scene should be bar");
}
