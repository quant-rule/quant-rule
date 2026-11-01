//! The `lab` commands.
//!
//! `lab summary` parses the lab result folder and its identity, then delegates
//! the bundle reading and summary rendering to [`crate::lab_summary`].
//! `lab diagnose` re-runs the config and delegates the diagnosis to
//! [`crate::diagnosis`]. `lab list` summarises the lab folders under a root.
//! The summary and list commands only read published results; diagnose runs the
//! pipeline, and none of them computes a model value in this module.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use quantrule_data::{format_utc_timestamp, parse_utc_timestamp};
use serde_json::Value;

use super::types::{
    Command, LabDiagnoseArgs, LabListArgs, LabOperator, LabRole, LabSummaryArgs, ParseError,
    RunError,
};

/// The full usage line shared by every `lab summary` parse error.
const USAGE: &str = "usage: lab summary <lab-dir> \
--role <researcher|developer|validator> \
--data-version <id> \
--config <run.json> \
--created-utc <YYYY-MM-DDTHH:MM:SSZ> \
[--operator <human|agent>] \
[--notes <path>] \
[--force]";

/// The usage line shared by every `lab diagnose` parse error.
const DIAGNOSE_USAGE: &str = "usage: lab diagnose <lab-dir> --config <run.json> \
[--out <dir>] [--json] [--strict]";

/// The usage line shared by every `lab list` parse error.
const LIST_USAGE: &str = "usage: lab list --root <dir> [--json]";

/// Parses the already-split arguments after `lab`.
///
/// # Errors
///
/// Returns [`ParseError::Lab`] for an unknown subcommand or an invalid
/// argument list.
pub(crate) fn parse_lab_arguments(arguments: &[String]) -> Result<Option<Command>, ParseError> {
    let Some(subcommand) = arguments.first() else {
        return Err(ParseError::Lab(format!("missing lab subcommand\n{USAGE}")));
    };
    match subcommand.as_str() {
        "summary" => parse_lab_summary(&arguments[1..]),
        "diagnose" => parse_lab_diagnose(&arguments[1..]),
        "list" => parse_lab_list(&arguments[1..]),
        other => Err(ParseError::Lab(format!(
            "unknown lab command '{other}'; expected summary, diagnose, or list\n{USAGE}"
        ))),
    }
}

