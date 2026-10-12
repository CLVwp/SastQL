# SastQL

Static analysis (SAST) for SQL. **SastQL** parses your `.sql` files into a real AST and flags dangerous or non-portable patterns,  before they reach your database.

Not regex on strings: every rule runs on a typed AST built by [`sqlparser-rs`](https://github.com/apache/datafusion-sqlparser-rs), with full dialect awareness for **PostgreSQL** and **SQLite**.

## Why

Most SQL bugs are boring and static: a `DELETE` without `WHERE`, a `SELECT *` that breaks when a column is added, a `NOT IN` subquery that silently returns nothing because of a `NULL`. Linters exist for code, rarely for SQL. SastQL is that linter.

## Features (v0.1)

| Rule | Id | PostgreSQL | SQLite | Default severity |
|---|---|:---:|:---:|---|
| `SELECT *` usage | `select-star` | ✅ | ✅ | warning |
| `UPDATE` / `DELETE` without `WHERE` | `no-where-on-mutation` | ✅ | ✅ | error |
| `SELECT` without `LIMIT` (opt-in) | `missing-limit` | ✅ | ✅ | info |
| `NOT IN` over a subquery (NULL trap) | `not-in-null-trap` | ✅ | ✅ | warning |
| Comparison with `NULL` (`=`, `!=`, `<>` never match) | `null-comparison` | ✅ | ✅ | warning |
| Functions / operators that don't exist in the target dialect | `dialect-portability` | ✅ | ✅ | warning |
| Column types with surprising semantics (e.g. `DATETIME` → NUMERIC affinity in SQLite) | `type-affinity` | ✅ | ✅ | warning |

## Install

```bash
# Any Node.js project — no Rust toolchain needed
npx sastql check migrations/ --dialect postgres    # or: pnpm dlx / bunx

# As a dev dependency (adds the `sastql` binary to node_modules/.bin)
npm install --save-dev sastql

# From source
cargo install --git https://github.com/CLVwp/SastQL
```

Prebuilt binaries for Windows x64, Linux x64/arm64 (glibc & musl) and macOS x64/arm64 are shipped as per-platform npm packages; npm installs only the one matching your machine.

## Usage

```bash
# Analyze a directory for PostgreSQL
sastql check migrations/ --dialect postgres

# Analyze one file for SQLite, JSON output (CI-friendly)
sastql check schema.sql --dialect sqlite --format json

# Run only specific rules
sastql check queries/ --dialect postgres --rules select-star,no-where-on-mutation

# Opt-in rules only run when listed, e.g. missing-limit
sastql check queries/ --dialect postgres --rules missing-limit
```

Exit codes: `0` clean · `1` violations found · `2` parse or IO error. Designed to drop straight into CI.

Example output (real, from the fixtures below):

```
tests/fixtures/dirty/mutations.sql: error no-where-on-mutation: DELETE from sessions has no WHERE clause; every row is affected
    DELETE FROM sessions
tests/fixtures/dirty/portability.sql: warning not-in-null-trap: NOT IN over a subquery returns no rows if the subquery yields NULL; use NOT EXISTS
    SELECT id FROM users WHERE id NOT IN (SELECT manager_id FROM users)

5 file(s) scanned, 6 violation(s)
```

## Try the demo

The repo ships the fixtures the integration tests use — one clean set, one dirty set (one violation per rule):

```
tests/fixtures/clean/   clean on both dialects
tests/fixtures/dirty/   mutations.sql · portability.sql · ddl.sql · null_comparison.sql
```

```bash
# Dirty set: 6 violations, exit code 1
cargo run -- check tests/fixtures/dirty --dialect sqlite

# Same rules, Postgres target: type-affinity is SQLite-only, so fewer violations
cargo run -- check tests/fixtures/dirty --dialect postgres

# Clean set: 0 violations, exit code 0
cargo run -- check tests/fixtures/clean --dialect postgres

# Try the opt-in rule on the dirty set: 2 violations
cargo run -- check tests/fixtures/dirty --dialect sqlite --rules missing-limit
```

## Testing

```bash
cargo test                    # unit tests (one per rule, in its file) + end-to-end tests
cargo clippy -- -D warnings
cargo fmt --check
```

Per the conventions in [CLAUDE.md](CLAUDE.md): each rule has positive and negative unit tests; every rule that behaves differently per dialect is tested for each dialect; the integration tests in `tests/integration.rs` run the real binary over `tests/fixtures/` (parse → analyze → report) and assert on exit codes, rule ids and violation counts for both dialects.

## Design

- **Parser**: `sqlparser-rs`, one dialect per run (`PostgreSqlDialect` / `SQLiteDialect`). Unparseable SQL is a parse error (exit `2`), never silently skipped.
- **Rules**: a single `Rule` trait over the AST. One file per rule, registered in one place.
- **Output**: human-readable by default, `--format json` for machines.

Full feature mapping and UML diagrams: [docs/UML.md](docs/UML.md).

## Roadmap

v0.1 is feature-complete: the seven rules above are implemented and tested. Next, in order:

| # | Item | What it means |
|---|---|---|
| 1 | **Precise line/column** | Track token spans so violations point at the exact line. Until then, violations carry the offending SQL snippet only — per CLAUDE.md, we don't fake precision we don't have. |
| 2 | **`sastql.toml`** | Per-rule severity overrides and named rule sets, so teams stop pasting long `--rules` lists into CI. |
| 3 | **SARIF output + GitHub Action** | `--format sarif` for code-scanning, plus a ready-made action for inline PR annotations. |
| 4 | **`--check-app`** | Scan SQL strings inside application source (Rust/TS/Python) for injection-prone concatenation. |

## License

Copyright © 2026 Clément Viellard, all rights reserved.

SastQL is **source-available** - deliberately **not** open source. The code is public to read, learn from and contribute to, but it is not offered under an open-source license. See [LICENSE](LICENSE) (English) and [LICENSE.fr](LICENSE.fr) (French, authoritative version):

- **Free** for personal, educational and research use.
- **Any commercial use requires the Author's prior written permission**, with financial terms set at the Author's sole discretion.
- **No redistribution, no published forks, no derivative works** without permission - public forks are tolerated solely to propose contributions back.
- Contributions transfer full perpetual rights to the Author.
- Want to contribute? Read [CONTRIBUTING.md](CONTRIBUTING.md) first — PR rules and the fork policy are spelled out there.
