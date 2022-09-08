WITH groups_is_in AS (
    SELECT
        group_id
    FROM
        chain_commit_group_members CommitGroup
        INNER JOIN messages Messages ON Messages.hash = :genesis_hash
        AND Messages.height = 0
        AND CommitGroup.member_id = Messages.message_id
)
SELECT
    Messages.body,
    max(Messages.height)
FROM
    chain_commit_group_members GroupMembers
    INNER JOIN groups_is_in InGroups ON GroupMembers.group_id = InGroups.group_id
    INNER JOIN messages Messages ON GroupMembers.member_id = Messages.message_id
WHERE
    Messages.connected
GROUP BY
    Messages.height