use sqlparser::ast::Statement;
use sqlparser::dialect::{Dialect, PostgreSqlDialect, SQLiteDialect};
use sqlparser::parser::{Parser, ParserError};

/// Target SQL dialect, selected from the CLI.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum DialectKind {
    Postgres,
    Sqlite,
}

fn dialect(kind: DialectKind) -> &'static dyn Dialect {
    match kind {
        DialectKind::Postgres => &PostgreSqlDialect {},
        DialectKind::Sqlite => &SQLiteDialect {},
    }
}

/// Parse one file's `sql` into statements. Dialect selection lives here only;
/// everything downstream is dialect-agnostic (per CLAUDE.md).
pub fn parse(sql: &str, kind: DialectKind) -> Result<Vec<Statement>, ParserError> {
    // Strip a UTF-8 BOM: sqlparser chokes on it and real files have one.
    Parser::parse_sql(dialect(kind), sql.trim_start_matches('\u{feff}'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bom_is_stripped() {
        let stmts = parse("\u{feff}SELECT 1", DialectKind::Postgres).expect("BOM must be stripped");
        assert_eq!(stmts.len(), 1);
    }

    #[test]
    fn sqlite_dialect_is_lenient_with_postgres_casts() {
        // CLAUDE.md: SQLiteDialect is lenient — it accepts Postgres-only
        // syntax like `1::text`. Dialect rules must inspect the AST, never
        // rely on parse failure.
        parse("SELECT 1::text", DialectKind::Sqlite).expect("sqlite accepts :: casts");
    }

    #[test]
    fn invalid_sql_is_an_error() {
        assert!(parse("SELEC 1", DialectKind::Postgres).is_err());
    }
}
