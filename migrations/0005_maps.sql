-- 切片4 D1: 地图视觉背景层。三层架构第一层（issue #0010）。
-- bg_key = assets.storage_key（NULL = 用静态 bar.json tile 渲染兜底）。
-- 可走/碰撞网格层仍来自 assets/maps/bar.json（静态 include_str!），本表只管视觉背景。
CREATE TABLE IF NOT EXISTS maps (
    scene      TEXT PRIMARY KEY,
    width      INTEGER NOT NULL,
    height     INTEGER NOT NULL,
    bg_key     TEXT NULL,
    updated_at INTEGER NOT NULL
);

-- seed bar 场景（15×10 = 240×160 px @ TILE=16）。bg_key NULL = 用静态 tile 渲染。
INSERT OR IGNORE INTO maps (scene, width, height, bg_key, updated_at)
VALUES ('bar', 15, 10, NULL, CAST(strftime('%s','now') AS INTEGER));
