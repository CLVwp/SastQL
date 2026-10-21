mod analyzer;
mod appscan;
mod config;
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
use rules::Rule;

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
        /// Only run these rules or named sets from the config (comma-separated or repeated).
        #[arg(long, value_delimiter = ',')]
        rules: Option<Vec<String>>,
        /// Scan app source (Rust/TS/JS/Python) for injection-prone SQL string
        /// concatenation instead of analyzing .sql files.
        #[arg(long)]
        check_app: bool,
        /// Config file (default: auto-discover ./sastql.toml).
        #[arg(long, value_name = "PATH")]
        config: Option<PathBuf>,
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
        config: config_path,
        check_app,
    } = Cli::parse().command;

    if paths.is_empty() {
        eprintln!("error: no paths given (pass .sql files or directories)");
        return 2;
    }

    // Fail loudly rather than silently ignoring the flag combination.
    if check_app && rules_filter.is_some() {
        eprintln!("error: --rules does not apply to --check-app");
        return 2;
    }

    // An invalid config is a usage error; a missing auto-discovered file is
    // not an error — default behavior applies.
    let config = match config::Config::load(config_path.as_deref()) {
        Ok(Some(c)) => c,
        Ok(None) => config::Config::default(),
        Err(err) => {
            eprintln!("error: {err}");
            return 2;
        }
    };

    // --check-app targets application source; otherwise the usual .sql files.
    const SQL_EXTS: &[&str] = &["sql"];
    const APP_EXTS: &[&str] = &["rs", "py", "ts", "tsx", "js", "mjs", "cjs", "jsx"];
    let (wanted, ext_list) = if check_app {
        (APP_EXTS, ".rs/.py/.ts/.tsx/.js/.mjs/.cjs/.jsx")
    } else {
        (SQL_EXTS, ".sql")
    };

    let mut files = Vec::new();
    for path in &paths {
        if path.is_file()
            && !path
                .extension()
                .is_some_and(|ext| wanted.contains(&ext.to_string_lossy().as_ref()))
        {
            eprintln!(
                "warning: only {ext_list} files are scanned, skipping {}",
                path.display()
            );
        }
        if let Err(err) = collect_files(path, wanted, &mut files) {
            eprintln!("error: {}: {err}", path.display());
            return 2;
        }
    }
    files.sort_unstable();
    files.dedup();
    if files.is_empty() {
        if check_app {
            eprintln!("error: no Rust/TS/JS/Python source files found in the given paths");
        } else {
            eprintln!("error: no .sql files found in the given paths");
        }
        return 2;
    }

    let all = rules::all_rules();
    let selected: Vec<_> = match rules_filter {
        None => rules::default_rules(),
        Some(tokens) => {
            // Each token resolves as an exact rule id first, then as a named
            // set from the config. Expansion preserves the set's order; ids
            // are deduped across tokens.
            let mut ids: Vec<&str> = Vec::new();
            for token in &tokens {
                let resolved: Option<Vec<&str>> = if all.iter().any(|r| r.id() == token) {
                    Some(vec![token])
                } else {
                    config
                        .sets
                        .get(token)
                        .map(|set| set.iter().map(String::as_str).collect())
                };
                match resolved {
                    Some(expanded) => {
                        for id in expanded {
                            if !ids.contains(&id) {
                                ids.push(id);
                            }
                        }
                    }
                    None => {
                        eprintln!("error: unknown rule or set '{token}'");
                        eprintln!("available rules:");
                        for r in &all {
                            eprintln!("  {} — {}", r.id(), r.description());
                        }
                        if config.sets.is_empty() {
                            eprintln!(
                                "available sets: (none — declare [sets.<name>] in sastql.toml)"
                            );
                        } else {
                            eprintln!("available sets:");
                            for name in config.sets.keys() {
                                eprintln!("  {name}");
                            }
                        }
                        return 2;
                    }
                }
            }
            // Keep the resolved order (a set's order is the author's intent).
            // Registry is tiny — position+remove beats a lifetime-carrying map.
            let mut all = all;
            let mut selected: Vec<Box<dyn Rule>> = Vec::with_capacity(ids.len());
            for id in &ids {
                let pos = all
                    .iter()
                    .position(|r| r.id() == *id)
                    .expect("ids validated against rules and sets");
                selected.push(all.remove(pos));
            }
            selected
        }
    };

    let analyzer = analyzer::Analyzer { rules: selected };
    let mut violations = Vec::new();
    for file in &files {
        let content = match fs::read_to_string(file) {
            Ok(content) => content,
            Err(err) => {
                eprintln!("error: {}: {err}", file.display());
                return 2;
            }
        };
        if check_app {
            // App-source mode: the SQL parse + rules pipeline is skipped
            // entirely; appscan finds injection-prone SQL concatenation.
            let ext = file
                .extension()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            let lang =
                appscan::lang_for(&ext).expect("collected files all have a wanted extension");
            violations.extend(appscan::scan(
                &content,
                lang,
                dialect,
                &file.display().to_string(),
            ));
        } else {
            // Fail loudly on parse errors: never silently skip a file (CLAUDE.md).
            let statements = match parser::parse(&content, dialect) {
                Ok(statements) => statements,
                Err(err) => {
                    eprintln!("error: {}: parse error: {err}", file.display());
                    return 2;
                }
            };
            violations.extend(analyzer.analyze(&file.display().to_string(), &statements, dialect));
        }
    }

    // Config severity overrides apply after analysis — rules stay stateless.
    config.apply_severities(&mut violations);

    let report = Report {
        violations,
        files_scanned: files.len(),
    };
    print!("{}", format.render(&report));
    report.exit_code()
}

fn collect_files(path: &Path, wanted: &[&str], out: &mut Vec<PathBuf>) -> io::Result<()> {
    if path.is_dir() {
        // ponytail: stdlib walk; add `walkdir` only if symlink/depth handling matters.
        let mut entries: Vec<_> = fs::read_dir(path)?.collect::<Result<Vec<_>, _>>()?;
        entries.sort_unstable_by_key(|entry| entry.file_name());
        for entry in entries {
            collect_files(&entry.path(), wanted, out)?;
        }
    } else if path
        .extension()
        .is_some_and(|ext| wanted.contains(&ext.to_string_lossy().as_ref()))
    {
        out.push(path.to_path_buf());
    }
    Ok(())
}
