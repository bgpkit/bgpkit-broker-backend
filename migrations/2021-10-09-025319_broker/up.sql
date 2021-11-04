-- Your SQL goes here
CREATE TABLE IF NOT EXISTS collectors (
    id TEXT PRIMARY KEY,
    project TEXT NOT NULL,
    url TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS items (
    collector_id TEXT NOT NULL,
    timestamp BIGINT,
    data_type TEXT NOT NULL,
    url TEXT NOT NULL,
    FOREIGN KEY(collector_id) REFERENCES  collectors(id),
    PRIMARY KEY (timestamp, collector_id, data_type)
);

-- View: public.latest_times

-- DROP VIEW public.latest_times;

CREATE OR REPLACE VIEW latest_times
AS
SELECT items."timestamp",
       items.collector_id,
       items.data_type,
       collectors.project,
       collectors.url AS collector_url,
       items.url AS item_url
FROM ( SELECT max(items_1."timestamp") AS "timestamp",
              items_1.collector_id,
              items_1.data_type
       FROM items items_1
       GROUP BY items_1.collector_id, items_1.data_type) nested
         JOIN collectors ON nested.collector_id = collectors.id
         JOIN items ON nested."timestamp" = items."timestamp" AND nested.collector_id = items.collector_id AND items.data_type = nested.data_type;