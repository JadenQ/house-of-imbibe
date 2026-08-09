//! 切片3b accessories 测试：equip/unequip + config_json equipped 字段 + 权限 + slot 校验。
//! 全程离线（无 PIXELLAB/MINIMAX key）。

mod common;
use common::{register_and_login, spawn_app};

/// equip → 200 + config_json.equipped 含 asset_key（= storage_key）→ unequip → 空。
#[tokio::test]
async fn equip_persists_asset_key_then_unequip_clears() {
    let app = spawn_app().await;
    let base = &app.base_url;
    let cookie = register_and_login(base, "alice").await;
    let client = reqwest::Client::new();

    // 放一个 modular avatar
    let resp = client
        .put(format!("{base}/api/avatar"))
        .header(reqwest::header::COOKIE, &cookie)
        .json(&serde_json::json!({
            "config": {
                "kind": "modular",
                "skin": "#f0c8a0", "hair": "#503018", "shirt": "#3868b0", "pants": "#404048"
            }
        }))
        .send()
        .await
        .expect("put avatar");
    assert_eq!(resp.status().as_u16(), 204, "put avatar should be 204");

    // 插一个假 accessory asset（owner_id=1=alice, kind='accessory'）
    sqlx::query(
        "INSERT INTO assets (id, owner_id, kind, storage_key, meta_json, created_at)
         VALUES ('acc-1', 1, 'accessory', 'acc/test.png', '{}', 0)",
    )
    .execute(&app.db)
    .await
    .expect("insert asset");

    // POST equip {slot:"hand", asset_id:"acc-1"}
    let resp = client
        .post(format!("{base}/api/avatar/equip"))
        .header(reqwest::header::COOKIE, &cookie)
        .json(&serde_json::json!({ "slot": "hand", "asset_id": "acc-1" }))
        .send()
        .await
        .expect("equip");
    assert_eq!(resp.status().as_u16(), 200, "equip should be 200: {}", resp.status());

    let body: serde_json::Value = resp.json().await.expect("equip json");
    let avatar = &body["avatar"];
    let equipped = avatar["equipped"]
        .as_array()
        .expect("equipped must be an array");
    assert_eq!(equipped.len(), 1, "one equipped item");
    assert_eq!(equipped[0]["slot"], "hand");
    assert_eq!(equipped[0]["asset_id"], "acc-1");
    assert_eq!(
        equipped[0]["asset_key"], "acc/test.png",
        "asset_key must be storage_key from assets table"
    );

    // 读回 DB 确认持久化
    let (cfg,): (String,) = sqlx::query_as("SELECT config_json FROM avatars WHERE user_id = 1")
        .fetch_one(&app.db)
        .await
        .expect("query avatar");
    let v: serde_json::Value = serde_json::from_str(&cfg).expect("parse config");
    let eq = v["equipped"].as_array().expect("equipped in db");
    assert_eq!(eq[0]["asset_key"], "acc/test.png");

    // unequip
    let resp = client
        .post(format!("{base}/api/avatar/unequip"))
        .header(reqwest::header::COOKIE, &cookie)
        .json(&serde_json::json!({ "slot": "hand" }))
        .send()
        .await
        .expect("unequip");
    assert_eq!(resp.status().as_u16(), 200, "unequip should be 200");

    let body: serde_json::Value = resp.json().await.expect("unequip json");
    let equipped = body["avatar"]["equipped"].as_array();
    assert!(
        equipped.is_none() || equipped.unwrap().is_empty(),
        "equipped should be empty or absent after unequip"
    );

    // DB 也确认
    let (cfg2,): (String,) = sqlx::query_as("SELECT config_json FROM avatars WHERE user_id = 1")
        .fetch_one(&app.db)
        .await
        .expect("query avatar 2");
    let v2: serde_json::Value = serde_json::from_str(&cfg2).expect("parse config 2");
    let eq2 = v2["equipped"].as_array();
    assert!(
        eq2.is_none() || eq2.unwrap().is_empty(),
        "equipped should be empty in DB after unequip"
    );
}

