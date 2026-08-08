//! 切片1 端到端实时脊椎测试（dev-plan §三 切片1 验收清单）。
//! 全程离线（无 PIXELLAB/MINIMAX key）。禁令 #2 机器守卫：聊天不入库。

mod common;
use common::{register_and_login, spawn_app, ws_connect};

use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use house_of_imbibe::realtime::protocol::{ClientMsg, PlayerSnap, ServerMsg};
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

async fn send(ws: &mut WS, m: &ClientMsg) {
    ws.send(Message::Text(serde_json::to_string(m).unwrap()))
        .await
        .expect("ws send");
}

/// 在 deadline 内收一条含满足 pred 的 upsert 的 snapshot_delta；超时 None。
async fn recv_delta_with(ws: &mut WS, mut pred: impl FnMut(&PlayerSnap) -> bool) -> Option<PlayerSnap> {
    let deadline = Instant::now() + Duration::from_millis(700);
    while Instant::now() < deadline {
        match timeout(Duration::from_millis(120), ws.next()).await {
            Ok(Some(Ok(Message::Text(t)))) => {
                if let Ok(ServerMsg::SnapshotDelta { upsert, .. }) =
                    serde_json::from_str::<ServerMsg>(&t.to_string())
                {
                    for p in &upsert {
                        if pred(p) {
                            return Some(p.clone());
                        }
                    }
                }
            }
            _ => continue,
        }
    }
    None
}

#[tokio::test]
async fn realtime_spine_end_to_end() {
    let app = spawn_app().await;
    let base = &app.base_url;

    let cookie_a = register_and_login(base, "alice").await;
    let cookie_b = register_and_login(base, "bob").await;

    // A 连接：welcome → snapshot_full → chat_backlog
    let mut ws_a = ws_connect(base, &cookie_a).await;
    assert!(matches!(recv(&mut ws_a).await, Some(ServerMsg::Welcome { .. })), "A welcome");
    assert!(matches!(recv(&mut ws_a).await, Some(ServerMsg::SnapshotFull { .. })), "A snapshot_full");
    let _ = recv(&mut ws_a).await; // chat_backlog

    // B 连接：snapshot_full 必须含 A
    let mut ws_b = ws_connect(base, &cookie_b).await;
    assert!(matches!(recv(&mut ws_b).await, Some(ServerMsg::Welcome { .. })), "B welcome");
    let snap_b = recv(&mut ws_b).await;
    match snap_b {
        Some(ServerMsg::SnapshotFull { players, .. }) => {
            assert!(players.iter().any(|p| p.name == "alice"), "B snapshot_full missing A: {:?}", players);
        }
        other => panic!("B snapshot_full missing/wrong: {:?}", other),
    }
    let _ = recv(&mut ws_b).await; // chat_backlog

    // A move(5,3) → B 在 700ms 内收到含 A 新位置的 delta
    // (8,2) 是地板（row 2 全地板），可走；spawn 在 (7,6)。注意：move 到墙里会被 clamp。
    send(&mut ws_a, &ClientMsg::Move { v: 1, tx: 8, ty: 2 }).await;
    let moved = recv_delta_with(&mut ws_b, |p| p.name == "alice")
        .await
        .expect("B did not receive A's move delta");
    assert!(moved.x > 7.0 * 16.0 && moved.x < 9.0 * 16.0, "A x not near tile 8: {}", moved.x);
    assert!(moved.y > 1.0 * 16.0 && moved.y < 3.0 * 16.0, "A y not near tile 2: {}", moved.y);

    // A 非法 move(-999,-999) → 服务端 clamp；B 不应看到 A 出界
    send(&mut ws_a, &ClientMsg::Move { v: 1, tx: -999, ty: -999 }).await;
    let illegal = recv_delta_with(&mut ws_b, |p: &PlayerSnap| p.name == "alice" && (p.x < 0.0 || p.y < 0.0)).await;
    assert!(illegal.is_none(), "A appeared at illegal coord (clamp failed): {:?}", illegal);
    // 且任何后续 A 的 delta 都在合法区
    if let Some(p) = recv_delta_with(&mut ws_b, |p| p.name == "alice").await {
        assert!(p.x >= 0.0 && p.x <= 240.0 && p.y >= 0.0 && p.y <= 160.0, "A out of bounds: {:?}", p);
    }

    // A chat "hi" → B 收到
    send(&mut ws_a, &ClientMsg::Chat { v: 1, text: "hi".into() }).await;
    let chat_seen = loop {
        match recv(&mut ws_b).await {
            Some(ServerMsg::Chat { text, .. }) if text == "hi" => break true,
            Some(_) => continue,
            None => break false,
        }
    };
    assert!(chat_seen, "B did not receive chat 'hi'");

    // 新 C 连接 → chat_backlog 含 "hi"
    let cookie_c = register_and_login(base, "carol").await;
    let mut ws_c = ws_connect(base, &cookie_c).await;
    let _ = recv(&mut ws_c).await; // welcome
    let _ = recv(&mut ws_c).await; // snapshot_full
    let back = recv(&mut ws_c).await;
    match back {
        Some(ServerMsg::ChatBacklog { items, .. }) => {
            assert!(items.iter().any(|c| c.text == "hi"), "C backlog missing 'hi': {:?}", items);
        }
        other => panic!("C chat_backlog missing/wrong: {:?}", other),
    }

    // 未知 type：静默忽略，不断连（发裸 JSON；ClientMsg::Unknown 无法构造）
    ws_a.send(Message::Text(r#"{"v":1,"type":"totally_made_up"}"#.into()))
        .await
        .unwrap();
    send(&mut ws_a, &ClientMsg::Ping { v: 1, t: 0 }).await; // 确认连接仍活
    let alive = loop {
        match recv(&mut ws_a).await {
            Some(ServerMsg::Pong { .. }) => break true,
            Some(_) => continue,
            None => break false,
        }
    };
    assert!(alive, "A disconnected after unknown type (must silently ignore)");

    // interact → error{code:unimplemented}
    send(&mut ws_a, &ClientMsg::Interact { v: 1, target: "x".into() }).await;
    let err = loop {
        match recv(&mut ws_a).await {
            Some(ServerMsg::Error { code, .. }) if code == "unimplemented" => break true,
            Some(_) => continue,
            None => break false,
        }
    };
    assert!(err, "did not receive error unimplemented for interact");

    // 禁令 #2 机器守卫：聊天绝不入库
    let count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND (name LIKE '%message%' OR name LIKE '%chat%')",
    )
    .fetch_one(&app.db)
    .await
    .unwrap();
    assert_eq!(count.0, 0, "no chat/messages table may exist (ban #2)");
    let usernames: Vec<(String,)> = sqlx::query_as("SELECT username FROM users")
        .fetch_all(&app.db)
        .await
        .unwrap();
    assert!(!usernames.iter().any(|(u,)| u.contains("hi")), "no 'hi' in users: {:?}", usernames);
    let configs: Vec<(String,)> = sqlx::query_as("SELECT config_json FROM avatars")
        .fetch_all(&app.db)
        .await
        .unwrap();
    assert!(!configs.iter().any(|(c,)| c.contains("hi")), "no 'hi' in avatars");
}
