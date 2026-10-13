use sqlparser::ast::{SetExpr, SetOperator, SetQuantifier, Statement};

use super::Rule;
use crate::parser::DialectKind;
use crate::report::Severity;

/// A plain `UNION` de-duplicates its result with an implicit sort. Most
/// unions combine rows that cannot collide — `UNION ALL` skips the sort.
pub struct UnionWithoutAll;

impl Rule for UnionWithoutAll {
    fn id(&self) -> &'static str {
        "union-without-all"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn description(&self) -> &'static str {
        "UNION without ALL de-duplicates via a hidden sort"
    }

    fn opt_in(&self) -> bool {
        true
    }

    fn check(&self, stmt: &Statement, _dialect: DialectKind) -> Vec<String> {
        let Statement::Query(query) = stmt else {
            return Vec::new();
        };
        // ponytail: top-level set operation only; nested unions inside
        // parentheses are not visited yet.
        if let SetExpr::SetOperation {
            op: SetOperator::Union,
            set_quantifier: SetQuantifier::None,
            ..
        } = &*query.body
        {
            vec![
                "UNION de-duplicates with an implicit sort; use UNION ALL when duplicates are impossible or wanted"
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
    fn flags_plain_union() {
        assert_eq!(
            check_sql(
                &UnionWithoutAll,
                "SELECT id FROM users UNION SELECT id FROM admins",
                DialectKind::Postgres
            )
            .len(),
            1
        );
    }

    #[test]
    fn allows_all_and_explicit_distinct() {
        assert!(
            check_sql(
                &UnionWithoutAll,
                "SELECT id FROM users UNION ALL SELECT id FROM admins",
                DialectKind::Postgres
            )
            .is_empty()
        );
        assert!(
            check_sql(
                &UnionWithoutAll,
                "SELECT id FROM users UNION DISTINCT SELECT id FROM admins",
                DialectKind::Postgres
            )
            .is_empty()
        );
    }
}
