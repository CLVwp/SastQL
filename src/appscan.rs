//! `--check-app`: scan SQL strings inside application source (Rust, TS/JS,
//! Python) for injection-prone concatenation.
//!
//! The guardrail (CLAUDE.md: never hand-parse SQL, never regex SQL): the
//! lexers below only recognize the HOST language's string delimiters,
//! comments and `+` tokens — never SQL syntax. Whether a candidate string is
//! SQL is decided exclusively by feeding it through `parser::parse`
//! (sqlparser). A candidate that does not parse is skipped silently: that is
//! a scope decision (we only speak when we are sure it is SQL), not a skipped
//! error — the fail-loudly rule covers scan/parse errors of what we claim to
//! analyze, and app source holding non-SQL strings is out of scope by
//! definition.

use crate::parser::{self, DialectKind};
use crate::report::{Severity, Violation};

/// Rule id for app-source findings (stable, kebab-case). Deliberately NOT in
/// the rules registry: its input is app source, not SQL statements, so it
/// does not fit the `Rule` trait — see the CLAUDE.md conventions.
pub const RULE_ID: &str = "sql-concatenation";

/// Snippet ceiling: the assembled candidate SQL is truncated to this length.
const SNIPPET_MAX: usize = 120;

/// Host language of a scanned file, selected by extension.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Rust,
    Python,
    Js,
}

pub fn lang_for(ext: &str) -> Option<Lang> {
    match ext {
        "rs" => Some(Lang::Rust),
        "py" => Some(Lang::Python),
        "ts" | "tsx" | "js" | "mjs" | "cjs" | "jsx" => Some(Lang::Js),
        _ => None,
    }
}

/// One host-language token that matters for concat detection.
#[derive(Debug)]
enum Token {
    /// A string literal. `frag` is the candidate fragment: the literal's
    /// content with every interpolation hole already replaced by `0`;
    /// `holes` counts them (`${..}` in JS templates, `{..}` in f-strings and
    /// Rust format-style literals).
    Str {
        frag: String,
        holes: usize,
        line: u64,
        col: u64,
    },
    Plus,
    Other,
}

/// Outcome of one lexer step: a token, a skipped comment, or an unterminated
/// string (lexer ceiling — stop scanning the file, never panic).
enum Step {
    Tok(Token),
    Skip,
    Stop,
}

/// Char-by-char lexer over one source file. Tracks 1-based line/column and
/// knows only the host language's strings and comments — host-language
/// lexing stops there; SQL recognition is `parser::parse`'s job alone.
struct Lexer<'a> {
    src: &'a [char],
    pos: usize,
    line: u64,
    col: u64,
    lang: Lang,
}

impl<'a> Lexer<'a> {
    fn peek(&self, n: usize) -> Option<char> {
        self.src.get(self.pos + n).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.src.get(self.pos).copied()?;
        self.pos += 1;
        match c {
            '\n' => {
                self.line += 1;
                self.col = 1;
            }
            // CRLF is one line break, not two.
            '\r' => {
                if self.peek(0) == Some('\n') {
                    self.pos += 1;
                }
                self.line += 1;
                self.col = 1;
            }
            _ => self.col += 1,
        }
        Some(c)
    }

    fn tokenize(mut self) -> Vec<Token> {
        let mut out = Vec::new();
        while let Some(c) = self.peek(0) {
            if c.is_whitespace() {
                self.bump();
                continue;
            }
            let (line, col) = (self.line, self.col);
            match self.step(line, col) {
                Step::Tok(t) => out.push(t),
                Step::Skip => {}
                // Unterminated string: lexer ceiling — the file's tail is
                // skipped rather than mis-lexed; no panic, no crash.
                Step::Stop => break,
            }
        }
        out
    }

