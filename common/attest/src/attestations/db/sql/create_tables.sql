CREATE TABLE IF NOT EXISTS users (user_id INTEGER PRIMARY KEY, nickname TEXT , key TEXT UNIQUE);

CREATE TABLE IF NOT EXISTS messages
(message_id INTEGER PRIMARY KEY,
    body TEXT NOT NULL,
    hash TEXT NOT NULL,
    user_id INTEGER NOT NULL,
    received_time INTEGER NOT NULL,
    height INTEGER NOT NULL GENERATED ALWAYS AS (json_extract(body, '$.header.height')) STORED,
    sent_time INTEGER NOT NULL GENERATED ALWAYS AS (json_extract(body, '$.header.sent_time_ms')) STORED,
    nonce TEXT NOT NULL GENERATED ALWAYS AS (substr(json_extract(body, '$.header.unsigned.signature'), 0, 64)) STORED,
    FOREIGN KEY(user_id) references users(user_id),
    UNIQUE(received_time, body, user_id)
);

CREATE TABLE IF NOT EXISTS hidden_services (service_id INTEGER PRIMARY KEY, service_url TEXT NOT NULL, port INTEGER NOT NULL, UNIQUE(service_url, port));

CREATE TABLE IF NOT EXISTS private_keys
(key_id INTEGER PRIMARY KEY,
    public_key TEXT UNIQUE,
    private_key TEXT UNIQUE);

CREATE TABLE IF NOT EXISTS message_nonces (
    nonce_id INTEGER PRIMARY KEY,
    key_id INTEGER,
    private_key TEXT,
    public_key TEXT,
    FOREIGN KEY(key_id) REFERENCES private_keys(key_id),
    UNIQUE(key_id, private_key, public_key)
);

PRAGMA journal_mode=WAL;
