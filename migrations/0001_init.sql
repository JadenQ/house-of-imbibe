-- 工期1: auth + avatar。聊天绝不落库（决策8），故无 messages 表。
CREATE TABLE IF NOT EXISTS users (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    username      TEXT NOT NULL UNIQUE COLLATE NOCASE,
    password_hash TEXT NOT NULL,
    is_admin      INTEGER NOT NULL DEFAULT 0,
    created_at    INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS sessions (
    token      TEXT PRIMARY KEY,
    user_id    INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at INTEGER NOT NULL
);

-- 形象配置：工期1 只存模块化配色 JSON；kind 预留 (modular,generated)
CREATE TABLE IF NOT EXISTS avatars (
    user_id     INTEGER PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    kind        TEXT NOT NULL DEFAULT 'modular' CHECK (kind IN ('modular','generated')),
    config_json TEXT NOT NULL,
    updated_at  INTEGER NOT NULL
);
