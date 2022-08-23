SELECT
    messages.body
FROM
    messages
WHERE
    user_id = (
        SELECT
            user_id
        from
            users
        where
            key = ?
    )
    AND height = ?