//! The `verification` command: build the evidence manifest and matrices.

use std::path::Path;
use std::process::ExitCode;

mod build;
mod docs;
mod evidence;
mod json;

/// Run `qr-governance verification`.
///
/// Usage:
///
/// ```text
/// qr-governance verification build [--root <path>]
/// ```
#[must_use]
pub fn run(args: &[String]) -> ExitCode {
    let Some(command) = args.first().map(String::as_str) else {
        eprintln!("usage: qr-governance verification build [--root <path>]");
        return ExitCode::from(2);
    };
    if command != "build" {
        eprintln!("error: unknown verification command {command}");
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
            "--write" => {}
            other => {
                eprintln!("error: unknown argument {other}");
                return ExitCode::from(2);
            }
        }
        index += 1;
    }
    match build::run(&root) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    #[test]
    fn the_builder_reproduces_the_committed_artifacts() {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "qr-verification-build-{}-{unique}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("docs/registry")).expect("registry tree");
        fs::create_dir_all(root.join("docs/model/2026-11-06")).expect("model tree");
        fs::create_dir_all(root.join("crates/quantrule-model/tests/fixtures"))
            .expect("fixture tree");
        for relative in [
            "docs/registry/indicators.json",
            "docs/registry/validation.json",
            "crates/quantrule-model/tests/fixtures/extended_events_golden.csv",
            "crates/quantrule-model/tests/fixtures/session_events_golden.csv",
        ] {
            fs::copy(repo.join(relative), root.join(relative)).expect("copy");
        }
        build::run(&root).expect("build");
        for relative in [
            "docs/registry/verification.json",
            "docs/model/2026-11-06/indicator-verification.md",
            "docs/model/2026-11-06/event-verification.md",
        ] {
            assert_eq!(
                fs::read(root.join(relative)).expect("generated"),
                fs::read(repo.join(relative)).expect("committed"),
                "{relative} differs"
            );
        }
        fs::remove_dir_all(&root).expect("cleanup");
    }
}
