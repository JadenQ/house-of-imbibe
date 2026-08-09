//! 房间状态 + 10Hz delta tick task（dev-plan §三 切片1 的最难部分）。
//!
//! 三个经典陷阱（已规避）：
//! - Lagged → 由 recv 端补发 snapshot_full（不传播错误）。
//! - tick task 持 Weak<Room>，空 N 次自清理；join 是活 tick 的真相源。
//! - 绝不持 DashMap ref 跨 .await：先 upgrade() 拿 Arc<Room>，drop entry ref。

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, Weak};
use std::time::Duration;

use dashmap::DashMap;
use sqlx::SqlitePool;
use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinHandle;
use tokio::time::{interval, MissedTickBehavior};

use crate::realtime::grid::WalkGrid;
use crate::realtime::protocol::{now_ms, PlayerSnap, ServerMsg};

pub type PlayerId = u64;

const BROADCAST_CAP: usize = 64;
const IDLE_TICKS_BEFORE_CLEANUP: u32 = 30;

pub struct PlayerState {
    pub id: u64,
    pub name: String,
    pub tx: i32,
    pub ty: i32,
    pub dir: String,
    pub avatar: serde_json::Value,
    pub rev: u64,
}

pub struct Room {
    pub scene: String,
    pub grid: Arc<dyn WalkGrid>,
    pub players: DashMap<PlayerId, PlayerState>,
    pub broadcast_tx: broadcast::Sender<ServerMsg>,
    pub directed: DashMap<PlayerId, mpsc::UnboundedSender<ServerMsg>>,
    pub tick_alive: AtomicBool,
    pub last_broadcast_rev: AtomicU64,
    pub tick: AtomicU64,
    /// 装饰对象层（内存缓存；DB 为真相源，snapshot_full 从 DB 查）。
    pub decorations: Mutex<Vec<serde_json::Value>>,
}

impl Room {
    pub fn new(scene: String, grid: Arc<dyn WalkGrid>) -> Self {
        let (broadcast_tx, _) = broadcast::channel(BROADCAST_CAP);
        Self {
            scene,
            grid,
            players: DashMap::new(),
            broadcast_tx,
            directed: DashMap::new(),
            tick_alive: AtomicBool::new(false),
            last_broadcast_rev: AtomicU64::new(0),
            tick: AtomicU64::new(0),
            decorations: Mutex::new(Vec::new()),
        }
    }

    fn snap_from(p: &PlayerState) -> PlayerSnap {
        PlayerSnap {
            id: p.id,
            x: p.tx as f32 * 16.0 + 8.0,
            y: p.ty as f32 * 16.0 + 8.0,
            dir: p.dir.clone(),
            name: p.name.clone(),
            avatar: p.avatar.clone(),
            avatar_hash: avatar_hash(&p.avatar),
            target_tx: p.tx,
            target_ty: p.ty,
        }
    }

    pub fn player_snap(&self, id: PlayerId) -> Option<PlayerSnap> {
        let p = self.players.get(&id)?;
        Some(Self::snap_from(&p))
    }

