UPDATE
    messages as Mu
SET
    connected = 1
WHERE
    connected = 0
    AND EXISTS(
        SELECT
            1
        from
            messages M
        where
            M.connected = 1
            AND M.message_id = Mu.prev_msg_id
        LIMIT
            1
    )
ORDER BY
    height ASC
LIMIT
    IFNULL(
        :limit,
        (
            SELECT
                COUNT(*)
            FROM
                messages
            WHERE
                height = 0
        )
    )