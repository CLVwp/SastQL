use std::ops::ControlFlow;

use sqlparser::ast::visit_expressions;
use sqlparser::ast::{Expr, Statement};

use super::Rule;
use crate::parser::DialectKind;
use crate::report::Severity;

/// A `CASE` without `ELSE` silently yields `NULL` when no branch matches —
/// usually an oversight, since the author enumerated every expected case.
pub struct CaseWithoutElse;

impl Rule for CaseWithoutElse {
    fn id(&self) -> &'static str {
        "case-without-else"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn description(&self) -> &'static str {
        "CASE expression without ELSE returns NULL when no condition matches"
    }

    fn check(&self, stmt: &Statement, _dialect: DialectKind) -> Vec<String> {
        let mut messages = Vec::new();
        let _ = visit_expressions(stmt, |expr| {
            if let Expr::Case {
                else_result: None, ..
            } = expr
            {
                messages.push(
                    "CASE has no ELSE branch: it yields NULL when no condition matches; add an ELSE"
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
    fn flags_case_without_else() {
        assert_eq!(
            check_sql(
                &CaseWithoutElse,
                "SELECT CASE WHEN x > 0 THEN 'pos' END FROM t",
                DialectKind::Postgres
            )
            .len(),
            1
        );
    }

    #[test]
    fn counts_each_case_expression() {
        assert_eq!(
            check_sql(
                &CaseWithoutElse,
                "SELECT CASE WHEN a THEN 1 END, CASE WHEN b THEN 2 END FROM t",
                DialectKind::Postgres
            )
            .len(),
            2
        );
    }

    #[test]
    fn allows_explicit_else() {
        assert!(
            check_sql(
                &CaseWithoutElse,
                "SELECT CASE WHEN x > 0 THEN 'pos' ELSE 'neg' END FROM t",
                DialectKind::Postgres
            )
            .is_empty()
        );
    }
}
