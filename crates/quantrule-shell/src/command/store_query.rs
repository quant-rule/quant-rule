//! Parsing and rendering for the store query commands.
//!
//! The commands are read-only. They open the database through
//! [`Store::open_read_only`], render the query result, and never recompute a
//! stored value.

use std::fmt::Write as _;
use std::io::{self, Write};
use std::path::PathBuf;

use quantrule_data::format_utc_timestamp;

use super::parse_native_timeframe;

use super::types::{Command, DbArgs, DbKind, ParseError, RunError, StoreListArgs, StoreShowArgs};
use crate::store::{Cell, DatasetSummary, SeriesRow, Store, Table, TableRows};

const DEFAULT_LIMIT: usize = 100;
const MAX_LIMIT: usize = 1_000;
pub(super) const DEFAULT_DATABASE: &str = "data/db/quantrule.db";

/// Parses `store list`.
///
/// # Errors
///
/// Returns [`ParseError::Store`] for a repeated or unknown option.
pub(crate) fn parse_store_list(arguments: &[String], usage: &str) -> Result<Command, ParseError> {
    let mut database = PathBuf::from(DEFAULT_DATABASE);
    let mut index = 1;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--db" => {
                let value = value(arguments, &mut index, "--db", usage)?;
                database = PathBuf::from(value);
            }
            option => {
                return Err(ParseError::Store(format!(
                    "unknown argument '{option}'\n{usage}"
                )));
            }
        }
        index += 1;
    }
    Ok(Command::StoreList(StoreListArgs { database }))
}

/// Parses `store show`.
///
/// # Errors
///
/// Returns [`ParseError::Store`] for missing positionals, an unknown table, an
/// invalid or repeated option, or a filter that the table does not support.
pub(crate) fn parse_store_show(arguments: &[String], usage: &str) -> Result<Command, ParseError> {
    let mut dataset = None;
    let mut table = None;
    let mut timeframe = None;
    let mut bar = None;
    let mut limit = DEFAULT_LIMIT;
    let mut database = PathBuf::from(DEFAULT_DATABASE);
    let mut index = 1;

    while index < arguments.len() {
        match arguments[index].as_str() {
            "--timeframe" => {
                let text = value(arguments, &mut index, "--timeframe", usage)?;
                if timeframe.is_some() {
                    return Err(ParseError::Store(format!(
                        "--timeframe was supplied twice\n{usage}"
                    )));
                }
                timeframe = Some(parse_timeframe(text, usage)?);
            }
            "--bar" => {
                let text = value(arguments, &mut index, "--bar", usage)?;
                if bar.is_some() {
                    return Err(ParseError::Store(format!(
                        "--bar was supplied twice\n{usage}"
                    )));
                }
                bar = Some(parse_bar(text, usage)?);
            }
            "--limit" => {
                limit = parse_limit(arguments, &mut index, usage)?;
            }
            "--db" => {
                let text = value(arguments, &mut index, "--db", usage)?;
                database = PathBuf::from(text);
            }
            option if option.starts_with("--") => {
                return Err(ParseError::Store(format!(
                    "unknown option '{option}'\n{usage}"
                )));
            }
            text => {
                if dataset.is_none() {
                    dataset = Some(text.to_string());
                } else if table.is_none() {
                    table = Some(Table::from_name(text).ok_or_else(|| {
                        ParseError::Store(format!(
                            "unknown table '{text}'; use bar, consensus, layer3, pivot, level, event, or leg\n{usage}"
                        ))
                    })?);
                } else {
                    return Err(ParseError::Store(format!(
                        "unexpected argument '{text}'\n{usage}"
                    )));
                }
            }
        }
        index += 1;
    }

    let dataset =
        dataset.ok_or_else(|| ParseError::Store(format!("missing dataset name\n{usage}")))?;
    let table = table.ok_or_else(|| ParseError::Store(format!("missing table name\n{usage}")))?;
    if timeframe.is_some() && !table.has_timeframe() {
        return Err(ParseError::Store(format!(
            "{} has no timeframe_secs column\n{usage}",
            table_name(table)
        )));
    }
    if bar.is_some() && !table.has_bar_index() {
        return Err(ParseError::Store(format!(
            "{} has no bar_index column\n{usage}",
            table_name(table)
        )));
    }

    Ok(Command::StoreShow(StoreShowArgs {
        dataset,
        table,
        timeframe,
        bar,
        limit,
        database,
    }))
}

