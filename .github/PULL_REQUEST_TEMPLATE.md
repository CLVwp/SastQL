## What

<!-- One line: what this PR changes and why. -->

## Checklist

- [ ] `cargo test`, `cargo clippy -- -D warnings` and `cargo fmt --check` pass (CI enforces them)
- [ ] New rule? Followed the "Adding a rule" checklist in [CLAUDE.md](../CLAUDE.md): rule file + registration in `all_rules()` + unit tests (positive & negative) + fixture in `tests/fixtures/dirty/` + README/UML rows
- [ ] Dialect-specific behavior: one test per dialect
- [ ] No changes to `LICENSE`, `LICENSE.fr` or license wording anywhere — such PRs are rejected without review
- [ ] I agree that this contribution transfers full perpetual rights to the author, per [LICENSE](../LICENSE)

## Dialect impact

<!-- Does this change behave differently on PostgreSQL vs SQLite? Which rules/fixtures are affected? -->
