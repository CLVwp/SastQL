use sqlparser::ast::{FromTable, Statement};

use super::Rule;
use crate::parser::DialectKind;
use crate::report::Severity;

/// UPDATE/DELETE without WHERE touches every row — the classic production
/// outage. Severity: error.
pub struct NoWhereOnMutation;

impl Rule for NoWhereOnMutation {
    fn id(&self) -> &'static str {
        "no-where-on-mutation"
    }

    fn severity(&self) -> Severity {
        Severity::Error
    }

    fn description(&self) -> &'static str {
        "UPDATE or DELETE without WHERE affects every row"
    }

    fn check(&self, stmt: &Statement, _dialect: DialectKind) -> Vec<String> {
        match stmt {
            Statement::Update(update) if update.selection.is_none() => vec![format!(
                "UPDATE on {} has no WHERE clause; every row is affected",
                update.table
            )],
            Statement::Delete(delete) if delete.selection.is_none() => {
                let from = match &delete.from {
                    FromTable::WithFromKeyword(tables) | FromTable::WithoutKeyword(tables) => {
                        tables
                    }
                };
                vec![format!(
                    "DELETE from {} has no WHERE clause; every row is affected",
                    tables(from)
                )]
            }
            _ => Vec::new(),
        }
    }
}

fn tables(from: &[sqlparser::ast::TableWithJoins]) -> String {
    from.iter()
        .map(|t| t.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::check_sql;

    #[test]
    fn flags_update_and_delete_without_where() {
        assert_eq!(
            check_sql(
                &NoWhereOnMutation,
                "UPDATE users SET active = false",
                DialectKind::Postgres
            )
            .len(),
            1
        );
        assert_eq!(
            check_sql(
                &NoWhereOnMutation,
                "DELETE FROM users",
                DialectKind::Postgres
            )
            .len(),
            1
        );
    }

    #[test]
    fn allows_mutations_with_where() {
        assert!(
            check_sql(
                &NoWhereOnMutation,
                "DELETE FROM users WHERE id = 1",
                DialectKind::Sqlite
            )
            .is_empty()
        );
        assert!(
            check_sql(
                &NoWhereOnMutation,
                "UPDATE users SET active = false WHERE id = 1",
                DialectKind::Sqlite
            )
            .is_empty()
        );
    }

    #[test]
    fn ignores_select() {
        assert!(
            check_sql(
                &NoWhereOnMutation,
                "SELECT * FROM users",
                DialectKind::Postgres
            )
            .is_empty()
        );
    }
}