/// Parses one `db` view.
///
/// # Errors
///
/// Returns [`ParseError::Store`] for a missing or unknown view, a missing
/// dataset, an invalid or repeated option, or a filter the view does not
/// support.
pub(crate) fn parse_db_arguments(arguments: &[String]) -> Result<Option<Command>, ParseError> {
    const USAGE: &str = "usage: db series <dataset> [--timeframe <tf>] [--limit <n>] [--db <path>] | db bars <dataset> [--timeframe <tf>] [--bar <n>] [--limit <n>] [--db <path>] | db metrics <dataset> [--timeframe <tf>] [--bar <n>] [--limit <n>] [--db <path>] | db layer3 <dataset> [--bar <n>] [--limit <n>] [--db <path>]";
    let kind = match arguments.first().map(String::as_str) {
        Some("series") => DbKind::Series,
        Some("bars") => DbKind::Bars,
        Some("metrics") => DbKind::Metrics,
        Some("layer3") => DbKind::Layer3,
        _ => return Err(ParseError::Store(USAGE.to_string())),
    };

    let allow_timeframe = !matches!(kind, DbKind::Layer3);
    let allow_bar = matches!(kind, DbKind::Bars | DbKind::Metrics | DbKind::Layer3);
    let mut dataset = None;
    let mut timeframe = None;
    let mut bar = None;
    let mut limit = DEFAULT_LIMIT;
    let mut database = PathBuf::from(DEFAULT_DATABASE);
    let mut index = 1;

    while index < arguments.len() {
        match arguments[index].as_str() {
            "--timeframe" if allow_timeframe => {
                let text = value(arguments, &mut index, "--timeframe", USAGE)?;
                if timeframe.is_some() {
                    return Err(ParseError::Store(format!(
                        "--timeframe was supplied twice\n{USAGE}"
                    )));
                }
                timeframe = Some(parse_timeframe(text, USAGE)?);
            }
            "--bar" if allow_bar => {
                let text = value(arguments, &mut index, "--bar", USAGE)?;
                if bar.is_some() {
                    return Err(ParseError::Store(format!(
                        "--bar was supplied twice\n{USAGE}"
                    )));
                }
                bar = Some(parse_bar(text, USAGE)?);
            }
            "--limit" => {
                limit = parse_limit(arguments, &mut index, USAGE)?;
            }
            "--db" => {
                let text = value(arguments, &mut index, "--db", USAGE)?;
                database = PathBuf::from(text);
            }
            option if option.starts_with("--") => {
                return Err(ParseError::Store(format!(
                    "unknown option '{option}'\n{USAGE}"
                )));
            }
            text => {
                if dataset.is_none() {
                    dataset = Some(text.to_string());
                } else {
                    return Err(ParseError::Store(format!(
                        "unexpected argument '{text}'\n{USAGE}"
                    )));
                }
            }
        }
        index += 1;
    }

    let dataset =
        dataset.ok_or_else(|| ParseError::Store(format!("missing dataset name\n{USAGE}")))?;
    Ok(Some(Command::Db(DbArgs {
        kind,
        dataset,
        timeframe,
        bar,
        limit,
        database,
    })))
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

fn parse_timeframe(text: &str, usage: &str) -> Result<u64, ParseError> {
    let timeframe = parse_native_timeframe(text).ok_or_else(|| {
        ParseError::Store(format!(
            "--timeframe expects 1m, 5m, 15m, or 1h, found '{text}'\n{usage}"
        ))
    })?;
    Ok(u64::from(timeframe.seconds()))
}

fn parse_bar(text: &str, usage: &str) -> Result<usize, ParseError> {
    text.parse::<usize>().map_err(|_| {
        ParseError::Store(format!(
            "--bar expects a non-negative integer, found '{text}'\n{usage}"
        ))
    })
}

fn parse_limit(arguments: &[String], index: &mut usize, usage: &str) -> Result<usize, ParseError> {
    let text = value(arguments, index, "--limit", usage)?;
    let limit = text.parse::<usize>().map_err(|_| {
        ParseError::Store(format!(
            "--limit expects a positive integer, found '{text}'\n{usage}"
        ))
    })?;
    if limit == 0 || limit > MAX_LIMIT {
        return Err(ParseError::Store(format!(
            "--limit must be between 1 and {MAX_LIMIT}\n{usage}"
        )));
    }
    Ok(limit)
}

fn table_name(table: Table) -> &'static str {
    match table {
        Table::Bar => "bar",
        Table::Consensus => "consensus",
        Table::Layer3 => "layer3",
        Table::Pivot => "pivot",
        Table::Level => "level",
        Table::Event => "event",
        Table::Leg => "leg",
    }
}

/// Runs `store list`.
///
/// # Errors
///
/// Returns a [`RunError`] when the store cannot be read or the writer fails.
pub(crate) fn run_store_list(
    args: &StoreListArgs,
    writer: &mut impl Write,
) -> Result<(), RunError> {
    let store = Store::open_read_only(&args.database).map_err(RunError::Store)?;
    let summaries = store.list_datasets().map_err(RunError::Store)?;
    write_dataset_list(&summaries, writer).map_err(RunError::Output)?;
    writer.flush().map_err(RunError::Output)
}

