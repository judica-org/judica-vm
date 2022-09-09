SELECT
    CommitGroup.group_id,
    CommitGroup.name
FROM
    messages Msg
    INNER JOIN chain_commit_group_subscribers Subscription ON Subscription.member_id = Msg.message_id
    INNER JOIN chain_commit_groups CommitGroup ON CommitGroup.group_id = Subscription.group_id
WHERE
    Msg.hash = :genesis_hash
    AND Msg.height = 0