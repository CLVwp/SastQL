use std::ops::ControlFlow;

use sqlparser::ast::visit_expressions;
use sqlparser::ast::{CastKind, Expr, Statement};

use super::{Rule, last_fn_segment};
use crate::parser::DialectKind;
use crate::report::Severity;

/// Functions and operators that don't exist in the target dialect — the
/// classic portability break when the same .sql file targets both engines.
pub struct DialectPortability;

// (function name, fix hint). Matched case-insensitively on the last
// dotted segment, so pg_catalog.now() is caught too.
const SQLITE_ONLY: &[(&str, &str)] = &[
    ("now", "SQLite has no NOW(); use datetime('now')"),
    (
        "concat",
        "CONCAT() requires SQLite 3.44+; || works everywhere",
    ),
];
const PG_ONLY: &[(&str, &str)] = &[
    (
        "strftime",
        "strftime() is SQLite-only; use to_char() in PostgreSQL",
    ),
    ("julianday", "julianday() is SQLite-only"),
    ("sqlite_version", "sqlite_version() is SQLite-only"),
    (
        "ifnull",
        "IFNULL() does not exist in PostgreSQL; use COALESCE",
    ),
];

impl Rule for DialectPortability {
    fn id(&self) -> &'static str {
        "dialect-portability"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn description(&self) -> &'static str {
        "Function or operator absent from the target dialect"
    }

    fn check(&self, stmt: &Statement, dialect: DialectKind) -> Vec<String> {
        let functions = match dialect {
            DialectKind::Sqlite => SQLITE_ONLY,
            DialectKind::Postgres => PG_ONLY,
        };
        let mut messages = Vec::new();
        let _ = visit_expressions(stmt, |expr| {
            match expr {
                Expr::Function(f) => {
                    // ponytail: plain denylist; extend as real-world cases show up.
                    let last = last_fn_segment(f);
                    if let Some((_, hint)) = functions.iter().find(|(fn_name, _)| *fn_name == last)
                    {
                        messages.push((*hint).into());
                    }
                }
                // ponytail: only checked if the dialect parses `::` at all — SQLiteDialect
                // may reject it upstream, in which case the file already fails with exit 2.
                Expr::Cast {
                    kind: CastKind::DoubleColon,
                    ..
                } if dialect == DialectKind::Sqlite => {
                    messages.push("the :: cast is PostgreSQL syntax; use CAST(x AS type)".into())
                }
                _ => {}
            }
            ControlFlow::<()>::Continue(())
        });
        messages
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::check_sql;

    #[test]
    fn flags_now_and_concat_for_sqlite() {
        assert_eq!(
            check_sql(&DialectPortability, "SELECT NOW()", DialectKind::Sqlite).len(),
            1
        );
        assert_eq!(
            check_sql(
                &DialectPortability,
                "SELECT CONCAT(a, b) FROM t WHERE id = 1",
                DialectKind::Sqlite
            )
            .len(),
            1
        );
    }

    #[test]
    fn flags_sqlite_functions_for_postgres() {
        assert_eq!(
            check_sql(
                &DialectPortability,
                "SELECT strftime('%Y', created) FROM events WHERE id = 1",
                DialectKind::Postgres
            )
            .len(),
            1
        );
    }

    #[test]
    fn flags_double_colon_cast_for_sqlite() {
        // SQLiteDialect parses `::` (it is lenient), so the AST-level check fires.
        assert_eq!(
            check_sql(&DialectPortability, "SELECT 1::text", DialectKind::Sqlite).len(),
            1
        );
        // The same syntax is native PostgreSQL — no violation there.
        assert!(check_sql(&DialectPortability, "SELECT 1::text", DialectKind::Postgres).is_empty());
    }

    #[test]
    fn allows_native_functions() {
        assert!(
            check_sql(
                &DialectPortability,
                "SELECT datetime('now')",
                DialectKind::Sqlite
            )
            .is_empty()
        );
        assert!(
            check_sql(
                &DialectPortability,
                "SELECT to_char(created, 'YYYY') FROM events WHERE id = 1",
                DialectKind::Postgres
            )
            .is_empty()
        );
    }
}
