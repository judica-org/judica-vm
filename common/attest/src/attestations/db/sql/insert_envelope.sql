INSERT INTO
    messages (body, hash, user_id, received_time)
VALUES
    (
        ?,
        ?,
        (
            SELECT
                user_id
            FROM
                users
            WHERE
                key = ?
        ),
        ?
    )