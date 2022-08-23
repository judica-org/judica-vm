WITH RECURSIVE updatable(mid, prev, conn) AS (
    SELECT
        M.message_id,
        M.prev_msg_id,
        M.connected
    from
        messages M
    WHERE
        M.connected = 0
        AND M.prev_msg_id IS NOT NULL
        AND (
            SELECT
                X.connected
            from
                messages X
            where
                X.message_id = M.prev_msg_id
            LIMIT
                1
        ) --
        --
    UNION
    ALL --
    --
    SELECT
        U.mid,
        U.prev,
        U.conn
    FROM
        updatable U
    WHERE
        U.prev IS NOT NULL
        AND U.conn = 0
        AND IFNULL(
            (
                SELECT
                    X2.connected
                FROM
                    messages X2
                WHERE
                    X2.message_id = U.prev
            ),
            0
        ) = 1
    LIMIT
        1000
)
UPDATE
    messages
SET
    connected = 1
FROM
    updatable U
WHERE
    U.mid = messages.message_id
LIMIT
    -1