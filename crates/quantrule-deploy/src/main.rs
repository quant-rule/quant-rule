//! `qr-deploy` — validate, render, and check a multi-host deployment.
//!
//! The tool reads one `quantrule.deploy/v2` JSON descriptor, keeps secrets in a
//! separate dotenv file, and never edits the descriptor. `validate` is safe to
//! run in review; `render` writes service env files; `check` probes a deployed
//! stack and compares its image digest.

#![forbid(unsafe_code)]

mod check;
mod descriptor;
mod error;
mod render;
mod validate;

use std::path::PathBuf;
use std::process::ExitCode;

use error::Error;

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("qr-deploy: {error}");
            if matches!(error, Error::Usage(_)) {
                ExitCode::from(2)
            } else {
                ExitCode::FAILURE
            }
        }
    }
}

fn run() -> Result<ExitCode, Error> {
    let mut args = std::env::args().skip(1);
    let Some(command) = args.next() else {
        return Err(Error::Usage(usage().to_owned()));
    };
    match command.as_str() {
        "-h" | "--help" | "help" => {
            println!("{}", usage());
            Ok(ExitCode::SUCCESS)
        }
        "validate" => validate_command(args),
        "render" => render_command(args),
        "check" => check_command(args),
        other => Err(Error::Usage(format!(
            "unknown command {other}\n\n{}",
            usage()
        ))),
    }
}

fn validate_command(mut args: impl Iterator<Item = String>) -> Result<ExitCode, Error> {
    let config = config_arg(&mut args)?;
    let flags = flags(&mut args)?;
    if flags.out.is_some() {
        return Err(Error::Usage("validate does not accept --out".to_owned()));
    }
    let descriptor = descriptor::load(&config)?;
    let checked = if let Some(path) = flags.secrets {
        let secrets = descriptor::load_env(&path)?;
        validate::validate_with_secrets(&descriptor, &secrets)?;
        "secrets checked"
    } else {
        validate::validate(&descriptor)?;
        "secrets not checked; pass --secrets"
    };
    println!(
        "ok: {} hosts, {} services, image {}, gate {} ({checked})",
        descriptor.hosts.len(),
        descriptor.services.len(),
        descriptor.image.digest,
        descriptor.network_gate.as_str()
    );
    Ok(ExitCode::SUCCESS)
}

fn render_command(mut args: impl Iterator<Item = String>) -> Result<ExitCode, Error> {
    let config = config_arg(&mut args)?;
    let flags = flags(&mut args)?;
    let out = flags
        .out
        .ok_or_else(|| Error::Usage("render needs --out <dir>".to_owned()))?;
    let descriptor = descriptor::load(&config)?;
    let secrets_path = flags
        .secrets
        .unwrap_or_else(|| descriptor::secrets_path(&config, &descriptor));
    let secrets = descriptor::load_env(&secrets_path)?;
    let files = render::render(&descriptor, &secrets, &out)?;
    println!("wrote {} env files to {}", files.len(), out.display());
    for file in files {
        println!("  {}", file.display());
    }
    Ok(ExitCode::SUCCESS)
}

fn check_command(mut args: impl Iterator<Item = String>) -> Result<ExitCode, Error> {
    let config = config_arg(&mut args)?;
    let flags = flags(&mut args)?;
    if flags.out.is_some() {
        return Err(Error::Usage("check does not accept --out".to_owned()));
    }
    let descriptor = descriptor::load(&config)?;
    let secrets_path = flags
        .secrets
        .unwrap_or_else(|| descriptor::secrets_path(&config, &descriptor));
    let secrets = descriptor::load_env(&secrets_path)?;
    let rows = check::check(&descriptor, &secrets)?;
    let mut failed = false;
    for row in rows {
        let state = if row.skipped() {
            "SKIP"
        } else if row.ok() {
            "ok"
        } else {
            "FAIL"
        };
        if !row.ok() && !row.skipped() {
            failed = true;
        }
        match (&row.digest, &row.error) {
            (Some(digest), None) => {
                println!(
                    "{state}\t{}\t{}\t{}\t{digest}",
                    row.service, row.url, row.state
                )
            }
            (_, Some(error)) => {
                println!(
                    "{state}\t{}\t{}\t{}\t{error}",
                    row.service, row.url, row.state
                )
            }
            (None, None) => {
                println!(
                    "{state}\t{}\t{}\t{}\tcheck the container on this host",
                    row.service, row.url, row.state
                )
            }
        }
    }
    Ok(if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}

fn config_arg(args: &mut impl Iterator<Item = String>) -> Result<PathBuf, Error> {
    let value = args
        .next()
        .ok_or_else(|| Error::Usage("a descriptor path is required".to_owned()))?;
    if value.starts_with('-') {
        return Err(Error::Usage(format!(
            "expected a descriptor path, got {value}"
        )));
    }
    Ok(PathBuf::from(value))
}

struct Flags {
    secrets: Option<PathBuf>,
    out: Option<PathBuf>,
}

fn flags(args: &mut impl Iterator<Item = String>) -> Result<Flags, Error> {
    let mut parsed = Flags {
        secrets: None,
        out: None,
    };
    while let Some(value) = args.next() {
        match value.as_str() {
            "--secrets" => {
                let path = args
                    .next()
                    .ok_or_else(|| Error::Usage("--secrets needs a value".to_owned()))?;
                parsed.secrets = Some(PathBuf::from(path));
            }
            "--out" => {
                let path = args
                    .next()
                    .ok_or_else(|| Error::Usage("--out needs a value".to_owned()))?;
                parsed.out = Some(PathBuf::from(path));
            }
            other => return Err(Error::Usage(format!("unknown argument {other}"))),
        }
    }
    Ok(parsed)
}

fn usage() -> &'static str {
    "qr-deploy — validate, render, and check a multi-host deployment

Usage:
  qr-deploy validate <deploy.json> [--secrets <secrets.env>]
  qr-deploy render <deploy.json> --out <dir> [--secrets <secrets.env>]
  qr-deploy check <deploy.json> [--secrets <secrets.env>]

The descriptor is quantrule.deploy/v2. Secrets stay in a dotenv file; the
descriptor references them as ${NAME} in the database and queue URLs.

Exit codes: 0 success, 1 validation or check failure, 2 usage error."
}
