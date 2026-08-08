//! WS /ws/room 连接驱动（dev-plan §三 切片1）。
//! select! over：房间快照 broadcast + 全局聊天 broadcast + 定向 mpsc + 客户端帧。
//! Lagged → 补发 full/backlog（经定向通道）。绝不持 DashMap 引用跨 .await。

use std::sync::Arc;

use axum::extract::ws::{Message, WebSocketUpgrade};
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tracing::warn;

use crate::realtime::protocol::{now_ms, ChatItem, ClientMsg, ServerMsg};
use crate::realtime::room::Room;
use crate::realtime::RealtimeState;

pub async fn ws_room(
    State(state): State<Arc<crate::AppState>>,
    ws: WebSocketUpgrade,
    headers: HeaderMap,
) -> Result<Response, StatusCode> {
    let (uid, name, _) = crate::current_user(&state, &headers)
        .await
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let avatar: serde_json::Value = match sqlx::query_as::<_, (String,)>(
        "SELECT config_json FROM avatars WHERE user_id = ?",
    )
    .bind(uid)
    .fetch_optional(&state.db)
    .await
    {
        Ok(Some((j,))) => serde_json::from_str(&j).unwrap_or_else(|_| default_avatar()),
        _ => default_avatar(),
    };

    let rt = state.rt.clone();
    let my_name = name.clone();
    let (room, mut dir_rx, mut bcast_rx, mut chat_rx, pid) = rt.enter(uid, name, avatar).await;

    // 定向 tx（loop 内 Lagged 补发 / pong / error 用）
    let dir_tx: mpsc::UnboundedSender<ServerMsg> = room
        .directed
        .get(&pid)
        .map(|e| e.value().clone())
        .unwrap_or_else(|| mpsc::unbounded_channel().0);
    // 入场定向：welcome + chat_backlog（排队等 loop 的 dir_rx arm 发出）
    let _ = dir_tx.send(ServerMsg::Welcome {
        v: 1,
        self_id: pid,
        scene: "bar".into(),
        tick_hz: 10,
        server_time: now_ms(),
    });
    // 连接即发全量快照（dev-plan AC：on connect sends a full snapshot）——
    // 让新人看到当前所有玩家；否则静止的远端玩家永远看不到。
    let _ = dir_tx.send(room.snapshot_full(pid));
    let _ = dir_tx.send(ServerMsg::ChatBacklog {
        v: 1,
        items: rt.chat_backlog(),
    });

    Ok(ws.on_upgrade(move |socket| async move {
        let (mut ws_tx, mut ws_rx) = socket.split();
        loop {
            tokio::select! {
                msg = bcast_rx.recv() => match msg {
                    Ok(m) => {
                        let txt = serde_json::to_string(&m).unwrap_or_default();
                        if ws_tx.send(Message::Text(txt.into())).await.is_err() { break; }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        let _ = dir_tx.send(room.snapshot_full(pid));
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                },
                msg = chat_rx.recv() => match msg {
                    Ok(m) => {
                        let txt = serde_json::to_string(&m).unwrap_or_default();
                        if ws_tx.send(Message::Text(txt.into())).await.is_err() { break; }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        let _ = dir_tx.send(ServerMsg::ChatBacklog { v: 1, items: rt.chat_backlog() });
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                },
                Some(m) = dir_rx.recv() => {
                    let txt = serde_json::to_string(&m).unwrap_or_default();
                    if ws_tx.send(Message::Text(txt.into())).await.is_err() { break; }
                }
                frame = ws_rx.next() => match frame {
                    Some(Ok(Message::Text(t))) => {
                        handle_client(t.as_str(), &room, &rt, pid, &my_name, &dir_tx);
                    }
                    Some(Ok(_)) => {}
                    Some(Err(_)) | None => break,
                }
            }
        }
        rt.leave(pid).await;
    }))
}

fn handle_client(
    text: &str,
    room: &Arc<Room>,
    rt: &Arc<RealtimeState>,
    pid: u64,
    name: &str,
    dir_tx: &mpsc::UnboundedSender<ServerMsg>,
) {
    let msg: ClientMsg = match serde_json::from_str(text) {
        Ok(m) => m,
        Err(_) => {
            warn!("bad client msg, ignored: {text}");
            return; // 静默丢弃，不断连
        }
    };
    match msg {
        ClientMsg::Move { tx, ty, .. } => {
            if let Some(mut p) = room.players.get_mut(&pid) {
                let from = (p.tx, p.ty);
                let (nx, ny) = room.grid.clamp(from, (tx, ty));
                let dir = if ny > p.ty {
                    "s".to_string()
                } else if ny < p.ty {
                    "n".to_string()
                } else if nx > p.tx {
                    "e".to_string()
                } else if nx < p.tx {
                    "w".to_string()
                } else {
                    p.dir.clone()
                };
                if (nx, ny) != from || dir != p.dir {
                    p.tx = nx;
                    p.ty = ny;
                    p.dir = dir;
                    p.rev = p.rev.wrapping_add(1);
                }
            }
        }
        ClientMsg::Chat { text, .. } => {
            rt.push_chat(ChatItem {
                from: pid,
                name: name.to_string(),
                text,
                ts: now_ms(),
            });
        }
        ClientMsg::Ping { .. } => {
            let _ = dir_tx.send(ServerMsg::Pong { v: 1, t: now_ms() });
        }
        ClientMsg::Interact { .. } | ClientMsg::DialogueAdvance { .. } => {
            let _ = dir_tx.send(ServerMsg::Error {
                v: 1,
                code: "unimplemented".into(),
                msg: "not yet in this demo".into(),
            });
        }
        ClientMsg::Unknown => {}
    }
}

fn default_avatar() -> serde_json::Value {
    serde_json::json!({
        "kind": "modular",
        "skin": "#f0c8a0",
        "hair": "#503018",
        "shirt": "#3868b0",
        "pants": "#404048",
    })
}
