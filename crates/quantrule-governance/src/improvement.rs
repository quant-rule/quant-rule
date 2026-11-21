//! The `improvement` command: feasibility preflight and cleanup scan.

use std::path::Path;
use std::process::ExitCode;

mod cleanup;
mod evidence;
mod feasibility;
mod index;
mod manifest;
mod model_path;
mod report;
mod run_current;
mod store;

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
        "evidence" => evidence_command(&args[1..]),
        "index" => index_command(&args[1..]),
        "run-current" => run_current_command(&args[1..]),
        "validate" => validate_command(&args[1..]),
        "model-path" => match model_path::run(&args[1..]) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("error: {error}");
                ExitCode::FAILURE
            }
        },
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

fn evidence_command(args: &[String]) -> ExitCode {
    let Some(run_dir) = args.first() else {
        eprintln!("error: evidence needs a run directory");
        return ExitCode::FAILURE;
    };
    match evidence::run(Path::new(run_dir)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn index_command(args: &[String]) -> ExitCode {
    let mut root = None;
    let mut repo_root = Path::new(".").to_path_buf();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--root" => {
                let Some(value) = value(args, &mut index, "--root") else {
                    return ExitCode::from(2);
                };
                root = Some(Path::new(&value).to_path_buf());
            }
            "--repo-root" => {
                let Some(value) = value(args, &mut index, "--repo-root") else {
                    return ExitCode::from(2);
                };
                repo_root = Path::new(&value).to_path_buf();
            }
            other => {
                eprintln!("error: unknown argument {other}");
                return ExitCode::from(2);
            }
        }
        index += 1;
    }
    let Some(root) = root else {
        eprintln!("error: index needs --root");
        return ExitCode::from(2);
    };
    match index::build(&root, &repo_root) {
        Ok(count) => {
            println!("indexed {count} run(s) from {}", root.display());
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn validate_command(args: &[String]) -> ExitCode {
    let Some(manifest_path) = args.first() else {
        eprintln!("error: validate needs a manifest path");
        return ExitCode::from(2);
    };
    let mut repo_root = Path::new(".").to_path_buf();
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--repo-root" => {
                let Some(value) = value(args, &mut index, "--repo-root") else {
                    return ExitCode::from(2);
                };
                repo_root = Path::new(&value).to_path_buf();
            }
            other => {
                eprintln!("error: unknown argument {other}");
                return ExitCode::from(2);
            }
        }
        index += 1;
    }
    let path = Path::new(manifest_path);
    if !path.is_file() {
        eprintln!("error: manifest not found: {}", path.display());
        return ExitCode::FAILURE;
    }
    let manifest = match store::read_json(path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::FAILURE;
        }
    };
    let errors = manifest::validate(&manifest, &repo_root);
    if !errors.is_empty() {
        for error in errors {
            eprintln!("error: {error}");
        }
        return ExitCode::FAILURE;
    }
    let run_id = manifest
        .get("run_id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    println!("ok {run_id}");
    ExitCode::SUCCESS
}

fn run_current_command(args: &[String]) -> ExitCode {
    let Some(manifest_path) = args.first() else {
        eprintln!("error: run-current needs a manifest path");
        return ExitCode::from(2);
    };
    let mut qr = run_current::default_shell().display().to_string();
    let mut repo_root = Path::new(".").to_path_buf();
    let mut store_root = None;
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--qr" => {
                let Some(value) = value(args, &mut index, "--qr") else {
                    return ExitCode::from(2);
                };
                qr = value;
            }
            "--repo-root" => {
                let Some(value) = value(args, &mut index, "--repo-root") else {
                    return ExitCode::from(2);
                };
                repo_root = Path::new(&value).to_path_buf();
            }
            "--store-root" => {
                let Some(value) = value(args, &mut index, "--store-root") else {
                    return ExitCode::from(2);
                };
                store_root = Some(Path::new(&value).to_path_buf());
            }
            other => {
                eprintln!("error: unknown argument {other}");
                return ExitCode::from(2);
            }
        }
        index += 1;
    }
    let repo_root = repo_root.canonicalize().unwrap_or(repo_root);
    let store_root = store_root.unwrap_or_else(|| repo_root.join("data/improvement"));
    match run_current::run(Path::new(manifest_path), &qr, &repo_root, &store_root) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
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
