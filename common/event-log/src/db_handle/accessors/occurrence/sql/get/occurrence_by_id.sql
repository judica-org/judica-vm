SELECT
    O.occurrence_data,
    O.occurrence_time,
    O.occurrence_typeid,
    O.occurrence_unique_tag
FROM
    occurrence O
WHERE
    O.occurrence_id = :id