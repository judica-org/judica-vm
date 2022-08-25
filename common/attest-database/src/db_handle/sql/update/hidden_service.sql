INSERT INTO
    hidden_services (service_url, port, fetch_from, push_to)
VALUES
    (
        :service_url,
        :port,
        IFNULL(:fetch_from, 0),
        IFNULL(:push_to, 0)
    ) ON CONFLICT DO
UPDATE
SET
    fetch_from = IFNULL(:fetch_from, fetch_from),
    push_to = IFNULL(:push_to, push_to)