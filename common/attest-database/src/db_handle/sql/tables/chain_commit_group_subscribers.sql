CREATE TABLE IF NOT EXISTS chain_commit_group_subscribers (
    group_id INTEGER NOT NULL,
    member_id INTEGER NOT NULL,
    FOREIGN KEY (group_id) REFERENCES chain_commit_groups(group_id),
    -- Should Reference genesis_id column in queries
    FOREIGN KEY (member_id) REFERENCES messages(message_id),
    UNIQUE (group_id, member_id)
);