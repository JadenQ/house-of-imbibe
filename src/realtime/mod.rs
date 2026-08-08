//! 实时多人脊椎（切片 1）。dev-plan §2.6 / §三 切片1。
//!
//! 关键约束：
//! - 聊天是**全局单个** ring buffer（50，跨场景，禁令 #2：绝不落库），
//!   挂在 RealtimeState 上，不在 Room 上。
//! - 出向双通道：每房间 broadcast_tx（快照）+ 全局 chat_tx（聊天）+
//!   每连接 directed mpsc（welcome/backlog/pong/error/未来 dialogue）。
//! - 绝不在 .await 上持 DashMap 引用：先 clone Arc<Room>，drop 引用，再操作。

pub mod grid;
pub mod protocol;
pub mod room;
pub mod session;

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use dashmap::DashMap;
use tokio::sync::broadcast;

use crate::realtime::grid::{BarGrid, WalkGrid};
use crate::realtime::protocol::{now_ms, ChatItem, ServerMsg};
use crate::realtime::room::{PlayerState, Room, spawn_tick};

/// 房间键。demo 只有 "bar"；切片 5 加 yard 时此处不变。
pub type SceneId = &'static str;

const CHAT_CAP: usize = 50;
const BROADCAST_CAP: usize = 64;

pub struct RealtimeState {
    /// 所有房间。Arc 包裹以便 tick task 持副本做空闲清理。
    pub rooms: Arc<DashMap<SceneId, Arc<Room>>>,
    /// 全局聊天 ring buffer（禁令 #2：绝不落库，跨场景可见）。
    pub chat: Arc<Mutex<VecDeque<ChatItem>>>,
    /// 全局聊天广播通道（与每房间快照 broadcast_tx 分离）。
    pub chat_tx: broadcast::Sender<ServerMsg>,
}

impl RealtimeState {
    const BAR: SceneId = "bar";

    pub fn new() -> Self {
        let (chat_tx, _) = broadcast::channel(BROADCAST_CAP);
        Self {
            rooms: Arc::new(DashMap::new()),
            chat: Arc::new(Mutex::new(VecDeque::new())),
            chat_tx,
        }
    }

    /// 取或建 bar 房间；若 tick 死了则重启（join = 活 tick 的真相源，缩窄 remove 竞态）。
    fn ensure_bar(&self) -> Arc<Room> {
        // 快路径：存在且 tick 活
        if let Some(r) = self.rooms.get(Self::BAR) {
            if r.tick_alive.load(std::sync::atomic::Ordering::Relaxed) {
                return Arc::clone(&r);
            }
        }
        // 慢路径：entry（持 shard 写锁，仅同步操作，不 .await）
        let room = {
            let entry = self.rooms.entry(Self::BAR).or_insert_with(|| {
                let grid: Arc<dyn WalkGrid> = Arc::new(BarGrid::parse());
                Arc::new(Room::new(grid))
            });
            Arc::clone(entry.value())
        };
        // entry ref 已随语句结束 drop —— 抢 tick_alive：之前 false 则我们 spawn
        if !room
            .tick_alive
            .swap(true, std::sync::atomic::Ordering::Relaxed)
        {
            spawn_tick(
                Arc::downgrade(&room),
                Arc::clone(&self.rooms),
                Self::BAR,
            );
        }
        room
    }

    /// 入场：注册玩家 + 定向 mpsc + 广播订阅；返回所需 rx + player_id。
    #[allow(clippy::type_complexity)]
    pub async fn enter(
        &self,
        uid: i64,
        name: String,
        avatar: serde_json::Value,
    ) -> (
        Arc<Room>,
        tokio::sync::mpsc::UnboundedReceiver<ServerMsg>,
        broadcast::Receiver<ServerMsg>,
        broadcast::Receiver<ServerMsg>,
        u64,
    ) {
        let pid = uid as u64;
        let (dir_tx, dir_rx) = tokio::sync::mpsc::unbounded_channel();
        let room = self.ensure_bar();
        {
            let (tx, ty) = room.grid.spawn();
            room.players.insert(
                pid,
                PlayerState {
                    id: pid,
                    name,
                    tx,
                    ty,
                    dir: "s".into(),
                    avatar,
                    rev: 1,
                },
            );
            room.directed.insert(pid, dir_tx);
        }
        // 立即广播自己入场（不等 tick 的 100ms）
        if let Some(snap) = room.player_snap(pid) {
            let _ = room.broadcast_tx.send(ServerMsg::SnapshotDelta {
                v: 1,
                tick: room.tick.load(std::sync::atomic::Ordering::Relaxed),
                t: now_ms(),
                upsert: vec![snap],
                remove: vec![],
            });
        }
        let bcast_rx = room.broadcast_tx.subscribe();
        let chat_rx = self.chat_tx.subscribe();
        (room, dir_rx, bcast_rx, chat_rx, pid)
    }

    /// 离场：摘除玩家 + 定向通道，广播 remove。
    pub async fn leave(&self, pid: u64) {
        let room = match self.rooms.get(Self::BAR) {
            Some(r) => Arc::clone(&r),
            None => return,
        };
        room.players.remove(&pid);
        room.directed.remove(&pid);
        let _ = room.broadcast_tx.send(ServerMsg::SnapshotDelta {
            v: 1,
            tick: room.tick.load(std::sync::atomic::Ordering::Relaxed),
            t: now_ms(),
            upsert: vec![],
            remove: vec![pid],
        });
    }

    pub fn chat_backlog(&self) -> Vec<ChatItem> {
        self.chat.lock().unwrap().iter().cloned().collect()
    }

    /// 全局聊天入栈 + 广播。绝不落库。
    pub fn push_chat(&self, item: ChatItem) {
        {
            let mut c = self.chat.lock().unwrap();
            c.push_back(item.clone());
            while c.len() > CHAT_CAP {
                c.pop_front();
            }
        } // drop lock 后再 broadcast（broadcast::send 非阻塞）
        let _ = self.chat_tx.send(ServerMsg::Chat {
            v: 1,
            from: item.from,
            name: item.name,
            text: item.text,
            ts: item.ts,
        });
    }
}

impl Default for RealtimeState {
    fn default() -> Self {
        Self::new()
    }
}
