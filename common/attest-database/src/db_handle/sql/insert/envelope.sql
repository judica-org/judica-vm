INSERT INTO
    messages (
        body,
        hash,
        received_time,
        sent_time,
        genesis,
        prev_msg,
        height,
        nonce,
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
        :sent_time,
        :genesis,
        :prev_msg,
        :height,
        :nonce,
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
                M.hash = :prev_msg
            LIMIT
                1
        ), (
            SELECT
                M.message_id
            FROM
                messages M
            WHERE
                M.hash = :genesis
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
                            M.hash = :prev_msg
                    ),
                    :height = 0
                )
        )
    )