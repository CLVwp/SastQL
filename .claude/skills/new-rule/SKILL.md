---
name: new-rule
description: Add a SastQL rule end-to-end following CLAUDE.md conventions (rule file, registration, tests, fixtures, README, UML matrix). Use when the user asks for a new rule or linter check.
---

# Add a SastQL rule

Every step is mandatory — the checklist exists because steps get forgotten:

1. **Create `src/rules/<id>.rs`** — id is kebab-case and stable once released. Implement the `Rule` trait (`id`, `severity`, `description`, `check`). Match on the AST; never regex raw SQL. Dialect-specific logic checks the `dialect` parameter inside the rule, never in the parser layer.
2. **Register** in `all_rules()` in `rules/mod.rs`. Override `opt_in()` to return `true` only if the rule must run exclusively when listed in `--rules`.
3. **Unit tests** in the rule file, using the `check_sql` helper from `rules/mod.rs`: a positive case AND a negative case. If the rule reads `dialect`, one test per dialect.
4. **Fixture**: add a trigger to `tests/fixtures/dirty/` and update the exact violation count asserted in `tests/integration.rs` (it pins the number).
5. **README.md**: one row in the features table (rule, id, per-dialect columns, default severity).
6. **docs/UML.md**: one row in the rule matrix.

sqlparser 0.63 gotchas: see the "sqlparser 0.63 API notes" section in CLAUDE.md (`Expr::InSubquery { negated }`, `LimitClause` variants, `FromTable` enum, `SelectItem` alias arms, lenient SQLiteDialect — verify at the AST, not via parse failure).

Done = `cargo test` + `cargo clippy -- -D warnings` + `cargo fmt --check` all green.
