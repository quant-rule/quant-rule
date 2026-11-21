//! The `release note` command.

use std::path::Path;
use std::process::ExitCode;

/// Resolve a current tag to its release note and print the repository-relative
/// path.
#[must_use]
pub fn run(args: &[String]) -> ExitCode {
    let Some(subcommand) = args.first() else {
        eprintln!("usage: qr-governance release note --tag <vX.Y.Z>");
        return ExitCode::from(2);
    };
    if subcommand != "note" {
        eprintln!("error: unknown release command {subcommand}");
        return ExitCode::from(2);
    }
    let mut tag = None;
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--tag" => {
                index += 1;
                let Some(value) = args.get(index) else {
                    eprintln!("error: --tag needs a value");
                    return ExitCode::from(2);
                };
                tag = Some(value.clone());
            }
            other => {
                eprintln!("error: unknown argument {other}");
                return ExitCode::from(2);
            }
        }
        index += 1;
    }
    let Some(tag) = tag else {
        eprintln!("error: release note needs --tag <vX.Y.Z>");
        return ExitCode::from(2);
    };
    let root = Path::new(".");
    match crate::checks::releases::resolve(root, &tag) {
        Ok(note) => {
            println!(
                "{}",
                note.strip_prefix(root)
                    .unwrap_or(&note)
                    .to_string_lossy()
                    .replace('\\', "/")
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
