UPDATE
    messages as Mu
SET
    prev_msg_id = (
        SELECT
            M.message_id
        from
            messages M
        WHERE
            M.hash = Mu.prev_msg
    )
WHERE
    connected = 0
    AND prev_msg_id IS NULL
    AND EXISTS(
        SELECT
            1
        from
            messages M
        where
            M.hash = Mu.prev_msg
        LIMIT
            1
    )
LIMIT
    :limit