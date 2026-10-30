//! `market-report` parsing and execution.
//!
//! The command owns the source access and the artifact set. It loads the
//! extended M1 range, hands the candles to the data engine and the model, calls
//! the pure [`MarketReport`] view, renders every byte, and only then populates
//! the output directory. It computes no statistic: coverage, histograms, event
//! counts, and chart buckets come from the model crate.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use quantrule_data::Timeframe;
use quantrule_model::input::MarketStateConfig;
use quantrule_model::report::MarketReport;

use super::types::{MarketReportArgs, ParseError, ReportError, RunError};
use crate::chart;
use crate::export;
use crate::pipeline;
use crate::render_report;
use crate::source::DateBounds;

/// The artifact names, in the contract's fixed order.
const ARTIFACTS: [&str; 4] = ["summary.txt", "report.json", "price-axes.png", "events.png"];

/// Parses the already-split arguments of `market-report`.
///
/// # Errors
///
/// Returns [`ParseError::MarketReport`] for a missing path, a missing or empty
/// required value, a repeated option, an invalid UTC date, a reversed range, or
/// an unknown option.
pub(crate) fn parse_market_report_arguments(
    arguments: &[String],
) -> Result<MarketReportArgs, ParseError> {
    const USAGE: &str = "usage: market-report <csv-path> --data-version <id> [--from <date>] [--to <date>] --out <dir> [--json]";
    let mut path = None;
    let mut data_version = None;
    let mut from = None;
    let mut to = None;
    let mut out = None;
    let mut json = false;
    let mut index = 0;

    while index < arguments.len() {
        match arguments[index].as_str() {
            "--data-version" => {
                if data_version.is_some() {
                    return Err(ParseError::MarketReport(format!(
                        "--data-version was supplied twice\n{USAGE}"
                    )));
                }
                index += 1;
                let value = arguments
                    .get(index)
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| {
                        ParseError::MarketReport(format!(
                            "--data-version requires a non-empty value\n{USAGE}"
                        ))
                    })?;
                data_version = Some(value.clone());
            }
            "--from" => {
                if from.is_some() {
                    return Err(ParseError::MarketReport(format!(
                        "--from was supplied twice\n{USAGE}"
                    )));
                }
                index += 1;
                let value = arguments.get(index).ok_or_else(|| {
                    ParseError::MarketReport(format!("--from requires a date\n{USAGE}"))
                })?;
                from = Some(value.clone());
            }
            "--to" => {
                if to.is_some() {
                    return Err(ParseError::MarketReport(format!(
                        "--to was supplied twice\n{USAGE}"
                    )));
                }
                index += 1;
                let value = arguments.get(index).ok_or_else(|| {
                    ParseError::MarketReport(format!("--to requires a date\n{USAGE}"))
                })?;
                to = Some(value.clone());
            }
            "--out" => {
                if out.is_some() {
                    return Err(ParseError::MarketReport(format!(
                        "--out was supplied twice\n{USAGE}"
                    )));
                }
                index += 1;
                let value = arguments
                    .get(index)
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| {
                        ParseError::MarketReport(format!(
                            "--out requires a non-empty path\n{USAGE}"
                        ))
                    })?;
                out = Some(PathBuf::from(value));
            }
            "--json" => {
                if json {
                    return Err(ParseError::MarketReport(format!(
                        "--json was supplied twice\n{USAGE}"
                    )));
                }
                json = true;
            }
            option if option.starts_with("--") => {
                return Err(ParseError::MarketReport(format!(
                    "unknown option '{option}'\n{USAGE}"
                )));
            }
            value => {
                if path.is_some() {
                    return Err(ParseError::MarketReport(format!(
                        "unexpected argument '{value}'\n{USAGE}"
                    )));
                }
                if value.is_empty() {
                    return Err(ParseError::MarketReport(format!(
                        "the CSV path is empty\n{USAGE}"
                    )));
                }
                path = Some(PathBuf::from(value));
            }
        }
        index += 1;
    }

    let path =
        path.ok_or_else(|| ParseError::MarketReport(format!("missing CSV path\n{USAGE}")))?;
    let data_version = data_version
        .ok_or_else(|| ParseError::MarketReport(format!("--data-version is required\n{USAGE}")))?;
    let out = out.ok_or_else(|| ParseError::MarketReport(format!("--out is required\n{USAGE}")))?;
    let bounds = DateBounds::parse(from.as_deref(), to.as_deref())
        .map_err(|error| ParseError::MarketReport(format!("{error}\n{USAGE}")))?;

    Ok(MarketReportArgs {
        path,
        data_version,
        from,
        to,
        bounds,
        out,
        json,
    })
}

