use sqlparser::ast::Statement;

use super::{Rule, has_limit};
use crate::parser::DialectKind;
use crate::report::Severity;

/// `LIMIT` without `ORDER BY` returns an arbitrary subset of rows: pages
/// overlap, drop or duplicate between runs. Opt-in: `LIMIT 1` existence
/// checks are a common false positive.
pub struct LimitWithoutOrderBy;

impl Rule for LimitWithoutOrderBy {
    fn id(&self) -> &'static str {
        "limit-without-order-by"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn description(&self) -> &'static str {
        "LIMIT/FETCH without ORDER BY returns an arbitrary subset of rows"
    }

    fn opt_in(&self) -> bool {
        true
    }

    fn check(&self, stmt: &Statement, _dialect: DialectKind) -> Vec<String> {
        let Statement::Query(query) = stmt else {
            return Vec::new();
        };
        // ponytail: top-level query only, like missing-limit. A bare SELECT
        // without limit belongs to missing-limit — this rule needs a limit.
        if has_limit(query) && query.order_by.is_none() {
            vec![
                "LIMIT without ORDER BY returns an arbitrary subset; add ORDER BY for deterministic paging"
                    .into(),
            ]
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
    fn flags_limit_without_order_by() {
        assert_eq!(
            check_sql(
                &LimitWithoutOrderBy,
                "SELECT id FROM users LIMIT 5",
                DialectKind::Postgres
            )
            .len(),
            1
        );
        assert_eq!(
            check_sql(
                &LimitWithoutOrderBy,
                "SELECT id FROM users FETCH FIRST 5 ROWS ONLY",
                DialectKind::Postgres
            )
            .len(),
            1
        );
    }

    #[test]
    fn allows_order_by_and_bare_select() {
        assert!(
            check_sql(
                &LimitWithoutOrderBy,
                "SELECT id FROM users ORDER BY id LIMIT 5",
                DialectKind::Postgres
            )
            .is_empty()
        );
        assert!(
            check_sql(
                &LimitWithoutOrderBy,
                "SELECT id FROM users",
                DialectKind::Postgres
            )
            .is_empty()
        );
    }
}
