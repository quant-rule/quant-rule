//! The `registry` command: validate and regenerate the registry table.

use std::path::Path;
use std::process::ExitCode;

use crate::checks;
use crate::report::Report;

/// Run `qr-governance registry`.
///
/// Usage:
///
/// ```text
/// qr-governance registry [--write] [--root <path>]
/// ```
#[must_use]
pub fn run(args: &[String]) -> ExitCode {
    let mut write = false;
    let mut root = Path::new(".").to_path_buf();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--write" => write = true,
            "--root" => {
                index += 1;
                match args.get(index) {
                    Some(path) => root = Path::new(path).to_path_buf(),
                    None => {
                        eprintln!("error: --root needs a path");
                        return ExitCode::from(2);
                    }
                }
            }
            other => {
                eprintln!("error: unknown argument {other}");
                return ExitCode::from(2);
            }
        }
        index += 1;
    }
    if write && let Err(error) = checks::registry::regenerate(&root) {
        eprintln!("error: {error}");
        return ExitCode::FAILURE;
    }
    match checks::run(&root, Some("registry")) {
        Ok(report) => finish(&report, write),
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn finish(report: &Report, write: bool) -> ExitCode {
    if report.is_ok() {
        if write {
            println!("wrote docs/registry/indicators.md");
        } else {
            println!("registry ok");
        }
        ExitCode::SUCCESS
    } else {
        eprint!("{}", report.render());
        eprintln!(
            "registry check failed: {} finding(s)",
            report.diagnostics.len()
        );
        ExitCode::FAILURE
    }
}
