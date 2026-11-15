//! The desktop supervisor binary.
//!
//! `qr-desktop` supervises six sidecars in desktop profile, or does nothing in
//! external profile. The same binary runs `--guard` around one sidecar, which
//! is how a supervisor crash ends its children instead of leaving orphans. See
//! `lib.rs` and `README.md` for the boundary.

#![forbid(unsafe_code)]

use std::ffi::OsString;
use std::process::ExitCode;

use quantrule_desktop_supervisor::cli::{self, Mode};
use quantrule_desktop_supervisor::{guard, run};

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    if args.len() == 1
        && args
            .first()
            .is_some_and(|argument| matches!(argument.to_str(), Some("--help" | "-h")))
    {
        print!("{}", cli::USAGE);
        return ExitCode::SUCCESS;
    }
    match cli::parse(args) {
        Ok(Mode::Supervise(config)) => code(run::run(config)),
        Ok(Mode::Guard(args)) => code(guard::run(args)),
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(2)
        }
    }
}

fn code(value: i32) -> ExitCode {
    ExitCode::from(u8::try_from(value).unwrap_or(1))
}
