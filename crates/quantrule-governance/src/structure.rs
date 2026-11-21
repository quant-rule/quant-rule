//! The `structure` command: validate structure or print the scorecard.

use std::path::Path;
use std::process::ExitCode;

use crate::checks;
use crate::report::Report;

/// Run `qr-governance structure`.
///
/// Usage:
///
/// ```text
/// qr-governance structure [--report] [--root <path>]
/// ```
#[must_use]
pub fn run(args: &[String]) -> ExitCode {
    let mut report = false;
    let mut root = Path::new(".").to_path_buf();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--report" => report = true,
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
    if report {
        println!("{}", checks::structure::report(&root));
        return ExitCode::SUCCESS;
    }
    match checks::run(&root, Some("structure")) {
        Ok(report) => finish(&report),
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn finish(report: &Report) -> ExitCode {
    if report.is_ok() {
        println!(
            "structure ok: module docs, engine purity, naming, layout, and data allowlist pass"
        );
        ExitCode::SUCCESS
    } else {
        eprint!("{}", report.render());
        eprintln!(
            "structure check failed: {} finding(s)",
            report.diagnostics.len()
        );
        ExitCode::FAILURE
    }
}
