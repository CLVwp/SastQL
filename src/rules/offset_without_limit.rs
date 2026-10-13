use sqlparser::ast::{LimitClause, Statement};

use super::Rule;
use crate::parser::DialectKind;
use crate::report::Severity;

/// `OFFSET n` without `LIMIT` (valid in PostgreSQL) skips the first n rows
/// and returns everything after — usually a pagination `LIMIT` that was
/// lost along the way.
pub struct OffsetWithoutLimit;

impl Rule for OffsetWithoutLimit {
    fn id(&self) -> &'static str {
        "offset-without-limit"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn description(&self) -> &'static str {
        "OFFSET without LIMIT skips rows with no upper bound"
    }

    fn check(&self, stmt: &Statement, _dialect: DialectKind) -> Vec<String> {
        let Statement::Query(query) = stmt else {
            return Vec::new();
        };
        // ponytail: top-level query only, like missing-limit. A FETCH clause
        // bounds the result, so `OFFSET n ROWS FETCH NEXT ...` is fine.
        if query.fetch.is_some() {
            return Vec::new();
        }
        match &query.limit_clause {
            Some(LimitClause::LimitOffset {
                limit: None,
                offset: Some(_),
                ..
            }) => vec!["OFFSET without LIMIT skips rows with no bound; add a LIMIT".to_string()],
            // `LIMIT n OFFSET m` and `LIMIT offset, limit` both carry a limit.
            _ => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::check_sql;

    #[test]
    fn flags_bare_offset() {
        assert_eq!(
            check_sql(
                &OffsetWithoutLimit,
                "SELECT id FROM users OFFSET 10",
                DialectKind::Postgres
            )
            .len(),
            1
        );
    }

    #[test]
    fn allows_limit_offset_and_fetch() {
        assert!(
            check_sql(
                &OffsetWithoutLimit,
                "SELECT id FROM users LIMIT 5 OFFSET 10",
                DialectKind::Postgres
            )
            .is_empty()
        );
        assert!(
            check_sql(
                &OffsetWithoutLimit,
                "SELECT id FROM users OFFSET 10 ROWS FETCH NEXT 5 ROWS ONLY",
                DialectKind::Postgres
            )
            .is_empty()
        );
    }
}
