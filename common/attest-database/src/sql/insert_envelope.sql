INSERT INTO
    messages (body, hash, user_id, received_time)
VALUES
    (
        :body,
        :hash,
        (
            SELECT
                user_id
            FROM
                users
            WHERE
                key = :key
        ),
        :received_time
    )