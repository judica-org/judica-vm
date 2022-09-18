INSERT INTO
    occurrence(
        occurrence_data,
        occurrence_time,
        occurrence_typeid,
        occurrence_group_id
    )
VALUES
    (:data, :time, :typeid, :group_id)