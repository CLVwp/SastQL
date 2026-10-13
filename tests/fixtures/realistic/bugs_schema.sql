-- Deliberate schema bugs. Default-rule counts pinned in tests/integration.rs:
--   table-no-pk x1 (temp_cache)
--   float-money x3 (invoices: unit_price, grand_total, fee_rate)
--   type-affinity x2 on SQLite (import_batches: DATETIME + TIMESTAMP)
--   grant-all x1 (GRANT ALL PRIVILEGES)
-- Opt-in: destructive-ddl x3 (DROP TABLE / TRUNCATE / DROP COLUMN),
--         naive-timestamp x1 on PostgreSQL (loaded_at TIMESTAMP).

CREATE TABLE temp_cache (key TEXT, value TEXT);

CREATE TABLE invoices (
    id INTEGER PRIMARY KEY,
    number TEXT,
    unit_price REAL,
    grand_total DOUBLE PRECISION,
    fee_rate FLOAT,
    issued_on TEXT
);

CREATE TABLE import_batches (id INTEGER PRIMARY KEY, created DATETIME, loaded_at TIMESTAMP);

GRANT ALL PRIVILEGES ON orders TO reporting_app;

DROP TABLE legacy_users;
TRUNCATE TABLE audit_log;
ALTER TABLE sessions DROP COLUMN token;
