use std::ops::ControlFlow;

use sqlparser::ast::visit_expressions;
use sqlparser::ast::{BinaryOperator, Expr, Statement, Value};

use super::Rule;
use crate::parser::DialectKind;
use crate::report::Severity;

/// `x = NULL` and `x != NULL` are never true: in SQL three-valued logic any
/// comparison with `NULL` yields UNKNOWN, so the row is filtered out.
/// `IS NULL` / `IS NOT NULL` is the fix.
pub struct NullComparison;

/// `expr` is the literal `NULL` (as opposed to an `IS [NOT] NULL` predicate).
fn is_null_literal(expr: &Expr) -> bool {
    matches!(expr, Expr::Value(v) if v.value == Value::Null)
}

impl Rule for NullComparison {
    fn id(&self) -> &'static str {
        "null-comparison"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn description(&self) -> &'static str {
        "= NULL / != NULL / <> NULL never matches (three-valued logic) — use IS NULL / IS NOT NULL"
    }

    fn check(&self, stmt: &Statement, _dialect: DialectKind) -> Vec<String> {
        let mut messages = Vec::new();
        let _ = visit_expressions(stmt, |expr| {
            if let Expr::BinaryOp { op, left, right } = expr {
                let compares_null = is_null_literal(left) || is_null_literal(right);
                if compares_null && matches!(op, BinaryOperator::Eq | BinaryOperator::NotEq) {
                    messages.push(
                        "comparison with NULL is never true (=, !=, <> against NULL yield UNKNOWN); use IS NULL / IS NOT NULL"
                            .into(),
                    );
                }
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
    fn flags_eq_and_ne_null_on_both_sides() {
        assert_eq!(
            check_sql(
                &NullComparison,
                "SELECT id FROM users WHERE deleted = NULL",
                DialectKind::Sqlite
            )
            .len(),
            1
        );
        assert_eq!(
            check_sql(
                &NullComparison,
                "SELECT id FROM users WHERE deleted <> NULL",
                DialectKind::Sqlite
            )
            .len(),
            1
        );
        assert_eq!(
            check_sql(
                &NullComparison,
                "UPDATE users SET locked = true WHERE NULL = last_login",
                DialectKind::Postgres
            )
            .len(),
            1
        );
    }

    #[test]
    fn allows_is_null_and_normal_comparisons() {
        assert!(
            check_sql(
                &NullComparison,
                "SELECT id FROM users WHERE deleted IS NULL",
                DialectKind::Sqlite
            )
            .is_empty()
        );
        assert!(
            check_sql(
                &NullComparison,
                "SELECT id FROM users WHERE deleted IS NOT NULL AND id = 1",
                DialectKind::Postgres
            )
            .is_empty()
        );
    }
}
