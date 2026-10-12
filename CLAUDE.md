# CLAUDE.md

SastQL — static analyzer (SAST) for SQL files, targeting PostgreSQL and SQLite.

## Stack

- Rust (single crate, no workspace)
- Dependencies: `sqlparser` (AST + dialects), `clap` (CLI). Nothing else without justification.
- Parser is `sqlparser-rs`; never hand-write SQL parsing or use regex to match SQL constructs.

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
    mod.rs       Rule trait + rule registry (add_rule list)
    *.rs         one file per rule, unit tests inside each file
tests/fixtures/  .sql samples used by integration tests
docs/UML.md      feature mapping, diagrams, rule matrix
```

## Conventions

- **Adding a rule**: create `src/rules/<id>.rs` implementing `Rule`, register it in `rules/mod.rs`, add a unit test with a minimal SQL sample (positive + negative case), add the rule to the table in `README.md` and `docs/UML.md`.
- **Dialect-specific logic** goes inside the rule (check `dialect`), not in the parser layer. The parser must stay dialect-agnostic apart from selecting the `Dialect` impl.
- **Fail loudly**: a parse error is exit code `2` with the message — never skip a file silently. Violations found → exit `1`. Clean → `0`.
- Rule ids are kebab-case, stable once released (CI depends on them).
- No new dependency without a one-line justification in the PR/commit.
- Keep violations self-contained: rule id, severity, message, offending SQL snippet. (Line/col precision is a roadmap item — don't fake it.)

## Testing

- One unit test per rule, in the rule's file: parse a SQL string, assert on returned violations.
- Integration test in `tests/` walks `tests/fixtures/` end-to-end (parse → analyze → report) for both dialects.
- Every rule that behaves differently per dialect gets a test for each dialect.
