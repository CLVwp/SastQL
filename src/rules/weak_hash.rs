use std::ops::ControlFlow;

use sqlparser::ast::visit_expressions;
use sqlparser::ast::{Expr, Function, Statement};

use super::Rule;
use crate::parser::DialectKind;
use crate::report::Severity;

/// `md5()` and `sha1()` are broken for security purposes (collisions, GPU
/// brute-force) — flagged wherever they appear.
pub struct WeakHash;

impl Rule for WeakHash {
    fn id(&self) -> &'static str {
        "weak-hash"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn description(&self) -> &'static str {
        "md5()/sha1() are cryptographically broken; use a modern hash or KDF"
    }

    fn check(&self, stmt: &Statement, _dialect: DialectKind) -> Vec<String> {
        let mut messages = Vec::new();
        let _ = visit_expressions(stmt, |expr| {
            if let Expr::Function(f) = expr {
                if let Some(name) = weak_name(f) {
                    messages.push(format!(
                        "{name}() is a weak hash (collisions, GPU brute-force); use SHA-256+ or a password KDF"
                    ));
                }
            }
            ControlFlow::<()>::Continue(())
        });
        messages
    }
}

const WEAK_HASHES: &[&str] = &["md5", "sha1"];

fn weak_name(f: &Function) -> Option<String> {
    // Same idiom as dialect_portability: last dotted segment, case-insensitive,
    // so pg_catalog.md5() is caught too.
    let name = f.name.to_string().to_lowercase();
    let last = name.rsplit('.').next()?;
    WEAK_HASHES.contains(&last).then(|| last.to_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::check_sql;

    #[test]
    fn flags_md5_and_sha1_any_case() {
        assert_eq!(
            check_sql(
                &WeakHash,
                "SELECT md5(email) FROM users",
                DialectKind::Postgres
            )
            .len(),
            1
        );
        assert_eq!(
            check_sql(&WeakHash, "SELECT SHA1('x')", DialectKind::Postgres).len(),
            1
        );
    }

    #[test]
    fn catches_qualified_names() {
        assert_eq!(
            check_sql(
                &WeakHash,
                "SELECT pg_catalog.md5(email) FROM users",
                DialectKind::Postgres
            )
            .len(),
            1
        );
    }

    #[test]
    fn allows_strong_hashes() {
        assert!(check_sql(&WeakHash, "SELECT sha256('x')", DialectKind::Postgres).is_empty());
    }
}
