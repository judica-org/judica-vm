INSERT INTO
    chain_commit_group_members(member_id, group_id)
VALUES
    (
        (
            SELECT
                genesis_id
            FROM
                messages
            WHERE
                genesis = :genesis_hash
            LIMIT
                1
        ), :group_id
    )