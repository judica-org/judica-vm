SELECT
    body
from
    messages
WHERE
    nonce in (
        SELECT
            nonce
        FROM
            messages
        GROUP BY
            nonce,
            user_id
        HAVING
            COUNT(nonce) > 1
    )