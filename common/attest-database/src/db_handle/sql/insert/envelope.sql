INSERT INTO
    messages (
        body,
        hash,
        user_id,
        prev_msg_id,
        genesis_id,
        received_time
    )
VALUES
    (
        :body,
        :hash,
        (
            SELECT
                user_id
            FROM
                users
            WHERE
                key = :key
        ),
        NULL,
        (
            SELECT
                M.message_id
            FROM
                messages M
            WHERE
                M.hash = json_extract(:body, "$.header.genesis")
            LIMIT 1
        ),
        :received_time
    )