-- Your SQL goes here

CREATE TABLE IF NOT EXISTS collectors
(
    id text NOT NULL,
    project text NOT NULL,
    url text NOT NULL,
    CONSTRAINT collectors_pkey PRIMARY KEY (id)
);


CREATE TABLE IF NOT EXISTS items
(
    ts_start bigint NOT NULL,
    ts_end bigint NOT NULL,
    collector_id text NOT NULL,
    data_type text NOT NULL,
    url text NOT NULL,
    file_size bigint NOT NULL,
    CONSTRAINT items_pkey PRIMARY KEY (url),
    CONSTRAINT items_collector_id_fkey FOREIGN KEY (collector_id)
        REFERENCES collectors (id) MATCH SIMPLE
        ON UPDATE NO ACTION
        ON DELETE NO ACTION
);

CREATE MATERIALIZED VIEW IF NOT EXISTS latest_times
AS
SELECT items.ts_start AS "timestamp",
       items.collector_id,
       items.data_type,
       collectors.project,
       collectors.url AS collector_url,
       items.url AS item_url
FROM ( SELECT max(items_1.ts_start) AS ts_start,
              items_1.collector_id,
              items_1.data_type
       FROM items items_1
       GROUP BY items_1.collector_id, items_1.data_type) nested
         JOIN collectors ON nested.collector_id = collectors.id
         JOIN items ON nested.ts_start = items.ts_start AND nested.collector_id = items.collector_id AND items.data_type = nested.data_type;

ALTER TABLE IF EXISTS collectors
    OWNER to bgpkit_admin;

ALTER TABLE IF EXISTS items
    OWNER to bgpkit_admin;

ALTER MATERIALIZED VIEW IF EXISTS latest_times
    OWNER to bgpkit_admin;
