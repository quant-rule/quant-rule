//! The `gate` command: classify a change range for the scoped gate.

use std::path::Path;
use std::process::{Command, ExitCode};

use serde_json::json;

/// The gate scope for one change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Docs,
    Governance,
    Web,
    Full,
}

impl Mode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Docs => "docs",
            Self::Governance => "governance",
            Self::Web => "web",
            Self::Full => "full",
        }
    }

    fn checks(self) -> &'static [&'static str] {
        match self {
            Self::Docs => &["docs"],
            Self::Governance => &["docs", "governance"],
            Self::Web => &["docs", "web"],
            Self::Full => &["docs", "governance", "web"],
        }
    }
}

/// One classified change range.
struct Plan {
    mode: Mode,
    reasons: Vec<String>,
}

/// Run `qr-governance gate`.
///
/// Usage:
///
/// ```text
/// qr-governance gate plan --changed <base>..<head> [--json]
/// qr-governance gate plan --files <path>[,<path>...] [--json]
/// ```
#[must_use]
pub fn run(args: &[String]) -> ExitCode {
    match execute(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(CliError::Usage(message)) => {
            eprintln!("error: {message}");
            ExitCode::from(2)
        }
        Err(CliError::Failure(message)) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

enum CliError {
    Usage(String),
    Failure(String),
}

fn execute(args: &[String]) -> Result<(), CliError> {
    let Some(command) = args.first().map(String::as_str) else {
        return Err(CliError::Usage(
            "usage: qr-governance gate plan --changed <range>|--files <paths>".to_owned(),
        ));
    };
    if command != "plan" {
        return Err(CliError::Usage(format!("unknown gate command {command}")));
    }
    let mut changed = None;
    let mut files = None;
    let mut json = false;
    let mut field = None;
    let mut root = std::path::PathBuf::from(".");
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--changed" => changed = Some(value(args, &mut index, "--changed")?),
            "--files" => files = Some(value(args, &mut index, "--files")?),
            "--json" => json = true,
            "--field" => field = Some(value(args, &mut index, "--field")?),
            "--root" => root = std::path::PathBuf::from(value(args, &mut index, "--root")?),
            other => {
                return Err(CliError::Usage(format!("unknown argument {other}")));
            }
        }
        index += 1;
    }
    if changed.is_some() && files.is_some() {
        return Err(CliError::Usage(
            "--changed and --files are mutually exclusive".to_owned(),
        ));
    }
    let paths = match (changed, files) {
        (Some(range), None) => changed_paths(&root, &range)?,
        (None, Some(list)) => list
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .collect(),
        _ => {
            return Err(CliError::Usage(
                "choose --changed <range> or --files <paths>".to_owned(),
            ));
        }
    };
    let plan = classify(&paths);
    if let Some(field) = field {
        match field.as_str() {
            "mode" => println!("{}", plan.mode.as_str()),
            "checks" => println!("{}", plan.mode.checks().join(",")),
            other => {
                return Err(CliError::Usage(format!(
                    "unknown --field {other}; use mode or checks"
                )));
            }
        }
        return Ok(());
    }
    if json {
        let document = json!({
            "schema": "quantrule.gate-plan/v1",
            "mode": plan.mode.as_str(),
            "checks": plan.mode.checks(),
            "reasons": plan.reasons,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&document)
                .map_err(|error| CliError::Failure(error.to_string()))?
        );
    } else {
        println!("mode: {}", plan.mode.as_str());
        println!("checks: {}", plan.mode.checks().join(","));
        if plan.reasons.is_empty() {
            println!("reason: none");
        } else {
            for reason in &plan.reasons {
                println!("reason: {reason}");
            }
        }
    }
    Ok(())
}

fn value(args: &[String], index: &mut usize, name: &str) -> Result<String, CliError> {
    *index += 1;
    args.get(*index)
        .cloned()
        .ok_or_else(|| CliError::Usage(format!("{name} needs a value")))
}