    pub async fn snapshot_full(&self, _self_id: u64, db: &SqlitePool) -> ServerMsg {
        let players: Vec<PlayerSnap> = self
            .players
            .iter()
            .map(|e| Self::snap_from(e.value()))
            .collect();
        let tick = self.tick.load(Ordering::Relaxed);
        // 从 DB 查该 scene 的装饰（晚加入者能看到已有装饰；空时不破坏现有 spine 测试）
        // asset_key = LEFT JOIN assets.storage_key（无资产 → null）；不落库，join 出来。
        let decorations: Vec<serde_json::Value> =
            sqlx::query_as::<_, (String, String, i64, i64, Option<String>, i64, i64, Option<String>)>(
                "SELECT d.id, d.scene, d.tile_x, d.tile_y, d.asset_id, d.z_layer, d.placed_by, a.storage_key
                 FROM decorations d
                 LEFT JOIN assets a ON d.asset_id = a.id
                 WHERE d.scene = ? ORDER BY d.created_at",
            )
            .bind(&self.scene)
            .fetch_all(db)
            .await
            .map(|rows| {
                rows.iter()
                    .map(|(id, scene, tx, ty, aid, z, pb, akey)| {
                        serde_json::json!({
                            "id": id,
                            "scene": scene,
                            "tile_x": tx,
                            "tile_y": ty,
                            "asset_id": aid,
                            "asset_key": akey,
                            "z_layer": z,
                            "placed_by": pb,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        ServerMsg::SnapshotFull {
            v: 1,
            tick,
            t: now_ms(),
            players,
            decorations,
            npcs: vec![],
        }
    }

    /// 构建自 last_rev 以来变动的 delta，返回 (msg, 本 tick 发出的最大 rev)。
    pub fn build_delta(&self, last_rev: u64) -> (ServerMsg, u64) {
        let mut upsert = Vec::new();
        let mut max_rev = last_rev;
        for entry in self.players.iter() {
            let p = entry.value();
            if p.rev > last_rev {
                upsert.push(Self::snap_from(p));
                if p.rev > max_rev {
                    max_rev = p.rev;
                }
            }
        }
        (
            ServerMsg::SnapshotDelta {
                v: 1,
                tick: self.tick.load(Ordering::Relaxed),
                t: now_ms(),
                upsert,
                remove: vec![],
            },
            max_rev,
        )
    }

    /// admin 放置装饰：更新内存列表 + 广播 DecorationAdded。
    pub fn add_decoration(&self, decoration: serde_json::Value) {
        {
            let mut decs = self.decorations.lock().unwrap();
            decs.push(decoration.clone());
        } // lock 释放后再 broadcast（broadcast::send 非阻塞）
        let _ = self.broadcast_tx.send(ServerMsg::DecorationAdded {
            v: 1,
            decoration,
        });
    }

    /// admin 移除装饰：更新内存列表 + 广播 DecorationRemoved。
    pub fn remove_decoration(&self, id: String) {
        {
            let target = id.as_str();
            let mut decs = self.decorations.lock().unwrap();
            decs.retain(|d| {
                d.get("id")
                    .and_then(|v| v.as_str())
                    != Some(target)
            });
        }
        let _ = self.broadcast_tx.send(ServerMsg::DecorationRemoved {
            v: 1,
            id,
        });
    }
}

/// 启动 10Hz delta tick。持有 Weak<Room> + rooms 副本，空闲 N 次自清理。
/// 注意：本 task 内绝不持 DashMap entry 跨 .await —— 先 upgrade() 拿 Arc<Room>。
pub fn spawn_tick(
    weak: Weak<Room>,
    rooms: Arc<DashMap<crate::realtime::SceneId, Arc<Room>>>,
    scene: crate::realtime::SceneId,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut timer = interval(Duration::from_millis(100));
        timer.set_missed_tick_behavior(MissedTickBehavior::Delay);
        let mut idle = 0u32;
        loop {
            timer.tick().await; // 不持任何 DashMap 引用
            let Some(room) = weak.upgrade() else {
                return; // Room 已释放 → 退出
            };
            if room.players.is_empty() {
                idle += 1;
                if idle >= IDLE_TICKS_BEFORE_CLEANUP {
                    room.tick_alive.store(false, Ordering::Relaxed);
                    // 重检空：若此刻有人 join，tick_alive=false 让它重启 tick
                    if room.players.is_empty() {
                        rooms.remove(scene);
                    }
                    return;
                }
                continue;
            }
            idle = 0;
            room.tick.fetch_add(1, Ordering::Relaxed);
            let last = room.last_broadcast_rev.load(Ordering::Relaxed);
            let (delta, max_rev) = room.build_delta(last);
            room.last_broadcast_rev
                .store(max_rev.max(last), Ordering::Relaxed);
            // 广播：非阻塞；receiver 满则 Lagged（由 recv 端补发 full）
            let _ = room.broadcast_tx.send(delta);
        }
    })
}

/// per-avatar 稳定哈希（前端会自己重算缓存键，跨语言精确一致不要求）。
pub fn avatar_hash(a: &serde_json::Value) -> String {
    let s = serde_json::to_string(a).unwrap_or_default();
    let mut h: u64 = 5381;
    for b in s.as_bytes() {
        h = h.wrapping_mul(33).wrapping_add(*b as u64);
    }
    format!("{h:x}")
}
