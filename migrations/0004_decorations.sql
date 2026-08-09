-- 装饰对象层（issue #7 三层架构第三层）。admin 可编辑，服务端 clamp 读它。
-- 与视觉背景层（生成图）+ 可走/碰撞网格层解耦（见 issue #0010 三层架构）。
-- asset_id NULL = 占位装饰（无关联资产，后续切片可绑定）。
CREATE TABLE IF NOT EXISTS decorations (
    id          TEXT PRIMARY KEY,
    scene       TEXT NOT NULL,
    tile_x      INTEGER NOT NULL,
    tile_y      INTEGER NOT NULL,
    asset_id    TEXT NULL REFERENCES assets(id) ON DELETE SET NULL,
    z_layer     INTEGER NOT NULL DEFAULT 0,
    placed_by   INTEGER NOT NULL REFERENCES users(id) ON DELETE SET NULL,
    created_at  INTEGER NOT NULL
);
