//! Research command parsing and execution.
//!
//! The commands build the three analytical layers and either summarize them or
//! render the complete research dataset. The data engine and model engine stay
//! pure; this module performs the file writes.

use std::io::{self, Write};
use std::path::{Path, PathBuf};

use quantrule_analytics::research::{DatasetMeta, render_dataset_with_ladder};
use quantrule_analytics::summary::{DatasetSummary, summarize};
use quantrule_model::readings::PivotScale;

use super::layers::{LayerBuild, load_layers};
use super::types::{Command, ParseError, ResearchExportArgs, RunError};
use crate::export;

/// Parses the arguments after `research`.
///
/// The shape is `research export` with a CSV path, a dataset name, a required
/// `--data-version`, and an optional `--root`. This entry point is also used by
/// the non-interactive binary, where the operating system has already split the
/// arguments.
///
/// # Errors
///
/// Returns [`ParseError::Research`] for a missing subcommand, a missing
/// argument, an unknown option, or a repeated positional argument.
pub(crate) fn parse_research_arguments(
    arguments: &[String],
) -> Result<Option<Command>, ParseError> {
    const USAGE: &str = "usage: research export <csv-path> <dataset> --data-version <id> [--root <dir>] | research summary <csv-path>";
    if arguments.first().map(String::as_str) == Some("summary") {
        if arguments.len() != 2 {
            return Err(ParseError::Research(
                "usage: research summary <csv-path>".to_string(),
            ));
        }
        return Ok(Some(Command::ResearchSummary(PathBuf::from(&arguments[1]))));
    }
    if arguments.first().map(String::as_str) != Some("export") {
        return Err(ParseError::Research(USAGE.to_string()));
    }

    let mut path = None;
    let mut dataset = None;
    let mut data_version = None;
    let mut root = PathBuf::from("data/research");
    let mut data_version_seen = false;
    let mut root_seen = false;
    let mut index = 1;

    while index < arguments.len() {
        match arguments[index].as_str() {
            "--data-version" => {
                if data_version_seen {
                    return Err(ParseError::Research(format!(
                        "--data-version was supplied twice\n{USAGE}"
                    )));
                }
                index += 1;
                let value = arguments.get(index).ok_or_else(|| {
                    ParseError::Research(format!("--data-version needs a value\n{USAGE}"))
                })?;
                data_version = Some(value.clone());
                data_version_seen = true;
            }
            "--root" => {
                if root_seen {
                    return Err(ParseError::Research(format!(
                        "--root was supplied twice\n{USAGE}"
                    )));
                }
                index += 1;
                let value = arguments.get(index).ok_or_else(|| {
                    ParseError::Research(format!("--root needs a value\n{USAGE}"))
                })?;
                root = PathBuf::from(value);
                root_seen = true;
            }
            option if option.starts_with("--") => {
                return Err(ParseError::Research(format!(
                    "unknown option '{option}'\n{USAGE}"
                )));
            }
            value => {
                if path.is_none() {
                    path = Some(PathBuf::from(value));
                } else if dataset.is_none() {
                    dataset = Some(value.to_string());
                } else {
                    return Err(ParseError::Research(format!(
                        "unexpected argument '{value}'\n{USAGE}"
                    )));
                }
            }
        }
        index += 1;
    }

    let path = path.ok_or_else(|| ParseError::Research(format!("missing CSV path\n{USAGE}")))?;
    let dataset =
        dataset.ok_or_else(|| ParseError::Research(format!("missing dataset name\n{USAGE}")))?;
    let data_version = data_version
        .ok_or_else(|| ParseError::Research(format!("missing --data-version\n{USAGE}")))?;

    Ok(Some(Command::ResearchExport(ResearchExportArgs {
        path,
        dataset,
        data_version,
        root,
    })))
}

/// Prints the deterministic summary of the research layers.
///
/// # Errors
///
/// Returns a [`RunError`] when the source, the layer build, or the writer fails.
pub(crate) fn run_research_summary(path: &Path, writer: &mut impl Write) -> Result<(), RunError> {
    let LayerBuild { layers, .. } = load_layers(path)?;
    let summary = summarize(&layers);
    write_summary(&summary, writer).map_err(RunError::Output)?;
    writer.flush().map_err(RunError::Output)
}

fn write_summary(summary: &DatasetSummary, writer: &mut impl Write) -> io::Result<()> {
    writeln!(
        writer,
        "summary layer1={} layer2={} layer3={}",
        summary.layer1_rows, summary.layer2_rows, summary.layer3_rows
    )?;
    for axis in &summary.axes {
        write!(
            writer,
            "axis timeframe={} axis={} n={} null={} min={} max={} mean={} ordinals=",
            axis.timeframe_secs,
            axis.axis,
            axis.count,
            axis.nulls,
            number(axis.min),
            number(axis.max),
            number(axis.mean)
        )?;
        for (index, count) in axis.ordinal_counts.iter().enumerate() {
            if index > 0 {
                write!(writer, ",")?;
            }
            write!(writer, "{count}")?;
        }
        writeln!(writer, " ordinal_null={}", axis.ordinal_nulls)?;
    }
    for relation in &summary.relations {
        writeln!(
            writer,
            "relation timeframe={} left={} right={} n={} r={}",
            relation.timeframe_secs,
            relation.left,
            relation.right,
            relation.count,
            number(relation.correlation)
        )?;
    }
    Ok(())
}

fn number(value: Option<f64>) -> String {
    value
        .map(export::format_number)
        .unwrap_or_else(|| "null".to_string())
}

/// Writes the research dataset for an M1 CSV.
///
/// The dataset is rendered before any file is written, so a render or
/// validation failure writes nothing. Each file is then written atomically.
///
/// # Errors
///
/// Returns a [`RunError`] when the source, the layer build, the render, or a
/// file write fails.
pub(crate) fn run_research_export(
    args: &ResearchExportArgs,
    writer: &mut impl Write,
) -> Result<(), RunError> {
    let LayerBuild { layers, m1, higher } = load_layers(&args.path)?;

    let meta = DatasetMeta {
        root: args.root.display().to_string(),
        dataset: args.dataset.clone(),
        data_version: args.data_version.clone(),
        model_version: env!("CARGO_PKG_VERSION").to_string(),
        structure_scale_code: PivotScale::Major as u8,
    };
    let bundle = render_dataset_with_ladder(&layers, &m1, &meta, &higher)
        .map_err(RunError::ResearchExport)?;

    for file in &bundle.files {
        export::write_atomic(Path::new(&file.path), file.contents.as_bytes())
            .map_err(RunError::Write)?;
    }

    writeln!(writer, "exported {}", args.dataset).map_err(RunError::Output)?;
    writeln!(writer, "  data version: {}", args.data_version).map_err(RunError::Output)?;
    writeln!(writer, "  raw rows: {}", bundle.raw_rows).map_err(RunError::Output)?;
    writeln!(writer, "  layer1 rows: {}", bundle.layer1_rows).map_err(RunError::Output)?;
    writeln!(writer, "  layer2 rows: {}", bundle.layer2_rows).map_err(RunError::Output)?;
    writeln!(writer, "  layer3 rows: {}", bundle.layer3_rows).map_err(RunError::Output)?;
    writeln!(writer, "  files: {}", bundle.files.len()).map_err(RunError::Output)?;
    writer.flush().map_err(RunError::Output)
}
