use std::ops::ControlFlow;

use sqlparser::ast::visit_expressions;
use sqlparser::ast::{Expr, Statement, Value};

use super::Rule;
use crate::parser::DialectKind;
use crate::report::Severity;

/// A `LIKE` pattern starting with `%` or `_` cannot use a B-tree index:
/// the scan is linear whatever the index says.
pub struct LeadingWildcardLike;

impl Rule for LeadingWildcardLike {
    fn id(&self) -> &'static str {
        "leading-wildcard-like"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn description(&self) -> &'static str {
        "LIKE pattern starting with % or _ prevents index use"
    }

    fn check(&self, stmt: &Statement, _dialect: DialectKind) -> Vec<String> {
        let mut messages = Vec::new();
        let _ = visit_expressions(stmt, |expr| {
            match expr {
                // `NOT LIKE` (negated: true) scans just the same — flagged too.
                Expr::Like { pattern, .. } | Expr::ILike { pattern, .. }
                    if starts_with_wildcard(pattern) =>
                {
                    messages.push(
                        "LIKE pattern starts with a wildcard; the index cannot be used, matching scans every row — anchor the pattern or use full-text search"
                            .into(),
                    );
                }
                _ => {}
            }
            ControlFlow::<()>::Continue(())
        });
        messages
    }
}

fn starts_with_wildcard(pattern: &Expr) -> bool {
    let Expr::Value(v) = pattern else {
        // ponytail: only literal patterns; a pattern built from a column or
        // expression is left alone rather than guessed at.
        return false;
    };
    match &v.value {
        Value::SingleQuotedString(s) => s.starts_with('%') || s.starts_with('_'),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::check_sql;

    #[test]
    fn flags_leading_percent_underscore_and_not_like() {
        for sql in [
            "SELECT id FROM users WHERE email LIKE '%@gmail.com'",
            "SELECT id FROM users WHERE code LIKE '_AB'",
            "SELECT id FROM users WHERE email NOT LIKE '%@gmail.com'",
        ] {
            assert_eq!(
                check_sql(&LeadingWildcardLike, sql, DialectKind::Postgres).len(),
                1
            );
        }
    }

    #[test]
    fn flags_ilike() {
        assert_eq!(
            check_sql(
                &LeadingWildcardLike,
                "SELECT id FROM users WHERE email ILIKE '%@GMAIL.COM'",
                DialectKind::Postgres
            )
            .len(),
            1
        );
    }

    #[test]
    fn allows_trailing_wildcard_and_dynamic_pattern() {
        assert!(
            check_sql(
                &LeadingWildcardLike,
                "SELECT id FROM users WHERE email LIKE 'prefix%'",
                DialectKind::Postgres
            )
            .is_empty()
        );
        assert!(
            check_sql(
                &LeadingWildcardLike,
                "SELECT id FROM users WHERE email LIKE pattern",
                DialectKind::Postgres
            )
            .is_empty()
        );
    }
}
