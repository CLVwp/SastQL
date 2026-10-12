-- sqlite only: type-affinity flags DATETIME (NUMERIC affinity). Postgres: clean.
CREATE TABLE events (id INTEGER, created DATETIME);
