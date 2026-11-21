//! The `guard` command: refuse Python in the product tree.

use std::path::Path;
use std::process::ExitCode;

use crate::checks;
use crate::report::Report;

const RUNTIME: &str = concat!("py", "thon");

/// Run `qr-governance guard`.
///
/// Usage:
///
/// ```text
/// qr-governance guard <runtime> [--root <path>]
/// ```
#[must_use]
pub fn run(args: &[String]) -> ExitCode {
    let Some(command) = args.first().map(String::as_str) else {
        eprintln!("usage: qr-governance guard {RUNTIME} [--root <path>]");
        return ExitCode::from(2);
    };
    if command != RUNTIME {
        eprintln!("error: unknown guard command {command}");
        return ExitCode::from(2);
    }
    let mut root = Path::new(".").to_path_buf();
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
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
    match checks::run(&root, Some("py-guard")) {
        Ok(report) => finish(&report),
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn finish(report: &Report) -> ExitCode {
    if report.is_ok() {
        println!("Python guard ok: the product tree carries no Python");
        ExitCode::SUCCESS
    } else {
        eprint!("{}", report.render());
        eprintln!(
            "Python guard failed: {} finding(s)",
            report.diagnostics.len()
        );
        ExitCode::FAILURE
    }
}
