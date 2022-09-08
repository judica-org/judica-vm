SELECT
    group_id, name
FROM
    chain_commit_group_members Groups
    INNER JOIN messages Messages ON Messages.hash = :genesis_hash
    AND Messages.height = 0
    AND Groups.member_id = Messages.message_id