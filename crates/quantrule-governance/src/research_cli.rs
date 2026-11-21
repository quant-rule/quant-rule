//! The `research` command: program, cell, and registry tooling.

use std::process::ExitCode;

/// Run `qr-governance research`.
///
/// Usage:
///
/// ```text
/// qr-governance research program --program <path> [--phase dev|validation] [--check]
/// qr-governance research cell --program <path> --cell <id> --out <path>
/// qr-governance research registry --program <path> --report <path> [--out <path>]
/// ```
#[must_use]
pub fn run(args: &[String]) -> ExitCode {
    let Some(command) = args.first().map(String::as_str) else {
        eprintln!("usage: qr-governance research <program|cell|registry> ...");
        return ExitCode::from(2);
    };
    let result = match command {
        "program" => crate::research::program::run(&args[1..]),
        "batch" => crate::research::batch::run(&args[1..]),
        "cell" => crate::research::cell::run(&args[1..]),
        "registry" => crate::research::registry::run(&args[1..]),
        "report" => crate::research::report::run(&args[1..]),
        other => {
            eprintln!("error: unknown research command {other}");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
