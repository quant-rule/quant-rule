//! `qr-deploy` — validate, render, and check a QuantRule deployment.
//!
//! The tool reads one `quantrule.deploy/v2` or `quantrule.deploy/v3` JSON
//! descriptor and never edits it. v2 keeps a separate dotenv secrets file and
//! the original render/check behavior. v3 uses `secrets.files`, renders the
//! Swarm stack and pinned per-host unit skeletons, and checks the one HTTPS
//! ingress.

#![forbid(unsafe_code)]

mod check;
mod descriptor;
mod error;
mod render;
mod validate;

use std::path::PathBuf;
use std::process::ExitCode;

use descriptor::Document;
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
    reject_render_only(&flags, "validate")?;
    let document = descriptor::load(&config)?;
    match &document {
        Document::V2(descriptor) => {
            if flags.files {
                return Err(Error::Usage(
                    "validate --files is only valid for quantrule.deploy/v3".to_owned(),
                ));
            }
            let checked = if let Some(path) = &flags.secrets {
                let secrets = descriptor::load_env(path)?;
                validate::validate_with_secrets(descriptor, &secrets)?;
                "secrets checked"
            } else {
                validate::validate(descriptor)?;
                "secrets not checked; pass --secrets"
            };
            println!(
                "ok: {} hosts, {} services, image {}, gate {} ({checked})",
                descriptor.hosts.len(),
                descriptor.services.len(),
                descriptor.image.digest,
                descriptor.network_gate.as_str()
            );
        }
        Document::V3(descriptor) => {
            if flags.secrets.is_some() {
                return Err(Error::Usage(
                    "v3 secrets are named by secrets.files; use --files".to_owned(),
                ));
            }
            let checked = if flags.files {
                validate::validate_document_files(&document)?;
                "files checked"
            } else {
                validate::validate_document(&document)?;
                "files not checked; pass --files"
            };
            println!(
                "ok: v3 {}, {} hosts, {} services, image {}, ingress {}:{} ({checked})",
                descriptor.profile.as_str(),
                descriptor.hosts.len(),
                descriptor.services.len(),
                descriptor.image.digest,
                descriptor.ingress.dns,
                descriptor.ingress.port
            );
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn render_command(mut args: impl Iterator<Item = String>) -> Result<ExitCode, Error> {
    let config = config_arg(&mut args)?;
    let flags = flags(&mut args)?;
    if flags.remote.is_some() || flags.overlay {
        return Err(Error::Usage(
            "render does not accept --remote or --overlay".to_owned(),
        ));
    }
    let out = flags
        .out
        .clone()
        .ok_or_else(|| Error::Usage("render needs --out <dir>".to_owned()))?;
    let document = descriptor::load(&config)?;
    let contract = document.contract();
    match document {
        Document::V2(descriptor) => {
            if flags.target.is_some() {
                return Err(Error::Usage(format!(
                    "render --target is only valid for {}",
                    descriptor::v3::CONTRACT
                )));
            }
            let secrets_path = flags
                .secrets
                .unwrap_or_else(|| descriptor::secrets_path(&config, &descriptor));
            let secrets = descriptor::load_env(&secrets_path)?;
            let files = render::v2::render(&descriptor, &secrets, &out)?;
            println!("wrote {} env files to {}", files.len(), out.display());
            for file in files {
                println!("  {}", file.display());
            }
        }
        Document::V3(descriptor) => {
            if flags.files {
                return Err(Error::Usage(
                    "v3 render does not accept --files; the descriptor names secrets.files"
                        .to_owned(),
                ));
            }
            if flags.secrets.is_some() {
                return Err(Error::Usage(
                    "v3 secrets are named by secrets.files; --secrets is v2 only".to_owned(),
                ));
            }
            let target = flags.target.as_deref().ok_or_else(|| {
                Error::Usage(format!(
                    "{contract} render needs --target swarm|infra|all; v2 render needs no target"
                ))
            })?;
            let target = render::Target::parse(target)?;
            let report = render::render_v3(&descriptor, &out, target)?;
            println!(
                "ok: rendered v3 {} ({}) to {}",
                descriptor.profile.as_str(),
                target.as_str(),
                out.display()
            );
            println!("descriptor\t{}", report.descriptor_hash);
            println!("target\t{}", report.target.as_str());
            println!("image\t{}", report.app_image);
            println!("evidence\t{}", report.evidence);
            for file in report.relative_files(&out) {
                println!("  {file}");
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn check_command(mut args: impl Iterator<Item = String>) -> Result<ExitCode, Error> {
    let config = config_arg(&mut args)?;
    let flags = flags(&mut args)?;
    if flags.out.is_some() {
        return Err(Error::Usage("check does not accept --out".to_owned()));
    }
    if flags.target.is_some() {
        return Err(Error::Usage(
            "check --target is only valid for render".to_owned(),
        ));
    }
    let remote_docker = match flags.remote.as_deref() {
        None => false,
        Some("docker") => true,
        Some(other) => {
            return Err(Error::Usage(format!(
                "unknown --remote {other}; expected docker"
            )));
        }
    };
    let document = descriptor::load(&config)?;
    match document {
        Document::V2(descriptor) => {
            if remote_docker || flags.overlay {
                return Err(Error::Usage(
                    "check --remote and --overlay are only valid for quantrule.deploy/v3"
                        .to_owned(),
                ));
            }
            let secrets_path = flags
                .secrets
                .unwrap_or_else(|| descriptor::secrets_path(&config, &descriptor));
            let secrets = descriptor::load_env(&secrets_path)?;
            let rows = check::v2::check(&descriptor, &secrets)?;
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
        Document::V3(descriptor) => {
            if flags.secrets.is_some() {
                return Err(Error::Usage(
                    "v3 secrets are named by secrets.files; use --files".to_owned(),
                ));
            }
            if flags.files {
                return Err(Error::Usage(
                    "v3 check always checks the local secret files; --files is a validate flag"
                        .to_owned(),
                ));
            }
            let options = check::CheckOptions {
                remote_docker,
                overlay: flags.overlay,
            };
            let report = check::v3::check(&descriptor, options)?;
            print!("{}", report.render());
            Ok(if report.failed() {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            })
        }
    }
}

fn reject_render_only(flags: &Flags, command: &str) -> Result<(), Error> {
    if flags.out.is_some() {
        return Err(Error::Usage(format!("{command} does not accept --out")));
    }
    if flags.target.is_some() {
        return Err(Error::Usage(format!("{command} does not accept --target")));
    }
    if flags.remote.is_some() || flags.overlay {
        return Err(Error::Usage(format!(
            "{command} does not accept --remote or --overlay"
        )));
    }
    Ok(())
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
    target: Option<String>,
    remote: Option<String>,
    files: bool,
    overlay: bool,
}

fn flags(args: &mut impl Iterator<Item = String>) -> Result<Flags, Error> {
    let mut parsed = Flags {
        secrets: None,
        out: None,
        target: None,
        remote: None,
        files: false,
        overlay: false,
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
            "--target" => {
                let value = args
                    .next()
                    .ok_or_else(|| Error::Usage("--target needs a value".to_owned()))?;
                parsed.target = Some(value);
            }
            "--remote" => {
                let value = args
                    .next()
                    .ok_or_else(|| Error::Usage("--remote needs a value".to_owned()))?;
                parsed.remote = Some(value);
            }
            "--files" => parsed.files = true,
            "--overlay" => parsed.overlay = true,
            other => return Err(Error::Usage(format!("unknown argument {other}"))),
        }
    }
    Ok(parsed)
}

fn usage() -> &'static str {
    "qr-deploy — validate, render, and check a QuantRule deployment

Usage:
  qr-deploy validate <deploy.json> [--secrets <secrets.env> | --files]
  qr-deploy render <deploy.json> --out <dir> [--secrets <secrets.env>]        (v2)
  qr-deploy render <deploy.json> --out <dir> --target swarm|infra|all         (v3)
  qr-deploy check <deploy.json> [--secrets <secrets.env>]                     (v2)
  qr-deploy check <deploy.json> [--remote docker] [--overlay]                (v3)

The descriptor is quantrule.deploy/v2 or quantrule.deploy/v3. v2 secrets stay in
a dotenv file and are referenced as ${NAME}. v3 names secrets.files, renders the
Swarm stack and pinned per-host unit skeletons, and checks the one HTTPS ingress.
--remote docker and --overlay are explicit read-only Docker opt-ins; set
QR_DEPLOY_DOCKER to override the docker executable. A lab-single-host profile
reports Simulated HA, never Real HA.

Exit codes: 0 success, 1 validation or check failure, 2 usage error."
}
