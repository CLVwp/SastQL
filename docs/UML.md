# SastQL — Feature Mapping & UML

Complete feature mapping for SastQL v0.1. Target: analyze `.sql` files with dialect awareness for **PostgreSQL** and **SQLite**.

Each diagram lives in its own `.puml` file under [`docs/uml/`](uml/). To render: open the file in VS Code / IntelliJ with the PlantUML extension and hit **Alt+D** (or `plantuml <file>` on the CLI). Layout uses the built-in `smetana` engine — no Graphviz install needed.

| # | Diagram | File |
|---|---|---|
| 1 | Use cases (scope) | [`use-cases.puml`](uml/use-cases.puml) |
| 2 | Component flow | [`components.puml`](uml/components.puml) |
| 3 | Class diagram | [`classes.puml`](uml/classes.puml) |
| 4 | Sequence — `check` command | [`sequence-check.puml`](uml/sequence-check.puml) |
| 5 | State — analysis pipeline | [`pipeline-state.puml`](uml/pipeline-state.puml) |

Out of scope for v0.1 (see roadmap in README): scanning SQL strings inside application source, SARIF output, config file, autofix.

## Feature matrix — rules per dialect

| Rule | What it detects | PostgreSQL | SQLite | Severity |
|---|---|:---:|:---:|---|
| `select-star` | `SELECT *` (and `SELECT table.*`) | ✅ | ✅ | warning |
| `no-where-on-mutation` | `UPDATE`/`DELETE` with no `WHERE` clause | ✅ | ✅ | error |
| `missing-limit` | bare `SELECT` without `LIMIT` (opt-in via `--rules`) | ✅ | ✅ | info |
| `not-in-null-trap` | `expr NOT IN (subquery)` — returns no rows if subquery yields a `NULL` | ✅ | ✅ | warning |
| `dialect-portability` | functions/operators absent from target dialect (e.g. `NOW()` in SQLite, `::` cast in SQLite, `CONCAT()` pre-3.44) | ✅ | ✅ | warning |
| `type-affinity` | DDL types with surprising semantics (`DATETIME` → NUMERIC affinity in SQLite; non-existent types in PG) | — | ✅ | warning |

Note: output formatting is an enum + `match`, not a trait hierarchy — two formats don't justify polymorphism.

## Error model

| Situation | Behaviour | Exit code |
|---|---|---:|
| All files clean | report "0 violations" | `0` |
| ≥ 1 violation | violations rendered (human or JSON) | `1` |
| Parse error on any file | error to stderr, stop | `2` |
| Path not found / unreadable | error to stderr | `2` |
