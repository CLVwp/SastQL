use sqlparser::ast::{ColumnOption, Statement, TableConstraint};

use super::Rule;
use crate::parser::DialectKind;
use crate::report::Severity;

/// A table without `PRIMARY KEY` has no guaranteed row identity: updates
/// and deletes can hit several rows, replication and ORMs misbehave.
pub struct TableNoPk;

impl Rule for TableNoPk {
    fn id(&self) -> &'static str {
        "table-no-pk"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn description(&self) -> &'static str {
        "CREATE TABLE without a PRIMARY KEY — no guaranteed row identity"
    }

    fn check(&self, stmt: &Statement, _dialect: DialectKind) -> Vec<String> {
        let Statement::CreateTable(create) = stmt else {
            return Vec::new();
        };
        // CREATE TABLE ... AS SELECT declares no columns to inspect.
        if create.query.is_some() {
            return Vec::new();
        }
        let table_level = create
            .constraints
            .iter()
            .any(|c| matches!(c, TableConstraint::PrimaryKey(_)));
        let column_level = create.columns.iter().any(|col| {
            col.options
                .iter()
                .any(|opt| matches!(opt.option, ColumnOption::PrimaryKey(_)))
        });
        if table_level || column_level {
            Vec::new()
        } else {
            vec![format!(
                "table {table} has no PRIMARY KEY; declare one so rows have a stable identity",
                table = create.name
            )]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::check_sql;

    #[test]
    fn flags_table_without_pk() {
        assert_eq!(
            check_sql(
                &TableNoPk,
                "CREATE TABLE logs (id INTEGER, msg TEXT)",
                DialectKind::Postgres
            )
            .len(),
            1
        );
    }

    #[test]
    fn allows_column_and_table_level_pk() {
        assert!(
            check_sql(
                &TableNoPk,
                "CREATE TABLE logs (id INTEGER PRIMARY KEY, msg TEXT)",
                DialectKind::Postgres
            )
            .is_empty()
        );
        assert!(
            check_sql(
                &TableNoPk,
                "CREATE TABLE logs (id INTEGER, msg TEXT, PRIMARY KEY (id))",
                DialectKind::Postgres
            )
            .is_empty()
        );
    }

    #[test]
    fn unique_is_not_a_pk_and_ctas_is_skipped() {
        assert_eq!(
            check_sql(
                &TableNoPk,
                "CREATE TABLE logs (id INTEGER UNIQUE, msg TEXT)",
                DialectKind::Postgres
            )
            .len(),
            1
        );
        assert!(
            check_sql(
                &TableNoPk,
                "CREATE TABLE logs AS SELECT id FROM events",
                DialectKind::Postgres
            )
            .is_empty()
        );
    }
}