    fn step(&mut self, line: u64, col: u64) -> Step {
        let c = self.peek(0).expect("caller peeked");
        // Host-language comments are skipped so commented-out code never
        // produces violations.
        if self.lang != Lang::Python && c == '/' {
            if self.peek(1) == Some('/') {
                self.line_comment();
                return Step::Skip;
            }
            if self.peek(1) == Some('*') {
                self.block_comment();
                return Step::Skip;
            }
        }
        if self.lang == Lang::Python && c == '#' {
            self.line_comment();
            return Step::Skip;
        }
        if c == '+' {
            self.bump();
            return Step::Tok(Token::Plus);
        }
        match self.lang {
            Lang::Rust => self.step_rust(line, col),
            Lang::Python => self.step_python(line, col),
            Lang::Js => self.step_js(line, col),
        }
    }

    fn line_comment(&mut self) {
        while let Some(c) = self.peek(0) {
            if c == '\n' || c == '\r' {
                break;
            }
            self.bump();
        }
    }

    fn block_comment(&mut self) {
        self.bump();
        self.bump();
        while let Some(c) = self.peek(0) {
            if c == '*' && self.peek(1) == Some('/') {
                self.bump();
                self.bump();
                return;
            }
            self.bump();
        }
    }

    /// Any other construct: an identifier/number run or a single punctuation
    /// char — one opaque `Other` token either way.
    fn other(&mut self) -> Step {
        if self
            .peek(0)
            .is_some_and(|c| c.is_alphanumeric() || c == '_')
        {
            while self
                .peek(0)
                .is_some_and(|c| c.is_alphanumeric() || c == '_')
            {
                self.bump();
            }
        } else {
            self.bump();
        }
        Step::Tok(Token::Other)
    }

    /// Consumes a `{...}` run — the caller has only peeked the `{` — through
    /// its matching `}`. False on EOF (the caller then stops the file).
    fn skip_braced(&mut self) -> bool {
        self.bump(); // '{'
        let mut depth = 1usize;
        loop {
            match self.peek(0) {
                None => return false,
                Some('{') => {
                    self.bump();
                    depth += 1;
                }
                Some('}') => {
                    self.bump();
                    depth -= 1;
                    if depth == 0 {
                        return true;
                    }
                }
                Some(_) => {
                    self.bump();
                }
            }
        }
    }

    // --- Python ---------------------------------------------------------------

    fn step_python(&mut self, line: u64, col: u64) -> Step {
        let c = self.peek(0).expect("caller peeked");
        if c == '\'' || c == '"' {
            return self.py_string(c, false, line, col);
        }
        // String prefix (r, b, u, f and combos, case-insensitive) directly
        // before a quote. Anything longer or with other letters is just an
        // identifier.
        if c.is_ascii_alphabetic() {
            let mut n = 0usize;
            while self.peek(n).is_some_and(|ch| ch.is_ascii_alphabetic()) {
                n += 1;
            }
            let quoted = matches!(self.peek(n), Some('\'') | Some('"'));
            if quoted && n <= 3 {
                let run: String = (0..n).map(|i| self.src[self.pos + i]).collect();
                let lower = run.to_ascii_lowercase();
                if lower.chars().all(|ch| "rbuf".contains(ch)) {
                    let is_f = lower.contains('f');
                    for _ in 0..n {
                        self.bump();
                    }
                    let quote = self.peek(0).expect("checked above");
                    return self.py_string(quote, is_f, line, col);
                }
            }
        }
        self.other()
    }

