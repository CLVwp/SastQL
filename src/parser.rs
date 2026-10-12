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

/// Parse one file's SQL into statements. Dialect selection lives here only;
/// everything downstream is dialect-agnostic (per CLAUDE.md).
pub fn parse(sql: &str, kind: DialectKind) -> Result<Vec<Statement>, ParserError> {
    // Strip a UTF-8 BOM: sqlparser chokes on it and real files have one.
    Parser::parse_sql(dialect(kind), sql.trim_start_matches('\u{feff}'))
}
