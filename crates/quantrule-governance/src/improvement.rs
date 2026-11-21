//! The `improvement` command: feasibility preflight and cleanup scan.

use std::path::Path;
use std::process::ExitCode;

mod cleanup;
mod feasibility;
mod report;

/// Run `qr-governance improvement`.
///
/// Usage:
///
/// ```text
/// qr-governance improvement feasibility --strategy <path> --data <path> --cost-bps <bps>
/// qr-governance improvement cleanup --check [--paths ...] [--exclude ...] [--terms-only]
/// ```
#[must_use]
pub fn run(args: &[String]) -> ExitCode {
    let Some(command) = args.first().map(String::as_str) else {
        eprintln!("usage: qr-governance improvement <feasibility|cleanup> ...");
        return ExitCode::from(2);
    };
    match command {
        "feasibility" => feasibility_command(&args[1..]),
        "cleanup" => cleanup_command(&args[1..]),
        other => {
            eprintln!("error: unknown improvement command {other}");
            ExitCode::from(2)
        }
    }
}

fn feasibility_command(args: &[String]) -> ExitCode {
    let mut strategy = None;
    let mut data = None;
    let mut cost_bps = None;
    let mut max_bars = feasibility::DEFAULT_MAX_BARS;
    let mut out = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--strategy" => strategy = value(args, &mut index, "--strategy"),
            "--data" => data = value(args, &mut index, "--data"),
            "--cost-bps" => {
                let Some(raw) = value(args, &mut index, "--cost-bps") else {
                    return ExitCode::from(2);
                };
                match raw.parse::<f64>() {
                    Ok(raw) => cost_bps = Some(raw),
                    Err(_) => {
                        eprintln!("error: --cost-bps must be a number");
                        return ExitCode::from(2);
                    }
                }
            }
            "--max-bars" => {
                let Some(raw) = value(args, &mut index, "--max-bars") else {
                    return ExitCode::from(2);
                };
                match raw.parse::<usize>() {
                    Ok(raw) => max_bars = raw,
                    Err(_) => {
                        eprintln!("error: --max-bars must be an integer");
                        return ExitCode::from(2);
                    }
                }
            }
            "--out" => out = value(args, &mut index, "--out"),
            other => {
                eprintln!("error: unknown argument {other}");
                return ExitCode::from(2);
            }
        }
        index += 1;
    }
    let (Some(strategy), Some(data), Some(cost_bps)) = (strategy, data, cost_bps) else {
        eprintln!("error: --strategy, --data, and --cost-bps are required");
        return ExitCode::from(2);
    };
    let strategy = expand(&strategy);
    let data = expand(&data);
    let strategy = Path::new(&strategy);
    let data = Path::new(&data);
    let loaded_strategy = match feasibility::load_strategy(strategy) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::from(2);
        }
    };
    let loaded_data = match feasibility::load_data(data) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::from(2);
        }
    };
    let report = match feasibility::evaluate(&loaded_strategy, &loaded_data, cost_bps, max_bars) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::from(2);
        }
    };
    if let Some(out) = out {
        let out = expand(&out);
        match feasibility::write_report(&report, Path::new(&out)) {
            Ok(path) => println!("report: {}", path.display()),
            Err(error) => {
                eprintln!("error: {error}");
                return ExitCode::from(2);
            }
        }
    }
    println!("{}", feasibility::format_report(&report));
    if report.verdict == "pass" {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn cleanup_command(args: &[String]) -> ExitCode {
    let mut check = false;
    let mut paths = vec!["docs".to_owned(), "README.md".to_owned(), "data".to_owned()];
    let mut excludes = Vec::new();
    let mut terms_only = false;
    let mut root = ".".to_owned();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--check" => check = true,
            "--terms-only" => terms_only = true,
            "--paths" => {
                let values = values(args, &mut index, "--paths");
                if values.is_empty() {
                    return ExitCode::from(2);
                }
                paths = values;
            }
            "--exclude" => {
                let values = values(args, &mut index, "--exclude");
                if values.is_empty() {
                    return ExitCode::from(2);
                }
                excludes = values;
            }
            "--root" => {
                let Some(value) = value(args, &mut index, "--root") else {
                    return ExitCode::from(2);
                };
                root = value;
            }
            other => {
                eprintln!("error: unknown argument {other}");
                return ExitCode::from(2);
            }
        }
        index += 1;
    }
    if !check {
        eprintln!("error: --check is required; this command only reports, it never deletes");
        return ExitCode::from(2);
    }
    let options = cleanup::Options {
        root: Path::new(&root).to_path_buf(),
        paths,
        excludes,
        terms_only,
    };
    ExitCode::from(cleanup::run(&options))
}

fn value(args: &[String], index: &mut usize, name: &str) -> Option<String> {
    *index += 1;
    let value = args.get(*index).cloned();
    if value.is_none() {
        eprintln!("error: {name} needs a value");
    }
    value
}

fn values(args: &[String], index: &mut usize, name: &str) -> Vec<String> {
    let mut values = Vec::new();
    while *index + 1 < args.len() && !args[*index + 1].starts_with("--") {
        *index += 1;
        values.push(args[*index].clone());
    }
    if values.is_empty() {
        eprintln!("error: {name} needs at least one value");
    }
    values
}

fn expand(value: &str) -> String {
    if let Some(rest) = value.strip_prefix("~/")
        && let Ok(home) = std::env::var("HOME")
    {
        return format!("{home}/{rest}");
    }
    value.to_owned()
}
