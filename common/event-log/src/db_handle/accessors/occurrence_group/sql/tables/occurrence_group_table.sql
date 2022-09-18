CREATE TABLE IF NOT EXISTS occurrence_group (
    occurrence_group_id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    occurrence_group_key TEXT NOT NULL,
    UNIQUE(occurrence_group_key)
);