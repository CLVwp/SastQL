use sqlparser::ast::{DataType, Statement, TimezoneInfo};

use super::Rule;
use crate::parser::DialectKind;
use crate::report::Severity;

/// A PostgreSQL `TIMESTAMP` without time zone silently drops the offset on
/// write and guesses one on read — `TIMESTAMPTZ` stores a real instant.
pub struct NaiveTimestamp;

impl Rule for NaiveTimestamp {
    fn id(&self) -> &'static str {
        "naive-timestamp"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn description(&self) -> &'static str {
        "PostgreSQL: TIMESTAMP without time zone loses the offset; prefer TIMESTAMPTZ"
    }

    fn opt_in(&self) -> bool {
        true
    }

    fn check(&self, stmt: &Statement, dialect: DialectKind) -> Vec<String> {
        // SQLite has no timezone-aware types — type-affinity owns that side.
        if dialect != DialectKind::Postgres {
            return Vec::new();
        }
        let Statement::CreateTable(create) = stmt else {
            return Vec::new();
        };
        create
            .columns
            .iter()
            .filter(|col| matches!(col.data_type, DataType::Timestamp(_, TimezoneInfo::None)))
            .map(|col| {
                format!(
                    "column {table}.{col} uses TIMESTAMP without time zone; prefer TIMESTAMPTZ",
                    table = create.name,
                    col = col.name
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::check_sql;

    #[test]
    fn flags_naive_timestamp_in_postgres() {
        assert_eq!(
            check_sql(
                &NaiveTimestamp,
                "CREATE TABLE events (created TIMESTAMP)",
                DialectKind::Postgres
            )
            .len(),
            1
        );
    }

    #[test]
    fn allows_timestamptz() {
        assert!(
            check_sql(
                &NaiveTimestamp,
                "CREATE TABLE events (created TIMESTAMPTZ)",
                DialectKind::Postgres
            )
            .is_empty()
        );
        assert!(
            check_sql(
                &NaiveTimestamp,
                "CREATE TABLE events (created TIMESTAMP WITH TIME ZONE)",
                DialectKind::Postgres
            )
            .is_empty()
        );
    }

    #[test]
    fn is_postgres_only() {
        assert!(
            check_sql(
                &NaiveTimestamp,
                "CREATE TABLE events (created TIMESTAMP)",
                DialectKind::Sqlite
            )
            .is_empty()
        );
    }
}
