CREATE TABLE IF NOT EXISTS occurrence(
    occurrence_id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    occurrence_data TEXT NOT NULL,
    occurrence_time INTEGER NOT NULL,
    occurrence_typeid TEXT NOT NULL,
    occurrence_group_id INTEGER NOT NULL,
    occurrence_unique_tag TEXT NULL,
    FOREIGN KEY (occurrence_group_id) REFERENCES occurrence_group(occurrence_group_id)
    UNIQUE(occurrence_group_id, occurrence_unique_tag)
);