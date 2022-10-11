SELECT
    O.occurrence_id,
    O.occurrence_data,
    O.occurrence_time,
    O.occurrence_typeid,
    O.occurrence_unique_tag
FROM
    occurrence O
WHERE
    O.occurrence_group_id = :group_id
ORDER BY
    O.occurrence_id ASC