SELECT
    G.occurrence_group_id
FROM
    occurrence_group G
WHERE
    G.occurrence_group_key = :group_key
LIMIT 1