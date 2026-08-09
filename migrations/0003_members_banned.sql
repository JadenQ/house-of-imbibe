-- 切片4 Admin: 成员管理。banned 列用于封禁用户（login 拒绝 banned=1）。
-- SQLite 不支持 ADD COLUMN IF NOT EXISTS，但 sqlx-migrate 跟踪已应用迁移，仅执行一次。
ALTER TABLE users ADD COLUMN banned INTEGER NOT NULL DEFAULT 0;
