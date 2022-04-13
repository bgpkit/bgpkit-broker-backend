-- This file should undo anything in `up.sql`

DROP MATERIALIZED VIEW IF EXISTS latest_times;
DROP TABLE IF EXISTS items;
DROP TABLE IF EXISTS collectors;
