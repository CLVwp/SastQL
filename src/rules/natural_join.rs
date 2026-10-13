use sqlparser::ast::{JoinConstraint, Statement};

use super::{Rule, for_each_select, join_constraint};
use crate::parser::DialectKind;
use crate::report::Severity;

/// `NATURAL JOIN` matches columns by name across both tables. Adding a
/// column to either table silently changes the join condition — brittle
/// enough to always flag.
pub struct NaturalJoin;

impl Rule for NaturalJoin {
    fn id(&self) -> &'static str {
        "natural-join"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn description(&self) -> &'static str {
        "NATURAL JOIN matches columns by name — schema changes silently alter the join"
    }

    fn check(&self, stmt: &Statement, _dialect: DialectKind) -> Vec<String> {
        let mut messages = Vec::new();
        for_each_select(stmt, &mut |select| {
            for table_with_joins in &select.from {
                for join in &table_with_joins.joins {
                    if matches!(
                        join_constraint(&join.join_operator),
                        Some(JoinConstraint::Natural)
                    ) {
                        messages.push(format!(
                            "NATURAL JOIN to {relation} matches columns by name; write the ON condition explicitly",
                            relation = join.relation
                        ));
                    }
                }
            }
        });
        messages
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::check_sql;

    #[test]
    fn flags_natural_join() {
        assert_eq!(
            check_sql(
                &NaturalJoin,
                "SELECT * FROM users NATURAL JOIN orders",
                DialectKind::Postgres
            )
            .len(),
            1
        );
        assert_eq!(
            check_sql(
                &NaturalJoin,
                "SELECT * FROM users NATURAL LEFT JOIN orders",
                DialectKind::Postgres
            )
            .len(),
            1
        );
    }

    #[test]
    fn allows_explicit_conditions() {
        assert!(
            check_sql(
                &NaturalJoin,
                "SELECT * FROM users u JOIN orders o ON o.user_id = u.id",
                DialectKind::Postgres
            )
            .is_empty()
        );
    }
}
