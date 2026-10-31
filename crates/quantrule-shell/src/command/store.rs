//! Store command parsing and execution.
//!
//! The commands create, ingest, and remove one dataset at a time. Ingestion
//! builds the layers before opening the database, so a source or cap failure
//! leaves no partial dataset.

use std::io::Write;
use std::path::{Path, PathBuf};

use quantrule_model::readings::PivotScale;

use super::layers::{LayerBuild, load_layers};
use super::store_query::{self, DEFAULT_DATABASE};
use super::store_verify;
use super::types::{Command, ParseError, RunError, StoreIngestArgs, StoreRemoveArgs};
use crate::store::{DatasetRecord, IngestRows, RemoveReport, Store};

/// Parses the arguments after `store`.
///
/// `init`, `ingest`, and `remove` live here; `list` and `show` are delegated to
/// the query module, and `verify` to its own module.
///
/// # Errors
///
/// Returns [`ParseError::Store`] for a missing or unknown subcommand, a missing
/// option value, or a repeated option.
pub(crate) fn parse_store_arguments(arguments: &[String]) -> Result<Option<Command>, ParseError> {
    const USAGE: &str = "usage: store init [--db <path>] | store ingest <csv-path> --dataset <name> --data-version <id> [--db <path>] | store verify <csv-path> --dataset <name> --data-version <id> [--db <path>] | store list [--db <path>] | store show <dataset> <table> [--timeframe <tf>] [--bar <n>] [--limit <n>] [--db <path>] | store remove <dataset> [--db <path>]";
    match arguments.first().map(String::as_str) {
        Some("init") => parse_store_init(arguments, USAGE),
        Some("ingest") => parse_store_ingest(arguments, USAGE),
        Some("verify") => store_verify::parse_store_verify(arguments, USAGE).map(Some),
        Some("list") => store_query::parse_store_list(arguments, USAGE).map(Some),
        Some("show") => store_query::parse_store_show(arguments, USAGE).map(Some),
        Some("remove") => parse_store_remove(arguments, USAGE),
        _ => Err(ParseError::Store(USAGE.to_string())),
    }
}

fn parse_store_init(arguments: &[String], usage: &str) -> Result<Option<Command>, ParseError> {
    let mut path = PathBuf::from(DEFAULT_DATABASE);
    let mut index = 1;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--db" => {
                index += 1;
                let value = arguments
                    .get(index)
                    .ok_or_else(|| ParseError::Store(format!("--db needs a value\n{usage}")))?;
                if value.is_empty() {
                    return Err(ParseError::Store(format!(
                        "--db needs a non-empty value\n{usage}"
                    )));
                }
                path = PathBuf::from(value);
            }
            option => {
                return Err(ParseError::Store(format!(
                    "unknown argument '{option}'\n{usage}"
                )));
            }
        }
        index += 1;
    }

    Ok(Some(Command::StoreInit(path)))
}

fn parse_store_remove(arguments: &[String], usage: &str) -> Result<Option<Command>, ParseError> {
    let mut dataset = None;
    let mut database = PathBuf::from(DEFAULT_DATABASE);
    let mut index = 1;

    while index < arguments.len() {
        match arguments[index].as_str() {
            "--db" => {
                index += 1;
                let value = arguments
                    .get(index)
                    .ok_or_else(|| ParseError::Store(format!("--db needs a value\n{usage}")))?;
                if value.is_empty() {
                    return Err(ParseError::Store(format!(
                        "--db needs a non-empty value\n{usage}"
                    )));
                }
                database = PathBuf::from(value);
            }
            option if option.starts_with("--") => {
                return Err(ParseError::Store(format!(
                    "unknown option '{option}'\n{usage}"
                )));
            }
            value => {
                if dataset.is_some() {
                    return Err(ParseError::Store(format!(
                        "unexpected argument '{value}'\n{usage}"
                    )));
                }
                dataset = Some(value.to_owned());
            }
        }
        index += 1;
    }

    let dataset = dataset
        .ok_or_else(|| ParseError::Store(format!("store remove needs a dataset name\n{usage}")))?;
    Ok(Some(Command::StoreRemove(StoreRemoveArgs {
        dataset,
        database,
    })))
}

