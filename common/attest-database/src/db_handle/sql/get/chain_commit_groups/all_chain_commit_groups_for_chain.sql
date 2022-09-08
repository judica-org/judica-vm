SELECT
    CommitGroup.group_id,
    CommitGroup.name
FROM
    messages Messages
    INNER JOIN chain_commit_group_members GroupMember ON GroupMember.member_id = Messages.message_id
    INNER JOIN chain_commit_groups CommitGroup ON CommitGroup.group_id = GroupMember.group_id
WHERE
    Messages.hash = :genesis_hash
    AND Messages.height = 0