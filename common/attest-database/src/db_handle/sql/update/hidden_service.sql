INSERT INTO
    hidden_services (service_url, port, fetch_from, push_to)
VALUES
    (
        :service_url,
        :port,
        IFNULL(:fetch_from, 0),
        IFNULL(:push_to, 0)
    ) ON CONFLICT DO
SET
    (fetch_from, push_to) = (
        IFNULL(:fetch_from, fetch_from),
        IFNULL(:push_to, push_to)
    )