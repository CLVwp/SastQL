use std::ops::ControlFlow;

use sqlparser::ast::visit_expressions;
use sqlparser::ast::{Expr, Statement};

use super::Rule;
use crate::parser::DialectKind;
use crate::report::Severity;

/// `expr NOT IN (subquery)` returns **no rows at all** the moment the
/// subquery yields a single `NULL` — the classic silent-empty-result trap.
/// `NOT EXISTS` is the fix.
pub struct NotInNullTrap;

impl Rule for NotInNullTrap {
    fn id(&self) -> &'static str {
        "not-in-null-trap"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn description(&self) -> &'static str {
        "NOT IN over a subquery returns no rows if the subquery yields NULL — use NOT EXISTS"
    }

    fn check(&self, stmt: &Statement, _dialect: DialectKind) -> Vec<String> {
        let mut messages = Vec::new();
        let _ = visit_expressions(stmt, |expr| {
            if let Expr::InSubquery { negated: true, .. } = expr {
                messages.push(
                    "NOT IN over a subquery returns no rows if the subquery yields NULL; use NOT EXISTS"
                        .into(),
                );
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
    fn flags_not_in_over_subquery() {
        assert_eq!(
            check_sql(
                &NotInNullTrap,
                "SELECT id FROM users WHERE manager_id NOT IN (SELECT id FROM managers)",
                DialectKind::Postgres
            )
            .len(),
            1
        );
    }

    #[test]
    fn allows_not_in_over_literal_list_and_in_subquery() {
        assert!(
            check_sql(
                &NotInNullTrap,
                "SELECT id FROM users WHERE status NOT IN ('a', 'b')",
                DialectKind::Sqlite
            )
            .is_empty()
        );
        assert!(
            check_sql(
                &NotInNullTrap,
                "SELECT id FROM users WHERE id IN (SELECT manager_id FROM users)",
                DialectKind::Sqlite
            )
            .is_empty()
        );
    }

    #[test]
    fn covers_update_where() {
        assert_eq!(
            check_sql(
                &NotInNullTrap,
                "UPDATE users SET active = false WHERE id NOT IN (SELECT id FROM admins)",
                DialectKind::Postgres
            )
            .len(),
            1
        );
    }
}
