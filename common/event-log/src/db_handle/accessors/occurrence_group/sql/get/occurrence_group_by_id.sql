SELECT
    G.occurrence_group_key
FROM
    occurrence_group G
WHERE
    G.occurrence_group_id = :group_id
LIMIT
    1