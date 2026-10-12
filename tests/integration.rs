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
    ] {
        assert!(stdout.contains(rule_id), "missing {rule_id} in:\n{stdout}");
    }
    assert!(stdout.contains("6 violation(s)"), "stdout:\n{stdout}");
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
        6,
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
