mod analyzer;
mod parser;
mod report;
mod rules;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::exit;

use clap::{Parser as ClapParser, Subcommand};

use parser::DialectKind;
use report::{OutputFormat, Report};

/// SastQL — SAST linter for SQL files (PostgreSQL & SQLite).
#[derive(ClapParser)]
#[command(name = "sastql", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Check .sql files (directories are walked recursively).
    Check {
        /// Files or directories to scan.
        paths: Vec<PathBuf>,
        /// Target SQL dialect the code is written for.
        #[arg(long, value_enum, default_value_t = DialectKind::Postgres)]
        dialect: DialectKind,
        /// Report format.
        #[arg(long, value_enum, default_value = "human")]
        format: OutputFormat,
        /// Only run these rules (comma-separated or repeated).
        #[arg(long, value_delimiter = ',')]
        rules: Option<Vec<String>>,
    },
}

fn main() {
    exit(run());
}

fn run() -> i32 {
    let Command::Check {
        paths,
        dialect,
        format,
        rules: rules_filter,
    } = Cli::parse().command;

    if paths.is_empty() {
        eprintln!("error: no paths given (pass .sql files or directories)");
        return 2;
    }

    let mut files = Vec::new();
    for path in &paths {
        if path.is_file() && path.extension().is_some_and(|ext| ext != "sql") {
            eprintln!(
                "warning: only .sql files are scanned, skipping {}",
                path.display()
            );
        }
        if let Err(err) = collect_sql_files(path, &mut files) {
            eprintln!("error: {}: {err}", path.display());
            return 2;
        }
    }
    files.sort_unstable();
    files.dedup();
    if files.is_empty() {
        eprintln!("error: no .sql files found in the given paths");
        return 2;
    }

    let all = rules::all_rules();
    let selected: Vec<_> = match rules_filter {
        None => rules::default_rules(),
        Some(ids) => {
            if let Some(unknown) = ids.iter().find(|id| !all.iter().any(|r| r.id() == *id)) {
                eprintln!("error: unknown rule '{}'", unknown);
                eprintln!("available rules:");
                for r in &all {
                    eprintln!("  {} — {}", r.id(), r.description());
                }
                return 2;
            }
            all.into_iter()
                .filter(|r| ids.iter().any(|id| r.id() == id))
                .collect()
        }
    };

    let analyzer = analyzer::Analyzer { rules: selected };
    let mut violations = Vec::new();
    for file in &files {
        let sql = match fs::read_to_string(file) {
            Ok(sql) => sql,
            Err(err) => {
                eprintln!("error: {}: {err}", file.display());
                return 2;
            }
        };
        // Fail loudly on parse errors: never silently skip a file (CLAUDE.md).
        let statements = match parser::parse(&sql, dialect) {
            Ok(statements) => statements,
            Err(err) => {
                eprintln!("error: {}: parse error: {err}", file.display());
                return 2;
            }
        };
        violations.extend(analyzer.analyze(&file.display().to_string(), &statements, dialect));
    }

    let report = Report {
        violations,
        files_scanned: files.len(),
    };
    print!("{}", format.render(&report));
    report.exit_code()
}

fn collect_sql_files(path: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
    if path.is_dir() {
        // ponytail: stdlib walk; add `walkdir` only if symlink/depth handling matters.
        let mut entries: Vec<_> = fs::read_dir(path)?.collect::<Result<Vec<_>, _>>()?;
        entries.sort_unstable_by_key(|entry| entry.file_name());
        for entry in entries {
            collect_sql_files(&entry.path(), out)?;
        }
    } else if path.extension().is_some_and(|ext| ext == "sql") {
        out.push(path.to_path_buf());
    }
    Ok(())
}
