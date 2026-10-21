use sqlparser::ast::Statement;
use sqlparser::dialect::{Dialect, PostgreSqlDialect, SQLiteDialect};
use sqlparser::parser::{Parser, ParserError};
use sqlparser::tokenizer::{Token, TokenWithSpan, Tokenizer};

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

/// One parsed statement plus the source location of its first token.
pub struct LocatedStatement {
    pub stmt: Statement,
    /// 1-based line/column of the statement's first token. `None` when the
    /// tokenizer's `;`-split diverges from the parser's statement count —
    /// we never print a position we are not sure of.
    pub location: Option<(u64, u64)>,
}

/// Parse one file's `sql` into located statements. Dialect selection lives
/// here only; everything downstream is dialect-agnostic (per CLAUDE.md).
///
/// We tokenize ourselves (instead of letting `parse_sql` do it) to keep each
/// statement's start location; AST nodes carry no spans in sqlparser 0.63, so
/// statement-level is the finest precision we can attribute honestly.
pub fn parse(sql: &str, kind: DialectKind) -> Result<Vec<LocatedStatement>, ParserError> {
    // Strip a UTF-8 BOM: sqlparser chokes on it and real files have one.
    // Editors render it invisibly, so columns are computed on the stripped text.
    let sql = sql.trim_start_matches('\u{feff}');
    let tokens = Tokenizer::new(dialect(kind), sql)
        .tokenize_with_location()
        .map_err(|e| ParserError::TokenizerError(e.to_string()))?;
    let starts = statement_starts(&tokens);
    let statements = Parser::new(dialect(kind))
        .with_tokens_with_locations(tokens)
        .parse_statements()?;
    // Only attach locations when the two splits agree: fewer lines printed,
    // but never a wrong one.
    let located = starts.len() == statements.len();
    Ok(statements
        .into_iter()
        .enumerate()
        .map(|(i, stmt)| LocatedStatement {
            stmt,
            location: if located { Some(starts[i]) } else { None },
        })
        .collect())
}

/// Statement-start locations recovered from the token stream: a `;` at paren
/// depth 0 separates statements, and each statement starts at the next
/// non-whitespace token (comments are `Whitespace` tokens in sqlparser).
fn statement_starts(tokens: &[TokenWithSpan]) -> Vec<(u64, u64)> {
    let mut starts = Vec::new();
    let mut depth = 0usize;
    let mut expect_start = true;
    for tok in tokens {
        match &tok.token {
            Token::Whitespace(_) => {}
            Token::SemiColon if depth == 0 => expect_start = true,
            Token::LParen => {
                if expect_start {
                    starts.push(loc(tok));
                    expect_start = false;
                }
                depth += 1;
            }
            Token::RParen => depth = depth.saturating_sub(1),
            _ => {
                if expect_start {
                    starts.push(loc(tok));
                    expect_start = false;
                }
            }
        }
    }
    starts
}

fn loc(tok: &TokenWithSpan) -> (u64, u64) {
    (tok.span.start.line, tok.span.start.column)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn locations(sql: &str, kind: DialectKind) -> Vec<Option<(u64, u64)>> {
        parse(sql, kind)
            .expect("test SQL must parse")
            .iter()
            .map(|l| l.location)
            .collect()
    }

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

    #[test]
    fn statements_carry_line_numbers() {
        let sql = "SELECT 1;\n\n-- a comment\nSELECT 2;\nDELETE FROM t;\n";
        assert_eq!(
            locations(sql, DialectKind::Postgres),
            vec![Some((1, 1)), Some((4, 1)), Some((5, 1))]
        );
    }

    #[test]
    fn columns_count_indentation() {
        assert_eq!(
            locations("  SELECT 1;", DialectKind::Postgres),
            vec![Some((1, 3))]
        );
    }

    #[test]
    fn single_statement_without_semicolon() {
        assert_eq!(
            locations("SELECT 1", DialectKind::Postgres),
            vec![Some((1, 1))]
        );
    }

    #[test]
    fn parens_do_not_confuse_the_split() {
        let sql = "INSERT INTO t (a, b) VALUES (1, 2);\nSELECT 1;";
        assert_eq!(
            locations(sql, DialectKind::Postgres),
            vec![Some((1, 1)), Some((2, 1))]
        );
    }

    /// The honest-degradation gate: whatever sqlparser does with the `;`
    /// inside a trigger body, no statement may get a wrong line — either
    /// every location lines up or all are `None`.
    #[test]
    fn trigger_body_semicolons_never_mislead() {
        let sql = "CREATE TRIGGER trg AFTER INSERT ON t BEGIN \
                   INSERT INTO log VALUES (1); \
                   UPDATE t SET x = 1; \
                   END;\n\
                   SELECT 1;\n";
        let located = parse(sql, DialectKind::Sqlite).expect("test SQL must parse");
        if let Some((line, _)) = located[0].location {
            assert_eq!(line, 1, "trigger statement must point at its own line");
        }
        let last = located.last().expect("SELECT must parse");
        if let Some((line, _)) = last.location {
            assert_eq!(line, 5, "trailing SELECT must keep its own line");
        }
    }
}
