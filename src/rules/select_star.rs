use sqlparser::ast::{SelectItem, Statement};

use super::{Rule, for_each_select};
use crate::parser::DialectKind;
use crate::report::Severity;

/// `SELECT *` / `SELECT t.*`: brittle to schema changes, drags every column
/// over the wire, and defeats covering indexes.
pub struct SelectStar;

impl Rule for SelectStar {
    fn id(&self) -> &'static str {
        "select-star"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn description(&self) -> &'static str {
        "SELECT * or table.* — list columns explicitly"
    }

    fn check(&self, stmt: &Statement, _dialect: DialectKind) -> Vec<String> {
        let mut messages = Vec::new();
        // ponytail: for_each_select covers top-level selects, set-op arms and
        // CTE bodies; subqueries nested inside expressions are not visited
        // yet — walk Expr when needed.
        for_each_select(stmt, &mut |select| {
            for item in &select.projection {
                match item {
                    SelectItem::Wildcard(_) => {
                        messages.push("SELECT * selects every column; list them explicitly".into())
                    }
                    SelectItem::QualifiedWildcard(name, _) => messages.push(format!(
                        "SELECT {name}.* selects every column of {name}; list them explicitly"
                    )),
                    SelectItem::UnnamedExpr(_)
                    | SelectItem::ExprWithAlias { .. }
                    | SelectItem::ExprWithAliases { .. } => {}
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
    fn flags_star_and_qualified_star() {
        assert_eq!(
            check_sql(&SelectStar, "SELECT * FROM users", DialectKind::Postgres).len(),
            1
        );
        assert_eq!(
            check_sql(
                &SelectStar,
                "SELECT u.* FROM users u",
                DialectKind::Postgres
            )
            .len(),
            1
        );
    }

    #[test]
    fn covers_set_operation_arms() {
        assert_eq!(
            check_sql(
                &SelectStar,
                "SELECT id FROM users UNION SELECT * FROM admins",
                DialectKind::Postgres
            )
            .len(),
            1
        );
    }

    #[test]
    fn covers_cte_bodies() {
        assert_eq!(
            check_sql(
                &SelectStar,
                "WITH top AS (SELECT * FROM users) SELECT id FROM top",
                DialectKind::Postgres
            )
            .len(),
            1
        );
    }

    #[test]
    fn allows_explicit_columns() {
        assert!(
            check_sql(
                &SelectStar,
                "SELECT id, name FROM users",
                DialectKind::Sqlite
            )
            .is_empty()
        );
    }
}