    /// Python string literal: `'...'`, `"..."` and the triple-quoted
    /// variants; `is_f` comes from the prefix letters. `{...}` counts as an
    /// interpolation hole only in f-strings — a plain string holding braces
    /// is not an interpolation and stays silent. Raw prefixes change nothing
    /// here: Python tokenizes r"\"" as one string either way.
    fn py_string(&mut self, quote: char, is_f: bool, line: u64, col: u64) -> Step {
        let triple = self.peek(0) == Some(quote) && self.peek(1) == Some(quote);
        let qlen = if triple { 3 } else { 1 };
        for _ in 0..qlen {
            self.bump();
        }
        let mut frag = String::new();
        let mut holes = 0usize;
        loop {
            let Some(c) = self.peek(0) else {
                return Step::Stop;
            };
            // A backslash consumes the next char for termination purposes even
            // in raw strings (Python tokenizes r"\"" as one string).
            if c == '\\' {
                self.bump();
                let Some(esc) = self.bump() else {
                    return Step::Stop;
                };
                frag.push('\\');
                frag.push(esc);
                continue;
            }
            if c == quote {
                let close = !triple || (self.peek(1) == Some(quote) && self.peek(2) == Some(quote));
                if close {
                    for _ in 0..qlen {
                        self.bump();
                    }
                    return Step::Tok(Token::Str {
                        frag,
                        holes,
                        line,
                        col,
                    });
                }
            }
            if is_f && c == '{' {
                if self.peek(1) == Some('{') {
                    self.bump();
                    self.bump();
                    frag.push('{');
                    continue;
                }
                holes += 1;
                frag.push('0');
                // ponytail: hole scan is brace-depth only; a `}` inside a
                // nested string literal inside the hole ends it early (rare,
                // and the degraded candidate just fails the SQL gate).
                if !self.skip_braced() {
                    return Step::Stop;
                }
                continue;
            }
            if is_f && c == '}' && self.peek(1) == Some('}') {
                self.bump();
                self.bump();
                frag.push('}');
                continue;
            }
            frag.push(c);
            self.bump();
        }
    }

    // --- JavaScript / TypeScript ----------------------------------------------

    fn step_js(&mut self, line: u64, col: u64) -> Step {
        let c = self.peek(0).expect("caller peeked");
        match c {
            '\'' | '"' => self.js_quoted(c, line, col),
            '`' => self.js_template(line, col),
            _ => self.other(),
        }
    }

    /// `'...'` / `"..."` with escapes; no interpolation holes exist in
    /// quoted JS strings — only backtick templates interpolate.
    fn js_quoted(&mut self, quote: char, line: u64, col: u64) -> Step {
        self.bump();
        let mut frag = String::new();
        loop {
            let Some(c) = self.peek(0) else {
                return Step::Stop;
            };
            match c {
                '\\' => {
                    self.bump();
                    let Some(esc) = self.bump() else {
                        return Step::Stop;
                    };
                    frag.push('\\');
                    frag.push(esc);
                }
                c if c == quote => {
                    self.bump();
                    return Step::Tok(Token::Str {
                        frag,
                        holes: 0,
                        line,
                        col,
                    });
                }
                _ => {
                    frag.push(c);
                    self.bump();
                }
            }
        }
    }

    /// Backtick template: `${...}` counts as an interpolation hole.
    fn js_template(&mut self, line: u64, col: u64) -> Step {
        self.bump(); // backtick
        let mut frag = String::new();
        let mut holes = 0usize;
        loop {
            let Some(c) = self.peek(0) else {
                return Step::Stop;
            };
            match c {
                '\\' => {
                    self.bump();
                    let Some(esc) = self.bump() else {
                        return Step::Stop;
                    };
                    frag.push('\\');
                    frag.push(esc);
                }
                '`' => {
                    self.bump();
                    return Step::Tok(Token::Str {
                        frag,
                        holes,
                        line,
                        col,
                    });
                }
                '$' if self.peek(1) == Some('{') => {
                    holes += 1;
                    frag.push('0');
                    self.bump(); // '$'
                    // ponytail: brace depth only — a `}` inside a nested
                    // string literal inside the interpolation ends the hole
                    // early (rare, and the degraded candidate just fails the
                    // SQL gate).
                    if !self.skip_braced() {
                        return Step::Stop;
                    }
                }
                _ => {
                    frag.push(c);
                    self.bump();
                }
            }
        }
    }

    // --- Rust -----------------------------------------------------------------

