WITH RECURSIVE updatable(
    row_id,
    message_hash,
    prev_hash,
    is_connected,
    height,
    parent_id
) AS (
    SELECT
        M.message_id,
        M.hash,
        M.prev_msg,
        M.connected,
        M.height,
        Parent.message_id
    FROM
        messages M
        INNER JOIN messages Parent ON Parent.hash = M.prev_msg
        AND Parent.connected
    WHERE
        M.connected = 0
        /*
         -- Do a regular union so that we don't traverse more than once per entry
         */
    UNION
    /*
     
     */
    SELECT
        AsChild.message_id,
        AsChild.hash,
        AsChild.prev_msg,
        AsChild.connected,
        AsChild.height,
        AsParent.row_id
    FROM
        updatable AsParent
        INNER JOIN messages AsChild ON AsParent.message_hash = AsChild.prev_msg
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
    connected = 1,
    prev_msg_id = U.parent_id
FROM
    updatable U
WHERE
    U.row_id = messages.message_id
--    AND NOT U.is_connected