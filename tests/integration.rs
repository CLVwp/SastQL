//! End-to-end: run the real binary over tests/fixtures/ (CLAUDE.md).
use std::process::Command;

fn sastql(args: &[&str]) -> (i32, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_sastql"))
        .args(args)
        .output()
        .expect("sastql binary should run");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
    )
}

#[test]
fn dirty_fixture_exits_1_and_reports_every_default_rule() {
    let (code, stdout) = sastql(&["check", "tests/fixtures/dirty", "--dialect", "sqlite"]);
    assert_eq!(code, 1, "stdout:\n{stdout}");
    for rule_id in [
        "select-star",
        "no-where-on-mutation",
        "not-in-null-trap",
        "null-comparison",
        "dialect-portability",
        "type-affinity",
        "table-no-pk",
    ] {
        assert!(stdout.contains(rule_id), "missing {rule_id} in:\n{stdout}");
    }
    assert!(stdout.contains("7 violation(s)"), "stdout:\n{stdout}");
    assert!(stdout.contains("4 file(s)"), "stdout:\n{stdout}");
}

#[test]
fn clean_fixture_exits_0() {
    let (code, stdout) = sastql(&["check", "tests/fixtures/clean", "--dialect", "postgres"]);
    assert_eq!(code, 0, "stdout:\n{stdout}");
    assert!(stdout.contains("0 violation(s)"), "stdout:\n{stdout}");
}

#[test]
fn json_format_is_parseable_summary() {
    let (code, stdout) = sastql(&[
        "check",
        "tests/fixtures/dirty",
        "--dialect",
        "sqlite",
        "--format",
        "json",
    ]);
    assert_eq!(code, 1, "stdout:\n{stdout}");
    assert!(stdout.contains("\"files_scanned\": 4"), "stdout:\n{stdout}");
    assert_eq!(
        stdout.matches("\"rule_id\"").count(),
        7,
        "stdout:\n{stdout}"
    );
}

#[test]
fn opt_in_rule_only_runs_when_selected() {
    let (code, stdout) = sastql(&["check", "tests/fixtures/dirty", "--dialect", "sqlite"]);
    assert_eq!(code, 1, "stdout:\n{stdout}");
    assert!(!stdout.contains("missing-limit"), "stdout:\n{stdout}");

    let (code, stdout) = sastql(&[
        "check",
        "tests/fixtures/dirty",
        "--dialect",
        "sqlite",
        "--rules",
        "missing-limit",
    ]);
    assert_eq!(code, 1, "stdout:\n{stdout}");
    assert!(stdout.contains("missing-limit"), "stdout:\n{stdout}");
    assert!(stdout.contains("2 violation(s)"), "stdout:\n{stdout}");
}

#[test]
fn type_affinity_is_sqlite_only_end_to_end() {
    let (code, stdout) = sastql(&[
        "check",
        "tests/fixtures/dirty",
        "--dialect",
        "sqlite",
        "--rules",
        "type-affinity",
    ]);
    assert_eq!(code, 1, "stdout:\n{stdout}");
    assert!(stdout.contains("type-affinity"), "stdout:\n{stdout}");

    let (code, _) = sastql(&[
        "check",
        "tests/fixtures/dirty",
        "--dialect",
        "postgres",
        "--rules",
        "type-affinity",
    ]);
    assert_eq!(code, 0);
}

#[test]
fn unknown_rule_exits_2() {
    let (code, _) = sastql(&["check", "tests/fixtures/dirty", "--rules", "nope"]);
    assert_eq!(code, 2);
}

/// Per-rule expected counts for the realistic set (simulated shop DB),
/// default run. Counts were derived from the fixtures and are pinned
/// exactly — any drift means a rule or fixture changed behavior.
const REALISTIC_COUNTS_SQLITE: &[(&str, usize)] = &[
    ("select-star", 2),
    ("no-where-on-mutation", 1),
    ("not-in-null-trap", 1),
    ("null-comparison", 1),
    ("dialect-portability", 0),
    ("type-affinity", 2),
    ("case-without-else", 2),
    ("join-without-on", 2),
    ("natural-join", 1),
    ("insert-without-columns", 1),
    ("offset-without-limit", 1),
    ("order-by-random", 1),
    ("leading-wildcard-like", 2),
    ("string-plus", 1),
    ("weak-hash", 2),
    ("select-star-plus", 1),
    ("grant-all", 1),
    ("table-no-pk", 1),
    ("float-money", 3),
];

const REALISTIC_COUNTS_POSTGRES: &[(&str, usize)] = &[
    ("select-star", 2),
    ("no-where-on-mutation", 1),
    ("not-in-null-trap", 1),
    ("null-comparison", 1),
    ("dialect-portability", 0),
    ("type-affinity", 0),
    ("case-without-else", 2),
    ("join-without-on", 2),
    ("natural-join", 1),
    ("insert-without-columns", 1),
    ("offset-without-limit", 1),
    ("order-by-random", 1),
    ("leading-wildcard-like", 2),
    ("string-plus", 0),
    ("weak-hash", 2),
    ("select-star-plus", 1),
    ("grant-all", 1),
    ("table-no-pk", 1),
    ("float-money", 3),
];

#[test]
fn realistic_fixture_per_dialect_rule_counts() {
    for (dialect, counts) in [
        ("sqlite", REALISTIC_COUNTS_SQLITE),
        ("postgres", REALISTIC_COUNTS_POSTGRES),
    ] {
        let (code, stdout) = sastql(&[
            "check",
            "tests/fixtures/realistic",
            "--dialect",
            dialect,
            "--format",
            "json",
        ]);
        assert_eq!(code, 1, "stdout:\n{stdout}");
        assert!(stdout.contains("\"files_scanned\": 5"), "stdout:\n{stdout}");
        for (rule_id, expected) in counts {
            let found = stdout
                .matches(&format!("\"rule_id\": \"{rule_id}\""))
                .count();
            assert_eq!(
                &found, expected,
                "rule {rule_id} on {dialect} — stdout:\n{stdout}"
            );
        }
    }
}

#[test]
fn realistic_opt_in_rule_counts() {
    for (rule_id, dialect, expected) in [
        ("destructive-ddl", "sqlite", 3),
        ("destructive-ddl", "postgres", 3),
        ("union-without-all", "sqlite", 1),
        ("union-without-all", "postgres", 1),
        ("order-by-ordinal", "sqlite", 1),
        ("limit-without-order-by", "postgres", 1),
        ("naive-timestamp", "postgres", 1),
        ("naive-timestamp", "sqlite", 0),
    ] {
        let (code, stdout) = sastql(&[
            "check",
            "tests/fixtures/realistic",
            "--dialect",
            dialect,
            "--rules",
            rule_id,
        ]);
        let expected_code = if expected == 0 { 0 } else { 1 };
        assert_eq!(code, expected_code, "{rule_id}/{dialect}: {stdout}");
        assert!(
            stdout.contains(&format!("{expected} violation(s)")),
            "{rule_id}/{dialect}: {stdout}"
        );
    }
}
