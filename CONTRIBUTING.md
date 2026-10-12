# Contributing to SastQL

Thanks for reading the code and wanting to improve it. SastQL is **source-available, deliberately not open source** — see [LICENSE](LICENSE) and [LICENSE.fr](LICENSE.fr) (authoritative, French). Contributing means accepting those terms.

## Ground rules (non-negotiable)

- **License changes are rejected on sight.** PRs that edit `LICENSE`, `LICENSE.fr`, or reframe the licensing anywhere in the docs are closed without review.
- **Forks are tolerated for one purpose only**: preparing a contribution back to this repository. Publishing a fork, a derivative work, a mirror or a rebranded copy is not permitted without the author's prior written permission.
- **All arbitration belongs to the author** (Clément Viellard - clementviellard@gmail.com): commercial use, redistribution, exceptions to any term, and anything the license does not explicitly grant. Every decision is at the author's sole discretion.

## How to propose a change

1. Open an issue first for anything behavioral (new rule, changed severity, dialect semantics) — rule ids are stable and CI depends on them.
2. Fork (for a contribution back), create a branch, commit.
3. Open a PR — CI enforces `cargo fmt --check`, `cargo clippy -- -D warnings` and `cargo test`:
   - New rule? Follow the "Adding a rule" checklist in [CLAUDE.md](CLAUDE.md): rule file + registration in `all_rules()` + unit tests (positive & negative) + fixture in `tests/fixtures/dirty/` + README/UML rows.
   - Dialect-specific behavior gets one test per dialect.
   - Violation messages stay self-contained; no fake line/column precision.
4. By opening the PR you agree that the contribution transfers full perpetual rights to the author, per the license.

## Scope notes

- The parser is [`sqlparser-rs`](https://github.com/apache/datafusion-sqlparser-rs) — never hand-parse SQL or regex-match constructs.
- Rules live in `src/rules/`, one file per rule; the `Rule` trait is the only extension point.

Nothing here overrides [LICENSE](LICENSE) / [LICENSE.fr](LICENSE.fr); in case of conflict, the license wins (French version authoritative).