/// equip back slot → 再 equip hand slot → 两条 equipped，各 slot 一条。
/// 再 equip 同 slot → 替换（不追加）。
#[tokio::test]
async fn equip_multiple_slots_and_replace_same_slot() {
    let app = spawn_app().await;
    let base = &app.base_url;
    let cookie = register_and_login(base, "alice").await;
    let client = reqwest::Client::new();

    // avatar
    let _ = client
        .put(format!("{base}/api/avatar"))
        .header(reqwest::header::COOKIE, &cookie)
        .json(&serde_json::json!({
            "config": { "kind":"modular", "skin":"#f0c8a0","hair":"#503018","shirt":"#3868b0","pants":"#404048" }
        }))
        .send()
        .await
        .expect("put avatar");

    // 两个 asset
    sqlx::query(
        "INSERT INTO assets (id, owner_id, kind, storage_key, meta_json, created_at)
         VALUES ('acc-back', 1, 'accessory', 'acc/back.png', '{}', 0),
                ('acc-hand', 1, 'accessory', 'acc/hand.png', '{}', 0)",
    )
    .execute(&app.db)
    .await
    .expect("insert assets");

    // equip back
    let resp = client
        .post(format!("{base}/api/avatar/equip"))
        .header(reqwest::header::COOKIE, &cookie)
        .json(&serde_json::json!({ "slot": "back", "asset_id": "acc-back" }))
        .send()
        .await
        .expect("equip back");
    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value = resp.json().await.expect("json");
    let eq = body["avatar"]["equipped"].as_array().expect("equipped");
    assert_eq!(eq.len(), 1);
    assert_eq!(eq[0]["slot"], "back");

    // equip hand
    let resp = client
        .post(format!("{base}/api/avatar/equip"))
        .header(reqwest::header::COOKIE, &cookie)
        .json(&serde_json::json!({ "slot": "hand", "asset_id": "acc-hand" }))
        .send()
        .await
        .expect("equip hand");
    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value = resp.json().await.expect("json");
    let eq = body["avatar"]["equipped"].as_array().expect("equipped");
    assert_eq!(eq.len(), 2, "two slots equipped");

    // equip back again → replace, not append
    sqlx::query(
        "INSERT INTO assets (id, owner_id, kind, storage_key, meta_json, created_at)
         VALUES ('acc-back2', 1, 'accessory', 'acc/back2.png', '{}', 0)",
    )
    .execute(&app.db)
    .await
    .expect("insert asset 2");

    let resp = client
        .post(format!("{base}/api/avatar/equip"))
        .header(reqwest::header::COOKIE, &cookie)
        .json(&serde_json::json!({ "slot": "back", "asset_id": "acc-back2" }))
        .send()
        .await
        .expect("equip back2");
    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value = resp.json().await.expect("json");
    let eq = body["avatar"]["equipped"].as_array().expect("equipped");
    assert_eq!(eq.len(), 2, "replace, not append");
    let back_item = eq
        .iter()
        .find(|i| i["slot"] == "back")
        .expect("back slot exists");
    assert_eq!(back_item["asset_id"], "acc-back2", "replaced");
    assert_eq!(back_item["asset_key"], "acc/back2.png");
}

/// 非主人 asset → 403。
#[tokio::test]
async fn equip_non_owner_asset_403() {
    let app = spawn_app().await;
    let base = &app.base_url;
    let cookie_a = register_and_login(base, "alice").await;
    let cookie_b = register_and_login(base, "bob").await;
    let client = reqwest::Client::new();

    // alice + bob 都放 avatar
    for cookie in [&cookie_a, &cookie_b] {
        let _ = client
            .put(format!("{base}/api/avatar"))
            .header(reqwest::header::COOKIE, cookie)
            .json(&serde_json::json!({
                "config": { "kind":"modular", "skin":"#f0c8a0","hair":"#503018","shirt":"#3868b0","pants":"#404048" }
            }))
            .send()
            .await
            .expect("put avatar");
    }

    // alice 的 asset (owner_id=1)
    sqlx::query(
        "INSERT INTO assets (id, owner_id, kind, storage_key, meta_json, created_at)
         VALUES ('acc-alice', 1, 'accessory', 'acc/alice.png', '{}', 0)",
    )
    .execute(&app.db)
    .await
    .expect("insert");

    // bob 尝试 equip alice 的 asset → 403
    let resp = client
        .post(format!("{base}/api/avatar/equip"))
        .header(reqwest::header::COOKIE, &cookie_b)
        .json(&serde_json::json!({ "slot": "hand", "asset_id": "acc-alice" }))
        .send()
        .await
        .expect("equip");
    assert_eq!(
        resp.status().as_u16(),
        403,
        "non-owner equip should be 403, got {}",
        resp.status()
    );

    // bob 的 config_json 不应被修改
    let (cfg,): (String,) = sqlx::query_as("SELECT config_json FROM avatars WHERE user_id = 2")
        .fetch_one(&app.db)
        .await
        .expect("query bob avatar");
    let v: serde_json::Value = serde_json::from_str(&cfg).expect("parse");
    assert!(
        v.get("equipped").is_none(),
        "bob's config should not have equipped after 403"
    );
}

