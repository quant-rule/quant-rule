//! The `lab summary` command.
//!
//! The command parses the lab result folder and its identity, then delegates
//! the bundle reading and summary rendering to [`crate::lab_summary`]. It does
//! not open a source or compute a model value.

use std::io::Write;
use std::path::PathBuf;

use quantrule_data::{format_utc_timestamp, parse_utc_timestamp};

use super::types::{Command, LabOperator, LabRole, LabSummaryArgs, ParseError, RunError};

/// The full usage line shared by every `lab summary` parse error.
const USAGE: &str = "usage: lab summary <lab-dir> \
--role <researcher|developer|validator> \
--data-version <id> \
--config <run.json> \
--created-utc <YYYY-MM-DDTHH:MM:SSZ> \
[--operator <human|agent>] \
[--notes <path>] \
[--force]";

/// Parses the already-split arguments after `lab`.
///
/// # Errors
///
/// Returns [`ParseError::Lab`] for a missing `summary` subcommand, a missing or
/// repeated option, an unknown role or operator, an invalid UTC timestamp, an
/// unknown option, or an extra positional argument.
pub(crate) fn parse_lab_arguments(arguments: &[String]) -> Result<Option<Command>, ParseError> {
    let Some(subcommand) = arguments.first() else {
        return Err(ParseError::Lab(format!("missing lab subcommand\n{USAGE}")));
    };
    if subcommand != "summary" {
        return Err(ParseError::Lab(format!(
            "unknown lab command '{subcommand}'; expected summary\n{USAGE}"
        )));
    }

    let mut lab_dir = None;
    let mut role = None;
    let mut data_version = None;
    let mut config = None;
    let mut created_utc = None;
    let mut operator = None;
    let mut notes = None;
    let mut force = false;
    let mut index = 1;

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
