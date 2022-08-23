WITH RECURSIVE updatable(mid, prev, conn, height) AS (
    SELECT
        M.message_id,
        M.prev_msg_id,
        M.connected,
        M.height
    from
        messages M
        INNER JOIN messages Parent ON Parent.message_id = M.prev_msg_id
    WHERE
        M.connected = 0
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
        INNER JOIN messages UParent ON UParent.message_id = U.prev
    WHERE
        U.conn = 0
        AND UParent.connected
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