fn parse_lab_summary(arguments: &[String]) -> Result<Option<Command>, ParseError> {
    let mut lab_dir = None;
    let mut role = None;
    let mut data_version = None;
    let mut config = None;
    let mut created_utc = None;
    let mut operator = None;
    let mut notes = None;
    let mut force = false;
    let mut index = 0;

    while index < arguments.len() {
        match arguments[index].as_str() {
            "--role" => {
                if role.is_some() {
                    return Err(ParseError::Lab(format!(
                        "--role was supplied twice\n{USAGE}"
                    )));
                }
                let value = option_value(arguments, &mut index, "--role")?;
                role = Some(LabRole::parse(value).ok_or_else(|| {
                    ParseError::Lab(format!(
                        "--role expects researcher, developer, or validator, found '{value}'\n{USAGE}"
                    ))
                })?);
            }
            "--data-version" => {
                if data_version.is_some() {
                    return Err(ParseError::Lab(format!(
                        "--data-version was supplied twice\n{USAGE}"
                    )));
                }
                let value = option_value(arguments, &mut index, "--data-version")?;
                data_version = Some(value.to_owned());
            }
            "--config" => {
                if config.is_some() {
                    return Err(ParseError::Lab(format!(
                        "--config was supplied twice\n{USAGE}"
                    )));
                }
                let value = option_value(arguments, &mut index, "--config")?;
                config = Some(PathBuf::from(value));
            }
            "--created-utc" => {
                if created_utc.is_some() {
                    return Err(ParseError::Lab(format!(
                        "--created-utc was supplied twice\n{USAGE}"
                    )));
                }
                let value = option_value(arguments, &mut index, "--created-utc")?;
                validate_created_utc(value)?;
                created_utc = Some(value.to_owned());
            }
            "--operator" => {
                if operator.is_some() {
                    return Err(ParseError::Lab(format!(
                        "--operator was supplied twice\n{USAGE}"
                    )));
                }
                let value = option_value(arguments, &mut index, "--operator")?;
                operator = Some(LabOperator::parse(value).ok_or_else(|| {
                    ParseError::Lab(format!(
                        "--operator expects human or agent, found '{value}'\n{USAGE}"
                    ))
                })?);
            }
            "--notes" => {
                if notes.is_some() {
                    return Err(ParseError::Lab(format!(
                        "--notes was supplied twice\n{USAGE}"
                    )));
                }
                let value = option_value(arguments, &mut index, "--notes")?;
                notes = Some(PathBuf::from(value));
            }
            "--force" => {
                if force {
                    return Err(ParseError::Lab(format!(
                        "--force was supplied twice\n{USAGE}"
                    )));
                }
                force = true;
            }
            option if option.starts_with("--") => {
                return Err(ParseError::Lab(format!(
                    "unknown option '{option}'\n{USAGE}"
                )));
            }
            value => {
                if lab_dir.is_some() {
                    return Err(ParseError::Lab(format!(
                        "unexpected argument '{value}'\n{USAGE}"
                    )));
                }
                lab_dir = Some(PathBuf::from(value));
            }
        }
        index += 1;
    }

    let lab_dir =
        lab_dir.ok_or_else(|| ParseError::Lab(format!("missing lab result folder\n{USAGE}")))?;
    let role = role.ok_or_else(|| ParseError::Lab(format!("--role is required\n{USAGE}")))?;
    let data_version = data_version
        .ok_or_else(|| ParseError::Lab(format!("--data-version is required\n{USAGE}")))?;
    let config = config.ok_or_else(|| ParseError::Lab(format!("--config is required\n{USAGE}")))?;
    let created_utc = created_utc
        .ok_or_else(|| ParseError::Lab(format!("--created-utc is required\n{USAGE}")))?;

    Ok(Some(Command::LabSummary(LabSummaryArgs {
        lab_dir,
        role,
        data_version,
        config,
        created_utc,
        operator: operator.unwrap_or(LabOperator::Human),
        notes,
        force,
    })))
}

fn parse_lab_diagnose(arguments: &[String]) -> Result<Option<Command>, ParseError> {
    let mut lab_dir = None;
    let mut config = None;
    let mut out = None;
    let mut json = false;
    let mut strict = false;
    let mut index = 0;

    while index < arguments.len() {
        match arguments[index].as_str() {
            "--config" => {
                if config.is_some() {
                    return Err(ParseError::Lab(format!(
                        "--config was supplied twice\n{DIAGNOSE_USAGE}"
                    )));
                }
                let value = option_value(arguments, &mut index, "--config")?;
                config = Some(PathBuf::from(value));
            }
            "--out" => {
                if out.is_some() {
                    return Err(ParseError::Lab(format!(
                        "--out was supplied twice\n{DIAGNOSE_USAGE}"
                    )));
                }
                let value = option_value(arguments, &mut index, "--out")?;
                out = Some(PathBuf::from(value));
            }
            "--json" => {
                if json {
                    return Err(ParseError::Lab(format!(
                        "--json was supplied twice\n{DIAGNOSE_USAGE}"
                    )));
                }
                json = true;
            }
            "--strict" => {
                if strict {
                    return Err(ParseError::Lab(format!(
                        "--strict was supplied twice\n{DIAGNOSE_USAGE}"
                    )));
                }
                strict = true;
            }
            option if option.starts_with("--") => {
                return Err(ParseError::Lab(format!(
                    "unknown option '{option}'\n{DIAGNOSE_USAGE}"
                )));
            }
            value => {
                if lab_dir.is_some() {
                    return Err(ParseError::Lab(format!(
                        "unexpected argument '{value}'\n{DIAGNOSE_USAGE}"
                    )));
                }
                lab_dir = Some(PathBuf::from(value));
            }
        }
        index += 1;
    }

    let lab_dir = lab_dir
        .ok_or_else(|| ParseError::Lab(format!("missing lab result folder\n{DIAGNOSE_USAGE}")))?;
    let config =
        config.ok_or_else(|| ParseError::Lab(format!("--config is required\n{DIAGNOSE_USAGE}")))?;

    Ok(Some(Command::LabDiagnose(LabDiagnoseArgs {
        lab_dir,
        config,
        out,
        json,
        strict,
    })))
}

