CREATE TABLE IF NOT EXISTS private_keys (
    key_id INTEGER PRIMARY KEY,
    public_key TEXT UNIQUE,
    private_key TEXT UNIQUE
);