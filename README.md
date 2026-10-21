# SastQL

Static analysis (SAST) for SQL. **SastQL** parses your `.sql` files into a real AST and flags dangerous or non-portable patterns,  before they reach your database.

Not regex on strings: every rule runs on a typed AST built by [`sqlparser-rs`](https://github.com/apache/datafusion-sqlparser-rs), with full dialect awareness for **PostgreSQL** and **SQLite**.

## Why

Most SQL bugs are boring and static: a `DELETE` without `WHERE`, a `SELECT *` that breaks when a column is added, a `NOT IN` subquery that silently returns nothing because of a `NULL`. Linters exist for code, rarely for SQL. SastQL is that linter.

## Features (v0.2)

| Rule | Id | PostgreSQL | SQLite | Default severity |
|---|---|:---:|:---:|---|
| `SELECT *` usage | `select-star` | ✅ | ✅ | warning |
| `UPDATE` / `DELETE` without `WHERE` | `no-where-on-mutation` | ✅ | ✅ | error |
| `SELECT` without `LIMIT` (opt-in) | `missing-limit` | ✅ | ✅ | info |
| `NOT IN` over a subquery (NULL trap) | `not-in-null-trap` | ✅ | ✅ | warning |
| Comparison with `NULL` (`=`, `!=`, `<>` never match) | `null-comparison` | ✅ | ✅ | warning |
| Functions / operators that don't exist in the target dialect | `dialect-portability` | ✅ | ✅ | warning |
| Column types with surprising semantics (e.g. `DATETIME` → NUMERIC affinity in SQLite) | `type-affinity` | — | ✅ | warning |
| `CASE` without `ELSE` (silent `NULL`) | `case-without-else` | ✅ | ✅ | warning |
| `JOIN` without `ON`/`USING`, comma-separated `FROM` (cartesian product) | `join-without-on` | ✅ | ✅ | warning |
| `NATURAL JOIN` (columns matched by name) | `natural-join` | ✅ | ✅ | warning |
| `INSERT` without a column list | `insert-without-columns` | ✅ | ✅ | warning |
| `OFFSET` without `LIMIT` | `offset-without-limit` | ✅ | ✅ | warning |
| `ORDER BY random()` | `order-by-random` | ✅ | ✅ | warning |
| `LIKE` pattern starting with `%`/`_` (no index) | `leading-wildcard-like` | ✅ | ✅ | warning |
| `+` on strings in SQLite (coerces to number) | `string-plus` | — | ✅ | warning |
| `md5()` / `sha1()` (broken hashes) | `weak-hash` | ✅ | ✅ | warning |
| `SELECT *, col` (wildcard + explicit columns) | `select-star-plus` | ✅ | ✅ | warning |
| `GRANT ALL PRIVILEGES` | `grant-all` | ✅ | ✅ | warning |
| `CREATE TABLE` without `PRIMARY KEY` | `table-no-pk` | ✅ | ✅ | warning |
| Money-like column typed `REAL`/`FLOAT`/`DOUBLE` | `float-money` | ✅ | ✅ | warning |
| `UNION` without `ALL` (hidden dedup sort, opt-in) | `union-without-all` | ✅ | ✅ | warning (opt-in) |
| `ORDER BY 1` (positional, opt-in) | `order-by-ordinal` | ✅ | ✅ | warning (opt-in) |
| `TIMESTAMP` without time zone (opt-in) | `naive-timestamp` | ✅ | — | warning (opt-in) |
| `LIMIT`/`FETCH` without `ORDER BY` (opt-in) | `limit-without-order-by` | ✅ | ✅ | warning (opt-in) |
| `DROP TABLE`/`TRUNCATE`/`DROP COLUMN` (opt-in) | `destructive-ddl` | ✅ | ✅ | warning (opt-in) |

## Install

```bash
# Any Node.js project — no Rust toolchain needed
npx "@clv.wp/sastql" check migrations/ --dialect postgres    # or: pnpm dlx / bunx

# As a dev dependency (adds the unscoped `sastql` binary to node_modules/.bin)
npm install --save-dev "@clv.wp/sastql"

# From source
cargo install --git https://github.com/CLVwp/SastQL
```

> The quotes matter in **PowerShell** — `@clv.wp` collides with its `@` splatting operator. They are harmless in cmd, Git Bash and sh, so copy-paste as-is.

Prebuilt binaries for Windows x64, Linux x64/arm64 (glibc & musl) and macOS x64/arm64 are shipped as per-platform npm packages; npm installs only the one matching your machine.

## Usage

```bash
# Analyze a directory for PostgreSQL
sastql check migrations/ --dialect postgres

# Analyze one file for SQLite, JSON output (CI-friendly)
sastql check schema.sql --dialect sqlite --format json

# SARIF 2.1.0 for GitHub code scanning (see the GitHub Action below)
sastql check migrations/ --format sarif > sastql.sarif

# Run only specific rules
sastql check queries/ --dialect postgres --rules select-star,no-where-on-mutation

# Opt-in rules only run when listed, e.g. missing-limit
sastql check queries/ --dialect postgres --rules missing-limit

# Scan app source (Rust / TS / JS / Python) for injection-prone SQL concatenation
sastql check src/ --check-app --dialect postgres
```

`--check-app` scans SQL strings inside application source instead of `.sql` files. It lexes only the host language's string literals, comments and `+` tokens — never SQL syntax: whether a candidate string is SQL is decided the same way as everywhere else in SastQL, by parsing the assembled candidate with the real sqlparser. Strings joined by `+` chains or carrying interpolation holes (`${…}` templates, f-strings, `format!` braces) are flagged as `sql-concatenation` (error) when that candidate parses as SQL; parameterized queries are never flagged.

Exit codes: `0` clean · `1` violations found · `2` parse, config or IO error. Designed to drop straight into CI.

Example output (real, from the fixtures below — each violation points at the line of its offending statement):

```
tests/fixtures/dirty\ddl.sql:2:1: warning type-affinity: column created is declared DATETIME which gets NUMERIC affinity in SQLite; declare TEXT for ISO-8601 strings
    CREATE TABLE events (id INTEGER, created DATETIME)
tests/fixtures/dirty\ddl.sql:2:1: warning table-no-pk: table events has no PRIMARY KEY; declare one so rows have a stable identity
    CREATE TABLE events (id INTEGER, created DATETIME)
tests/fixtures/dirty\mutations.sql:2:1: warning select-star: SELECT * selects every column; list them explicitly
    SELECT * FROM users
tests/fixtures/dirty\mutations.sql:4:1: error no-where-on-mutation: DELETE from sessions has no WHERE clause; every row is affected
    DELETE FROM sessions
tests/fixtures/dirty\null_comparison.sql:2:1: warning null-comparison: comparison with NULL is never true (=, !=, <> against NULL yield UNKNOWN); use IS NULL / IS NOT NULL
    UPDATE users SET locked = true WHERE last_login = NULL
tests/fixtures/dirty\portability.sql:3:1: warning not-in-null-trap: NOT IN over a subquery returns no rows if the subquery yields NULL; use NOT EXISTS
    SELECT id FROM users WHERE id NOT IN (SELECT manager_id FROM users)
tests/fixtures/dirty\portability.sql:5:1: warning dialect-portability: SQLite has no NOW(); use datetime('now')
    UPDATE users SET seen_at = NOW() WHERE id = 1

4 file(s) scanned, 7 violation(s)
```

When a position cannot be established honestly, the violation carries no position at all (`file: severity rule-id: …`, `"line": null` in JSON) — never a wrong one.

## Configuration

A project can ship a `sastql.toml` for per-rule severity overrides and named rule sets:

```toml
[severity]
select-star = "warning"
float-money = "info"

[sets.ci]
rules = ["select-star", "no-where-on-mutation"]
```

- The file is auto-discovered as `sastql.toml` in the **current directory**, or passed explicitly with `--config <path>`. No file = default behavior, never an error.
- `--rules` accepts rule ids **or** set names, mixed freely: `--rules ci,no-where-on-mutation`.
- Fail loudly: an unknown rule id, set member, severity string or key is a config error — exit `2` with the offending entry named.
- `sql-concatenation` (the `--check-app` finding) is **not** part of the rules registry, so it cannot be selected with `--rules` nor severity-overridden in `sastql.toml` — an override would fail validation as an unknown rule id.

## GitHub Action

A ready-made composite action ships at the repo root ([action.yml](action.yml)): it runs the scan through npx and uploads the SARIF report to GitHub code scanning — which also renders the violations as inline PR annotations.

```yaml
- uses: CLVwp/SastQL@main   # pin to a release tag (e.g. @v0.3) once one exists
  with:
    paths: migrations/ tests/        # required
    dialect: postgres                # default: postgres
    # rules: select-star,no-where-on-mutation   # optional — default: all default rules
    # fail-on-violation: true                   # default: true — fails the job when violations were found
```

Requires the `security-events: write` permission in the calling workflow (code scanning upload).

## Try the demo

The repo ships the fixtures the integration tests use — one clean set, one dirty set, and a realistic simulated database:

```
tests/fixtures/clean/      clean on both dialects
tests/fixtures/dirty/      mutations.sql · portability.sql · ddl.sql · null_comparison.sql
tests/fixtures/realistic/  simulated shop DB: schema · seed · volume queries · deliberate bugs
```

```bash
# Dirty set: 7 violations, exit code 1
cargo run -- check tests/fixtures/dirty --dialect sqlite

# Same rules, Postgres target: SQLite-only rules stay silent, so fewer violations
cargo run -- check tests/fixtures/dirty --dialect postgres

# Clean set: 0 violations, exit code 0
cargo run -- check tests/fixtures/clean --dialect postgres

# Realistic set (simulated shop DB): 26 violations on SQLite, 23 on Postgres —
# a detailed schema, varied data formats and volume, plus deliberate bugs
cargo run -- check tests/fixtures/realistic --dialect sqlite

# Try the opt-in rule on the dirty set: 2 violations
cargo run -- check tests/fixtures/dirty --dialect sqlite --rules missing-limit
```

## Testing

```bash
cargo test                    # unit tests (one per rule, in its file) + end-to-end tests
cargo clippy -- -D warnings
cargo fmt --check
```

Per the conventions in [CLAUDE.md](CLAUDE.md): each rule has positive and negative unit tests; every rule that behaves differently per dialect is tested for each dialect; the integration tests in `tests/integration.rs` run the real binary over `tests/fixtures/` (parse → analyze → report) and assert on exit codes, rule ids and violation counts for both dialects. The CLI's fail-loudly contract is pinned end-to-end too: parse errors, missing paths and non-SQL input exit `2` with the message on stderr, and the parser layer has its own unit tests (BOM stripping, dialect leniency, invalid SQL, statement line/column recovery and its honest-degradation gate).

## Design

- **Parser**: `sqlparser-rs`, one dialect per run (`PostgreSqlDialect` / `SQLiteDialect`). Unparseable SQL is a parse error (exit `2`), never silently skipped.
- **Rules**: a single `Rule` trait over the AST. One file per rule, registered in one place.
- **Output**: human-readable by default, `--format json` / `--format sarif` for machines.

Full feature mapping and UML diagrams: [docs/UML.md](docs/UML.md).

## Roadmap

v0.2 ships 25 rules — the table above — all implemented and tested. Next, in order:

| # | Item | What it means |
|---|---|---|
| 1 | **Precise line/column** — ✅ Shipped | Violations carry the statement-level line/column of the offending statement's first token (`file:line:col`, `line`/`column` in JSON), degrading honestly to no position when it cannot be established. Statement-level is the ceiling: AST nodes carry no spans in sqlparser 0.63. |
| 2 | **`sastql.toml`** — ✅ Shipped | Per-rule severity overrides and named rule sets in `sastql.toml` (auto-discovered or `--config`); `--rules` accepts set names. Invalid config = exit `2`; missing config = never an error. |
| 3 | **SARIF output + GitHub Action** — ✅ Shipped | `--format sarif` (SARIF 2.1.0, severity mapped to SARIF levels with info → note, positions degrading honestly) for code scanning, plus a ready-made composite `action.yml`: npx scan → SARIF upload → inline PR annotations. |
| 4 | **`--check-app`** — ✅ Shipped | Scans SQL strings inside Rust/TS/JS/Python source for injection-prone concatenation (`sql-concatenation`): host-language string-literal lexing only, `+` chains and adjacent literals grouped, `${…}`/f-string/`format!` braces counted as interpolation holes, and SQL-ness validated exclusively by the real sqlparser — parameterized queries are never flagged. |

## License

Copyright © 2026 Clément Viellard, all rights reserved.

SastQL is **source-available** - deliberately **not** open source. The code is public to read, learn from and contribute to, but it is not offered under an open-source license. See [LICENSE](LICENSE) (English) and [LICENSE.fr](LICENSE.fr) (French, authoritative version):

- **Free** for personal, educational and research use.
- **Any commercial use requires the Author's prior written permission**, with financial terms set at the Author's sole discretion.
- **No redistribution, no published forks, no derivative works** without permission - public forks are tolerated solely to propose contributions back.
- Contributions transfer full perpetual rights to the Author.
- Want to contribute? Read [CONTRIBUTING.md](CONTRIBUTING.md) first — PR rules and the fork policy are spelled out there.
