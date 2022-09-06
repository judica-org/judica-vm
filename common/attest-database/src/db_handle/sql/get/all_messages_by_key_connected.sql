SELECT
        (messages.body)
FROM
        messages
        INNER JOIN users ON messages.user_id = users.user_id
WHERE
        users.key = ?
        AND messages.connected
ORDER BY
        messages.height ASC;