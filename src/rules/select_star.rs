use sqlparser::ast::{Query, SelectItem, SetExpr, Statement};

use super::Rule;
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
        if let Statement::Query(query) = stmt {
            check_query(query, &mut messages);
        }
        messages
    }
}

fn check_query(query: &Query, messages: &mut Vec<String>) {
    check_set_expr(&query.body, messages);
}

// ponytail: covers top-level selects and set-operation arms; subqueries
// nested inside WHERE expressions are not visited yet — walk Expr when needed.
fn check_set_expr(expr: &SetExpr, messages: &mut Vec<String>) {
    match expr {
        SetExpr::Select(select) => {
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
        }
        SetExpr::Query(inner) => check_query(inner, messages),
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, messages);
            check_set_expr(right, messages);
        }
        _ => {}
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
