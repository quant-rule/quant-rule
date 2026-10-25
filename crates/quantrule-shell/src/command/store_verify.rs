//! Parsing and execution for `store verify`.
//!
//! The command rebuilds the source layers in memory and asks the read-only store
//! to compare every stored table with the fresh values.

use std::io::Write;
use std::path::PathBuf;

use quantrule_model::readings::PivotScale;

use super::layers::{LayerBuild, load_layers};
use super::types::{Command, ParseError, RunError, StoreVerifyArgs};
use crate::store::{DatasetRecord, IngestRows, Store};

/// Parses `store verify`.
///
/// # Errors
///
/// Returns [`ParseError::Store`] for a missing path, dataset, or data version,
/// or for an unknown or repeated option.
pub(crate) fn parse_store_verify(arguments: &[String], usage: &str) -> Result<Command, ParseError> {
    let mut path = None;
    let mut dataset = None;
    let mut data_version = None;
    let mut database = PathBuf::from("data/quantrule.db");
    let mut index = 1;

    while index < arguments.len() {
        match arguments[index].as_str() {
            "--dataset" => {
                let value = value(arguments, &mut index, "--dataset", usage)?;
                if dataset.is_some() {
                    return Err(ParseError::Store(format!(
                        "--dataset was supplied twice\n{usage}"
                    )));
                }
                dataset = Some(value.to_string());
            }
            "--data-version" => {
                let value = value(arguments, &mut index, "--data-version", usage)?;
                if data_version.is_some() {
                    return Err(ParseError::Store(format!(
                        "--data-version was supplied twice\n{usage}"
                    )));
                }
                data_version = Some(value.to_string());
            }
            "--db" => {
                let value = value(arguments, &mut index, "--db", usage)?;
                database = PathBuf::from(value);
            }
            option if option.starts_with("--") => {
                return Err(ParseError::Store(format!(
                    "unknown option '{option}'\n{usage}"
                )));
            }
            text => {
                if path.is_none() {
                    path = Some(PathBuf::from(text));
                } else {
                    return Err(ParseError::Store(format!(
                        "unexpected argument '{text}'\n{usage}"
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

    Ok(Command::StoreVerify(StoreVerifyArgs {
        path,
        dataset,
        data_version,
        database,
    }))
}

fn value<'a>(
    arguments: &'a [String],
    index: &mut usize,
    option: &str,
    usage: &str,
) -> Result<&'a str, ParseError> {
    *index += 1;
    let value = arguments
        .get(*index)
        .ok_or_else(|| ParseError::Store(format!("{option} needs a value\n{usage}")))?;
    if value.is_empty() {
        return Err(ParseError::Store(format!(
            "{option} needs a non-empty value\n{usage}"
        )));
    }
    Ok(value)
}

/// Rebuilds the source layers and verifies the stored dataset.
///
/// # Errors
///
/// Returns a [`RunError`] when the source, layer build, read-only store,
/// verification, or writer fails.
pub(crate) fn run_store_verify(
    args: &StoreVerifyArgs,
    writer: &mut impl Write,
) -> Result<(), RunError> {
    let LayerBuild { layers, m1 } = load_layers(&args.path)?;
    let dataset = DatasetRecord {
        name: args.dataset.clone(),
        data_version: args.data_version.clone(),
        model_version: env!("CARGO_PKG_VERSION").to_string(),
        structure_scale_code: PivotScale::Major.code(),
        m1_bars: m1.len(),
    };
    let store = Store::open_read_only(&args.database).map_err(RunError::Store)?;
    let rows = IngestRows::new(
        &layers.layer2,
        &layers.consensus,
        &layers.layer3,
        &layers.pivots,
        &layers.levels,
        &layers.events,
        &layers.legs,
    );
    let report = store.verify(&dataset, &rows).map_err(RunError::Store)?;
    writeln!(
        writer,
        "verified {} bars={} tables={}",
        args.dataset, report.bars, report.tables
    )
    .map_err(RunError::Output)?;
    writer.flush().map_err(RunError::Output)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::parse_store_verify;
    use crate::command::types::StoreVerifyArgs;
    use crate::command::{Command, ParseError};

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    #[test]
    fn parses_store_verify() {
        assert_eq!(
            parse_store_verify(
                &args(&[
                    "verify",
                    "data/m1.csv",
                    "--dataset",
                    "demo",
                    "--data-version",
                    "source:1",
                ]),
                "usage"
            ),
            Ok(Command::StoreVerify(StoreVerifyArgs {
                path: PathBuf::from("data/m1.csv"),
                dataset: String::from("demo"),
                data_version: String::from("source:1"),
                database: PathBuf::from("data/quantrule.db"),
            }))
        );
        assert_eq!(
            parse_store_verify(
                &args(&[
                    "verify",
                    "--db",
                    "other.db",
                    "data/m1.csv",
                    "--data-version",
                    "source:1",
                    "--dataset",
                    "demo",
                ]),
                "usage"
            ),
            Ok(Command::StoreVerify(StoreVerifyArgs {
                path: PathBuf::from("data/m1.csv"),
                dataset: String::from("demo"),
                data_version: String::from("source:1"),
                database: PathBuf::from("other.db"),
            }))
        );
    }

    #[test]
    fn rejects_bad_store_verify_arguments() {
        for line in [
            &["verify"][..],
            &["verify", "data/m1.csv"][..],
            &["verify", "data/m1.csv", "--dataset", "demo"][..],
            &[
                "verify",
                "data/m1.csv",
                "--dataset",
                "demo",
                "--data-version",
            ][..],
            &[
                "verify",
                "data/m1.csv",
                "--dataset",
                "demo",
                "--data-version",
                "v",
                "--bogus",
                "x",
            ][..],
            &[
                "verify",
                "data/m1.csv",
                "--dataset",
                "a",
                "--dataset",
                "b",
                "--data-version",
                "v",
            ][..],
        ] {
            assert!(
                matches!(
                    parse_store_verify(&args(line), "usage"),
                    Err(ParseError::Store(_))
                ),
                "line should be rejected: {line:?}"
            );
        }
    }
}