/// 非法 slot → 400。
#[tokio::test]
async fn equip_invalid_slot_400() {
    let app = spawn_app().await;
    let base = &app.base_url;
    let cookie = register_and_login(base, "alice").await;
    let client = reqwest::Client::new();

    let _ = client
        .put(format!("{base}/api/avatar"))
        .header(reqwest::header::COOKIE, &cookie)
        .json(&serde_json::json!({
            "config": { "kind":"modular", "skin":"#f0c8a0","hair":"#503018","shirt":"#3868b0","pants":"#404048" }
        }))
        .send()
        .await
        .expect("put avatar");

    let resp = client
        .post(format!("{base}/api/avatar/equip"))
        .header(reqwest::header::COOKIE, &cookie)
        .json(&serde_json::json!({ "slot": "head", "asset_id": "whatever" }))
        .send()
        .await
        .expect("equip");
    assert_eq!(
        resp.status().as_u16(),
        400,
        "invalid slot should be 400, got {}",
        resp.status()
    );

    // unequip 非法 slot 也 400
    let resp = client
        .post(format!("{base}/api/avatar/unequip"))
        .header(reqwest::header::COOKIE, &cookie)
        .json(&serde_json::json!({ "slot": "feet" }))
        .send()
        .await
        .expect("unequip");
    assert_eq!(
        resp.status().as_u16(),
        400,
        "invalid unequip slot should be 400"
    );
}

/// equip 不存在的 asset → 404；无 avatar → 404。
#[tokio::test]
async fn equip_nonexistent_asset_404_and_no_avatar_404() {
    let app = spawn_app().await;
    let base = &app.base_url;
    let cookie = register_and_login(base, "alice").await;
    let client = reqwest::Client::new();

    // 无 avatar → equip 应 404
    let resp = client
        .post(format!("{base}/api/avatar/equip"))
        .header(reqwest::header::COOKIE, &cookie)
        .json(&serde_json::json!({ "slot": "hand", "asset_id": "nope" }))
        .send()
        .await
        .expect("equip no avatar");
    assert_eq!(
        resp.status().as_u16(),
        404,
        "equip without avatar should be 404"
    );

    // 放 avatar
    let _ = client
        .put(format!("{base}/api/avatar"))
        .header(reqwest::header::COOKIE, &cookie)
        .json(&serde_json::json!({
            "config": { "kind":"modular", "skin":"#f0c8a0","hair":"#503018","shirt":"#3868b0","pants":"#404048" }
        }))
        .send()
        .await
        .expect("put avatar");

    // 不存在的 asset → 404
    let resp = client
        .post(format!("{base}/api/avatar/equip"))
        .header(reqwest::header::COOKIE, &cookie)
        .json(&serde_json::json!({ "slot": "hand", "asset_id": "nonexistent-asset" }))
        .send()
        .await
        .expect("equip nonexistent");
    assert_eq!(
        resp.status().as_u16(),
        404,
        "nonexistent asset should be 404"
    );
}

