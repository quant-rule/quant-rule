//! The `market-dashboard` command.
//!
//! The module owns the command's argument grammar and its source-to-render
//! path. It loads the M1 series, lets the data engine build the ladder, asks the
//! model for the published dashboard value, and hands that value to the
//! renderer. It does not compute an axis, a reading, an aggregate, or a view-only
//! measurement.

use std::io::Write;
use std::path::PathBuf;

use quantrule_data::Timeframe;
use quantrule_model::dashboard::{DashboardConfig, DashboardState};

use super::model::load_model_run;
use super::types::{DashboardArgs, ParseError, RunError, WindowMode};
use crate::render_dashboard;

const USAGE: &str = "usage: market-dashboard <csv-path> [--window full|latest] [--json]";

/// Parses the already-split arguments of `market-dashboard`.
///
/// The path, the window option, and the JSON flag may appear in any order, but
/// each may appear only once. This is the single parser used by both the
/// interactive line and OS argv, so the two entry points cannot disagree about
/// what the command means.
///
/// # Errors
///
/// Returns a parse error for a missing or repeated path, a repeated or unknown
/// option, an invalid window value, or a duplicate `--json` flag.
pub(crate) fn parse_dashboard_arguments(arguments: &[String]) -> Result<DashboardArgs, ParseError> {
    let mut path = None;
    let mut window = WindowMode::Full;
    let mut window_seen = false;
    let mut json = false;
    let mut json_seen = false;
    let mut index = 0;

    while index < arguments.len() {
        match arguments[index].as_str() {
            "--window" => {
                if window_seen {
                    return Err(ParseError::Window(format!(
                        "--window was supplied twice\n{USAGE}"
                    )));
                }
                index += 1;
                let value = arguments.get(index).ok_or_else(|| {
                    ParseError::Window(format!("--window requires full or latest\n{USAGE}"))
                })?;
                window = WindowMode::from_code(value).ok_or_else(|| {
                    ParseError::Window(format!(
                        "--window expects full or latest, found '{value}'\n{USAGE}"
                    ))
                })?;
                window_seen = true;
            }
            "--json" => {
                if json_seen {
                    return Err(ParseError::Window(format!(
                        "--json was supplied twice\n{USAGE}"
                    )));
                }
                json = true;
                json_seen = true;
            }
            option if option.starts_with("--") => {
                return Err(ParseError::Window(format!(
                    "unknown option '{option}'\n{USAGE}"
                )));
            }
            value => {
                if path.is_some() {
                    return Err(ParseError::Window(format!(
                        "unexpected argument '{value}'\n{USAGE}"
                    )));
                }
                path = Some(PathBuf::from(value));
            }
        }
        index += 1;
    }

    let path = path.ok_or(ParseError::MissingPath("market-dashboard"))?;
    Ok(DashboardArgs { path, window, json })
}

/// Prints the four-axis dashboard for the M1/M5/M15/H1 ladder.
///
/// The configuration is the approved default; the command exposes no threshold
/// flags in this release.
///
/// # Errors
///
/// Returns a [`RunError`] when the source, the model, or the writer fails.
pub(crate) fn run_market_dashboard(
    args: &DashboardArgs,
    writer: &mut impl Write,
) -> Result<(), RunError> {
    let run = load_model_run(&args.path, args.window)?;
    let ladder: Vec<u64> = Timeframe::ALL
        .iter()
        .map(|timeframe| u64::from(timeframe.seconds()))
        .collect();
    let state = DashboardState::from_run(&run, &ladder, &DashboardConfig::default());

    if args.json {
        render_dashboard::write_json(&state, writer).map_err(RunError::Output)?;
    } else {
        render_dashboard::write_text(&state, args.window.code(), writer)
            .map_err(RunError::Output)?;
    }
    writer.flush().map_err(RunError::Output)
}

#[cfg(test)]
mod tests {
    use super::parse_dashboard_arguments;
    use crate::command::types::{DashboardArgs, ParseError, WindowMode};
    use std::path::PathBuf;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn parses_the_default_text_dashboard() {
        assert_eq!(
            parse_dashboard_arguments(&args(&["data/example.csv"])),
            Ok(DashboardArgs {
                path: PathBuf::from("data/example.csv"),
                window: WindowMode::Full,
                json: false,
            })
        );
    }

    #[test]
    fn parses_the_json_flag_and_window_in_any_order() {
        assert_eq!(
            parse_dashboard_arguments(&args(
                &["--json", "data/example.csv", "--window", "latest",]
            )),
            Ok(DashboardArgs {
                path: PathBuf::from("data/example.csv"),
                window: WindowMode::Latest,
                json: true,
            })
        );
    }

    #[test]
    fn rejects_missing_paths_repeated_options_and_unknown_options() {
        assert_eq!(
            parse_dashboard_arguments(&[]),
            Err(ParseError::MissingPath("market-dashboard"))
        );
        assert!(matches!(
            parse_dashboard_arguments(&args(&["data/example.csv", "--json", "--json"])),
            Err(ParseError::Window(_))
        ));
        assert!(matches!(
            parse_dashboard_arguments(&args(&[
                "data/example.csv",
                "--window",
                "full",
                "--window",
                "latest"
            ])),
            Err(ParseError::Window(_))
        ));
        assert!(matches!(
            parse_dashboard_arguments(&args(&["data/example.csv", "--bogus"])),
            Err(ParseError::Window(_))
        ));
        assert!(matches!(
            parse_dashboard_arguments(&args(&["data/example.csv", "--window", "300"])),
            Err(ParseError::Window(_))
        ));
    }

    #[test]
    fn rejects_a_second_path() {
        assert!(matches!(
            parse_dashboard_arguments(&args(&["one.csv", "two.csv"])),
            Err(ParseError::Window(_))
        ));
    }
}
