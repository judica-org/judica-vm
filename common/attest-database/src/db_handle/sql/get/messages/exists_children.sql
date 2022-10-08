SELECT
    1
FROM
    messages M
WHERE
    M.prev_msg = :prev_msg
LIMIT
    1