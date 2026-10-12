# CLAUDE.md

SastQL — static analyzer (SAST) for SQL files, targeting PostgreSQL and SQLite.

## Stack

- Rust (single crate, no workspace)
- Dependencies: `sqlparser` (AST + dialects), `clap` (CLI). Nothing else without justification.
- Parser is `sqlparser-rs`; never hand-write SQL parsing or use regex to match SQL constructs.

## sqlparser 0.63 API notes

Hard-won facts for this exact version — when in doubt, grep the registry source (`~/.cargo/registry/src/*/sqlparser-0.63.0/src/`) instead of guessing:

- The `visitor` feature is **not** a default feature: `visit_expressions` only resolves if `Cargo.toml` keeps `features = ["visitor"]`.
- IN / NOT IN are not `BinaryOperator`s in 0.63 — use `Expr::InSubquery { expr, subquery, negated }` (`negated: true` = NOT IN).
- `visit_expressions` is re-exported at `sqlparser::ast::visit_expressions`; the `ast::visitor` module itself is private.
- `Query.limit_clause: Option<LimitClause>` with variants `LimitOffset { limit, .. }` / `OffsetCommaLimit { .. }`; `Query.fetch` is a separate field.
- `Delete.from` is the `FromTable` enum — match `WithFromKeyword(tables) | WithoutKeyword(tables)`.
- `SelectItem` has `ExprWithAlias { .. }` and `ExprWithAliases { .. }` arms — exhaustive matches need them.
- `SQLiteDialect` is lenient: it accepts Postgres-only syntax like `1::text`. Dialect-specific rules must inspect the AST, never rely on parse failure.
- A literal `NULL` is `Expr::Value(v)` with `v.value == Value::Null` — `Expr::Value` wraps a `ValueWithSpan`, not a plain `Value`.

## Conventions (session)

- Language: the conversation with the user is in French; code, comments and docs are in English.
- License stance is fixed: **source-available, deliberately not open source** (see `LICENSE` / `LICENSE.fr`, French version authoritative). Never propose MIT or any OSS license, never relax the no-commercial-use / no-redistribution terms.
- `Cargo.lock` is committed on purpose (binary project) — don't add it to `.gitignore`.

## Commands

```bash
cargo build
cargo test
cargo clippy -- -D warnings
cargo fmt --check
cargo run -- check tests/fixtures/ --dialect sqlite
```

## Structure

```
src/
  main.rs        CLI entry (clap): check subcommand, --dialect, --format, --rules
  parser.rs      parse(sql, dialect) -> Vec<Statement>; dialect selection lives here
  analyzer.rs    runs every rule over the parsed statements -> Report
  report.rs      Violation, Report, OutputFormat (Human | Json), rendering, exit codes
  rules/
    mod.rs       Rule trait + registry (all_rules / default_rules; opt_in filters defaults)
    *.rs         one file per rule, unit tests inside each file
tests/fixtures/  .sql samples used by integration tests
docs/UML.md      feature mapping, diagrams, rule matrix
```

## Conventions

- **Adding a rule**: create `src/rules/<id>.rs` implementing `Rule`, register it in `rules/mod.rs`, add a unit test with a minimal SQL sample (positive + negative case), add the rule to the table in `README.md` and `docs/UML.md`.
  Use the `check_sql` helper in `rules/mod.rs` for unit tests. Rules run by default; override `opt_in()` to make one `--rules`-only. Add a trigger in `tests/fixtures/dirty/` and update the violation count asserted in `tests/integration.rs` — it pins the exact number.
- **Dialect-specific logic** goes inside the rule (check `dialect`), not in the parser layer. The parser must stay dialect-agnostic apart from selecting the `Dialect` impl.
- **Fail loudly**: a parse error is exit code `2` with the message — never skip a file silently. Violations found → exit `1`. Clean → `0`.
- Rule ids are kebab-case, stable once released (CI depends on them).
- No new dependency without a one-line justification in the PR/commit.
- Keep violations self-contained: rule id, severity, message, offending SQL snippet. (Line/col precision is a roadmap item — don't fake it.)

## Testing

- One unit test per rule, in the rule's file: parse a SQL string, assert on returned violations.
- Integration test in `tests/` walks `tests/fixtures/` end-to-end (parse → analyze → report) for both dialects.
- Every rule that behaves differently per dialect gets a test for each dialect.
