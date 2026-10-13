use sqlparser::ast::{Privileges, Statement};

use super::Rule;
use crate::parser::DialectKind;
use crate::report::Severity;

/// `GRANT ALL` hands out every privilege — including the right to grant
/// privileges to others. Production grants should enumerate what is needed.
pub struct GrantAll;

impl Rule for GrantAll {
    fn id(&self) -> &'static str {
        "grant-all"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn description(&self) -> &'static str {
        "GRANT ALL gives every privilege; enumerate the ones actually needed"
    }

    fn check(&self, stmt: &Statement, _dialect: DialectKind) -> Vec<String> {
        let Statement::Grant(grant) = stmt else {
            return Vec::new();
        };
        if matches!(grant.privileges, Privileges::All { .. }) {
            vec![
                "GRANT ALL gives every privilege (including granting to others); list the needed ones"
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
    fn flags_all_privileges() {
        assert_eq!(
            check_sql(
                &GrantAll,
                "GRANT ALL PRIVILEGES ON orders TO app_admin",
                DialectKind::Postgres
            )
            .len(),
            1
        );
        assert_eq!(
            check_sql(
                &GrantAll,
                "GRANT ALL ON orders TO app_admin",
                DialectKind::Postgres
            )
            .len(),
            1
        );
    }

    #[test]
    fn allows_enumerated_privileges() {
        assert!(
            check_sql(
                &GrantAll,
                "GRANT SELECT, INSERT ON orders TO app_admin",
                DialectKind::Postgres
            )
            .is_empty()
        );
    }
}