fn changed_paths(root: &Path, range: &str) -> Result<Vec<String>, CliError> {
    if range.trim().is_empty() {
        return Err(CliError::Usage("--changed range is empty".to_owned()));
    }
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["diff", "--name-only", "--no-renames", range])
        .output()
        .map_err(|error| CliError::Failure(format!("cannot run git: {error}")))?;
    if !output.status.success() {
        return Err(CliError::Failure(format!(
            "git diff failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let text = String::from_utf8(output.stdout)
        .map_err(|error| CliError::Failure(format!("git output is not UTF-8: {error}")))?;
    Ok(text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| line.replace('\\', "/"))
        .collect())
}

fn classify(paths: &[String]) -> Plan {
    let mut docs = false;
    let mut governance = false;
    let mut web = false;
    let mut full = false;
    for path in paths {
        match class_path(path) {
            Mode::Docs => docs = true,
            Mode::Governance => governance = true,
            Mode::Web => web = true,
            Mode::Full => full = true,
        }
    }
    let mode = if full || (governance && web) {
        Mode::Full
    } else if governance {
        Mode::Governance
    } else if web {
        Mode::Web
    } else {
        let _ = docs;
        Mode::Docs
    };
    let mut reasons: Vec<String> = paths
        .iter()
        .map(|path| path.trim().to_owned())
        .filter(|path| !path.is_empty())
        .collect();
    reasons.sort();
    reasons.dedup();
    Plan { mode, reasons }
}

fn class_path(path: &str) -> Mode {
    let path = path.trim();
    if path.starts_with("docs/port-parity/") {
        return Mode::Full;
    }
    if path.starts_with("docs/") {
        return Mode::Docs;
    }
    if !path.contains('/') && path.ends_with(".md") {
        return Mode::Docs;
    }
    if path.starts_with("apps/web/") {
        return Mode::Web;
    }
    if path.starts_with("crates/quantrule-governance/") {
        return Mode::Governance;
    }
    if path.starts_with("crates/") {
        return Mode::Full;
    }
    if path.starts_with("config/")
        || path.starts_with("data/")
        || path.starts_with("deploy/")
        || path.starts_with("docker/")
        || path.starts_with(".github/")
        || path.starts_with(".cargo/")
        || path.starts_with("tools/")
        || path.starts_with("apps/")
    {
        if path.starts_with("tools/") && path.ends_with(".md") {
            return Mode::Docs;
        }
        return Mode::Full;
    }
    if matches!(
        path,
        "Cargo.toml"
            | "Cargo.lock"
            | "rust-toolchain.toml"
            | "rustfmt.toml"
            | "clippy.toml"
            | "Makefile"
            | "compose.yaml"
            | "compose.yml"
    ) || path.starts_with("Dockerfile")
    {
        return Mode::Full;
    }
    Mode::Full
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mode(paths: &[&str]) -> Mode {
        let paths: Vec<String> = paths.iter().map(|path| (*path).to_owned()).collect();
        classify(&paths).mode
    }

    #[test]
    fn the_path_table_is_conservative() {
        assert_eq!(mode(&["docs/example.md"]), Mode::Docs);
        assert_eq!(mode(&["README.md"]), Mode::Docs);
        assert_eq!(mode(&["tools/README.md"]), Mode::Docs);
        assert_eq!(mode(&["docs/port-parity/registry.json"]), Mode::Full);
        assert_eq!(
            mode(&["crates/quantrule-governance/src/gate.rs"]),
            Mode::Governance
        );
        assert_eq!(mode(&["apps/web/src/lib.rs"]), Mode::Web);
        assert_eq!(mode(&["crates/quantrule-model/src/lib.rs"]), Mode::Full);
        assert_eq!(mode(&["config/examples/verify-mini/run.json"]), Mode::Full);
        assert_eq!(mode(&["Cargo.lock"]), Mode::Full);
        assert_eq!(mode(&["rust-toolchain.toml"]), Mode::Full);
        assert_eq!(mode(&[".github/workflows/ci.yml"]), Mode::Full);
        assert_eq!(mode(&["tools/gate.sh"]), Mode::Full);
        assert_eq!(mode(&["unknown/path.bin"]), Mode::Full);
    }

    #[test]
    fn mixed_scopes_are_conservative() {
        assert_eq!(
            mode(&["docs/example.md", "crates/quantrule-governance/src/gate.rs"]),
            Mode::Governance
        );
        assert_eq!(mode(&["docs/example.md", "apps/web/src/lib.rs"]), Mode::Web);
        assert_eq!(
            mode(&[
                "crates/quantrule-governance/src/gate.rs",
                "apps/web/src/lib.rs"
            ]),
            Mode::Full
        );
        assert_eq!(mode(&["docs/example.md", "Cargo.lock"]), Mode::Full);
    }

    #[test]
    fn an_empty_change_is_docs() {
        assert_eq!(mode(&[]), Mode::Docs);
    }
}