    fn step_rust(&mut self, line: u64, col: u64) -> Step {
        let c = self.peek(0).expect("caller peeked");
        // Raw strings r"..", r#".."#, br"..", br#".."#. A `"` must follow the
        // hashes so raw IDENTIFIERS (r#async) are not mistaken for one.
        if c == 'r' && self.peek(1) == Some('"') {
            self.bump();
            return self.rust_string(None, line, col);
        }
        if c == 'r' && self.peek(1) == Some('#') && self.peek(2) == Some('"') {
            self.bump();
            self.bump();
            return self.rust_string(Some(1), line, col);
        }
        if c == 'b' {
            if self.peek(1) == Some('"') {
                self.bump();
                return self.rust_string(None, line, col);
            }
            if self.peek(1) == Some('r') && self.peek(2) == Some('"') {
                self.bump();
                self.bump();
                return self.rust_string(None, line, col);
            }
            if self.peek(1) == Some('r') && self.peek(2) == Some('#') && self.peek(3) == Some('"') {
                self.bump();
                self.bump();
                self.bump();
                return self.rust_string(Some(1), line, col);
            }
            if self.peek(1) == Some('\'') {
                return self.rust_char_or_lifetime(line, col);
            }
        }
        if c == '\'' {
            return self.rust_char_or_lifetime(line, col);
        }
        if c == '"' {
            return self.rust_string(None, line, col);
        }
        self.other()
    }

    /// A Rust string literal; `Some(h)` is a raw string closed by `"` followed
    /// by `h` hashes (no escapes there), `None` a normal one with escapes.
    ///
    /// `{...}` inside ANY Rust string literal counts as a format!-style
    /// interpolation hole — ponytail heuristic: detecting a real format!
    /// context is the ceiling; the accepted false positive is SQL containing
    /// literal braces (rare), and raw strings keep the holes because
    /// `format!(r".. {} ..", x)` is common Rust.
    fn rust_string(&mut self, raw_hashes: Option<usize>, line: u64, col: u64) -> Step {
        self.bump(); // opening quote
        let hashes = raw_hashes.unwrap_or(0);
        let mut frag = String::new();
        let mut holes = 0usize;
        loop {
            let Some(c) = self.peek(0) else {
                return Step::Stop;
            };
            // Closing quote: raw needs `hashes` '#' right after it.
            if c == '"' {
                let mut n = 1usize;
                while n <= hashes && self.peek(n) == Some('#') {
                    n += 1;
                }
                if n > hashes {
                    for _ in 0..=hashes {
                        self.bump();
                    }
                    return Step::Tok(Token::Str {
                        frag,
                        holes,
                        line,
                        col,
                    });
                }
                frag.push(c);
                self.bump();
                continue;
            }
            match raw_hashes {
                // Escapes only close over the next char in non-raw strings.
                None if c == '\\' => {
                    self.bump();
                    let Some(esc) = self.bump() else {
                        return Step::Stop;
                    };
                    frag.push('\\');
                    frag.push(esc);
                }
                _ if c == '{' && self.peek(1) == Some('{') => {
                    self.bump();
                    self.bump();
                    frag.push('{');
                }
                _ if c == '{' => {
                    holes += 1;
                    frag.push('0');
                    if !self.skip_braced() {
                        return Step::Stop;
                    }
                }
                _ if c == '}' && self.peek(1) == Some('}') => {
                    self.bump();
                    self.bump();
                    frag.push('}');
                }
                _ => {
                    frag.push(c);
                    self.bump();
                }
            }
        }
    }

    /// `'x'`-style char literals are inert (never SQL, never interpolated);
    /// any other `'` is a lifetime (`'a`, `'static`) — code that must not
    /// swallow the rest of the file as an unterminated string.
    fn rust_char_or_lifetime(&mut self, line: u64, col: u64) -> Step {
        let mut n = 1usize;
        let mut closed = None;
        while n < 24 {
            match self.peek(n) {
                None => break,
                Some('\\') => n += 2,
                Some('\'') => {
                    closed = Some(n);
                    break;
                }
                Some(_) => n += 1,
            }
        }
        match closed {
            Some(end) => {
                let frag: String = (1..end).map(|i| self.src[self.pos + i]).collect();
                for _ in 0..=end {
                    self.bump();
                }
                Step::Tok(Token::Str {
                    frag,
                    holes: 0,
                    line,
                    col,
                })
            }
            None => {
                self.bump(); // '\''
                self.other() // lifetime: lexed as an identifier run
            }
        }
    }
}

