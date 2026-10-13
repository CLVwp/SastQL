use std::ops::ControlFlow;

use sqlparser::ast::visit_expressions;
use sqlparser::ast::{BinaryOperator, Expr, Statement, Value};

use super::Rule;
use crate::parser::DialectKind;
use crate::report::Severity;

/// In SQLite, `+` is numeric addition: string operands are coerced to 0, so
/// `'a' + 'b'` is `0`. Concatenation is `||`.
pub struct StringPlus;

impl Rule for StringPlus {
    fn id(&self) -> &'static str {
        "string-plus"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn description(&self) -> &'static str {
        "SQLite: + on strings coerces them to numbers; use || to concatenate"
    }

    fn check(&self, stmt: &Statement, dialect: DialectKind) -> Vec<String> {
        if dialect != DialectKind::Sqlite {
            return Vec::new();
        }
        let mut messages = Vec::new();
        let _ = visit_expressions(stmt, |expr| {
            if let Expr::BinaryOp {
                op: BinaryOperator::Plus,
                left,
                right,
            } = expr
            {
                if is_string_literal(left) || is_string_literal(right) {
                    messages.push(
                        "SQLite + is numeric addition: a string operand coerces to 0; use || to concatenate"
                            .into(),
                    );
                }
            }
            ControlFlow::<()>::Continue(())
        });
        messages
    }
}

fn is_string_literal(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::Value(v) if matches!(v.value, Value::SingleQuotedString(_))
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::check_sql;

    #[test]
    fn flags_plus_on_strings_in_sqlite() {
        assert_eq!(
            check_sql(&StringPlus, "SELECT 'a' + 'b'", DialectKind::Sqlite).len(),
            1
        );
        assert_eq!(
            check_sql(
                &StringPlus,
                "SELECT name + '!' FROM users",
                DialectKind::Sqlite
            )
            .len(),
            1
        );
    }

    #[test]
    fn is_sqlite_only() {
        assert!(check_sql(&StringPlus, "SELECT 'a' + 'b'", DialectKind::Postgres).is_empty());
    }

    #[test]
    fn allows_numeric_addition_and_concat() {
        assert!(check_sql(&StringPlus, "SELECT 1 + 2", DialectKind::Sqlite).is_empty());
        assert!(
            check_sql(
                &StringPlus,
                "SELECT name || '!' FROM users",
                DialectKind::Sqlite
            )
            .is_empty()
        );
    }
}
