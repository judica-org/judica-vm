CREATE TABLE IF NOT EXISTS hidden_services (
    service_id INTEGER PRIMARY KEY,
    service_url TEXT NOT NULL,
    port INTEGER NOT NULL,
    fetch_from BOOLEAN NOT NULL,
    push_to BOOLEAN NOT NULL,
    allow_unsolicited_tips BOOLEAN NOT NULL,
    UNIQUE(service_url, port)
);