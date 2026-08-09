//! 切片3a 地基测试：admin bootstrap 原子性 + avatar job 落表 + asset serving。
//! 全程离线（无 PIXELLAB/MINIMAX key）。

mod common;
use common::{register_and_login, spawn_app};

/// 首个注册用户 is_admin=true，第二个 false（原子 INSERT...SELECT 消除竞态）。
#[tokio::test]
async fn admin_bootstrap_first_true_second_false() {
    let app = spawn_app().await;
    let base = &app.base_url;

    let cookie_a = register_and_login(base, "alice").await;
    // 从 /api/me 读 is_admin
    let client = reqwest::Client::new();
    let me: serde_json::Value = client
        .get(format!("{base}/api/me"))
        .header(reqwest::header::COOKIE, &cookie_a)
        .send()
        .await
        .expect("me")
        .json()
        .await
        .expect("me json");
    assert_eq!(me["is_admin"], true, "first user must be admin");

    let cookie_b = register_and_login(base, "bob").await;
    let me_b: serde_json::Value = client
        .get(format!("{base}/api/me"))
        .header(reqwest::header::COOKIE, &cookie_b)
        .send()
        .await
        .expect("me b")
        .json()
        .await
        .expect("me b json");
    assert_eq!(me_b["is_admin"], false, "second user must NOT be admin");

    // 也直接查 DB 确认
    let admins: Vec<(String, bool)> = sqlx::query_as("SELECT username, is_admin FROM users ORDER BY id")
        .fetch_all(&app.db)
        .await
        .expect("query users");
    assert_eq!(admins.len(), 2);
    assert!(admins[0].1, "alice is_admin");
    assert!(!admins[1].1, "bob not admin");
}

