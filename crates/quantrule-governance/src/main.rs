//! The `qr-governance` command.
//!
//! Usage:
//!
//! ```text
//! qr-governance check --all | --checker <name> | --list [--root <path>]
//! qr-governance history list|archive|erase ...
//! ```

#![forbid(unsafe_code)]
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(command) = args.first() else {
        eprintln!("usage: qr-governance <check|history> ...");
        return ExitCode::from(2);
    };
    match command.as_str() {
        "check" => quantrule_governance::check::run(&args[1..]),
        "release" => quantrule_governance::release::run(&args[1..]),
        "history" => match quantrule_governance::history::run(&args) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("error: {error}");
                ExitCode::FAILURE
            }
        },
        other => {
            eprintln!("error: unknown command {other}");
            ExitCode::from(2)
        }
    }
}
