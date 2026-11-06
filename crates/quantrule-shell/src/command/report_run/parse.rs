//! Argument parsing for the `report` command.
//!
//! The parser owns the command shape and the error messages. It does not load a
//! configuration or write a bundle.

use std::path::PathBuf;

use crate::command::types::{ParseError, ReportArgs};

pub(crate) fn parse_report_arguments(arguments: &[String]) -> Result<ReportArgs, ParseError> {
    const USAGE: &str =
        "usage: report model --config <run.json> [--spec <spec.json>] [--out <dir>] [--json]";
    let mut engine = None;
    let mut config = None;
    let mut spec = None;
    let mut out = None;
    let mut json = false;
    let mut index = 0;

    while index < arguments.len() {
        match arguments[index].as_str() {
            "--config" => {
                if config.is_some() {
                    return Err(ParseError::Report(format!(
                        "--config was supplied twice\n{USAGE}"
                    )));
                }
                index += 1;
                let value = arguments
                    .get(index)
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| {
                        ParseError::Report(format!("--config requires a non-empty path\n{USAGE}"))
                    })?;
                config = Some(PathBuf::from(value));
            }
            "--spec" => {
                if spec.is_some() {
                    return Err(ParseError::Report(format!(
                        "--spec was supplied twice\n{USAGE}"
                    )));
                }
                index += 1;
                let value = arguments
                    .get(index)
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| {
                        ParseError::Report(format!("--spec requires a non-empty path\n{USAGE}"))
                    })?;
                spec = Some(PathBuf::from(value));
            }
            "--out" => {
                if out.is_some() {
                    return Err(ParseError::Report(format!(
                        "--out was supplied twice\n{USAGE}"
                    )));
                }
                index += 1;
                let value = arguments
                    .get(index)
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| {
                        ParseError::Report(format!("--out requires a non-empty path\n{USAGE}"))
                    })?;
                out = Some(PathBuf::from(value));
            }
            "--json" => {
                if json {
                    return Err(ParseError::Report(format!(
                        "--json was supplied twice\n{USAGE}"
                    )));
                }
                json = true;
            }
            option if option.starts_with("--") => {
                return Err(ParseError::Report(format!(
                    "unknown option '{option}'\n{USAGE}"
                )));
            }
            value => {
                if engine.is_some() {
                    return Err(ParseError::Report(format!(
                        "unexpected argument '{value}'\n{USAGE}"
                    )));
                }
                if value != "model" {
                    return Err(ParseError::Report(format!(
                        "unknown report engine '{value}'; use model\n{USAGE}"
                    )));
                }
                engine = Some(value);
            }
        }
        index += 1;
    }

    let engine =
        engine.ok_or_else(|| ParseError::Report(format!("missing report engine\n{USAGE}")))?;
    let config =
        config.ok_or_else(|| ParseError::Report(format!("--config is required\n{USAGE}")))?;
    let _ = engine;

    Ok(ReportArgs {
        config,
        spec,
        out,
        json,
    })
}

#[cfg(test)]
mod tests {
    use super::parse_report_arguments;

    fn arguments(tokens: &[&str]) -> Vec<String> {
        tokens.iter().map(|token| (*token).to_owned()).collect()
    }

    #[test]
    fn parses_the_model_shape() {
        let parsed = parse_report_arguments(&arguments(&[
            "model",
            "--config",
            "run.json",
            "--out",
            "target/report",
        ]))
        .expect("the shape is valid");
        assert_eq!(parsed.config.to_str(), Some("run.json"));
        assert_eq!(
            parsed.out.as_deref().and_then(|path| path.to_str()),
            Some("target/report")
        );
    }

    #[test]
    fn rejects_other_engines_and_missing_config() {
        assert!(parse_report_arguments(&arguments(&["strategy", "--config", "run.json"])).is_err());
        assert!(parse_report_arguments(&arguments(&["model"])).is_err());
        assert!(parse_report_arguments(&arguments(&["model", "--config"])).is_err());
    }
}