/// One operand of a concat group: `Some(fragment)` is a string literal's
/// candidate fragment, `None` a non-literal (`Other`) position rendered as
/// `0` in the candidate.
type Part = Option<String>;

struct Group {
    parts: Vec<Part>,
    /// ≥ 1 interpolation hole, or ≥ 1 `+` with a non-literal operand.
    prone: bool,
    /// Position of the group's first literal.
    line: u64,
    col: u64,
}

/// Maximal concat groups: `+` chains in every language, plus implicit
/// adjacency of neighboring literals in Rust and Python (those languages
/// concatenate adjacent literals; JS does not). Groups start at a literal —
/// a chain whose first operand is a variable has no literal to carry SQL
/// text, and its candidate would start with the placeholder anyway
/// (documented ceiling: such chains are skipped).
fn groups(tokens: &[Token], lang: Lang) -> Vec<Group> {
    let mut out = Vec::new();
    let mut cur: Option<Group> = None;
    let mut pending_plus = false;
    for tok in tokens {
        match tok {
            Token::Str {
                frag,
                holes,
                line,
                col,
            } => {
                let mut extended = false;
                if let Some(g) = cur.as_mut() {
                    let joins = pending_plus
                        || (lang != Lang::Js && matches!(g.parts.last(), Some(Some(_))));
                    if joins {
                        g.parts.push(Some(frag.clone()));
                        g.prone |= *holes > 0;
                        extended = true;
                    }
                }
                if !extended {
                    if let Some(g) = cur.take() {
                        out.push(g);
                    }
                    cur = Some(Group {
                        parts: vec![Some(frag.clone())],
                        prone: *holes > 0,
                        line: *line,
                        col: *col,
                    });
                }
                pending_plus = false;
            }
            Token::Plus => pending_plus = true,
            Token::Other => {
                if pending_plus {
                    if let Some(g) = cur.as_mut() {
                        g.parts.push(None);
                        g.prone = true;
                    }
                } else {
                    // A non-`+`-joined non-literal ends the current group.
                    if let Some(g) = cur.take() {
                        out.push(g);
                    }
                }
                pending_plus = false;
            }
        }
    }
    if let Some(g) = cur {
        out.push(g);
    }
    out
}

/// The SQL-ness gate (the anti-regex guardrail): only `parser::parse`
/// decides. A group that does not assemble into parseable SQL is skipped
/// silently — a scope decision, not a swallowed error.
fn judge(g: &Group, dialect: DialectKind, file: &str) -> Option<Violation> {
    if !g.prone {
        // Pure-literal group, zero holes: a readable multi-line SQL constant
        // is good practice, not an injection.
        return None;
    }
    let mut candidate = String::new();
    for part in &g.parts {
        match part {
            Some(frag) => candidate.push_str(frag),
            // `0` is valid in expression and string-literal positions alike.
            None => candidate.push('0'),
        }
    }
    let is_sql = matches!(
        parser::parse(&candidate, dialect),
        Ok(stmts) if !stmts.is_empty()
    );
    if !is_sql {
        return None;
    }
    Some(Violation {
        file: file.to_string(),
        rule_id: RULE_ID,
        severity: Severity::Error,
        message: "SQL is assembled by concatenating or interpolating strings, so input can change the query's structure (injection); use a parameterized query with bind parameters".to_string(),
        snippet: truncate(&candidate),
        location: Some((g.line, g.col)),
    })
}

/// Scans one application source file for injection-prone SQL concatenation.
pub fn scan(source: &str, lang: Lang, dialect: DialectKind, file: &str) -> Vec<Violation> {
    let chars: Vec<char> = source.chars().collect();
    let tokens = Lexer {
        src: &chars,
        pos: 0,
        line: 1,
        col: 1,
        lang,
    }
    .tokenize();
    groups(&tokens, lang)
        .iter()
        .filter_map(|g| judge(g, dialect, file))
        .collect()
}

