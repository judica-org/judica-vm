SELECT
    O.occurrence_data,
    O.occurrence_time,
    O.occurrence_typeid
FROM
    occurrence O
WHERE
    O.occurrence_id > :after_id
    AND O.occurrence_group_id = :group_id