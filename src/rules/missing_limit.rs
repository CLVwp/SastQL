use sqlparser::ast::Statement;

use super::{Rule, has_limit};
use crate::parser::DialectKind;
use crate::report::Severity;

/// A bare `SELECT` without `LIMIT`/`FETCH` can return unbounded rows.
/// Opt-in: too common in ad-hoc scripts to be on by default.
pub struct MissingLimit;

impl Rule for MissingLimit {
    fn id(&self) -> &'static str {
        "missing-limit"
    }

    fn severity(&self) -> Severity {
        Severity::Info
    }

    fn description(&self) -> &'static str {
        "SELECT without LIMIT can return unbounded rows"
    }

    fn opt_in(&self) -> bool {
        true
    }

    fn check(&self, stmt: &Statement, _dialect: DialectKind) -> Vec<String> {
        let Statement::Query(query) = stmt else {
            return Vec::new();
        };
        // ponytail: top-level query only; CTE/subquery LIMITs are not inspected.
        if has_limit(query) {
            Vec::new()
        } else {
            vec!["SELECT has no LIMIT; it can return unbounded rows".into()]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::check_sql;

    #[test]
    fn flags_select_without_limit() {
        assert_eq!(
            check_sql(&MissingLimit, "SELECT id FROM users", DialectKind::Postgres).len(),
            1
        );
    }

    #[test]
    fn allows_limit_offset_and_fetch() {
        assert!(
            check_sql(
                &MissingLimit,
                "SELECT id FROM users LIMIT 10",
                DialectKind::Postgres
            )
            .is_empty()
        );
        assert!(
            check_sql(
                &MissingLimit,
                "SELECT id FROM users OFFSET 5 ROWS FETCH NEXT 10 ROWS ONLY",
                DialectKind::Postgres
            )
            .is_empty()
        );
    }

    #[test]
    fn ignores_non_select_statements() {
        assert!(
            check_sql(
                &MissingLimit,
                "UPDATE users SET id = 1 WHERE id = 1",
                DialectKind::Sqlite
            )
            .is_empty()
        );
    }
}
