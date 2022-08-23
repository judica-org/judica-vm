SELECT
    M.body,
    max(M.height)
FROM
    messages M
WHERE
    M.connected = 1
GROUP BY
    M.user_id