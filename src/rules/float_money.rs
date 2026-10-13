use sqlparser::ast::{DataType, Statement};

use super::Rule;
use crate::parser::DialectKind;
use crate::report::Severity;

/// Floating point cannot represent most decimal amounts exactly (`0.1 +
/// 0.2 != 0.3`): money columns must be `NUMERIC`/`DECIMAL`.
pub struct FloatMoney;

impl Rule for FloatMoney {
    fn id(&self) -> &'static str {
        "float-money"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn description(&self) -> &'static str {
        "floating point column for a money-like name — use NUMERIC/DECIMAL"
    }

    fn check(&self, stmt: &Statement, _dialect: DialectKind) -> Vec<String> {
        let Statement::CreateTable(create) = stmt else {
            return Vec::new();
        };
        // ponytail: name heuristic (exact token split on '_') — total_count
        // fires too; tighten the token list if real-world noise shows up.
        const MONEY_TOKENS: &[&str] = &[
            "price", "amount", "total", "cost", "balance", "salary", "fee", "rate",
        ];
        create
            .columns
            .iter()
            .filter(|col| is_float(&col.data_type))
            .filter(|col| {
                col.name
                    .value
                    .to_lowercase()
                    .split('_')
                    .any(|token| MONEY_TOKENS.contains(&token))
            })
            .map(|col| {
                format!(
                    "column {table}.{col} stores money as floating point; use NUMERIC/DECIMAL",
                    table = create.name,
                    col = col.name
                )
            })
            .collect()
    }
}

fn is_float(data_type: &DataType) -> bool {
    matches!(
        data_type,
        DataType::Float(_) | DataType::Real | DataType::Double(_) | DataType::DoublePrecision
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::check_sql;

    #[test]
    fn flags_float_money_columns() {
        assert_eq!(
            check_sql(
                &FloatMoney,
                "CREATE TABLE invoices (unit_price REAL, memo TEXT)",
                DialectKind::Postgres
            )
            .len(),
            1
        );
        assert_eq!(
            check_sql(
                &FloatMoney,
                "CREATE TABLE invoices (grand_total DOUBLE PRECISION, fee_rate FLOAT)",
                DialectKind::Postgres
            )
            .len(),
            2
        );
    }

    #[test]
    fn allows_numeric_and_unrelated_names() {
        assert!(
            check_sql(
                &FloatMoney,
                "CREATE TABLE invoices (total_price NUMERIC(12,2), note TEXT, updated_at TIMESTAMP)",
                DialectKind::Postgres
            )
            .is_empty()
        );
    }
}