fn parse_store_ingest(arguments: &[String], usage: &str) -> Result<Option<Command>, ParseError> {
    let mut path = None;
    let mut dataset = None;
    let mut data_version = None;
    let mut database = PathBuf::from(DEFAULT_DATABASE);
    let mut index = 1;

    while index < arguments.len() {
        match arguments[index].as_str() {
            "--dataset" => {
                index += 1;
                let value = arguments.get(index).ok_or_else(|| {
                    ParseError::Store(format!("--dataset needs a value\n{usage}"))
                })?;
                if value.is_empty() {
                    return Err(ParseError::Store(format!(
                        "--dataset needs a non-empty value\n{usage}"
                    )));
                }
                if dataset.is_some() {
                    return Err(ParseError::Store(format!(
                        "--dataset was supplied twice\n{usage}"
                    )));
                }
                dataset = Some(value.clone());
            }
            "--data-version" => {
                index += 1;
                let value = arguments.get(index).ok_or_else(|| {
                    ParseError::Store(format!("--data-version needs a value\n{usage}"))
                })?;
                if value.is_empty() {
                    return Err(ParseError::Store(format!(
                        "--data-version needs a non-empty value\n{usage}"
                    )));
                }
                if data_version.is_some() {
                    return Err(ParseError::Store(format!(
                        "--data-version was supplied twice\n{usage}"
                    )));
                }
                data_version = Some(value.clone());
            }
            "--db" => {
                index += 1;
                let value = arguments
                    .get(index)
                    .ok_or_else(|| ParseError::Store(format!("--db needs a value\n{usage}")))?;
                if value.is_empty() {
                    return Err(ParseError::Store(format!(
                        "--db needs a non-empty value\n{usage}"
                    )));
                }
                database = PathBuf::from(value);
            }
            option if option.starts_with("--") => {
                return Err(ParseError::Store(format!(
                    "unknown option '{option}'\n{usage}"
                )));
            }
            value => {
                if path.is_none() {
                    path = Some(PathBuf::from(value));
                } else {
                    return Err(ParseError::Store(format!(
                        "unexpected argument '{value}'\n{usage}"
                    )));
                }
            }
        }
        index += 1;
    }

    let path = path.ok_or_else(|| ParseError::Store(format!("missing CSV path\n{usage}")))?;
    let dataset =
        dataset.ok_or_else(|| ParseError::Store(format!("missing --dataset\n{usage}")))?;
    let data_version = data_version
        .ok_or_else(|| ParseError::Store(format!("missing --data-version\n{usage}")))?;

    Ok(Some(Command::StoreIngest(StoreIngestArgs {
        path,
        dataset,
        data_version,
        database,
    })))
}

/// Creates or opens the store and prints its schema version.
///
/// # Errors
///
/// Returns a [`RunError`] when the store cannot be opened or the writer fails.
pub(crate) fn run_store_init(path: &Path, writer: &mut impl Write) -> Result<(), RunError> {
    let store = Store::open(path).map_err(RunError::Store)?;
    let version = store.schema_version().map_err(RunError::Store)?;
    writeln!(writer, "store {} schema={version}", path.display()).map_err(RunError::Output)?;
    writer.flush().map_err(RunError::Output)
}

/// Removes one dataset and its cascading records.
///
/// # Errors
///
/// Returns a [`RunError`] when the store cannot be opened, the dataset is
/// missing, the delete fails, or the output writer fails.
pub(crate) fn run_store_remove(
    args: &StoreRemoveArgs,
    writer: &mut impl Write,
) -> Result<(), RunError> {
    let mut store = Store::open_existing(&args.database).map_err(RunError::Store)?;
    let RemoveReport { name, bars } = store.remove(&args.dataset).map_err(RunError::Store)?;
    writeln!(writer, "removed {name} bars={bars}").map_err(RunError::Output)?;
    writer.flush().map_err(RunError::Output)
}

/// Builds the layers and writes one dataset to the store.
///
/// The layers are built before the database is opened, so a source, model, or
/// cap failure leaves no database file and no partial dataset. The writer then
/// commits the dataset row, every Layer 2 bar row, every pivot occurrence, and
/// every confirmed level in one transaction.
///
/// # Errors
///
/// Returns a [`RunError`] when the source, the layer build, the store, or the
/// writer fails.
pub(crate) fn run_store_ingest(
    args: &StoreIngestArgs,
    writer: &mut impl Write,
) -> Result<(), RunError> {
    let LayerBuild { layers, m1, .. } = load_layers(&args.path)?;
    let dataset = DatasetRecord {
        name: args.dataset.clone(),
        data_version: args.data_version.clone(),
        model_version: env!("CARGO_PKG_VERSION").to_string(),
        structure_scale_code: PivotScale::Major.code(),
        m1_bars: m1.len(),
    };

    let mut store = Store::open(&args.database).map_err(RunError::Store)?;
    let rows = IngestRows::new(
        &layers.layer2,
        &layers.consensus,
        &layers.layer3,
        &layers.pivots,
        &layers.levels,
        &layers.events,
        &layers.legs,
    );
    let report = store.ingest(&dataset, &rows).map_err(RunError::Store)?;

    let verb = if report.unchanged {
        "unchanged"
    } else {
        "ingested"
    };
    writeln!(
        writer,
        "{verb} {} bars={} timeframes={}",
        args.dataset, report.bars, report.timeframes
    )
    .map_err(RunError::Output)?;
    writer.flush().map_err(RunError::Output)
}
