SELECT
    M.body,
    M.user_id,
    min(M.height)
FROM
    messages M
WHERE
    M.height > 0
    AND M.connected = 0
GROUP BY
    M.genesis_id