INSERT INTO
    message_nonces (key_id, public_key, private_key)
VALUES
    (
        (
            SELECT
                key_id
            FROM
                private_keys
            WHERE
                public_key = ?
        ),
        ?,
        ?
    )