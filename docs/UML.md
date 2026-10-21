# SastQL — Feature Mapping & UML

Complete feature mapping for SastQL v0.2. Target: analyze `.sql` files with dialect awareness for **PostgreSQL** and **SQLite**.

Each diagram lives in its own `.puml` file under [`docs/uml/`](uml/). To render: open the file in VS Code / IntelliJ with the PlantUML extension and hit **Alt+D** (or `plantuml <file>` on the CLI). Layout uses the built-in `smetana` engine — no Graphviz install needed.

| # | Diagram | File |
|---|---|---|
| 1 | Use cases (scope) | [`use-cases.puml`](uml/use-cases.puml) |
| 2 | Component flow | [`components.puml`](uml/components.puml) |
| 3 | Class diagram | [`classes.puml`](uml/classes.puml) |
| 4 | Sequence — `check` command | [`sequence-check.puml`](uml/sequence-check.puml) |
| 5 | State — analysis pipeline | [`pipeline-state.puml`](uml/pipeline-state.puml) |

Out of scope for v0.2 (see roadmap in README): scanning SQL strings inside application source, SARIF output, autofix. Config file (`sastql.toml`: severity overrides + named rule sets) shipped since — see the README's Configuration section.

## Feature matrix — rules per dialect

| Rule | What it detects | PostgreSQL | SQLite | Severity |
|---|---|:---:|:---:|---|
| `select-star` | `SELECT *` (and `SELECT table.*`) | ✅ | ✅ | warning |
| `no-where-on-mutation` | `UPDATE`/`DELETE` with no `WHERE` clause | ✅ | ✅ | error |
| `missing-limit` | bare `SELECT` without `LIMIT` (opt-in via `--rules`) | ✅ | ✅ | info |
| `not-in-null-trap` | `expr NOT IN (subquery)` — returns no rows if subquery yields a `NULL` | ✅ | ✅ | warning |
| `null-comparison` | comparing with `NULL` via `=` / `!=` / `<>` — never true (three-valued logic); use `IS [NOT] NULL` | ✅ | ✅ | warning |
| `dialect-portability` | functions/operators absent from target dialect (e.g. `NOW()` in SQLite, `::` cast in SQLite, `CONCAT()` pre-3.44) | ✅ | ✅ | warning |
| `type-affinity` | DDL types with surprising semantics (`DATETIME` → NUMERIC affinity in SQLite; non-existent types in PG) | — | ✅ | warning |
| `case-without-else` | `CASE` without `ELSE` — yields `NULL` when no condition matches | ✅ | ✅ | warning |
| `join-without-on` | `JOIN` without `ON`/`USING`, comma-separated `FROM` — cartesian product | ✅ | ✅ | warning |
| `natural-join` | `NATURAL JOIN` — columns matched by name, schema changes silently alter the join | ✅ | ✅ | warning |
| `insert-without-columns` | `INSERT` without a column list — values bind to the current column order | ✅ | ✅ | warning |
| `offset-without-limit` | `OFFSET` without `LIMIT` — skips rows with no upper bound | ✅ | ✅ | warning |
| `order-by-random` | `ORDER BY random()` — sorts the whole table to sample rows | ✅ | ✅ | warning |
| `leading-wildcard-like` | `LIKE` pattern starting with `%`/`_` — index cannot be used | ✅ | ✅ | warning |
| `string-plus` | `+` on strings in SQLite — coerces them to numbers; `||` concatenates | — | ✅ | warning |
| `weak-hash` | `md5()`/`sha1()` — broken hashes (collisions, GPU brute-force) | ✅ | ✅ | warning |
| `select-star-plus` | `SELECT *, col` — wildcard mixed with explicit columns | ✅ | ✅ | warning |
| `grant-all` | `GRANT ALL [PRIVILEGES]` — hands out every privilege | ✅ | ✅ | warning |
| `table-no-pk` | `CREATE TABLE` without `PRIMARY KEY` (column- or table-level) | ✅ | ✅ | warning |
| `float-money` | money-like column (`price`, `amount`, `total`, …) typed `REAL`/`FLOAT`/`DOUBLE` | ✅ | ✅ | warning |
| `union-without-all` | `UNION` without `ALL` — hidden dedup sort (opt-in via `--rules`) | ✅ | ✅ | warning (opt-in) |
| `order-by-ordinal` | `ORDER BY 1` — positional reference, fragile (opt-in via `--rules`) | ✅ | ✅ | warning (opt-in) |
| `naive-timestamp` | `TIMESTAMP` without time zone — prefer `TIMESTAMPTZ` (opt-in via `--rules`) | ✅ | — | warning (opt-in) |
| `limit-without-order-by` | `LIMIT`/`FETCH` without `ORDER BY` — arbitrary subset (opt-in via `--rules`) | ✅ | ✅ | warning (opt-in) |
| `destructive-ddl` | `DROP TABLE`/`SCHEMA`/`DATABASE`, `TRUNCATE`, `ALTER TABLE DROP COLUMN` (opt-in via `--rules`) | ✅ | ✅ | warning (opt-in) |

Note: output formatting is an enum + `match`, not a trait hierarchy — two formats don't justify polymorphism.

## Error model

| Situation | Behaviour | Exit code |
|---|---|---:|
| All files clean | report "0 violations" | `0` |
| ≥ 1 violation | violations rendered (human or JSON) | `1` |
| Parse error on any file | error to stderr, stop | `2` |
| Path not found / unreadable | error to stderr | `2` |
| Invalid `sastql.toml` (unknown rule id, set member, severity or key) | error to stderr naming the offender | `2` |
