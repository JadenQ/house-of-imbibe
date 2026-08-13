-- 地图可走/碰撞网格层（issue #0010 三层架构第二层）。
-- walkable = JSON 2D 数组文本（0=可走, 1=阻挡）；NULL = 用静态 BAR_MAP 兜底（向后兼容）。
-- SQLite 不支持 ADD COLUMN IF NOT EXISTS，但 sqlx-migrate 跟踪已应用迁移，仅执行一次（见 0003）。
ALTER TABLE maps ADD COLUMN walkable TEXT;
