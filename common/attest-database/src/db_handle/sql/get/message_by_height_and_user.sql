SELECT
    M.body
FROM
    messages M
    INNER JOIN users U on U.user_id = M.user_id
WHERE
    U.key = :key
    AND M.height = :height
LIMIT
    1