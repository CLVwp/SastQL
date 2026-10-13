use sqlparser::ast::{JoinConstraint, Select, Statement};

use super::{Rule, for_each_select, join_constraint};
use crate::parser::DialectKind;
use crate::report::Severity;

/// A `JOIN` without `ON`/`USING` — and a comma-separated `FROM` — is a
/// cartesian product: every row of one table paired with every row of the
/// other. Almost always a forgotten join condition.
pub struct JoinWithoutOn;

impl Rule for JoinWithoutOn {
    fn id(&self) -> &'static str {
        "join-without-on"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn description(&self) -> &'static str {
        "JOIN without ON/USING or comma-separated FROM — implicit cartesian product"
    }

    fn check(&self, stmt: &Statement, _dialect: DialectKind) -> Vec<String> {
        let mut messages = Vec::new();
        for_each_select(stmt, &mut |select| check_select(select, &mut messages));
        messages
    }
}

fn check_select(select: &Select, messages: &mut Vec<String>) {
    if select.from.len() > 1 {
        // `FROM a, b` — sqlparser parses this as several TableWithJoins.
        messages.push(format!(
            "FROM lists {} tables separated by commas (implicit cross join); use explicit JOIN ... ON",
            select.from.len()
        ));
    }
    for table_with_joins in &select.from {
        for join in &table_with_joins.joins {
            let unconstrained = matches!(
                join_constraint(&join.join_operator),
                Some(JoinConstraint::None)
            );
            // CROSS JOIN carries JoinConstraint::None too, but it is the
            // explicit way to ask for a cartesian product — not flagged.
            let is_cross = matches!(
                join.join_operator,
                sqlparser::ast::JoinOperator::CrossJoin(_)
            );
            if unconstrained && !is_cross {
                messages.push(format!(
                    "JOIN to {relation} has no ON or USING condition; it pairs every row of both tables",
                    relation = join.relation
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::check_sql;

    #[test]
    fn flags_bare_join_and_comma_from() {
        assert_eq!(
            check_sql(
                &JoinWithoutOn,
                "SELECT u.id FROM users u JOIN orders o",
                DialectKind::Postgres
            )
            .len(),
            1
        );
        assert_eq!(
            check_sql(
                &JoinWithoutOn,
                "SELECT u.id FROM users u, orders o",
                DialectKind::Postgres
            )
            .len(),
            1
        );
    }

    #[test]
    fn allows_on_using_and_cross_join() {
        assert!(
            check_sql(
                &JoinWithoutOn,
                "SELECT u.id FROM users u JOIN orders o ON o.user_id = u.id",
                DialectKind::Postgres
            )
            .is_empty()
        );
        assert!(
            check_sql(
                &JoinWithoutOn,
                "SELECT id FROM users JOIN orders USING (id)",
                DialectKind::Postgres
            )
            .is_empty()
        );
        assert!(
            check_sql(
                &JoinWithoutOn,
                "SELECT u.id FROM users u CROSS JOIN settings s",
                DialectKind::Postgres
            )
            .is_empty()
        );
    }

    #[test]
    fn covers_cte_bodies() {
        assert_eq!(
            check_sql(
                &JoinWithoutOn,
                "WITH top AS (SELECT id FROM users JOIN orders) SELECT id FROM top",
                DialectKind::Postgres
            )
            .len(),
            1
        );
    }
}
