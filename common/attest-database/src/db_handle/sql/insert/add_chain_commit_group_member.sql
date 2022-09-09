INSERT INTO
    chain_commit_group_members(member_id, group_id)
VALUES
    (
        (
            SELECT
                M.message_id
            FROM
                messages M
            WHERE
                M.hash = :genesis_hash
            LIMIT
                1
        ), :group_id
    )