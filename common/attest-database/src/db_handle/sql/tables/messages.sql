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
            height = 0
            OR (genesis_id IS NOT NULL)
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
                hash
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