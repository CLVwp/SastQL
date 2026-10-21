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
- `.claude/settings.json` hooks: `.rs` edits are auto-formatted on save; `Cargo.lock` edits and `cargo add/rm` commands are **blocked** by the dep guard (dependency changes need user confirmation first).

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
.github/workflows/  ci.yml (PR/push gate: fmt+clippy+test) · release.yml (tag v* → npm publish via OIDC)
npm/             npm packaging (main pkg + per-platform; pkgs/ generated, gitignored)
CONTRIBUTING.md  fork policy, PR rules — license-touching PRs rejected on sight
.claude/         hooks (fmt on edit, dep guard) · /new-rule skill · rule-reviewer agent
```

## Conventions

- **Adding a rule**: create `src/rules/<id>.rs` implementing `Rule`, register it in `rules/mod.rs`, add a unit test with a minimal SQL sample (positive + negative case), add the rule to the table in `README.md` and `docs/UML.md`.
  Use the `check_sql` helper in `rules/mod.rs` for unit tests. Rules run by default; override `opt_in()` to make one `--rules`-only. Add a trigger in `tests/fixtures/dirty/` (minimal set, one hit per file) or in `tests/fixtures/realistic/bugs_*.sql` (simulated shop DB, per-rule counts pinned in `tests/integration.rs`), then update the pinned counts in `tests/integration.rs`.
  The `/new-rule` skill encodes this checklist step by step; run the `rule-reviewer` agent on the finished rule.
- **Dialect-specific logic** goes inside the rule (check `dialect`), not in the parser layer. The parser must stay dialect-agnostic apart from selecting the `Dialect` impl.
- **Fail loudly**: a parse error is exit code `2` with the message — never skip a file silently. Violations found → exit `1`. Clean → `0`.
- Rule ids are kebab-case, stable once released (CI depends on them).
- No new dependency without a one-line justification in the PR/commit.
- Keep violations self-contained: rule id, severity, message, offending SQL snippet, plus the statement-level line/column of the first token. When the position cannot be established honestly, the violation carries no position at all — never print a wrong one ("honest degradation"). Per-expression spans are impossible in sqlparser 0.63: AST nodes carry no spans, so statement-level is the finest precision we can attribute.

## Distribution (npm)

- `npm/` wraps the binary for `npx` / `pnpm dlx` / `bunx` (esbuild/biome pattern): main package `@clv.wp/sastql` (npm's typosquat filter blocks unscoped `sastql` — too close to `fastq`; the `bin` name stays unscoped `sastql`) + one optionalDependency package per platform (`sastql-<os>-<arch>[-gnu|-musl]`).
- Platform packages are **generated, not checked in** (`npm/scripts/make-platform-package.mjs`, output in `npm/pkgs/`, gitignored). Targets → package names are mapped in that script; adding a target means adding it there **and** to `npm/package.json` `optionalDependencies` **and** the release matrix.
- Cargo version is the single source of truth: releases are tag-driven (`git tag vX.Y.Z && git push origin vX.Y.Z`, only from merged `main`). **Bump `Cargo.toml` via PR before tagging** — the binary's `--version` must equal the npm version (CI re-stamps npm files from the tag).
- Auth is npm **trusted publishing (OIDC)**, no secret. Three invariants that each broke once — keep them true:
  - each package's Trusted Publisher on npmjs.com: repo `CLVwp/SastQL`, workflow `release.yml`, **empty environment**, `npm publish` toggle ON — new configs default to **stage-only** (since Sep 2026 it's a separate toggle);
  - `release.yml` keeps `permissions: id-token: write`, Node 24 (npm ≥ 11.5), `npm publish --provenance`;
  - platform manifests keep `repository` matching the GitHub repo (`make-platform-package.mjs`) — npm cross-checks it; missing = 403 "OIDC permission denied".
- npm packages say `SEE LICENSE IN LICENSE` — `LICENSE` ships inside every package, plus a README stating the terms (`npm/README.md` main · `npm/PLATFORM_README.md` copied into platform packages): the npm page only renders the README.

## Git & CI

- Remote `github.com/CLVwp/SastQL`. `main` is protected: direct pushes are **rejected** — PR required, `ci` check must pass, `enforce_admins` on (applies to the owner too). Workflow: branch → push → PR → merge when CI is green.

## Testing

- One unit test per rule, in the rule's file: parse a SQL string, assert on returned violations.
- Integration test in `tests/` walks `tests/fixtures/` end-to-end (parse → analyze → report) for both dialects.
- Every rule that behaves differently per dialect gets a test for each dialect.
