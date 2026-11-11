//! The `run check` command.
//!
//! The command loads the run configuration. The loader validates the data,
//! model, execution, and portfolio contracts and their cross-config rules, so
//! this command is the human-visible proof that the run composition agrees.

use std::io::Write;
use std::path::Path;

use crate::config::RunConfig;
use crate::labels;

use super::types::RunError;
use crate::request::RunCheckArgs;

/// Parses the already-split arguments after `run`.
///
/// # Errors
///
/// Returns a usage error unless the shape is `check --config <run.json>`.
pub(crate) fn parse_run_check_arguments(
    arguments: &[String],
) -> Result<super::types::Command, super::types::ParseError> {
    const USAGE: &str = "usage: run check --config <run.json>";
    if arguments.first().map(String::as_str) != Some("check") {
        return Err(super::types::ParseError::RunCheck(USAGE.to_owned()));
    }

    let mut config = None;
    let mut index = 1;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--config" => {
                if config.is_some() {
                    return Err(super::types::ParseError::RunCheck(format!(
                        "--config was supplied twice\n{USAGE}"
                    )));
                }
                index += 1;
                let value = arguments
                    .get(index)
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| {
                        super::types::ParseError::RunCheck(format!(
                            "--config requires a non-empty path\n{USAGE}"
                        ))
                    })?;
                config = Some(std::path::PathBuf::from(value));
            }
            option if option.starts_with("--") => {
                return Err(super::types::ParseError::RunCheck(format!(
                    "unknown option '{option}'\n{USAGE}"
                )));
            }
            value => {
                return Err(super::types::ParseError::RunCheck(format!(
                    "unexpected argument '{value}'\n{USAGE}"
                )));
            }
        }
        index += 1;
    }

    let config = config.ok_or_else(|| {
        super::types::ParseError::RunCheck(format!("--config is required\n{USAGE}"))
    })?;
    Ok(super::types::Command::RunCheck(RunCheckArgs { config }))
}

/// Loads and validates the run and prints a deterministic summary.
///
/// # Errors
///
/// Returns [`RunError::Config`] when any config file is invalid, and
/// [`RunError::Output`] when the writer fails.
pub(crate) fn run_run_check(args: &RunCheckArgs, writer: &mut impl Write) -> Result<(), RunError> {
    let config = RunConfig::load(Path::new(&args.config)).map_err(RunError::Config)?;
    let ladder = config
        .data
        .available_timeframes
        .iter()
        .map(|timeframe| timeframe.code())
        .collect::<Vec<_>>()
        .join(",");
    let model_ladder = config
        .model_ladder
        .as_ref()
        .unwrap_or(&config.data.available_timeframes)
        .iter()
        .map(|timeframe| timeframe.code())
        .collect::<Vec<_>>()
        .join(",");
    writeln!(
        writer,
        "run ok run={} instrument={} data_version={} data_timeframes={} model={} model_ladder={} execution={} portfolio={} cash={} leverage={} positions={}",
        config.name,
        config.data.instrument,
        config.data.data_version,
        ladder,
        config.model_id.as_deref().unwrap_or("-"),
        model_ladder,
        labels::timeframe_label(u64::from(config.execution.fill_timeframe.seconds())),
        config.portfolio_id.as_deref().unwrap_or("-"),
        config.portfolio.initial_cash,
        config.portfolio.leverage,
        config.portfolio.max_open_positions,
    )
    .map_err(RunError::Output)?;
    writer.flush().map_err(RunError::Output)
}

#[cfg(test)]
mod tests {
    use super::parse_run_check_arguments;

    fn arguments(tokens: &[&str]) -> Vec<String> {
        tokens.iter().map(|token| (*token).to_owned()).collect()
    }

    #[test]
    fn parses_the_check_shape() {
        let parsed = parse_run_check_arguments(&arguments(&["check", "--config", "run.json"]))
            .expect("the command parses");
        assert!(matches!(parsed, super::super::types::Command::RunCheck(_)));
    }

    #[test]
    fn rejects_other_shapes() {
        assert!(parse_run_check_arguments(&arguments(&["check"])).is_err());
        assert!(parse_run_check_arguments(&arguments(&["other"])).is_err());
    }
}
