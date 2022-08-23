WITH RECURSIVE updatable(mid, prev, conn, height) AS (
    SELECT
        M.message_id,
        M.prev_msg_id,
        M.connected,
        M.height
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
        )
        /*
         -- Do a regular union so that we don't traverse more than once per entry
         */
    UNION
    /*

     */
    SELECT
        U.mid,
        U.prev,
        U.conn,
        U.height
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
        /*

         Order By Height not strictly required because the first query already
         gets just connectable messages

         ORDER BY
         U.height ASC

         */
        /*

         We can safely do LIMIT -1 (unlimited) because we are in a UNION so it is at worst all messages once

         */
    LIMIT
        -1
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