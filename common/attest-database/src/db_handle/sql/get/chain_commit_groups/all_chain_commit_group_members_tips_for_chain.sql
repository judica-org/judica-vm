WITH groups_is_in AS (
    SELECT
        CommitGroup.group_id
    FROM
        chain_commit_group_members CommitGroup
        INNER JOIN users Users
        INNER JOIN messages Messages ON Messages.user_id = Users.user_id
        AND CommitGroup.member_id = Messages.message_id
    WHERE
        Messages.height = 0
        AND Users.key = :key
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