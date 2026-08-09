//! WS 协议 —— Client/Server 消息形状的唯一权威（dev-plan §2.5）。
//!
//! TS 侧 `web/src/protocol/types.ts` 手工镜像本文件；漂移由集成测试 +
//! fixture 桥捕获。规则：每个变体带 `v: u8`(=1)；未知 type 静默忽略
//!（ClientMsg 末尾 `#[serde(other)] Unknown`）；已声明但未实现的客户端
//! 意图，服务端回 `error{code:"unimplemented"}`。

use serde::{Deserialize, Serialize};

/// 单调服务器时间（毫秒），用作客户端插值时钟基准。
pub fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 形象快照 = 原始 config_json（modular 配色或 generated frames）。
/// 服务端对 kind 透明；装载层（前端 prepareCharacterSheet）负责解析。
pub type AvatarSnapshot = serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerSnap {
    pub id: u64,
    pub x: f32,
    pub y: f32,
    pub dir: String,
    pub name: String,
    pub avatar: AvatarSnapshot,
    pub avatar_hash: String,
    pub target_tx: i32,
    pub target_ty: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatItem {
    pub from: u64,
    pub name: String,
    pub text: String,
    pub ts: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMsg {
    Move { v: u8, tx: i32, ty: i32 },
    Chat { v: u8, text: String },
    Interact { v: u8, target: String },
    DialogueAdvance { v: u8, npc: String, choice: Option<String> },
    Ping { v: u8, t: u64 },
    /// 未知/未识别 type —— 静默忽略（前向兼容）。
    #[serde(other)]
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMsg {
    Welcome {
        v: u8,
        self_id: u64,
        scene: String,
        tick_hz: u8,
        server_time: u64,
    },
    SnapshotFull {
        v: u8,
        tick: u64,
        t: u64,
        players: Vec<PlayerSnap>,
        decorations: Vec<serde_json::Value>,
        npcs: Vec<serde_json::Value>,
    },
    SnapshotDelta {
        v: u8,
        tick: u64,
        t: u64,
        upsert: Vec<PlayerSnap>,
        remove: Vec<u64>,
    },
    Chat {
        v: u8,
        from: u64,
        name: String,
        text: String,
        ts: u64,
    },
    ChatBacklog {
        v: u8,
        items: Vec<ChatItem>,
    },
    // 以下为形状稳定预留（切片 5/6 才发送）；demo 永不发送。
    Dialogue {
        v: u8,
        npc: String,
        node: String,
        menu: Option<serde_json::Value>,
    },
    DecorationAdded {
        v: u8,
        decoration: serde_json::Value,
    },
    DecorationRemoved {
        v: u8,
        id: String,
    },
    SceneChanged {
        v: u8,
        scene: String,
        spawn: (i32, i32),
    },
    Kicked {
        v: u8,
        reason: String,
    },
    Error {
        v: u8,
        code: String,
        msg: String,
    },
    Pong {
        v: u8,
        t: u64,
    },
}