/// Writes the four report artifacts and prints the contract's stdout.
///
/// # Errors
///
/// Returns [`RunError`] when the source, the report, a chart, an artifact, or
/// the writer fails.
pub(crate) fn run_market_report(
    args: &MarketReportArgs,
    writer: &mut impl Write,
) -> Result<(), RunError> {
    ensure_output_available(&args.out).map_err(RunError::Report)?;

    let set = pipeline::load_extended_set(&args.path, args.bounds).map_err(RunError::Source)?;
    if set.series(Timeframe::M1).is_empty() {
        return Err(RunError::Report(ReportError::EmptyPeriod {
            period: args.requested_period(),
        }));
    }

    let run = pipeline::model_run_from_set(&set, &MarketStateConfig::default());
    let aggregates = quantrule_analytics::cross_timeframe(&run);
    let report = MarketReport::compute(&run, &pipeline::ladder_secs(), &aggregates);
    if report.charts.bars.is_empty() {
        return Err(RunError::Report(ReportError::EmptyChart {
            period: args.requested_period(),
        }));
    }

    let summary =
        render_report::summary_bytes(&report, &args.data_version, &args.requested_period())
            .map_err(RunError::Output)?;
    let json = render_report::json_bytes(
        &report,
        &args.data_version,
        args.from.as_deref(),
        args.to.as_deref(),
    )
    .map_err(RunError::Output)?;
    let price_axes =
        chart::render_report_price_axes_png(&report.charts).map_err(RunError::Chart)?;
    let events = chart::render_report_events_png(&report.charts).map_err(RunError::Chart)?;

    let artifacts: [(&str, &[u8]); 4] = [
        (ARTIFACTS[0], summary.as_slice()),
        (ARTIFACTS[1], json.as_slice()),
        (ARTIFACTS[2], price_axes.as_slice()),
        (ARTIFACTS[3], events.as_slice()),
    ];
    write_artifacts(&args.out, &artifacts).map_err(RunError::Report)?;

    if args.json {
        writer.write_all(&json).map_err(RunError::Output)?;
    } else {
        for (name, _) in &artifacts {
            writeln!(writer, "{}", args.out.join(name).display()).map_err(RunError::Output)?;
        }
    }
    writer.flush().map_err(RunError::Output)
}

fn ensure_output_available(out: &Path) -> Result<(), ReportError> {
    match fs::metadata(out) {
        Ok(metadata) => {
            if !metadata.is_dir() {
                return Err(ReportError::OutputNotDirectory {
                    path: out.to_path_buf(),
                });
            }
            let mut entries = fs::read_dir(out).map_err(|source| ReportError::ReadOutput {
                path: out.to_path_buf(),
                source,
            })?;
            match entries.next() {
                None => Ok(()),
                Some(Ok(_)) => Err(ReportError::OutputNotEmpty {
                    path: out.to_path_buf(),
                }),
                Some(Err(source)) => Err(ReportError::ReadOutput {
                    path: out.to_path_buf(),
                    source,
                }),
            }
        }
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(ReportError::ReadOutput {
            path: out.to_path_buf(),
            source,
        }),
    }
}

fn write_artifacts(out: &Path, artifacts: &[(&str, &[u8])]) -> Result<(), ReportError> {
    let created = !out.exists();
    if created {
        fs::create_dir_all(out).map_err(|source| ReportError::CreateDirectory {
            path: out.to_path_buf(),
            source,
        })?;
    }

    for (name, bytes) in artifacts {
        let path = out.join(name);
        if let Err(source) = export::write_atomic(&path, bytes) {
            cleanup(out, created);
            return Err(ReportError::Artifact { path, source });
        }
    }
    Ok(())
}

