SELECT
    m.body
FROM
    messages m
    INNER JOIN users u ON m.user_id = u.user_id
WHERE
    m.user_id = (
        SELECT
            user_id
        FROM
            users
        where
            key = ?
    )
ORDER BY
    m.height DESC
LIMIT
    1