fn write_dataset_list(summaries: &[DatasetSummary], writer: &mut impl Write) -> io::Result<()> {
    let columns = vec![
        "name",
        "data_version",
        "model_version",
        "structure_scale_code",
        "m1_bars",
        "layer3_materialized",
        "bar_rows",
        "consensus_rows",
        "layer3_rows",
        "pivot_rows",
        "level_rows",
        "event_rows",
        "leg_rows",
    ];
    let rows = summaries
        .iter()
        .map(|summary| {
            vec![
                Cell::Text(summary.name.clone()),
                Cell::Text(summary.data_version.clone()),
                Cell::Text(summary.model_version.clone()),
                Cell::Integer(summary.structure_scale_code),
                Cell::Integer(summary.m1_bars),
                Cell::Integer(summary.layer3_materialized),
                Cell::Integer(summary.bar_rows),
                Cell::Integer(summary.consensus_rows),
                Cell::Integer(summary.layer3_rows),
                Cell::Integer(summary.pivot_rows),
                Cell::Integer(summary.level_rows),
                Cell::Integer(summary.event_rows),
                Cell::Integer(summary.leg_rows),
            ]
        })
        .collect();
    let table = TableRows {
        columns: columns.into_iter().map(String::from).collect(),
        rows,
    };
    table.write_csv(writer)
}

/// Runs `store show`.
///
/// # Errors
///
/// Returns a [`RunError`] when the store cannot be read or the writer fails.
pub(crate) fn run_store_show(
    args: &StoreShowArgs,
    writer: &mut impl Write,
) -> Result<(), RunError> {
    let store = Store::open_read_only(&args.database).map_err(RunError::Store)?;
    let table = store
        .show_table(
            &args.dataset,
            args.table,
            args.timeframe,
            args.bar,
            args.limit,
        )
        .map_err(RunError::Store)?;
    table.write_csv(writer).map_err(RunError::Output)?;
    writer.flush().map_err(RunError::Output)
}

/// Runs one `db` view.
///
/// # Errors
///
/// Returns a [`RunError`] when the store cannot be read or the writer fails.
pub(crate) fn run_db(args: &DbArgs, writer: &mut impl Write) -> Result<(), RunError> {
    let store = Store::open_read_only(&args.database).map_err(RunError::Store)?;
    match args.kind {
        DbKind::Series => {
            let rows = store
                .series(&args.dataset, args.timeframe, args.limit)
                .map_err(RunError::Store)?;
            write_series(&args.dataset, &rows, writer).map_err(RunError::Output)?;
        }
        DbKind::Bars => {
            let table = store
                .show_table(
                    &args.dataset,
                    Table::Bar,
                    args.timeframe,
                    args.bar,
                    args.limit,
                )
                .map_err(RunError::Store)?;
            table.write_csv(writer).map_err(RunError::Output)?;
        }
        DbKind::Metrics => {
            let table = store
                .metrics(&args.dataset, args.timeframe, args.bar, args.limit)
                .map_err(RunError::Store)?;
            table.write_csv(writer).map_err(RunError::Output)?;
        }
        DbKind::Layer3 => {
            let table = store
                .show_table(&args.dataset, Table::Layer3, None, args.bar, args.limit)
                .map_err(RunError::Store)?;
            table.write_csv(writer).map_err(RunError::Output)?;
        }
    }
    writer.flush().map_err(RunError::Output)
}

fn write_series(dataset: &str, rows: &[SeriesRow], writer: &mut impl Write) -> io::Result<()> {
    for row in rows {
        write!(
            writer,
            "{{\"dataset\":\"{}\",\"timeframe_secs\":{},\"bar_index\":{},\"open_time\":\"{}\",\"open\":{},\"high\":{},\"low\":{},\"close\":{},\"volume\":",
            json_escape(dataset),
            row.timeframe_secs,
            row.bar_index,
            format_utc_timestamp(row.open_time),
            format_float(row.open),
            format_float(row.high),
            format_float(row.low),
            format_float(row.close)
        )?;
        match row.volume {
            Some(volume) => write!(writer, "{}", format_float(volume))?,
            None => writer.write_all(b"null")?,
        }
        writer.write_all(b"}\n")?;
    }
    Ok(())
}

fn json_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            value if (value as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", value as u32);
            }
            value => out.push(value),
        }
    }
    out
}

