WITH groups_this_chain_subscribes_to AS (
    SELECT
        Subscription.group_id
    FROM
        users User
        INNER JOIN chain_commit_group_subscribers Subscription
        INNER JOIN messages Msg ON Msg.user_id = User.user_id
        AND Subscription.member_id = Msg.message_id
    WHERE
        User.key = :key
        AND Msg.height = 0
)
SELECT
    Msg.body, Msg.message_id
FROM
    groups_this_chain_subscribes_to Subscription
    INNER JOIN chain_commit_group_members GroupMembers ON GroupMembers.group_id = Subscription.group_id
    INNER JOIN messages Msg ON (
        GroupMembers.member_id = Msg.message_id
        OR GroupMembers.member_id = Msg.genesis_id
    )
WHERE
    Msg.message_id > :after