/// put_avatar 保留 equipped 字段（不剥离）。
#[tokio::test]
async fn put_avatar_preserves_equipped() {
    let app = spawn_app().await;
    let base = &app.base_url;
    let cookie = register_and_login(base, "alice").await;
    let client = reqwest::Client::new();

    // 先放 avatar + equip 一个 asset
    let _ = client
        .put(format!("{base}/api/avatar"))
        .header(reqwest::header::COOKIE, &cookie)
        .json(&serde_json::json!({
            "config": { "kind":"modular", "skin":"#f0c8a0","hair":"#503018","shirt":"#3868b0","pants":"#404048" }
        }))
        .send()
        .await
        .expect("put avatar");

    sqlx::query(
        "INSERT INTO assets (id, owner_id, kind, storage_key, meta_json, created_at)
         VALUES ('acc-keep', 1, 'accessory', 'acc/keep.png', '{}', 0)",
    )
    .execute(&app.db)
    .await
    .expect("insert");

    let _ = client
        .post(format!("{base}/api/avatar/equip"))
        .header(reqwest::header::COOKIE, &cookie)
        .json(&serde_json::json!({ "slot": "back", "asset_id": "acc-keep" }))
        .send()
        .await
        .expect("equip");

    // 再 PUT avatar（改颜色），带 equipped → 应保留
    let resp = client
        .put(format!("{base}/api/avatar"))
        .header(reqwest::header::COOKIE, &cookie)
        .json(&serde_json::json!({
            "config": {
                "kind":"modular",
                "skin":"#aabbcc","hair":"#503018","shirt":"#3868b0","pants":"#404048",
                "equipped": [{"slot":"back","asset_id":"acc-keep","asset_key":"acc/keep.png"}]
            }
        }))
        .send()
        .await
        .expect("put avatar with equipped");
    assert_eq!(resp.status().as_u16(), 204);

    let (cfg,): (String,) = sqlx::query_as("SELECT config_json FROM avatars WHERE user_id = 1")
        .fetch_one(&app.db)
        .await
        .expect("query");
    let v: serde_json::Value = serde_json::from_str(&cfg).expect("parse");
    assert_eq!(v["skin"], "#aabbcc", "color updated");
    let eq = v["equipped"].as_array().expect("equipped preserved");
    assert_eq!(eq.len(), 1);
    assert_eq!(eq[0]["slot"], "back");
    assert_eq!(eq[0]["asset_key"], "acc/keep.png");

    // PUT 不带 equipped → equipped 不保留（前端不传就不存；equipped 只在 equip 端点权威写入）
    let resp = client
        .put(format!("{base}/api/avatar"))
        .header(reqwest::header::COOKIE, &cookie)
        .json(&serde_json::json!({
            "config": { "kind":"modular", "skin":"#aabbcc","hair":"#503018","shirt":"#3868b0","pants":"#404048" }
        }))
        .send()
        .await
        .expect("put without equipped");
    assert_eq!(resp.status().as_u16(), 204);
    let (cfg2,): (String,) = sqlx::query_as("SELECT config_json FROM avatars WHERE user_id = 1")
        .fetch_one(&app.db)
        .await
        .expect("query 2");
    let v2: serde_json::Value = serde_json::from_str(&cfg2).expect("parse 2");
    assert!(
        v2.get("equipped").is_none(),
        "equipped should be absent when not in request"
    );
}

/// D4: generated avatar 也能 equip（不拒绝）。
#[tokio::test]
async fn equip_works_for_generated_avatar() {
    let app = spawn_app().await;
    let base = &app.base_url;
    let cookie = register_and_login(base, "alice").await;
    let client = reqwest::Client::new();

    // 直接插一个 generated avatar config
    sqlx::query(
        "INSERT INTO avatars (user_id, kind, config_json, updated_at)
         VALUES (1, 'generated', ?, 0)",
    )
    .bind(r#"{"kind":"generated","character_id":"char-1","frames":{"south":["s.png"],"north":["n.png"],"west":["w.png"],"east":["e.png"]}}"#)
    .execute(&app.db)
    .await
    .expect("insert generated avatar");

    sqlx::query(
        "INSERT INTO assets (id, owner_id, kind, storage_key, meta_json, created_at)
         VALUES ('acc-gen', 1, 'accessory', 'acc/gen.png', '{}', 0)",
    )
    .execute(&app.db)
    .await
    .expect("insert asset");

    // equip on generated → 200 (D4: 不拒绝 generated)
    let resp = client
        .post(format!("{base}/api/avatar/equip"))
        .header(reqwest::header::COOKIE, &cookie)
        .json(&serde_json::json!({ "slot": "hand", "asset_id": "acc-gen" }))
        .send()
        .await
        .expect("equip");
    assert_eq!(
        resp.status().as_u16(),
        200,
        "equip on generated should succeed (D4)"
    );

    let body: serde_json::Value = resp.json().await.expect("json");
    assert_eq!(body["avatar"]["kind"], "generated");
    let eq = body["avatar"]["equipped"].as_array().expect("equipped");
    assert_eq!(eq.len(), 1);
    assert_eq!(eq[0]["asset_key"], "acc/gen.png");
}