fn parse_lab_list(arguments: &[String]) -> Result<Option<Command>, ParseError> {
    let mut root = None;
    let mut json = false;
    let mut index = 0;

    while index < arguments.len() {
        match arguments[index].as_str() {
            "--root" => {
                if root.is_some() {
                    return Err(ParseError::Lab(format!(
                        "--root was supplied twice\n{LIST_USAGE}"
                    )));
                }
                let value = option_value(arguments, &mut index, "--root")?;
                root = Some(PathBuf::from(value));
            }
            "--json" => {
                if json {
                    return Err(ParseError::Lab(format!(
                        "--json was supplied twice\n{LIST_USAGE}"
                    )));
                }
                json = true;
            }
            option if option.starts_with("--") => {
                return Err(ParseError::Lab(format!(
                    "unknown option '{option}'\n{LIST_USAGE}"
                )));
            }
            value => {
                return Err(ParseError::Lab(format!(
                    "unexpected argument '{value}'\n{LIST_USAGE}"
                )));
            }
        }
        index += 1;
    }

    let root = root.ok_or_else(|| ParseError::Lab(format!("--root is required\n{LIST_USAGE}")))?;
    Ok(Some(Command::LabList(LabListArgs { root, json })))
}

fn option_value<'a>(
    arguments: &'a [String],
    index: &mut usize,
    option: &str,
) -> Result<&'a str, ParseError> {
    *index += 1;
    arguments
        .get(*index)
        .map(String::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ParseError::Lab(format!("{option} requires a non-empty value\n{USAGE}")))
}

fn validate_created_utc(text: &str) -> Result<(), ParseError> {
    let bytes = text.as_bytes();
    if text.len() != 20 || bytes[10] != b'T' || bytes[19] != b'Z' {
        return Err(ParseError::Lab(format!(
            "--created-utc expects YYYY-MM-DDTHH:MM:SSZ, found '{text}'\n{USAGE}"
        )));
    }
    let normalized = format!("{} {}+00:00", &text[..10], &text[11..19]);
    let seconds = parse_utc_timestamp(&normalized).map_err(|source| {
        ParseError::Lab(format!("invalid --created-utc '{text}': {source}\n{USAGE}"))
    })?;
    if format_utc_timestamp(seconds) != text {
        return Err(ParseError::Lab(format!(
            "--created-utc expects YYYY-MM-DDTHH:MM:SSZ, found '{text}'\n{USAGE}"
        )));
    }
    Ok(())
}

/// Writes the lab summary and prints its path.
///
/// # Errors
///
/// Returns [`RunError::Lab`] when the summary cannot be read or written, and
/// [`RunError::Output`] when stdout fails.
pub(crate) fn run_lab_summary(
    args: &LabSummaryArgs,
    writer: &mut impl Write,
) -> Result<(), RunError> {
    let path = crate::lab_summary::write(args).map_err(RunError::Lab)?;
    writeln!(writer, "wrote {}", path.display()).map_err(RunError::Output)?;
    writer.flush().map_err(RunError::Output)
}

struct LabIdentity {
    config_hash: String,
    data_version: String,
    model_version: String,
    final_equity: f64,
    net_pnl: f64,
    trades: u64,
    max_drawdown: f64,
    halted: bool,
}

