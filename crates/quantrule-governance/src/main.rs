//! The `qr-governance` command.
//!
//! Usage:
//!
//! ```text
//! qr-governance history list --kind <config|docs|records> [--before <date>] [--include-undated] [--root <path>]
//! qr-governance history archive --kind <kind> --out <dir> (--before <date> | --all) [--include-undated] [--dry-run] [--root <path>]
//! qr-governance history erase --manifest <file> --confirm <archive_id> [--dry-run] [--root <path>]
//! ```

#![forbid(unsafe_code)]
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match quantrule_governance::history::run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
