SELECT
    user_id,
    nickname
FROM
    users
WHERE
    key = ?
LIMIT
    1