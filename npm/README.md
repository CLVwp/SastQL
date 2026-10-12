# SastQL

Static analysis (SAST) for SQL files, targeting **PostgreSQL** and **SQLite**.

SastQL parses your SQL into a real AST ([sqlparser-rs](https://github.com/apache/datafusion-sqlparser-rs)) — not regex on strings — and flags dangerous or non-portable patterns before they reach your database: `DELETE` without `WHERE`, `SELECT *`, the `NOT IN` NULL trap, `= NULL` comparisons, dialect-incompatible functions, surprising column types.

## Install

```bash
npx @clv.wp/sastql check migrations/ --dialect postgres   # or pnpm dlx / bunx
npm install --save-dev @clv.wp/sastql                     # adds the `sastql` binary
```

## Usage

```bash
sastql check migrations/ --dialect postgres
sastql check schema.sql --dialect sqlite --format json   # CI-friendly
sastql check queries/ --rules select-star,no-where-on-mutation
```

Exit codes: `0` clean · `1` violations found · `2` parse or IO error.

Seven rules shipped; per-dialect behavior documented in the [rule matrix](https://github.com/CLVwp/SastQL/blob/main/docs/UML.md).

## License

Source-available, deliberately **not** open source — © 2026 Clément Viellard, all rights reserved. Free for personal, educational and research use. **Any commercial use requires the author's prior written permission.** No redistribution or published derivative works without permission. See the `LICENSE` file shipped in this package and [LICENSE.fr](https://github.com/CLVwp/SastQL/blob/main/LICENSE.fr) (authoritative).
