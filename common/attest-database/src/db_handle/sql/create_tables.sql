PRAGMA foreign_keys = ON;

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
    genesis_id INTEGER,
    height INTEGER NOT NULL,
    sent_time INTEGER NOT NULL,
    prev_msg TEXT NOT NULL,
    genesis TEXT NOT NULL,
    nonce TEXT NOT NULL,
    connected BOOLEAN NOT NULL,
    FOREIGN KEY(genesis_id) references messages(message_id) ON DELETE CASCADE,
    FOREIGN KEY(user_id) references users(user_id),
    FOREIGN KEY(prev_msg_id) references messages(message_id) ON DELETE
    SET
        NULL,
        UNIQUE(hash),
        CHECK(
            genesis_id IS NOT NULL
            OR height = 0
        ),
        CHECK(
            (
                connected
                AND prev_msg_id IS NOT NULL
                AND genesis_id IS NOT NULL
            )
            OR NOT connected
            OR (
                height = 0
                AND connected
            )
        ),
        -- only paid attention to for the genesis column
        CHECK(json_valid(body)),
        CHECK(
            height > 0
            OR (
                height = 0
                AND prev_msg = "0000000000000000000000000000000000000000000000000000000000000000"
            )
        ),
        CHECK(
            IFNULL(
                json(body) ->> '$.header.ancestors.genesis',
                genesis
            ) = genesis
        ),
        CHECK(
            IFNULL(
                json(body) ->> '$.header.ancestors.prev_msg',
                prev_msg
            ) = prev_msg
        ),
        CHECK(
            (json(body) ->> '$.header.height') = height
        ),
        CHECK(
            (json(body) ->> '$.header.sent_time_ms') = sent_time
        ),
        CHECK(
            substr(
                json(body) ->> '$.header.nonce',
                0,
                64
            ) = nonce
        )
);

/* When the new incoming message has a disconnected child,
 we update that child to have it's prev_msg_id set to match.
 N.B. the select should have a AND M.gensis_id IS NULL, but this must always be
 true so we don't need it.
 */
CREATE TRIGGER IF NOT EXISTS message_parents
AFTER
INSERT
    ON messages -- when there are messages who think this is their parent message
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
    fetch_from BOOLEAN NOT NULL,
    push_to BOOLEAN NOT NULL,
    allow_unsolicited_tips BOOLEAN NOT NULL,
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