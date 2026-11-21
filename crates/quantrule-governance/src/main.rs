//! The `qr-governance` command.
//!
//! Usage:
//!
//! ```text
//! qr-governance check --all | --checker <name> | --list [--root <path>]
//! qr-governance registry [--write] [--root <path>]
//! qr-governance structure [--report] [--root <path>]
//! qr-governance worktree check|archive|remove ...
//! qr-governance labels generate [--write] [--root <path>]
//! qr-governance verification build [--root <path>]
//! qr-governance improvement feasibility|evidence|index|run-current|validate|model-condition|model-path|cleanup ...
//! qr-governance guard <runtime> [--root <path>]
//! qr-governance history list|archive|erase ...
//! ```

#![forbid(unsafe_code)]
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(command) = args.first() else {
        eprintln!(
            "usage: qr-governance <check|registry|structure|worktree|labels|verification|improvement|guard|release|history> ..."
        );
        return ExitCode::from(2);
    };
    match command.as_str() {
        "check" => quantrule_governance::check::run(&args[1..]),
        "registry" => quantrule_governance::registry::run(&args[1..]),
        "structure" => quantrule_governance::structure::run(&args[1..]),
        "worktree" => quantrule_governance::worktree::run(&args[1..]),
        "labels" => quantrule_governance::labels::run(&args[1..]),
        "guard" => quantrule_governance::guard::run(&args[1..]),
        "verification" => quantrule_governance::verification::run(&args[1..]),
        "improvement" => quantrule_governance::improvement::run(&args[1..]),
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
