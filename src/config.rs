//! `sastql.toml` — per-rule severity overrides and named rule sets, so teams
//! stop pasting long `--rules` lists into CI.
//!
//! Fail-loudly semantics (CLAUDE.md): an invalid config — bad TOML, unknown
//! key, unknown rule id, bad severity string — is a usage error (exit 2) with
//! the offending key named. The *absence* of a config file is normal and never
//! an error: default behavior applies.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use serde::Deserialize;

use crate::report::{Severity, Violation};
use crate::rules;

/// Auto-discovered in the current directory when `--config` is absent.
pub const CONFIG_FILE: &str = "sastql.toml";

/// Validated config: every rule id below is known to the registry and every
/// severity string parsed.
#[derive(Debug, Default)]
pub struct Config {
    /// Rule id → severity override. Applied after analysis; rules stay
    /// stateless.
    pub severity: BTreeMap<String, Severity>,
    /// Named set → rule ids (references rule ids only — a set naming a set is
    /// rejected at load).
    pub sets: BTreeMap<String, Vec<String>>,
}

/// Raw TOML shape. `deny_unknown_fields` turns a typo in the file into a
/// load error instead of a silently ignored section.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    #[serde(default)]
    severity: BTreeMap<String, String>,
    #[serde(default)]
    sets: BTreeMap<String, RawSet>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSet {
    rules: Vec<String>,
}

impl Config {
    /// Parse + validate a config body. Severity strings go through
    /// `Severity::from_str` (keeps serde out of report.rs); rule ids are
    /// checked against the registry, naming the offender.
    pub fn parse(text: &str) -> Result<Config, String> {
        let raw: RawConfig = toml::from_str(text).map_err(|e| e.to_string())?;
        let known: Vec<&str> = rules::all_rules().iter().map(|r| r.id()).collect();

        let mut severity = BTreeMap::new();
        for (id, sev) in raw.severity {
            if !known.contains(&id.as_str()) {
                return Err(format!("[severity] '{id}' is not a known rule id"));
            }
            let parsed = Severity::from_str(&sev).map_err(|e| format!("[severity] {id}: {e}"))?;
            severity.insert(id, parsed);
        }

        let mut sets = BTreeMap::new();
        for (name, set) in raw.sets {
            for id in &set.rules {
                if !known.contains(&id.as_str()) {
                    return Err(format!(
                        "[sets.{name}] '{id}' is not a known rule id (sets reference rules only, not other sets)"
                    ));
                }
            }
            sets.insert(name, set.rules);
        }

        Ok(Config { severity, sets })
    }

    /// Load the effective config. Explicit `--config <path>` must exist and
    /// parse; otherwise `sastql.toml` is auto-discovered in the current
    /// directory, where a missing file is `Ok(None)` (default behavior).
    pub fn load(explicit: Option<&Path>) -> Result<Option<Config>, String> {
        let path: PathBuf = match explicit {
            Some(p) => p.to_path_buf(),
            None => {
                let p = Path::new(CONFIG_FILE);
                if !p.is_file() {
                    return Ok(None);
                }
                p.to_path_buf()
            }
        };
        let text = fs::read_to_string(&path).map_err(|e| match e.kind() {
            io::ErrorKind::NotFound => format!("config file not found: {}", path.display()),
            _ => format!("cannot read config {}: {e}", path.display()),
        })?;
        Config::parse(&text)
            .map_err(|e| format!("{}: {e}", path.display()))
            .map(Some)
    }

    /// Replaces each violation's severity when its rule id is overridden.
    /// Runs after analysis — the `Rule` trait, the analyze loop and the
    /// rendering are untouched.
    pub fn apply_severities(&self, violations: &mut [Violation]) {
        for v in violations.iter_mut() {
            if let Some(sev) = self.severity.get(v.rule_id) {
                v.severity = *sev;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_config_with_comments_and_blank_lines() {
        let c = Config::parse(
            "# team config\n\n[severity]\nselect-star = \"warning\"\nfloat-money = \"info\"\n\n# CI set\n[sets.ci]\nrules = [\"select-star\", \"no-where-on-mutation\"]\n",
        )
        .unwrap();
        assert_eq!(c.severity.get("select-star"), Some(&Severity::Warning));
        assert_eq!(c.severity.get("float-money"), Some(&Severity::Info));
        assert_eq!(
            c.sets.get("ci").unwrap(),
            &[
                "select-star".to_string(),
                "no-where-on-mutation".to_string()
            ]
        );
    }

    #[test]
    fn missing_sections_default_to_empty() {
        let c = Config::parse("").unwrap();
        assert!(c.severity.is_empty() && c.sets.is_empty());

        let c = Config::parse("[severity]\nselect-star = \"info\"\n").unwrap();
        assert!(c.sets.is_empty());

        let c = Config::parse("[sets.ci]\nrules = [\"select-star\"]\n").unwrap();
        assert!(c.severity.is_empty());
    }

    #[test]
    fn unknown_severity_key_fails() {
        let err = Config::parse("[severity]\nnot-a-rule = \"info\"\n").unwrap_err();
        assert!(err.contains("not-a-rule"), "{err}");
    }

    #[test]
    fn bad_severity_string_fails() {
        let err = Config::parse("[severity]\nselect-star = \"warn\"\n").unwrap_err();
        assert!(err.contains("invalid severity 'warn'"), "{err}");
    }

    #[test]
    fn unknown_set_member_fails() {
        let err = Config::parse("[sets.ci]\nrules = [\"nope\"]\n").unwrap_err();
        assert!(err.contains("'nope'") && err.contains("ci"), "{err}");
    }

    #[test]
    fn set_referencing_a_set_fails() {
        let err =
            Config::parse("[sets.a]\nrules = [\"b\"]\n\n[sets.b]\nrules = [\"select-star\"]\n")
                .unwrap_err();
        assert!(err.contains("'b'"), "{err}");
    }

    #[test]
    fn unknown_top_level_key_fails() {
        let err = Config::parse("severities = {}\n").unwrap_err();
        assert!(err.contains("unknown field"), "{err}");
    }

    #[test]
    fn unknown_key_inside_set_fails() {
        let err = Config::parse("[sets.ci]\nrulez = [\"select-star\"]\n").unwrap_err();
        assert!(err.contains("unknown field"), "{err}");
    }

    #[test]
    fn severity_overrides_apply_after_analysis() {
        let c = Config::parse("[severity]\nselect-star = \"error\"\n").unwrap();
        let mut violations = vec![Violation {
            file: "f.sql".into(),
            rule_id: "select-star",
            severity: Severity::Warning,
            message: "m".into(),
            snippet: "SELECT * FROM t".into(),
            location: None,
        }];
        c.apply_severities(&mut violations);
        assert_eq!(violations[0].severity, Severity::Error);
    }
}
