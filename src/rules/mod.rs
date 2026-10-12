pub mod dialect_portability;
pub mod missing_limit;
pub mod no_where_on_mutation;
pub mod not_in_null_trap;
pub mod select_star;
pub mod type_affinity;

use sqlparser::ast::Statement;

use crate::parser::DialectKind;
use crate::report::Severity;

/// One rule = one violation producer. Rule ids are kebab-case and stable
/// once released (CI depends on them).
pub trait Rule {
    fn id(&self) -> &'static str;
    fn severity(&self) -> Severity;
    fn description(&self) -> &'static str;
    /// Returns one message per violation (empty = clean).
    fn check(&self, stmt: &Statement, dialect: DialectKind) -> Vec<String>;
    /// Opt-in rules only run when selected via `--rules`.
    fn opt_in(&self) -> bool {
        false
    }
}

/// Registry: register every rule here. `--rules` filters this list by id.
pub fn all_rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(select_star::SelectStar),
        Box::new(no_where_on_mutation::NoWhereOnMutation),
        Box::new(missing_limit::MissingLimit),
        Box::new(not_in_null_trap::NotInNullTrap),
        Box::new(dialect_portability::DialectPortability),
        Box::new(type_affinity::TypeAffinity),
    ]
}

/// What runs when no `--rules` is given: everything except opt-in rules.
pub fn default_rules() -> Vec<Box<dyn Rule>> {
    all_rules()
        .into_iter()
        .filter(|rule| !rule.opt_in())
        .collect()
}

/// Shared test helper: parse `sql` with `dialect` and run `rule` over it.
#[cfg(test)]
pub(crate) fn check_sql(rule: &dyn Rule, sql: &str, dialect: DialectKind) -> Vec<String> {
    crate::parser::parse(sql, dialect)
        .expect("test SQL must parse")
        .iter()
        .flat_map(|stmt| rule.check(stmt, dialect))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opt_in_rules_are_not_in_defaults() {
        assert!(all_rules().iter().any(|rule| rule.id() == "missing-limit"));
        assert!(
            !default_rules()
                .iter()
                .any(|rule| rule.id() == "missing-limit")
        );
        // Ids are unique — CI filters on them.
        let mut ids: Vec<_> = all_rules().iter().map(|rule| rule.id()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), all_rules().len());
    }
}
