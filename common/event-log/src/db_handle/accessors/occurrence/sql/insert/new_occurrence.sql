INSERT INTO
    occurrence(
        occurrence_data,
        occurrence_time,
        occurrence_typeid,
        occurrence_group_id,
        occurrence_unique_tag
    )
VALUES
    (:data, :time, :typeid, :group_id, :unique_tag)