fn cleanup(out: &Path, created: bool) {
    for name in ARTIFACTS {
        let _ = fs::remove_file(out.join(name));
    }
    if created {
        let _ = fs::remove_dir(out);
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::{parse_market_report_arguments, write_artifacts};
    use crate::command::types::{ParseError, ReportError};

    fn arguments(tokens: &[&str]) -> Vec<String> {
        tokens.iter().map(|token| (*token).to_owned()).collect()
    }

    #[test]
    fn parses_every_option() {
        let args = parse_market_report_arguments(&arguments(&[
            "data/m1.csv",
            "--data-version",
            "source:sol:60:0-1000",
            "--from",
            "2025-01-01",
            "--to",
            "2025-12-31",
            "--out",
            "data/report",
            "--json",
        ]))
        .expect("the arguments are valid");

        assert_eq!(args.path, PathBuf::from("data/m1.csv"));
        assert_eq!(args.data_version, "source:sol:60:0-1000");
        assert_eq!(args.from.as_deref(), Some("2025-01-01"));
        assert_eq!(args.to.as_deref(), Some("2025-12-31"));
        assert_eq!(args.out, PathBuf::from("data/report"));
        assert!(args.json);
        assert_eq!(args.requested_period(), "2025-01-01..2025-12-31");
    }

    #[test]
    fn bounds_are_optional() {
        let args = parse_market_report_arguments(&arguments(&[
            "m1.csv",
            "--data-version",
            "v1",
            "--out",
            "out",
        ]))
        .expect("the arguments are valid");

        assert_eq!(args.bounds.start_seconds(), None);
        assert_eq!(args.bounds.end_exclusive_seconds(), None);
        assert_eq!(args.requested_period(), "full file");
        assert!(!args.json);
    }

    #[test]
    fn rejects_missing_required_options() {
        assert!(matches!(
            parse_market_report_arguments(&arguments(&[
                "m1.csv",
                "--out",
                "out",
            ])),
            Err(ParseError::MarketReport(message)) if message.contains("--data-version is required")
        ));
        assert!(matches!(
            parse_market_report_arguments(&arguments(&[
                "m1.csv",
                "--data-version",
                "v1",
            ])),
            Err(ParseError::MarketReport(message)) if message.contains("--out is required")
        ));
        assert!(matches!(
            parse_market_report_arguments(&arguments(&[
                "--data-version",
                "v1",
                "--out",
                "out",
            ])),
            Err(ParseError::MarketReport(message)) if message.contains("missing CSV path")
        ));
    }

    #[test]
    fn rejects_invalid_dates() {
        assert!(matches!(
            parse_market_report_arguments(&arguments(&[
                "m1.csv",
                "--data-version",
                "v1",
                "--from",
                "2025-02-30",
                "--out",
                "out",
            ])),
            Err(ParseError::MarketReport(message)) if message.contains("invalid UTC date")
        ));
        assert!(matches!(
            parse_market_report_arguments(&arguments(&[
                "m1.csv",
                "--data-version",
                "v1",
                "--from",
                "2025-01-02",
                "--to",
                "2025-01-01",
                "--out",
                "out",
            ])),
            Err(ParseError::MarketReport(message)) if message.contains("is after")
        ));
    }

    #[test]
    fn rejects_repeated_and_unknown_options() {
        assert!(matches!(
            parse_market_report_arguments(&arguments(&[
                "m1.csv",
                "--data-version",
                "v1",
                "--data-version",
                "v2",
                "--out",
                "out",
            ])),
            Err(ParseError::MarketReport(message)) if message.contains("twice")
        ));
        assert!(matches!(
            parse_market_report_arguments(&arguments(&[
                "m1.csv",
                "--data-version",
                "v1",
                "--out",
                "out",
                "--json",
                "--json",
            ])),
            Err(ParseError::MarketReport(message)) if message.contains("twice")
        ));
        assert!(matches!(
            parse_market_report_arguments(&arguments(&[
                "m1.csv",
                "--data-version",
                "v1",
                "--out",
                "out",
                "--window",
                "full",
            ])),
            Err(ParseError::MarketReport(message)) if message.contains("unknown option")
        ));
    }

    #[test]
    fn rejects_an_empty_data_version() {
        assert!(matches!(
            parse_market_report_arguments(&arguments(&[
                "m1.csv",
                "--data-version",
                "",
                "--out",
                "out",
            ])),
            Err(ParseError::MarketReport(message)) if message.contains("non-empty")
        ));
    }

    #[test]
    fn a_failed_artifact_write_leaves_no_artifact_files() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/report-tests/atomic-failure");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("the test directory is created");
        let out = root.join("blocked");
        fs::write(&out, b"blocker").expect("the blocker file is written");

        let artifacts: [(&str, &[u8]); 1] = [("summary.txt", b"summary")];
        let error = write_artifacts(&out, &artifacts).expect_err("the file blocks the directory");
        assert!(matches!(error, ReportError::Artifact { .. }));
        assert_eq!(
            fs::read(&out).expect("the blocker file is readable"),
            b"blocker"
        );
        assert!(!root.join("summary.txt").exists());

        fs::remove_dir_all(&root).expect("the test directory is removed");
    }
}
