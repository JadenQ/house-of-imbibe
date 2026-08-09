-- 切片3a: assets + generation_jobs 表。
-- assets: 落地后的二进制资产（avatar PNG / 生成的像素图），存 storage_key 不存 URL。
-- generation_jobs: 分钟级异步生成任务落库（禁令#1 修复：不在 HTTP 路径等待生成）。

CREATE TABLE IF NOT EXISTS assets (
    id           TEXT PRIMARY KEY,
    owner_id     INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    kind         TEXT NOT NULL,
    storage_key  TEXT NOT NULL,
    meta_json    TEXT,
    created_at   INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS generation_jobs (
    id               TEXT PRIMARY KEY,
    owner_id         INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    kind             TEXT NOT NULL,
    status           TEXT NOT NULL CHECK(status IN('pending','running','done','failed')),
    params_json      TEXT,
    result_asset_id TEXT NULL REFERENCES assets(id) ON DELETE SET NULL,
    error            TEXT NULL,
    created_at       INTEGER NOT NULL,
    completed_at     INTEGER NULL
);
