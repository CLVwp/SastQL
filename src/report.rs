use std::fmt;

/// Exit codes: 0 clean, 1 violations, 2 usage/parse/IO error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
        })
    }
}

/// Parses the severity strings accepted in `sastql.toml` — case-sensitive
/// lowercase, so a typo fails loudly instead of silently defaulting.
impl std::str::FromStr for Severity {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "error" => Ok(Severity::Error),
            "warning" => Ok(Severity::Warning),
            "info" => Ok(Severity::Info),
            _ => Err(format!(
                "invalid severity '{s}' (expected \"error\", \"warning\" or \"info\")"
            )),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Violation {
    pub file: String,
    pub rule_id: &'static str,
    pub severity: Severity,
    pub message: String,
    pub snippet: String,
    /// 1-based line/column of the offending statement's first token.
    /// `None` = position unknown — we render no position at all rather
    /// than a wrong one (statement-level is the honest ceiling: AST
    /// nodes carry no spans in sqlparser 0.63).
    pub location: Option<(u64, u64)>,
}

#[derive(Clone, Debug)]
pub struct Report {
    pub violations: Vec<Violation>,
    pub files_scanned: usize,
}

impl Report {
    pub fn exit_code(&self) -> i32 {
        if self.violations.is_empty() { 0 } else { 1 }
    }
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum OutputFormat {
    Human,
    Json,
}

impl OutputFormat {
    pub fn render(&self, report: &Report) -> String {
        match self {
            OutputFormat::Human => human(report),
            OutputFormat::Json => json(report),
        }
    }
}

fn human(report: &Report) -> String {
    let mut out = String::new();
    for v in &report.violations {
        match v.location {
            Some((line, col)) => out.push_str(&format!(
                "{}:{line}:{col}: {} {}: {}\n    {}\n",
                v.file, v.severity, v.rule_id, v.message, v.snippet
            )),
            // No location known: the old position-less format, never a fake one.
            None => out.push_str(&format!(
                "{}: {} {}: {}\n    {}\n",
                v.file, v.severity, v.rule_id, v.message, v.snippet
            )),
        }
    }
    out.push_str(&format!(
        "\n{} file(s) scanned, {} violation(s)\n",
        report.files_scanned,
        report.violations.len()
    ));
    out
}

fn json(report: &Report) -> String {
    // ponytail: hand-rolled JSON; serde_json only if the format grows.
    let items: Vec<String> = report
        .violations
        .iter()
        .map(|v| {
            let (line, column) = match v.location {
                Some((line, column)) => (line.to_string(), column.to_string()),
                None => ("null".into(), "null".into()),
            };
            format!(
                "    {{\"file\": \"{}\", \"rule_id\": \"{}\", \"severity\": \"{}\", \"line\": {line}, \"column\": {column}, \"message\": \"{}\", \"snippet\": \"{}\"}}",
                escape(&v.file),
                escape(v.rule_id),
                escape(&v.severity.to_string()),
                escape(&v.message),
                escape(&v.snippet)
            )
        })
        .collect();
    format!(
        "{{\n  \"files_scanned\": {},\n  \"violations\": [\n{}\n  ]\n}}\n",
        report.files_scanned,
        items.join(",\n")
    )
}

fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Report {
        Report {
            violations: vec![Violation {
                file: "m/db.sql".into(),
                rule_id: "no-where-on-mutation",
                severity: Severity::Error,
                message: "no WHERE".into(),
                snippet: "DELETE FROM \"t\" WHERE \"x\" = \"1\"".into(),
                location: Some((3, 1)),
            }],
            files_scanned: 1,
        }
    }

    #[test]
    fn exit_codes() {
        assert_eq!(sample().exit_code(), 1);
        assert_eq!(
            Report {
                violations: vec![],
                files_scanned: 1
            }
            .exit_code(),
            0
        );
    }

    #[test]
    fn json_escapes_quotes() {
        let j = OutputFormat::Json.render(&sample());
        assert!(j.contains("\\\"t\\\""), "quotes must be escaped: {j}");
        assert!(j.contains("\"files_scanned\": 1"));
    }

    #[test]
    fn json_escapes_newlines() {
        let mut report = sample();
        report.violations[0].snippet = "SELECT 1;\nDELETE FROM t".into();
        let j = OutputFormat::Json.render(&report);
        assert!(
            j.contains("SELECT 1;\\nDELETE"),
            "newline must be escaped: {j}"
        );
    }

    #[test]
    fn human_prints_statement_location() {
        let h = OutputFormat::Human.render(&sample());
        assert!(
            h.contains("m/db.sql:3:1: error no-where-on-mutation:"),
            "human output must carry file:line:col: {h}"
        );
    }

    #[test]
    fn json_prints_statement_location() {
        let j = OutputFormat::Json.render(&sample());
        assert!(
            j.contains("\"line\": 3, \"column\": 1"),
            "json output must carry line/column: {j}"
        );
    }

    #[test]
    fn human_without_location_has_no_fake_position() {
        let mut report = sample();
        report.violations[0].location = None;
        let h = OutputFormat::Human.render(&report);
        assert!(
            h.starts_with("m/db.sql: error no-where-on-mutation:"),
            "no location means the position-less format: {h}"
        );
        assert!(
            !h.contains("db.sql:3:1"),
            "no file:line:col position may be printed: {h}"
        );
    }

    #[test]
    fn json_without_location_is_null() {
        let mut report = sample();
        report.violations[0].location = None;
        let j = OutputFormat::Json.render(&report);
        assert!(
            j.contains("\"line\": null, \"column\": null"),
            "json must use null when the position is unknown: {j}"
        );
    }
}
