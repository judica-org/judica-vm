INSERT INTO
    messages (
        body,
        hash,
        received_time,
        user_id,
        prev_msg_id,
        genesis_id,
        connected
    )
VALUES
    (
        :body,
        :hash,
        :received_time,
        (
            SELECT
                U.user_id
            FROM
                users U
            WHERE
                U.key = :key
            LIMIT
                1
        ), (
            SELECT
                M.message_id
            FROM
                messages M
            WHERE
                M.hash = json_extract(:body, "$.header.prev_msg")
            LIMIT
                1
        ), (
            SELECT
                M.message_id
            FROM
                messages M
            WHERE
                M.hash = json_extract(:body, "$.header.genesis")
            LIMIT
                1
        ), (
            SELECT
                IFNULL(
                    (
                        SELECT
                            connected
                        FROM
                            messages M
                        WHERE
                            M.hash = json_extract(:body, "$.header.prev_msg")
                    ),
                    json_extract(:body, "$.header.height") = 0
                )
        )
    )