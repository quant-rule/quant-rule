//! The `check` command.

use std::path::Path;
use std::process::ExitCode;

use crate::checks;
use crate::report::Report;

/// Run `qr-governance check`.
///
/// Usage:
///
/// ```text
/// qr-governance check --all [--root <path>]
/// qr-governance check --checker <name> [--root <path>]
/// qr-governance check --list
/// ```
#[must_use]
pub fn run(args: &[String]) -> ExitCode {
    match parse(args) {
        Ok(Action::List) => {
            print!("{}", checks::names());
            ExitCode::SUCCESS
        }
        Ok(Action::Check { only, root }) => match checks::run(&root, only.as_deref()) {
            Ok(report) => finish(&report),
            Err(error) => {
                eprintln!("error: {error}");
                ExitCode::FAILURE
            }
        },
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(2)
        }
    }
}

fn finish(report: &Report) -> ExitCode {
    if report.is_ok() {
        println!("governance check ok");
        ExitCode::SUCCESS
    } else {
        eprint!("{}", report.render());
        eprintln!(
            "governance check failed: {} finding(s)",
            report.diagnostics.len()
        );
        ExitCode::FAILURE
    }
}

enum Action {
    List,
    Check {
        only: Option<String>,
        root: std::path::PathBuf,
    },
}

fn parse(args: &[String]) -> Result<Action, String> {
    let mut all = false;
    let mut only = None;
    let mut list = false;
    let mut root = Path::new(".").to_path_buf();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--all" => all = true,
            "--list" => list = true,
            "--checker" => {
                index += 1;
                only = Some(
                    args.get(index)
                        .ok_or_else(|| "--checker needs a name".to_owned())?
                        .clone(),
                );
            }
            "--root" => {
                index += 1;
                root = std::path::PathBuf::from(
                    args.get(index)
                        .ok_or_else(|| "--root needs a path".to_owned())?,
                );
            }
            other => return Err(format!("unknown argument {other}")),
        }
        index += 1;
    }
    if list {
        return Ok(Action::List);
    }
    if all && only.is_some() {
        return Err("--all and --checker are mutually exclusive".to_owned());
    }
    if !all && only.is_none() {
        return Err("choose --all, --checker <name>, or --list".to_owned());
    }
    Ok(Action::Check { only, root })
}
