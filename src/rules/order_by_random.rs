use sqlparser::ast::{Expr, OrderByKind, Statement};

use super::Rule;
use crate::parser::DialectKind;
use crate::report::Severity;

/// `ORDER BY random()` computes a random value for every row and sorts the
/// whole table — cost grows with the data, not the result size.
pub struct OrderByRandom;

impl Rule for OrderByRandom {
    fn id(&self) -> &'static str {
        "order-by-random"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn description(&self) -> &'static str {
        "ORDER BY random() sorts the whole table to pick rows at random"
    }

    fn check(&self, stmt: &Statement, _dialect: DialectKind) -> Vec<String> {
        let Statement::Query(query) = stmt else {
            return Vec::new();
        };
        // ponytail: top-level ORDER BY only; CTE/subquery ORDER BY is not
        // inspected (same ceiling as missing-limit).
        let Some(order_by) = &query.order_by else {
            return Vec::new();
        };
        let OrderByKind::Expressions(items) = &order_by.kind else {
            return Vec::new();
        };
        items
            .iter()
            .filter(|item| is_random(&item.expr))
            .map(|_| {
                "ORDER BY random() sorts every row to sample results; consider an indexed random pick"
                    .to_string()
            })
            .collect()
    }
}

fn is_random(expr: &Expr) -> bool {
    let Expr::Function(f) = expr else {
        return false;
    };
    // Same idiom as dialect_portability: last dotted segment, case-insensitive,
    // so pg_catalog.random() is caught too.
    f.name
        .to_string()
        .to_lowercase()
        .rsplit('.')
        .next()
        .is_some_and(|last| last == "random")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::check_sql;

    #[test]
    fn flags_random_in_order_by() {
        assert_eq!(
            check_sql(
                &OrderByRandom,
                "SELECT id FROM users ORDER BY RANDOM() LIMIT 1",
                DialectKind::Postgres
            )
            .len(),
            1
        );
        assert_eq!(
            check_sql(
                &OrderByRandom,
                "SELECT id FROM users ORDER BY random()",
                DialectKind::Sqlite
            )
            .len(),
            1
        );
    }

    #[test]
    fn allows_deterministic_order_and_random_elsewhere() {
        assert!(
            check_sql(
                &OrderByRandom,
                "SELECT id FROM users ORDER BY created_at",
                DialectKind::Postgres
            )
            .is_empty()
        );
        assert!(
            check_sql(
                &OrderByRandom,
                "SELECT random() FROM users",
                DialectKind::Postgres
            )
            .is_empty()
        );
    }
}
