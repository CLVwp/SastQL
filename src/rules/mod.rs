pub mod case_without_else;
pub mod destructive_ddl;
pub mod dialect_portability;
pub mod float_money;
pub mod grant_all;
pub mod insert_without_columns;
pub mod join_without_on;
pub mod leading_wildcard_like;
pub mod limit_without_order_by;
pub mod missing_limit;
pub mod naive_timestamp;
pub mod natural_join;
pub mod no_where_on_mutation;
pub mod not_in_null_trap;
pub mod null_comparison;
pub mod offset_without_limit;
pub mod order_by_ordinal;
pub mod order_by_random;
pub mod select_star;
pub mod select_star_plus;
pub mod string_plus;
pub mod table_no_pk;
pub mod type_affinity;
pub mod union_without_all;
pub mod weak_hash;

use sqlparser::ast::{JoinConstraint, JoinOperator, Query, Select, SetExpr, Statement};

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
        Box::new(null_comparison::NullComparison),
        Box::new(dialect_portability::DialectPortability),
        Box::new(type_affinity::TypeAffinity),
        Box::new(case_without_else::CaseWithoutElse),
        Box::new(join_without_on::JoinWithoutOn),
        Box::new(natural_join::NaturalJoin),
        Box::new(insert_without_columns::InsertWithoutColumns),
        Box::new(offset_without_limit::OffsetWithoutLimit),
        Box::new(order_by_random::OrderByRandom),
        Box::new(leading_wildcard_like::LeadingWildcardLike),
        Box::new(string_plus::StringPlus),
        Box::new(weak_hash::WeakHash),
        Box::new(select_star_plus::SelectStarPlus),
        Box::new(grant_all::GrantAll),
        Box::new(table_no_pk::TableNoPk),
        Box::new(float_money::FloatMoney),
        Box::new(union_without_all::UnionWithoutAll),
        Box::new(order_by_ordinal::OrderByOrdinal),
        Box::new(naive_timestamp::NaiveTimestamp),
        Box::new(limit_without_order_by::LimitWithoutOrderBy),
        Box::new(destructive_ddl::DestructiveDdl),
    ]
}

/// What runs when no `--rules` is given: everything except opt-in rules.
pub fn default_rules() -> Vec<Box<dyn Rule>> {
    all_rules()
        .into_iter()
        .filter(|rule| !rule.opt_in())
        .collect()
}

/// Walks every `Select` of a statement's queries: top-level body,
/// set-operation arms, and CTE bodies. Subqueries nested inside expressions
/// are not visited (`select_star` documents the same ceiling).
pub(crate) fn for_each_select(stmt: &Statement, f: &mut impl FnMut(&Select)) {
    if let Statement::Query(query) = stmt {
        walk_query_selects(query, f);
    }
}

fn walk_query_selects(query: &Query, f: &mut impl FnMut(&Select)) {
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            walk_query_selects(&cte.query, f);
        }
    }
    walk_set_expr_selects(&query.body, f);
}

fn walk_set_expr_selects(set_expr: &SetExpr, f: &mut impl FnMut(&Select)) {
    match set_expr {
        SetExpr::Select(select) => f(select),
        SetExpr::Query(inner) => walk_query_selects(inner, f),
        SetExpr::SetOperation { left, right, .. } => {
            walk_set_expr_selects(left, f);
            walk_set_expr_selects(right, f);
        }
        _ => {}
    }
}

/// The constraint carried by a join operator, if any (`CrossApply` /
/// `OuterApply` have none).
pub(crate) fn join_constraint(op: &JoinOperator) -> Option<&JoinConstraint> {
    match op {
        JoinOperator::Join(c)
        | JoinOperator::Inner(c)
        | JoinOperator::Left(c)
        | JoinOperator::LeftOuter(c)
        | JoinOperator::Right(c)
        | JoinOperator::RightOuter(c)
        | JoinOperator::FullOuter(c)
        | JoinOperator::CrossJoin(c)
        | JoinOperator::Semi(c)
        | JoinOperator::LeftSemi(c)
        | JoinOperator::RightSemi(c)
        | JoinOperator::Anti(c)
        | JoinOperator::LeftAnti(c)
        | JoinOperator::RightAnti(c)
        | JoinOperator::StraightJoin(c) => Some(c),
        // CrossApply/OuterApply, ClickHouse ASOF and the ARRAY JOIN family
        // carry no JoinConstraint.
        _ => None,
    }
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
        for id in [
            "missing-limit",
            "union-without-all",
            "order-by-ordinal",
            "naive-timestamp",
            "limit-without-order-by",
            "destructive-ddl",
        ] {
            assert!(all_rules().iter().any(|rule| rule.id() == id));
            assert!(
                !default_rules().iter().any(|rule| rule.id() == id),
                "{id} must stay opt-in"
            );
        }
        // Ids are unique — CI filters on them.
        let mut ids: Vec<_> = all_rules().iter().map(|rule| rule.id()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), all_rules().len());
    }
}
