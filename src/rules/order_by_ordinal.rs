use sqlparser::ast::{Expr, OrderByKind, Statement, Value};

use super::Rule;
use crate::parser::DialectKind;
use crate::report::Severity;

/// `ORDER BY 1` sorts by the Nth output column: adding, removing or
/// reordering a column in the SELECT silently changes the sort.
pub struct OrderByOrdinal;

impl Rule for OrderByOrdinal {
    fn id(&self) -> &'static str {
        "order-by-ordinal"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn description(&self) -> &'static str {
        "ORDER BY <position> is fragile — sort by column name instead"
    }

    fn opt_in(&self) -> bool {
        true
    }

    fn check(&self, stmt: &Statement, _dialect: DialectKind) -> Vec<String> {
        let Statement::Query(query) = stmt else {
            return Vec::new();
        };
        let Some(order_by) = &query.order_by else {
            return Vec::new();
        };
        let OrderByKind::Expressions(items) = &order_by.kind else {
            return Vec::new();
        };
        items
            .iter()
            .filter(|item| is_ordinal(&item.expr))
            .map(|_| {
                "ORDER BY <position> breaks when the SELECT list changes; order by column name"
                    .to_string()
            })
            .collect()
    }
}

fn is_ordinal(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::Value(v) if matches!(v.value, Value::Number(..))
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::check_sql;

    #[test]
    fn flags_positional_order_by() {
        assert_eq!(
            check_sql(
                &OrderByOrdinal,
                "SELECT id, name FROM users ORDER BY 1",
                DialectKind::Postgres
            )
            .len(),
            1
        );
        assert_eq!(
            check_sql(
                &OrderByOrdinal,
                "SELECT id, name FROM users ORDER BY 1, name",
                DialectKind::Postgres
            )
            .len(),
            1
        );
    }

    #[test]
    fn allows_named_columns() {
        assert!(
            check_sql(
                &OrderByOrdinal,
                "SELECT id, name FROM users ORDER BY name",
                DialectKind::Postgres
            )
            .is_empty()
        );
    }
}