fn read_lab_identity(lab_dir: &Path) -> Result<LabIdentity, RunError> {
    if !lab_dir.is_dir() {
        return Err(RunError::Diagnosis(format!(
            "{} is not a lab result directory",
            lab_dir.display()
        )));
    }

    let mut config_hash: Option<String> = None;
    let mut data_version: Option<String> = None;
    let mut model_version: Option<String> = None;
    for bundle in ["model", "strategy", "execution", "portfolio", "analytical"] {
        let report_path = lab_dir.join(bundle).join("report.json");
        let provenance_path = lab_dir.join(bundle).join("provenance.json");
        let report = read_json(&report_path)?;
        let provenance = read_json(&provenance_path)?;
        let bundle_data_version = string_at(&report, "data_version")?;
        let bundle_model_version = string_at(&report, "model_version")?;
        let provenance_data_version =
            provenance_string_at(&provenance, &provenance_path, "data_version")?;
        if provenance_data_version != bundle_data_version {
            return Err(RunError::Diagnosis(format!(
                "{} data_version is {}, expected {} from {}",
                provenance_path.display(),
                provenance_data_version,
                bundle_data_version,
                report_path.display()
            )));
        }
        let provenance_model_version =
            provenance_string_at(&provenance, &provenance_path, "model_version")?;
        if provenance_model_version != bundle_model_version {
            return Err(RunError::Diagnosis(format!(
                "{} model_version is {}, expected {} from {}",
                provenance_path.display(),
                provenance_model_version,
                bundle_model_version,
                report_path.display()
            )));
        }
        let expected_version = if bundle == "model" {
            crate::render_report::VIEW_CONTRACT
        } else {
            crate::render_report::REPORT_VERSION
        };
        let actual_version = report
            .get("report_version")
            .or_else(|| report.get("view_contract"))
            .and_then(Value::as_str)
            .ok_or_else(|| {
                RunError::Diagnosis(format!("{} has no report version", report_path.display()))
            })?;
        if actual_version != expected_version {
            return Err(RunError::Diagnosis(format!(
                "stale lab: {} report version {actual_version} does not match {expected_version}",
                report_path.display()
            )));
        }
        let bundle_config_hash =
            provenance_string_at(&provenance, &provenance_path, "config_hash")?;
        set_once(
            &mut data_version,
            bundle_data_version,
            "data_version",
            &report_path,
        )?;
        set_once(
            &mut model_version,
            bundle_model_version,
            "model_version",
            &report_path,
        )?;
        set_once(
            &mut config_hash,
            bundle_config_hash,
            "config_hash",
            &provenance_path,
        )?;
    }

    let portfolio_path = lab_dir.join("portfolio").join("report.json");
    let portfolio = read_json(&portfolio_path)?;
    let final_equity = f64_at(&portfolio, "summary.final_equity")?;
    let net_pnl = f64_at(&portfolio, "summary.net_pnl")?;
    let trades = u64_at(&portfolio, "summary.trades")?;
    let max_drawdown = f64_at(&portfolio, "summary.max_drawdown")?;
    let halted = portfolio
        .pointer("/summary/halted")
        .and_then(Value::as_bool)
        .ok_or_else(|| {
            RunError::Diagnosis(format!(
                "{} summary.halted is not a boolean",
                portfolio_path.display()
            ))
        })?;

    Ok(LabIdentity {
        config_hash: config_hash
            .ok_or_else(|| RunError::Diagnosis(String::from("the lab has no config_hash")))?,
        data_version: data_version
            .ok_or_else(|| RunError::Diagnosis(String::from("the lab has no data_version")))?,
        model_version: model_version
            .ok_or_else(|| RunError::Diagnosis(String::from("the lab has no model_version")))?,
        final_equity,
        net_pnl,
        trades,
        max_drawdown,
        halted,
    })
}

fn set_once(
    slot: &mut Option<String>,
    value: &str,
    field: &str,
    path: &Path,
) -> Result<(), RunError> {
    match slot {
        Some(expected) if expected != value => Err(RunError::Diagnosis(format!(
            "{} {field} is {value}, expected {expected}",
            path.display()
        ))),
        Some(_) => Ok(()),
        None => {
            *slot = Some(value.to_owned());
            Ok(())
        }
    }
}

fn read_json(path: &Path) -> Result<Value, RunError> {
    let bytes = fs::read(path).map_err(|source| {
        RunError::Diagnosis(format!("cannot read {}: {source}", path.display()))
    })?;
    serde_json::from_slice(&bytes)
        .map_err(|source| RunError::Diagnosis(format!("cannot parse {}: {source}", path.display())))
}

fn lab_config_hash(path: &Path) -> Result<String, RunError> {
    let Ok(provenance) = read_json(path) else {
        return Ok(String::new());
    };
    let Some(value) = provenance.get("config_hash") else {
        return Ok(String::new());
    };
    let invalid = || {
        RunError::Diagnosis(format!(
            "{} has an invalid config_hash: expected 16 hex characters",
            path.display()
        ))
    };
    let Some(hash) = value.as_str() else {
        return Err(invalid());
    };
    if hash.len() != 16 || !hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(invalid());
    }
    Ok(hash.to_owned())
}

fn halted_label(halted: Option<bool>) -> &'static str {
    match halted {
        Some(true) => "yes",
        Some(false) => "no",
        None => "—",
    }
}