/// POST /api/avatar/generate 落 generation_jobs 表 pending；worker 无 key → failed。
#[tokio::test]
async fn avatar_submit_lands_job_pending_then_failed() {
    let app = spawn_app().await;
    let base = &app.base_url;

    let cookie = register_and_login(base, "alice").await;
    let client = reqwest::Client::new();

    // 构造最小 PNG（8 字节 header + IHDR-ish），够 detect_mime 识别
    let png_bytes: &[u8] = b"\x89PNG\r\n\x1a\n\x00\x00\x00\x0dIHDR";

    // multipart 上传
    let form = reqwest::multipart::Form::new()
        .part(
            "image",
            reqwest::multipart::Part::bytes(png_bytes.to_vec())
                .file_name("photo.png")
                .mime_str("image/png")
                .expect("mime"),
        );

    let resp = client
        .post(format!("{base}/api/avatar/generate"))
        .header(reqwest::header::COOKIE, &cookie)
        .multipart(form)
        .send()
        .await
        .expect("submit");
    assert!(
        resp.status().is_success(),
        "submit should succeed: {}",
        resp.status()
    );
    let body: serde_json::Value = resp.json().await.expect("json");
    let job_id = body["job_id"].as_str().expect("job_id").to_string();
    assert!(!job_id.is_empty());

    // 确认 generation_jobs 表有 pending 行
    let row: Option<(String,)> =
        sqlx::query_as("SELECT status FROM generation_jobs WHERE id = ?")
            .bind(&job_id)
            .fetch_optional(&app.db)
            .await
            .expect("query job");
    assert!(row.is_some(), "job must be in DB");
    // 初始可能是 pending 或 running（worker 异步认领）
    let status = row.unwrap().0;
    assert!(
        status == "pending" || status == "running" || status == "failed",
        "unexpected status: {status}"
    );

    // 等待 worker（无 key）把 job 标记 failed
    for _ in 0..20 {
        let row: (String, Option<String>) =
            sqlx::query_as("SELECT status, error FROM generation_jobs WHERE id = ?")
                .bind(&job_id)
                .fetch_one(&app.db)
                .await
                .expect("poll job");
        if row.0 == "failed" {
            assert!(row.1.is_some(), "failed job must have error");
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    panic!("job did not reach failed status (no API key → should fail fast)");
}

/// GET /api/assets/:key — 404 不存在、400 穿越尝试。
#[tokio::test]
async fn asset_serving_404_and_traversal_400() {
    let app = spawn_app().await;
    let base = &app.base_url;
    let client = reqwest::Client::new();

    // 不存在的 key → 404
    let resp = client
        .get(format!("{base}/api/assets/avatar/u1/nonexistent.png"))
        .send()
        .await
        .expect("get");
    assert_eq!(
        resp.status().as_u16(),
        404,
        "nonexistent asset should 404"
    );

    // 穿越 key → 400
    let resp = client
        .get(format!("{base}/api/assets/..%2f..%2fetc%2fpasswd"))
        .send()
        .await
        .expect("get traversal");
    let status = resp.status().as_u16();
    assert!(
        status == 400 || status == 404,
        "traversal should 400 or 404, got {status}"
    );

    // 直接含 .. → 400
    let resp = client
        .get(format!("{base}/api/assets/../../etc/passwd"))
        .send()
        .await
        .expect("get traversal 2");
    let status = resp.status().as_u16();
    assert!(
        status == 400 || status == 404,
        "traversal 2 should 400 or 404, got {status}"
    );
}

/// GET /api/avatar/generate/{job_id} 未知 job → 404。
#[tokio::test]
async fn avatar_poll_unknown_job_404() {
    let app = spawn_app().await;
    let base = &app.base_url;
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{base}/api/avatar/generate/nonexistent-job-id"))
        .send()
        .await
        .expect("poll");
    assert_eq!(resp.status().as_u16(), 404, "unknown job should 404");
}

/// POST /api/avatar/generate-text {description} 落 generation_jobs 表 pending；worker 无 key → failed。
#[tokio::test]
async fn avatar_text_submit_lands_job_pending_then_failed() {
    let app = spawn_app().await;
    let base = &app.base_url;
    let cookie = register_and_login(base, "alice").await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/api/avatar/generate-text"))
        .header(reqwest::header::COOKIE, &cookie)
        .json(&serde_json::json!({ "description": "a brave knight in green armor" }))
        .send()
        .await
        .expect("submit text");
    assert!(
        resp.status().is_success(),
        "text submit should succeed: {}",
        resp.status()
    );
    let body: serde_json::Value = resp.json().await.expect("json");
    let job_id = body["job_id"].as_str().expect("job_id").to_string();
    assert!(!job_id.is_empty());

    // 确认落表
    let row: Option<(String,)> =
        sqlx::query_as("SELECT status FROM generation_jobs WHERE id = ?")
            .bind(&job_id)
            .fetch_optional(&app.db)
            .await
            .expect("query text job");
    assert!(row.is_some(), "text job must be in DB");

    // 等待 worker（无 pixellab key）→ failed
    for _ in 0..20 {
        let row: (String, Option<String>) =
            sqlx::query_as("SELECT status, error FROM generation_jobs WHERE id = ?")
                .bind(&job_id)
                .fetch_one(&app.db)
                .await
                .expect("poll text job");
        if row.0 == "failed" {
            assert!(row.1.is_some(), "failed text job must have error");
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    panic!("text job did not reach failed (no API key → should fail fast)");
}

/// POST /api/avatar/generate-text 空/空白描述 → 400（不落 job）。
#[tokio::test]
async fn avatar_text_empty_description_400() {
    let app = spawn_app().await;
    let base = &app.base_url;
    let cookie = register_and_login(base, "alice").await;
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{base}/api/avatar/generate-text"))
        .header(reqwest::header::COOKIE, &cookie)
        .json(&serde_json::json!({ "description": "   " }))
        .send()
        .await
        .expect("empty desc");
    assert_eq!(resp.status().as_u16(), 400, "empty description should 400");
    // 不应落 job
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM generation_jobs")
        .fetch_one(&app.db)
        .await
        .expect("count jobs");
    assert_eq!(count.0, 0, "empty-description 400 must not create a job");
}

/// PUT /api/avatar 持久化 modular 样式字段（hairStyle/topStyle/...），不只 4 色。
/// 远端玩家经 WS 快照（AvatarSnapshot=config_json）能拿到完整样式。
#[tokio::test]
async fn put_avatar_persists_style_fields() {
    let app = spawn_app().await;
    let base = &app.base_url;
    let cookie = register_and_login(base, "alice").await;
    let client = reqwest::Client::new();

    let resp = client
        .put(format!("{base}/api/avatar"))
        .header(reqwest::header::COOKIE, &cookie)
        .json(&serde_json::json!({
            "config": {
                "kind": "modular",
                "skin": "#f0c8a0", "hair": "#503018", "shirt": "#3868b0", "pants": "#404048", "shoes": "#201510",
                "hairStyle": "cap", "topStyle": "vest", "bottomStyle": "skirt", "shoeStyle": "sneakers"
            }
        }))
        .send()
        .await
        .expect("put avatar");
    assert_eq!(resp.status().as_u16(), 204, "put avatar should be 204: {}", resp.status());

    // 读回 config_json，确认样式字段被持久化（不被剥离）
    let (cfg,): (String,) = sqlx::query_as("SELECT config_json FROM avatars WHERE user_id = 1")
        .fetch_one(&app.db)
        .await
        .expect("query avatar");
    let v: serde_json::Value = serde_json::from_str(&cfg).expect("parse config");
    assert_eq!(v["kind"], "modular");
    assert_eq!(v["hairStyle"], "cap", "hairStyle must persist: {cfg}");
    assert_eq!(v["topStyle"], "vest");
    assert_eq!(v["bottomStyle"], "skirt");
    assert_eq!(v["shoeStyle"], "sneakers");
    assert_eq!(v["shoes"], "#201510");

    // 非法样式值 → fallback（不报错），非法颜色 → 400
    let resp2 = client
        .put(format!("{base}/api/avatar"))
        .header(reqwest::header::COOKIE, &cookie)
        .json(&serde_json::json!({
            "config": { "kind":"modular", "skin":"#f0c8a0","hair":"#503018","shirt":"#3868b0","pants":"#404048",
                        "hairStyle": "NOPE" }
        }))
        .send()
        .await
        .expect("put bad style");
    assert_eq!(resp2.status().as_u16(), 204, "unknown style falls back, not 400");
    let (cfg2,): (String,) = sqlx::query_as("SELECT config_json FROM avatars WHERE user_id = 1")
        .fetch_one(&app.db).await.expect("q2");
    let v2: serde_json::Value = serde_json::from_str(&cfg2).expect("parse");
    assert_eq!(v2["hairStyle"], "short", "unknown style → default short");

    let resp3 = client
        .put(format!("{base}/api/avatar"))
        .header(reqwest::header::COOKIE, &cookie)
        .json(&serde_json::json!({
            "config": { "kind":"modular", "skin":"not-a-color","hair":"#503018","shirt":"#3868b0","pants":"#404048" }
        }))
        .send()
        .await
        .expect("put bad color");
    assert_eq!(resp3.status().as_u16(), 400, "bad color must 400");
}
