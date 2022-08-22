SELECT
    M.body,
    M.user_id,
    M.height
FROM
    messages M
    INNER JOIN users U ON U.user_id = M.user_id
    INNER JOIN private_keys K ON K.public_key = U.key
-- Filter out only disconnected tips
WHERE EXISTS (SELECT * from messages where hash = M.prev_msg LIMIT 1)
GROUP BY
    U.user_id