fn format_float(value: f64) -> String {
    if value == 0.0 {
        "0".to_string()
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{parse_db_arguments, parse_store_list, parse_store_show};
    use crate::command::Command;
    use crate::command::types::{DbArgs, DbKind, StoreListArgs, StoreShowArgs};
    use crate::store::Table;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    #[test]
    fn parses_store_list_with_defaults_and_a_database() {
        assert_eq!(
            parse_store_list(&args(&["list"]), "usage"),
            Ok(Command::StoreList(StoreListArgs {
                database: PathBuf::from("data/db/quantrule.db"),
            }))
        );
        assert_eq!(
            parse_store_list(&args(&["list", "--db", "other.db"]), "usage"),
            Ok(Command::StoreList(StoreListArgs {
                database: PathBuf::from("other.db"),
            }))
        );
    }

    #[test]
    fn parses_store_show_with_filters() {
        assert_eq!(
            parse_store_show(
                &args(&[
                    "show",
                    "demo",
                    "bar",
                    "--timeframe",
                    "5m",
                    "--bar",
                    "3",
                    "--limit",
                    "10",
                    "--db",
                    "other.db",
                ]),
                "usage"
            ),
            Ok(Command::StoreShow(StoreShowArgs {
                dataset: String::from("demo"),
                table: Table::Bar,
                timeframe: Some(300),
                bar: Some(3),
                limit: 10,
                database: PathBuf::from("other.db"),
            }))
        );
    }

    #[test]
    fn rejects_bad_store_query_arguments() {
        for line in [
            &["show"][..],
            &["show", "demo"][..],
            &["show", "demo", "bogus"][..],
            &["show", "demo", "bar", "--timeframe", "30m"][..],
            &["show", "demo", "bar", "--bar", "-1"][..],
            &["show", "demo", "bar", "--limit", "0"][..],
            &["show", "demo", "bar", "--limit", "1001"][..],
            &["show", "demo", "layer3", "--timeframe", "1m"][..],
            &["show", "demo", "event", "--bar", "0"][..],
            &["show", "demo", "bar", "--bogus", "x"][..],
            &["list", "--bogus", "x"][..],
        ] {
            assert!(
                matches!(
                    if line[0] == "list" {
                        parse_store_list(&args(line), "usage")
                    } else {
                        parse_store_show(&args(line), "usage")
                    },
                    Err(crate::command::ParseError::Store(_))
                ),
                "line should be rejected: {line:?}"
            );
        }
    }

    #[test]
    fn parses_each_db_view() {
        assert_eq!(
            parse_db_arguments(&args(&[
                "series",
                "demo",
                "--timeframe",
                "1m",
                "--limit",
                "5"
            ])),
            Ok(Some(Command::Db(DbArgs {
                kind: DbKind::Series,
                dataset: String::from("demo"),
                timeframe: Some(60),
                bar: None,
                limit: 5,
                database: PathBuf::from("data/db/quantrule.db"),
            })))
        );
        assert_eq!(
            parse_db_arguments(&args(&["bars", "demo", "--bar", "2"])),
            Ok(Some(Command::Db(DbArgs {
                kind: DbKind::Bars,
                dataset: String::from("demo"),
                timeframe: None,
                bar: Some(2),
                limit: 100,
                database: PathBuf::from("data/db/quantrule.db"),
            })))
        );
        assert_eq!(
            parse_db_arguments(&args(&["metrics", "demo", "--timeframe", "15m"])),
            Ok(Some(Command::Db(DbArgs {
                kind: DbKind::Metrics,
                dataset: String::from("demo"),
                timeframe: Some(900),
                bar: None,
                limit: 100,
                database: PathBuf::from("data/db/quantrule.db"),
            })))
        );
        assert_eq!(
            parse_db_arguments(&args(&["layer3", "demo", "--bar", "1", "--db", "x.db"])),
            Ok(Some(Command::Db(DbArgs {
                kind: DbKind::Layer3,
                dataset: String::from("demo"),
                timeframe: None,
                bar: Some(1),
                limit: 100,
                database: PathBuf::from("x.db"),
            })))
        );
    }

    #[test]
    fn rejects_bad_db_views() {
        for line in [
            &["bogus", "demo"][..],
            &["series"][..],
            &["series", "demo", "--bar", "1"][..],
            &["layer3", "demo", "--timeframe", "1m"][..],
            &["series", "demo", "--bogus", "x"][..],
            &["bars", "demo", "--limit", "0"][..],
            &["bars", "demo", "--limit", "1001"][..],
            &["bars", "demo", "--timeframe", "30m"][..],
            &["bars", "demo", "--bar", "x"][..],
        ] {
            assert!(
                matches!(
                    parse_db_arguments(&args(line)),
                    Err(crate::command::ParseError::Store(_))
                ),
                "line should be rejected: {line:?}"
            );
        }
    }
}
