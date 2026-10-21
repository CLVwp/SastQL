//! End-to-end: run the real binary over tests/fixtures/ (CLAUDE.md).
use std::process::Command;

fn sastql(args: &[&str]) -> (i32, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_sastql"))
        .args(args)
        .output()
        .expect("sastql binary should run");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn dirty_fixture_exits_1_and_reports_every_default_rule() {
    let (code, stdout, _) = sastql(&["check", "tests/fixtures/dirty", "--dialect", "sqlite"]);
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
    let (code, stdout, _) = sastql(&["check", "tests/fixtures/clean", "--dialect", "postgres"]);
    assert_eq!(code, 0, "stdout:\n{stdout}");
    assert!(stdout.contains("0 violation(s)"), "stdout:\n{stdout}");
}

#[test]
fn json_format_is_parseable_summary() {
    let (code, stdout, _) = sastql(&[
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
    let (code, stdout, _) = sastql(&["check", "tests/fixtures/dirty", "--dialect", "sqlite"]);
    assert_eq!(code, 1, "stdout:\n{stdout}");
    assert!(!stdout.contains("missing-limit"), "stdout:\n{stdout}");

    let (code, stdout, _) = sastql(&[
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
    let (code, stdout, _) = sastql(&[
        "check",
        "tests/fixtures/dirty",
        "--dialect",
        "sqlite",
        "--rules",
        "type-affinity",
    ]);
    assert_eq!(code, 1, "stdout:\n{stdout}");
    assert!(stdout.contains("type-affinity"), "stdout:\n{stdout}");

    let (code, _, _) = sastql(&[
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
    let (code, _, _) = sastql(&["check", "tests/fixtures/dirty", "--rules", "nope"]);
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
        let (code, stdout, _) = sastql(&[
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
        let (code, stdout, _) = sastql(&[
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

#[test]
fn parse_error_exits_2_with_file_and_message() {
    // No fixture: a broken .sql under tests/fixtures/ would break the
    // `check tests/fixtures/` walkthrough and the pinned counts.
    let dir = std::env::temp_dir().join(format!("sastql-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir should be creatable");
    let broken = dir.join("broken.sql");
    std::fs::write(&broken, "SELEC 1 FROM t;").expect("temp file should be writable");

    let path = broken.to_str().expect("utf-8 path");
    let (code, stdout, stderr) = sastql(&["check", path]);
    assert_eq!(code, 2, "stdout:\n{stdout}");
    assert!(stderr.contains("parse error"), "stderr:\n{stderr}");
    assert!(stderr.contains("broken.sql"), "stderr:\n{stderr}");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn no_paths_exits_2() {
    let (code, _, stderr) = sastql(&["check"]);
    assert_eq!(code, 2);
    assert!(stderr.contains("no paths given"), "stderr:\n{stderr}");
}

#[test]
fn missing_path_exits_2() {
    let (code, _, stderr) = sastql(&["check", "tests/fixtures/does-not-exist.sql"]);
    assert_eq!(code, 2);
    assert!(stderr.contains("does-not-exist.sql"), "stderr:\n{stderr}");
}

#[test]
fn non_sql_file_warns_and_exits_2() {
    let (code, _, stderr) = sastql(&["check", "README.md"]);
    assert_eq!(code, 2);
    assert!(
        stderr.contains("only .sql files are scanned"),
        "stderr:\n{stderr}"
    );
    assert!(stderr.contains("no .sql files found"), "stderr:\n{stderr}");
}

#[test]
fn single_file_scan_and_dedup() {
    let (code, stdout, _) = sastql(&[
        "check",
        "tests/fixtures/clean/schema.sql",
        "tests/fixtures/clean/schema.sql",
        "--format",
        "json",
    ]);
    assert_eq!(code, 0, "stdout:\n{stdout}");
    assert!(stdout.contains("\"files_scanned\": 1"), "stdout:\n{stdout}");
}

#[test]
fn multiple_rules_comma_separated() {
    let (code, stdout, _) = sastql(&[
        "check",
        "tests/fixtures/realistic",
        "--dialect",
        "postgres",
        "--rules",
        "select-star,grant-all",
    ]);
    assert_eq!(code, 1, "stdout:\n{stdout}");
    assert!(stdout.contains("select-star"), "stdout:\n{stdout}");
    assert!(stdout.contains("grant-all"), "stdout:\n{stdout}");
    assert!(stdout.contains("3 violation(s)"), "stdout:\n{stdout}"); // 2 + 1
}

/// Statement-level line/column (roadmap item 1): the violation points at the
/// line of its offending statement. Pinned from real output, never invented:
/// `DELETE FROM sessions` sits on line 4 of tests/fixtures/dirty/mutations.sql.
/// The file field is deliberately left out of the pin (path separators differ
/// across platforms); rule/severity/line/column are platform-stable.
#[test]
fn violation_carries_statement_line_and_column() {
    let (code, stdout, _) = sastql(&[
        "check",
        "tests/fixtures/dirty",
        "--dialect",
        "sqlite",
        "--format",
        "json",
    ]);
    assert_eq!(code, 1, "stdout:\n{stdout}");
    assert!(
        stdout.contains(
            "\"rule_id\": \"no-where-on-mutation\", \"severity\": \"error\", \"line\": 4, \"column\": 1"
        ),
        "pinned line/column missing in:\n{stdout}"
    );
}

// --- SARIF (roadmap item 3) ---------------------------------------------------

/// SARIF for GitHub code scanning, pinned from the same fixture as
/// `violation_carries_statement_line_and_column` (`DELETE FROM sessions` sits
/// on line 4 of tests/fixtures/dirty/mutations.sql).
#[test]
fn sarif_format_pins_rule_and_position() {
    let (code, stdout, _) = sastql(&[
        "check",
        "tests/fixtures/dirty",
        "--dialect",
        "sqlite",
        "--format",
        "sarif",
    ]);
    assert_eq!(code, 1, "stdout:\n{stdout}");
    assert!(
        stdout.contains("\"version\": \"2.1.0\""),
        "stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("\"ruleId\": \"no-where-on-mutation\""),
        "stdout:\n{stdout}"
    );
    assert!(stdout.contains("\"startLine\": 4"), "stdout:\n{stdout}");
    assert!(
        stdout.contains("\"uri\": \"tests/fixtures/dirty/mutations.sql\""),
        "forward-slash uri missing in:\n{stdout}"
    );
}

// --- sastql.toml: severity overrides and named rule sets ---------------------

/// Per-test temp dir: same pattern as `parse_error_exits_2_with_file_and_message`,
/// suffixed per test so parallel runs never share state.
fn temp_dir_named(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("sastql-test-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir should be creatable");
    dir
}

#[test]
fn config_severity_override_changes_emitted_severity() {
    let dir = temp_dir_named("sev-override");
    let cfg = dir.join("sastql.toml");
    std::fs::write(&cfg, "[severity]\nselect-star = \"error\"\n").expect("config writable");
    let path = cfg.to_str().expect("utf-8 path");

    let (code, stdout, stderr) = sastql(&[
        "check",
        "tests/fixtures/dirty",
        "--dialect",
        "sqlite",
        "--format",
        "json",
        "--config",
        path,
        path,
    ]);
    assert_eq!(code, 1, "stdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(
        stdout.contains("\"rule_id\": \"select-star\", \"severity\": \"error\""),
        "override must reach the JSON output: {stdout}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn rules_accepts_named_set_from_config() {
    let dir = temp_dir_named("rules-set");
    let cfg = dir.join("sastql.toml");
    std::fs::write(
        &cfg,
        "[sets.danger]\nrules = [\"select-star\", \"table-no-pk\"]\n",
    )
    .expect("config writable");
    let path = cfg.to_str().expect("utf-8 path");

    let (code, stdout, stderr) = sastql(&[
        "check",
        "tests/fixtures/dirty",
        "--dialect",
        "sqlite",
        "--config",
        path,
        "--rules",
        "danger",
    ]);
    assert_eq!(code, 1, "stdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(stdout.contains("select-star"), "stdout:\n{stdout}");
    assert!(stdout.contains("table-no-pk"), "stdout:\n{stdout}");
    assert!(
        !stdout.contains("no-where-on-mutation"),
        "only the set's rules may run: {stdout}"
    );
    assert!(stdout.contains("2 violation(s)"), "stdout:\n{stdout}");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn config_missing_explicit_path_exits_2() {
    let (code, _, stderr) = sastql(&[
        "check",
        "tests/fixtures/clean",
        "--config",
        "no-such-config.toml",
    ]);
    assert_eq!(code, 2);
    assert!(stderr.contains("no-such-config.toml"), "stderr:\n{stderr}");
}

#[test]
fn unknown_token_lists_rules_and_sets() {
    let dir = temp_dir_named("unknown-token");
    let cfg = dir.join("sastql.toml");
    std::fs::write(&cfg, "[sets.danger]\nrules = [\"select-star\"]\n").expect("config writable");
    let path = cfg.to_str().expect("utf-8 path");

    let (code, _, stderr) = sastql(&[
        "check",
        "tests/fixtures/clean",
        "--config",
        path,
        "--rules",
        "nope",
    ]);
    assert_eq!(code, 2);
    assert!(
        stderr.contains("unknown rule or set 'nope'"),
        "stderr:\n{stderr}"
    );
    assert!(stderr.contains("available rules:"), "stderr:\n{stderr}");
    assert!(stderr.contains("available sets:"), "stderr:\n{stderr}");
    assert!(stderr.contains("danger"), "stderr:\n{stderr}");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn config_auto_discovered_in_current_directory() {
    let dir = temp_dir_named("autodiscovery");
    std::fs::write(
        dir.join("sastql.toml"),
        "[severity]\nno-where-on-mutation = \"info\"\n",
    )
    .expect("config writable");
    let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/dirty")
        .to_str()
        .expect("utf-8 path")
        .to_string();

    // `--config` absent: the binary must pick up ./sastql.toml from its cwd.
    let output = Command::new(env!("CARGO_BIN_EXE_sastql"))
        .args([
            "check",
            &fixtures,
            "--dialect",
            "sqlite",
            "--format",
            "json",
        ])
        .current_dir(&dir)
        .output()
        .expect("sastql binary should run");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    assert_eq!(code, 1, "stdout:\n{stdout}");
    assert!(
        stdout.contains("\"rule_id\": \"no-where-on-mutation\", \"severity\": \"info\""),
        "auto-discovered config must apply (default severity is error): {stdout}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

// --- --check-app: SQL strings in application source (roadmap item 4) ----------

#[test]
fn check_app_flags_sql_concatenation_in_ts() {
    let dir = temp_dir_named("check-app-ts");
    let file = dir.join("db.ts");
    std::fs::write(
        &file,
        "const id = req.params.id;\nconst q = \"SELECT * FROM users WHERE id = \" + id;\n",
    )
    .expect("temp file should be writable");
    let path = file.to_str().expect("utf-8 path");

    let (code, stdout, stderr) = sastql(&["check", path, "--check-app", "--format", "json"]);
    assert_eq!(code, 1, "stdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(
        stdout.contains("\"rule_id\": \"sql-concatenation\""),
        "stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("\"line\": 2"),
        "pinned line missing:\n{stdout}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn check_app_clean_parameterized_files_exit_0() {
    let dir = temp_dir_named("check-app-clean");
    std::fs::write(
        dir.join("app.py"),
        "q = \"SELECT * FROM users WHERE id = ?\"\n",
    )
    .expect("temp file should be writable");
    std::fs::write(
        dir.join("main.rs"),
        "static QUERY: &str = \"SELECT * FROM users WHERE id = $1\";\n",
    )
    .expect("temp file should be writable");
    let path = dir.to_str().expect("utf-8 path");

    let (code, stdout, stderr) = sastql(&["check", path, "--check-app"]);
    assert_eq!(code, 0, "stdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(
        stdout.contains("2 file(s) scanned, 0 violation(s)"),
        "stdout:\n{stdout}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn check_app_rejects_rules_flag() {
    let (code, _, stderr) = sastql(&[
        "check",
        "tests/fixtures/clean",
        "--check-app",
        "--rules",
        "select-star",
    ]);
    assert_eq!(code, 2);
    assert!(
        stderr.contains("--rules does not apply to --check-app"),
        "stderr:\n{stderr}"
    );
}

#[test]
fn check_app_no_app_files_exits_2() {
    // tests/fixtures/clean holds only .sql files.
    let (code, _, stderr) = sastql(&["check", "tests/fixtures/clean", "--check-app"]);
    assert_eq!(code, 2);
    assert!(
        stderr.contains("no Rust/TS/JS/Python source files found"),
        "stderr:\n{stderr}"
    );
}
