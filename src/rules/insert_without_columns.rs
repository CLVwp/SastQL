use sqlparser::ast::Statement;

use super::Rule;
use crate::parser::DialectKind;
use crate::report::Severity;

/// `INSERT INTO t VALUES (...)` without a column list silently binds values
/// to whatever the current column order is — a schema change away from
/// corrupting data.
pub struct InsertWithoutColumns;

impl Rule for InsertWithoutColumns {
    fn id(&self) -> &'static str {
        "insert-without-columns"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn description(&self) -> &'static str {
        "INSERT without a column list binds values to the current column order"
    }

    fn check(&self, stmt: &Statement, _dialect: DialectKind) -> Vec<String> {
        let Statement::Insert(insert) = stmt else {
            return Vec::new();
        };
        if insert.columns.is_empty() && insert.source.is_some() {
            vec![format!(
                "INSERT INTO {table} lists no columns; name them explicitly so column order changes cannot corrupt data",
                table = insert.table
            )]
        } else {
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::check_sql;

    #[test]
    fn flags_values_and_insert_select() {
        assert_eq!(
            check_sql(
                &InsertWithoutColumns,
                "INSERT INTO audit_log VALUES (1, 'x')",
                DialectKind::Postgres
            )
            .len(),
            1
        );
        assert_eq!(
            check_sql(
                &InsertWithoutColumns,
                "INSERT INTO audit_log SELECT id, action FROM staging",
                DialectKind::Postgres
            )
            .len(),
            1
        );
    }

    #[test]
    fn allows_explicit_column_list() {
        assert!(
            check_sql(
                &InsertWithoutColumns,
                "INSERT INTO audit_log (id, action) VALUES (1, 'x')",
                DialectKind::Postgres
            )
            .is_empty()
        );
    }
}
