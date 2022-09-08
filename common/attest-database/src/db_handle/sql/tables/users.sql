CREATE TABLE IF NOT EXISTS users (
    user_id INTEGER PRIMARY KEY,
    nickname TEXT,
    key TEXT UNIQUE
);