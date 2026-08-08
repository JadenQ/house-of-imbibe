//! 走路网格抽象 + bar 地图实现（dev-plan §2.6）。
//! 碰撞单一来源 = assets/maps/bar.json，Rust 与 TS 共享同一份，
//! 不允许在 Rust 里手写第二份（两份必然漂移）。

use serde::Deserialize;
use std::collections::HashSet;

/// 场景传送门占位（切片 5 才有内容）。
#[allow(dead_code)]
pub struct Portal;

/// 走路网格：服务端 clamp 的唯一接口。切片 1 用 BarGrid，切片 5 换 TmjGrid。
pub trait WalkGrid: Send + Sync {
    fn is_walkable(&self, tx: i32, ty: i32) -> bool;
    /// 从 from 走向 to：能到 to 就到 to；否则逐轴滑行；否则原地不动
    /// （镜像客户端 canStop 的贴墙语义，作反作弊兜底）。
    fn clamp(&self, from: (i32, i32), to: (i32, i32)) -> (i32, i32);
    fn spawn(&self) -> (i32, i32);
    fn portals(&self) -> &[Portal] {
        &[]
    }
}

#[derive(Debug, Deserialize)]
struct BarMapJson {
    rows: Vec<String>,
    solid: Vec<String>,
    #[allow(dead_code)]
    interact: std::collections::HashMap<String, String>,
    spawn: SpawnJson,
}

#[derive(Debug, Deserialize)]
struct SpawnJson {
    tx: i32,
    ty: i32,
}

/// bar 房间走路网格（15×10）。启动时解析一次 include_str!。
pub struct BarGrid {
    rows: Vec<String>,
    solid: HashSet<char>,
    spawn: (i32, i32),
}

impl BarGrid {
    pub fn parse() -> Self {
        let json: BarMapJson = serde_json::from_str(include_str!("../../assets/maps/bar.json"))
            .expect("assets/maps/bar.json must parse");
        let solid: HashSet<char> = json.solid.iter().filter_map(|s| s.chars().next()).collect();
        Self {
            rows: json.rows,
            solid,
            spawn: (json.spawn.tx, json.spawn.ty),
        }
    }
}

impl WalkGrid for BarGrid {
    fn is_walkable(&self, tx: i32, ty: i32) -> bool {
        if ty < 0 || tx < 0 {
            return false;
        }
        let row = match self.rows.get(ty as usize) {
            Some(r) => r,
            None => return false,
        };
        match row.chars().nth(tx as usize) {
            Some(ch) => !self.solid.contains(&ch),
            None => false,
        }
    }

    fn clamp(&self, from: (i32, i32), to: (i32, i32)) -> (i32, i32) {
        if self.is_walkable(to.0, to.1) {
            return to;
        }
        let (fx, fy) = from;
        let mut x = fx;
        let mut y = fy;
        if to.0 != fx && self.is_walkable(to.0, fy) {
            x = to.0;
        }
        if to.1 != fy && self.is_walkable(x, to.1) {
            y = to.1;
        }
        (x, y)
    }

    fn spawn(&self) -> (i32, i32) {
        self.spawn
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid() -> BarGrid {
        BarGrid::parse()
    }

    #[test]
    fn all_rows_are_15_wide() {
        let g = grid();
        assert_eq!(g.rows.len(), 10);
        for (i, r) in g.rows.iter().enumerate() {
            assert_eq!(r.chars().count(), 15, "row {i} not 15 wide: {r:?}");
        }
    }

    #[test]
    fn spawn_is_walkable() {
        let g = grid();
        let (tx, ty) = g.spawn();
        assert!(g.is_walkable(tx, ty));
    }

    #[test]
    fn walls_are_solid() {
        let g = grid();
        assert!(!g.is_walkable(0, 0)); // 墙
        assert!(!g.is_walkable(14, 0)); // 墙
        assert!(g.is_walkable(7, 2)); // 地板
        assert!(!g.is_walkable(1, 1)); // 酒架 S
        assert!(!g.is_walkable(1, 3)); // 吧台 B
    }

    #[test]
    fn clamp_blocks_into_wall() {
        let g = grid();
        // 从地板走到墙里：保持原位
        // (0,2) 是墙 → clamp 保持原位（贴墙）
        assert_eq!(g.clamp((7, 2), (0, 2)), (7, 2));
        // 走进吧台（1,3 是 B，solid）
        assert_eq!(g.clamp((2, 3), (1, 3)), (2, 3));
        // 走到地板：通过
        assert_eq!(g.clamp((7, 2), (8, 2)), (8, 2));
    }
}
