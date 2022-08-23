CREATE TABLE IF NOT EXISTS users (
    user_id INTEGER PRIMARY KEY,
    nickname TEXT,
    key TEXT UNIQUE
);

CREATE TABLE IF NOT EXISTS messages (
    message_id INTEGER PRIMARY KEY AUTOINCREMENT,
    body TEXT NOT NULL,
    hash TEXT NOT NULL,
    user_id INTEGER NOT NULL,
    received_time INTEGER NOT NULL,
    prev_msg_id INTEGER,
    height INTEGER NOT NULL GENERATED ALWAYS AS (json_extract(body, '$.header.height')) STORED,
    sent_time INTEGER NOT NULL GENERATED ALWAYS AS (json_extract(body, '$.header.sent_time_ms')) STORED,
    prev_msg TEXT NOT NULL GENERATED ALWAYS AS (json_extract(body, '$.header.prev_msg')) STORED,
    nonce TEXT NOT NULL GENERATED ALWAYS AS (
        substr(
            json_extract(body, '$.header.unsigned.signature'),
            0,
            64
        )
    ) STORED,
    FOREIGN KEY(user_id) references users(user_id),
    FOREIGN KEY(prev_msg_id) references messages(message_id) ON DELETE
    SET
        NULL,
        UNIQUE(received_time, hash, user_id)
);

/* Connect the new message to it's parent, if the parent exists */
CREATE TRIGGER IF NOT EXISTS message_connectedness
AFTER
INSERT
    ON messages
    WHEN EXISTS (
        SELECT
            M.message_id
        FROM
            messages M
        WHERE
            M.hash = NEW.prev_msg
        LIMIT
            1
    ) BEGIN
UPDATE
    messages
SET
    prev_msg_id = (
        SELECT
            M.message_id
        FROM
            messages M
        WHERE
            M.hash = NEW.prev_msg
        LIMIT
            1
    )
WHERE
    message_id = NEW.message_id;

END;

/* When the new incoming message has a disconnected child,
 we update that child to have it's prev_msg_id set to match.
 
 N.B. the select should have a AND M.gensis_id IS NULL, but this must always be
 true so we don't need it.
 */
CREATE TRIGGER IF NOT EXISTS message_connectedness2
AFTER
INSERT
    ON messages
    WHEN EXISTS (
        SELECT
            *
        FROM
            messages M
        WHERE
            M.prev_msg = NEW.hash
    ) BEGIN
UPDATE
    messages
SET
    prev_msg_id = NEW.message_id
WHERE
    prev_msg = NEW.hash;

END;

CREATE TABLE IF NOT EXISTS hidden_services (
    service_id INTEGER PRIMARY KEY,
    service_url TEXT NOT NULL,
    port INTEGER NOT NULL,
    UNIQUE(service_url, port)
);

CREATE TABLE IF NOT EXISTS private_keys (
    key_id INTEGER PRIMARY KEY,
    public_key TEXT UNIQUE,
    private_key TEXT UNIQUE
);

CREATE TABLE IF NOT EXISTS message_nonces (
    nonce_id INTEGER PRIMARY KEY,
    key_id INTEGER,
    private_key TEXT,
    public_key TEXT,
    FOREIGN KEY(key_id) REFERENCES private_keys(key_id),
    UNIQUE(key_id, private_key, public_key)
);

PRAGMA journal_mode = WAL;