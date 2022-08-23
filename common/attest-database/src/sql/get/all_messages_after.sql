SELECT
    M.body,
    M.message_id,
FROM
    messages M
where
    M.message_id > ?