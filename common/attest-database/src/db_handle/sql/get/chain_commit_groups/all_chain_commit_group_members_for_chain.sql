WITH subscribing_to AS (
    SELECT
        group_id
    FROM
        chain_commit_group_subscribers Subscription
        INNER JOIN messages Msg ON Subscription.member_id = Msg.message_id
    WHERE
        Msg.hash = :genesis_hash
        AND Msg.height = 0
)
SELECT
    DISTINCT member_id
FROM
    chain_commit_group_members GroupMember
    INNER JOIN subscribing_to Subscription ON GroupMember.group_id = Subscription.group_id