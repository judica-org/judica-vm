SELECT
    M.body,
    M.user_id,
    max(M.height)
FROM
    messages M
    INNER JOIN users U ON U.user_id = M.user_id
    INNER JOIN private_keys K ON K.public_key = U.key
GROUP BY
    U.user_id