SELECT
    M.body,
    max(M.height)
FROM
    messages M
    INNER JOIN users U ON U.user_id = M.user_id
    INNER JOIN private_keys K ON K.public_key = U.key
WHERE
    M.connected
GROUP BY
    M.user_id