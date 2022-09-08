SELECT
    (private_key)
FROM
    message_nonces
where
    public_key = ?