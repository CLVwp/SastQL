use sqlparser::ast::{DataType, Statement};

use super::Rule;
use crate::parser::DialectKind;
use crate::report::Severity;

/// SQLite turns every declared type into one of 5 affinities. `DATETIME`
/// gets NUMERIC affinity, which mangles ISO-8601 strings on read — TEXT is
/// the right declaration.
pub struct TypeAffinity;

impl Rule for TypeAffinity {
    fn id(&self) -> &'static str {
        "type-affinity"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn description(&self) -> &'static str {
        "Column type with surprising affinity in SQLite (DATETIME/TIMESTAMP -> NUMERIC)"
    }

    fn check(&self, stmt: &Statement, dialect: DialectKind) -> Vec<String> {
        let Statement::CreateTable(create) = stmt else {
            return Vec::new();
        };
        if dialect != DialectKind::Sqlite {
            return Vec::new();
        }
        // ponytail: DATETIME/TIMESTAMP only; BOOLEAN and TINYINT hit the same
        // NUMERIC affinity — add arms when it earns its place.
        create
            .columns
            .iter()
            .filter(|col| {
                matches!(
                    col.data_type,
                    DataType::Datetime(_) | DataType::Timestamp(_, _)
                )
            })
            .map(|col| format!(
                "column {} is declared {} which gets NUMERIC affinity in SQLite; declare TEXT for ISO-8601 strings",
                col.name, col.data_type
            ))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::check_sql;

    #[test]
    fn flags_datetime_columns_for_sqlite() {
        assert_eq!(
            check_sql(
                &TypeAffinity,
                "CREATE TABLE events (id INTEGER, created DATETIME)",
                DialectKind::Sqlite
            )
            .len(),
            1
        );
    }

    #[test]
    fn ignores_other_types_and_dialects() {
        assert!(
            check_sql(
                &TypeAffinity,
                "CREATE TABLE events (id INTEGER, created TEXT)",
                DialectKind::Sqlite
            )
            .is_empty()
        );
        // The matrix marks this rule SQLite-only: Postgres has real types.
        assert!(
            check_sql(
                &TypeAffinity,
                "CREATE TABLE events (id INTEGER, created DATETIME)",
                DialectKind::Postgres
            )
            .is_empty()
        );
    }
}