fn truncate(s: &str) -> String {
    if s.chars().count() <= SNIPPET_MAX {
        s.to_string()
    } else {
        let head: String = s.chars().take(SNIPPET_MAX).collect();
        format!("{head}…")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan_rust(src: &str) -> Vec<Violation> {
        scan(src, Lang::Rust, DialectKind::Postgres, "t.rs")
    }

    fn scan_py(src: &str) -> Vec<Violation> {
        scan(src, Lang::Python, DialectKind::Postgres, "t.py")
    }

    fn scan_js(src: &str) -> Vec<Violation> {
        scan(src, Lang::Js, DialectKind::Postgres, "t.ts")
    }

    #[test]
    fn rust_concat_is_flagged_with_pinned_position() {
        let src = "let id = user_input();\nlet q = \"SELECT * FROM users WHERE id = \" + id;\n";
        let v = scan_rust(src);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].rule_id, "sql-concatenation");
        assert_eq!(v[0].severity, Severity::Error);
        assert_eq!(v[0].location, Some((2, 9)));
        assert_eq!(v[0].snippet, "SELECT * FROM users WHERE id = 0");
    }

    #[test]
    fn rust_adjacent_literals_only_are_not_flagged() {
        // Implicit concatenation of pure literals: a readable SQL constant,
        // not an injection.
        let src = "let q = \"SELECT 1\" \" FROM t\";\n";
        assert!(scan_rust(src).is_empty());
    }

    #[test]
    fn rust_format_macro_heuristic() {
        // Braces inside any Rust string literal count as a format!-style
        // hole (ponytail heuristic — see rust_string's doc).
        let src = "let q = format!(\"SELECT * FROM t WHERE id = {}\", id);\n";
        let v = scan_rust(src);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].snippet, "SELECT * FROM t WHERE id = 0");
    }

    #[test]
    fn rust_lifetimes_and_raw_identifiers_do_not_derail() {
        let src =
            "fn f<'a>(x: &'a str) { let q = r#type; q = \"SELECT * FROM t WHERE id = \" + x; }";
        assert_eq!(scan_rust(src).len(), 1);
    }

    #[test]
    fn ts_template_hole_is_flagged() {
        let src = "const q = `SELECT * FROM users WHERE name = '${name}'`;";
        let v = scan_js(src);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].snippet, "SELECT * FROM users WHERE name = '0'");
    }

    #[test]
    fn ts_parameterized_query_is_not_flagged() {
        let src = "const q = \"SELECT * FROM users WHERE id = ?\";";
        assert!(scan_js(src).is_empty());
    }

    #[test]
    fn py_fstring_hole_is_flagged() {
        let src = "q = f\"SELECT * FROM users WHERE name = '{name}'\"";
        let v = scan_py(src);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].snippet, "SELECT * FROM users WHERE name = '0'");
    }

    #[test]
    fn py_plain_string_with_braces_is_not_flagged() {
        // No f prefix: the braces are literal content, no interpolation.
        let src = "q = \"SELECT * FROM users WHERE name = '{name}'\"";
        assert!(scan_py(src).is_empty());
    }

    #[test]
    fn non_sql_concat_fails_the_sql_gate() {
        // The SQL-ness gate: "Hello " + name + "!" assembles into text that
        // does not parse as SQL — skipped, never reported.
        let src = "const msg = \"Hello \" + name + \"!\";";
        assert!(scan_js(src).is_empty());
    }

    #[test]
    fn commented_out_concat_never_flags() {
        assert!(scan_rust("// let q = \"SELECT 1\" + id;").is_empty());
        assert!(scan_py("# q = \"SELECT 1\" + id").is_empty());
        assert!(scan_js("/* let q = \"SELECT 1\" + id; */").is_empty());
    }

    #[test]
    fn unterminated_string_stops_gracefully() {
        // Lexer ceiling: the file's tail is skipped, never a panic.
        let src = "let q = \"SELECT * FROM users WHERE id = ";
        assert!(scan_rust(src).is_empty());
    }

    #[test]
    fn long_candidate_snippet_is_truncated() {
        let src = format!("let q = \"SELECT {} FROM t\" + x;", "a".repeat(130));
        let v = scan_rust(&src);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].snippet.chars().count(), SNIPPET_MAX + 1);
        assert!(v[0].snippet.ends_with('…'));
    }
}