fn provenance_string_at<'a>(
    value: &'a Value,
    path: &Path,
    field: &str,
) -> Result<&'a str, RunError> {
    value.get(field).and_then(Value::as_str).ok_or_else(|| {
        RunError::Diagnosis(format!(
            "{} {field} is missing or not a string",
            path.display()
        ))
    })
}

fn string_at<'a>(value: &'a Value, field: &str) -> Result<&'a str, RunError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| RunError::Diagnosis(format!("{field} is missing or not a string")))
}

fn f64_at(value: &Value, field: &str) -> Result<f64, RunError> {
    let pointer = format!("/{}", field.replace('.', "/"));
    value
        .pointer(&pointer)
        .and_then(Value::as_f64)
        .ok_or_else(|| RunError::Diagnosis(format!("{field} is missing or not a number")))
}

fn u64_at(value: &Value, field: &str) -> Result<u64, RunError> {
    let pointer = format!("/{}", field.replace('.', "/"));
    value
        .pointer(&pointer)
        .and_then(Value::as_u64)
        .ok_or_else(|| RunError::Diagnosis(format!("{field} is missing or not an integer")))
}

fn close(left: f64, right: f64) -> bool {
    (left - right).abs() <= 1e-6
}

/// Re-runs a lab's config, verifies its identity, and writes the diagnosis.
///
/// # Errors
///
/// Returns [`RunError::Diagnosis`] when the lab is missing or stale, when the
/// rerun disagrees with the lab, when a strict integrity check fails, or when
/// the diagnosis cannot be written. Returns [`RunError::Output`] when stdout
/// fails.
pub(crate) fn run_lab_diagnose(
    args: &LabDiagnoseArgs,
    writer: &mut impl Write,
) -> Result<(), RunError> {
    let identity = read_lab_identity(&args.lab_dir)?;
    let config = crate::config::RunConfig::load(&args.config).map_err(RunError::Config)?;

    if identity.config_hash != config.config_hash {
        return Err(RunError::Diagnosis(format!(
            "stale lab: config_hash {} does not match {} from {}",
            identity.config_hash,
            config.config_hash,
            args.config.display()
        )));
    }
    if identity.data_version != config.data.data_version {
        return Err(RunError::Diagnosis(format!(
            "stale lab: data_version {} does not match {}",
            identity.data_version, config.data.data_version
        )));
    }
    let model_version = env!("CARGO_PKG_VERSION");
    if identity.model_version != model_version {
        return Err(RunError::Diagnosis(format!(
            "stale lab: model_version {} does not match {model_version}",
            identity.model_version
        )));
    }

    let run = super::report_run::run_pipeline(&config)?;
    let metrics = run.metrics();
    if !close(identity.final_equity, metrics.final_equity)
        || !close(identity.net_pnl, metrics.net_pnl)
        || !close(identity.max_drawdown, metrics.max_drawdown)
        || identity.trades != metrics.trades as u64
        || identity.halted != metrics.halted
    {
        return Err(RunError::Diagnosis(format!(
            "rerun does not reproduce the lab: equity {} vs {}, net {} vs {}, trades {} vs {}, drawdown {} vs {}, halted {} vs {}",
            identity.final_equity,
            metrics.final_equity,
            identity.net_pnl,
            metrics.net_pnl,
            identity.trades,
            metrics.trades,
            identity.max_drawdown,
            metrics.max_drawdown,
            identity.halted,
            metrics.halted
        )));
    }

    let diagnosis_dir = args
        .out
        .clone()
        .unwrap_or_else(|| args.lab_dir.join("diagnosis"));
    let output = crate::diagnosis::write(&config, &run, &diagnosis_dir, Some(&args.lab_dir))?;

    if args.strict {
        let integrity = output
            .diagnosis
            .checks
            .iter()
            .filter(|check| check.id.starts_with("identity.") || check.id.starts_with("data."))
            .filter(|check| {
                check.status == quantrule_analytics::diagnosis::types::CheckStatus::Fail
            })
            .map(|check| check.id.as_str())
            .collect::<Vec<_>>();
        if !integrity.is_empty() {
            return Err(RunError::Diagnosis(format!(
                "strict integrity checks failed: {}",
                integrity.join(", ")
            )));
        }
    }

    if args.json {
        writer.write_all(&output.json).map_err(RunError::Output)?;
    } else {
        writeln!(writer, "wrote {}", output.path.display()).map_err(RunError::Output)?;
    }
    writer.flush().map_err(RunError::Output)
}

