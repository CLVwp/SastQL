use sqlparser::ast::Statement;

use crate::parser::DialectKind;
use crate::report::Violation;
use crate::rules::Rule;

pub struct Analyzer {
    pub rules: Vec<Box<dyn Rule>>,
}

impl Analyzer {
    /// Run every rule over every statement of one parsed file.
    pub fn analyze(
        &self,
        file: &str,
        statements: &[Statement],
        dialect: DialectKind,
    ) -> Vec<Violation> {
        let mut violations = Vec::new();
        for stmt in statements {
            for rule in &self.rules {
                let messages = rule.check(stmt, dialect);
                if messages.is_empty() {
                    continue;
                }
                let snippet = stmt.to_string();
                for message in messages {
                    violations.push(Violation {
                        file: file.into(),
                        rule_id: rule.id(),
                        severity: rule.severity(),
                        message,
                        snippet: snippet.clone(),
                    });
                }
            }
        }
        violations
    }
}
