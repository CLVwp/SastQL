# SastQL

Static analysis (SAST) for SQL files, targeting **PostgreSQL** and **SQLite**.

SastQL parses your SQL into a real AST ([sqlparser-rs](https://github.com/apache/datafusion-sqlparser-rs)) — not regex on strings — and flags dangerous or non-portable patterns before they reach your database. **25 rules**: `DELETE`/`UPDATE` without `WHERE`, cartesian `JOIN` without `ON`, `SELECT *`, `CASE` without `ELSE`, the `NOT IN` NULL trap, `= NULL` comparisons, `md5()`, `ORDER BY random()`, money stored in floats, tables without `PRIMARY KEY`, dialect-incompatible functions, and more.

## Try it in one line

```bash
npx "@clv.wp/sastql" check migrations/ --dialect postgres
```

Every row of output is a violation; exit code `1` if any, `0` if clean, `2` on parse error — CI-ready. `pnpm dlx` and `bunx` work the same.

> The quotes matter in **PowerShell** (`@clv.wp` collides with its `@` splatting operator); harmless in cmd, Git Bash and sh — copy-paste as-is.

## Install

```bash
npm install --save-dev "@clv.wp/sastql"
```

This adds the unscoped `sastql` binary to `node_modules/.bin` (prebuilt binaries for Windows x64, Linux x64/arm64 glibc & musl, macOS x64/arm64 — npm installs only the one matching your machine).

## Usage

```bash
sastql check migrations/ --dialect postgres
sastql check schema.sql --dialect sqlite --format json
sastql check queries/ --rules select-star,no-where-on-mutation
sastql check dumps/ --rules missing-limit,destructive-ddl
```

`--rules` also unlocks the opt-in rules (`missing-limit`, `union-without-all`, `order-by-ordinal`, `naive-timestamp`, `limit-without-order-by`, `destructive-ddl`).

Rule-by-dialect matrix: [docs/UML.md](https://github.com/CLVwp/SastQL/blob/main/docs/UML.md).

## License

Source-available, deliberately **not** open source — © 2026 Clément Viellard, all rights reserved. Free for personal, educational and research use. **Any commercial use requires the author's prior written permission.** No redistribution or published derivative works without permission. See the `LICENSE` file shipped in this package and [LICENSE.fr](https://github.com/CLVwp/SastQL/blob/main/LICENSE.fr) (authoritative).
