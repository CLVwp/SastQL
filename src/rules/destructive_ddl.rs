use sqlparser::ast::{AlterTableOperation, ObjectType, Statement};

use super::Rule;
use crate::parser::DialectKind;
use crate::report::Severity;

/// `DROP TABLE`/`SCHEMA`/`DATABASE`, `TRUNCATE` and `ALTER TABLE DROP
/// COLUMN` destroy data irrecoverably. Opt-in: migrations legitimately
/// contain them — run this rule on production-bound scripts.
pub struct DestructiveDdl;

impl Rule for DestructiveDdl {
    fn id(&self) -> &'static str {
        "destructive-ddl"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn description(&self) -> &'static str {
        "DDL that destroys data irrecoverably (DROP, TRUNCATE, DROP COLUMN)"
    }

    fn opt_in(&self) -> bool {
        true
    }

    fn check(&self, stmt: &Statement, _dialect: DialectKind) -> Vec<String> {
        match stmt {
            Statement::Drop { object_type, .. }
                if matches!(
                    object_type,
                    ObjectType::Table | ObjectType::Schema | ObjectType::Database
                ) =>
            {
                vec![format!(
                    "DROP {object_type} destroys the objects and their data irrecoverably",
                )]
            }
            Statement::Truncate(_) => {
                vec!["TRUNCATE removes every row of the table irrecoverably".to_string()]
            }
            Statement::AlterTable(alter)
                if alter
                    .operations
                    .iter()
                    .any(|op| matches!(op, AlterTableOperation::DropColumn { .. })) =>
            {
                vec![format!(
                    "ALTER TABLE {table} DROP COLUMN destroys the column data irrecoverably",
                    table = alter.name
                )]
            }
            _ => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::check_sql;

    #[test]
    fn flags_drop_truncate_and_drop_column() {
        for sql in [
            "DROP TABLE legacy_users",
            "TRUNCATE TABLE audit_log",
            "ALTER TABLE sessions DROP COLUMN token",
        ] {
            assert_eq!(
                check_sql(&DestructiveDdl, sql, DialectKind::Postgres).len(),
                1,
                "expected 1 hit for: {sql}"
            );
        }
    }

    #[test]
    fn allows_other_ddl_and_views() {
        assert!(
            check_sql(
                &DestructiveDdl,
                "ALTER TABLE users ADD COLUMN note TEXT",
                DialectKind::Postgres
            )
            .is_empty()
        );
        assert!(
            check_sql(
                &DestructiveDdl,
                "DROP VIEW stale_report",
                DialectKind::Postgres
            )
            .is_empty()
        );
    }
}
