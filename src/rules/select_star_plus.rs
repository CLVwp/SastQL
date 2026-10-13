use sqlparser::ast::{Select, SelectItem, Statement};

use super::{Rule, for_each_select};
use crate::parser::DialectKind;
use crate::report::Severity;

/// `SELECT *, col` drags every column AND names one explicitly — the
/// explicit name breaks the moment a column is renamed, while `*` keeps
/// the query opaque. Pick one style.
pub struct SelectStarPlus;

impl Rule for SelectStarPlus {
    fn id(&self) -> &'static str {
        "select-star-plus"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn description(&self) -> &'static str {
        "SELECT mixes * with explicit columns — brittle and self-defeating"
    }

    fn check(&self, stmt: &Statement, _dialect: DialectKind) -> Vec<String> {
        let mut messages = Vec::new();
        for_each_select(stmt, &mut |select| {
            if mixes_wildcard_with_columns(select) {
                messages.push(
                    "SELECT mixes * with explicit columns; the explicit ones break on renames — list all columns explicitly instead"
                        .into(),
                );
            }
        });
        messages
    }
}

fn mixes_wildcard_with_columns(select: &Select) -> bool {
    let has_wildcard = select.projection.iter().any(|item| {
        matches!(
            item,
            SelectItem::Wildcard(_) | SelectItem::QualifiedWildcard(..)
        )
    });
    let has_explicit = select.projection.iter().any(|item| {
        matches!(
            item,
            SelectItem::UnnamedExpr(_)
                | SelectItem::ExprWithAlias { .. }
                | SelectItem::ExprWithAliases { .. }
        )
    });
    has_wildcard && has_explicit
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::check_sql;

    #[test]
    fn flags_wildcard_with_explicit_columns() {
        assert_eq!(
            check_sql(
                &SelectStarPlus,
                "SELECT *, created_at FROM users",
                DialectKind::Postgres
            )
            .len(),
            1
        );
        assert_eq!(
            check_sql(
                &SelectStarPlus,
                "SELECT u.*, u.id FROM users u",
                DialectKind::Postgres
            )
            .len(),
            1
        );
    }

    #[test]
    fn allows_pure_star_and_pure_columns() {
        assert!(
            check_sql(
                &SelectStarPlus,
                "SELECT * FROM users",
                DialectKind::Postgres
            )
            .is_empty()
        );
        assert!(
            check_sql(
                &SelectStarPlus,
                "SELECT id, created_at FROM users",
                DialectKind::Postgres
            )
            .is_empty()
        );
    }
}
