SELECT
    M_Outer.body
FROM
    messages M_Outer
WHERE
    (M_Outer.nonce, M_Outer.user_id) in (
        SELECT
            M.nonce,
            M.user_id
        FROM
            messages M
        GROUP BY
            M.nonce,
            M.user_id
        HAVING
            COUNT(*) > 1
    )