/// Lists the lab result folders under a root.
///
/// # Errors
///
/// Returns [`RunError::Diagnosis`] when the root is missing or a report cannot
/// be read, and [`RunError::Output`] when stdout fails.
pub(crate) fn run_lab_list(args: &LabListArgs, writer: &mut impl Write) -> Result<(), RunError> {
    let root = &args.root;
    if !root.is_dir() {
        return Err(RunError::Diagnosis(format!(
            "{} is not a directory",
            root.display()
        )));
    }
    let mut labs = Vec::new();
    let entries = fs::read_dir(root).map_err(|source| {
        RunError::Diagnosis(format!("cannot read {}: {source}", root.display()))
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| {
            RunError::Diagnosis(format!("cannot read {}: {source}", root.display()))
        })?;
        let path = entry.path();
        if !path.is_dir() || !path.join("portfolio").join("report.json").is_file() {
            continue;
        }
        let lab = entry.file_name().to_string_lossy().into_owned();
        let portfolio = read_json(&path.join("portfolio").join("report.json"))?;
        let data_version = string_at(&portfolio, "data_version")?.to_owned();
        let model_version = string_at(&portfolio, "model_version")?.to_owned();
        let config_hash = lab_config_hash(&path.join("portfolio").join("provenance.json"))?;
        let final_equity = f64_at(&portfolio, "summary.final_equity")?;
        let total_return = f64_at(&portfolio, "summary.total_return")?;
        let net_pnl = f64_at(&portfolio, "summary.net_pnl")?;
        let trades = u64_at(&portfolio, "summary.trades")?;
        let halted = portfolio
            .pointer("/summary/halted")
            .and_then(Value::as_bool);
        let diagnosis = path.join("diagnosis").join("diagnosis.json").is_file();
        labs.push((
            lab,
            data_version,
            model_version,
            config_hash,
            final_equity,
            total_return,
            net_pnl,
            trades,
            halted,
            diagnosis,
        ));
    }
    labs.sort_by(|left, right| left.0.cmp(&right.0));

    if args.json {
        let values: Vec<Value> = labs
            .iter()
            .map(
                |(
                    lab,
                    data_version,
                    model_version,
                    config_hash,
                    final_equity,
                    total_return,
                    net_pnl,
                    trades,
                    halted,
                    diagnosis,
                )| {
                    serde_json::json!({
                        "lab": lab,
                        "data_version": data_version,
                        "model_version": model_version,
                        "config_hash": config_hash,
                        "final_equity": final_equity,
                        "total_return": total_return,
                        "net_pnl": net_pnl,
                        "trades": trades,
                        "halted": halted,
                        "diagnosis": diagnosis,
                    })
                },
            )
            .collect();
        let text = serde_json::to_string_pretty(&values)
            .map_err(|error| RunError::Diagnosis(format!("cannot serialize lab list: {error}")))?;
        writeln!(writer, "{text}").map_err(RunError::Output)?;
    } else {
        writeln!(
            writer,
            "{:<28} {:<54} {:<12} {:>12} {:>8} {:>10} {:>6} {:<6} diagnosis",
            "lab",
            "data_version",
            "config_hash",
            "final_equity",
            "return",
            "net_pnl",
            "trades",
            "halted"
        )
        .map_err(RunError::Output)?;
        for (
            lab,
            data_version,
            _model_version,
            config_hash,
            final_equity,
            total_return,
            net_pnl,
            trades,
            halted,
            diagnosis,
        ) in labs
        {
            writeln!(
                writer,
                "{:<28} {:<54} {:<12} {:>12.2} {:>8.4} {:>10.2} {:>6} {:<6} {}",
                lab,
                data_version,
                config_hash.get(..12).unwrap_or(&config_hash),
                final_equity,
                total_return,
                net_pnl,
                trades,
                halted_label(halted),
                if diagnosis { "yes" } else { "no" }
            )
            .map_err(RunError::Output)?;
        }
    }
    writer.flush().map_err(RunError::Output)
}
