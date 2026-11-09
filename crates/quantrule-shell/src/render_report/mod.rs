//! Rendering for the flat model report.
//!
//! The shell copies the numeric view into `summary.txt` and the JSON files; it
//! computes no statistic. The text maps stable codes to the shell's fixed
//! labels, and the JSON carries codes and numbers only.

mod bundle;
mod json;
mod provenance;

pub(crate) use bundle::{BundleError, ensure_available, write_bundle, write_stage_bundle};
pub(crate) use json::{json_bytes, model_result_bytes};
pub(crate) use provenance::{Provenance, bytes as provenance_bytes};

use std::io::{self, Write};

use quantrule_data::format_utc_timestamp;
use quantrule_model::report::{FlatEventSeries, FlatReport};

use crate::labels;

/// The report view-contract version carried by the JSON envelope.
pub(crate) const VIEW_CONTRACT: &str = "2026-11-05";

/// Renders `summary.txt`.
///
/// The sections and their order are fixed. Every value comes from
/// [`FlatReport`]; the shell only formats it.
///
/// # Errors
///
/// Returns the writer's error when a line cannot be written.
pub(crate) fn summary_bytes(
    report: &FlatReport,
    data_version: &str,
    requested_period: &str,
) -> io::Result<Vec<u8>> {
    let mut out = Vec::new();
    let first = report
        .timeframes
        .iter()
        .filter_map(|timeframe| timeframe.first_open_time)
        .min();
    let last = report
        .timeframes
        .iter()
        .filter_map(|timeframe| timeframe.last_open_time)
        .max();
    let ladder = report
        .timeframes
        .iter()
        .map(|timeframe| labels::timeframe_label(timeframe.timeframe_secs))
        .collect::<Vec<_>>()
        .join(" ");

    writeln!(out, "QuantRule model report")?;
    writeln!(out, "data_version: {data_version}")?;
    writeln!(out, "requested_period: {requested_period}")?;
    writeln!(
        out,
        "actual_period: {}..{}",
        opt_time(first),
        opt_time(last)
    )?;
    writeln!(out, "ladder: {ladder}")?;
    writeln!(out)?;

    write_coverage(&mut out, report)?;
    write_events(&mut out, report)?;
    write_latest(&mut out, report)?;
    write_progression(&mut out, report)?;

    Ok(out)
}

fn write_coverage(out: &mut impl Write, report: &FlatReport) -> io::Result<()> {
    writeln!(out, "[coverage]")?;
    writeln!(
        out,
        "{:<10} {:>8} {:>9} {:>8} {:>7} {:>9} {:>8} {:>7} {:<21} last",
        "timeframe",
        "bars",
        "expected",
        "missing",
        "share",
        "s_expected",
        "s_missing",
        "s_share",
        "first"
    )?;
    for timeframe in &report.timeframes {
        writeln!(
            out,
            "{:<10} {:>8} {:>9} {:>8} {:>7} {:>9} {:>8} {:>7} {:<21} {}",
            labels::timeframe_label(timeframe.timeframe_secs),
            timeframe.bars,
            timeframe.expected_bars,
            timeframe.missing_bars,
            opt_share(timeframe.coverage_share),
            timeframe.session_expected_bars,
            timeframe.session_missing_bars,
            opt_share(timeframe.session_coverage_share),
            opt_time(timeframe.first_open_time),
            opt_time(timeframe.last_open_time)
        )?;
    }
    writeln!(out)
}

fn write_events(out: &mut impl Write, report: &FlatReport) -> io::Result<()> {
    writeln!(out, "[events]")?;
    writeln!(
        out,
        "{:<10} {:>6} {:>11} {:>7} {:>6} {:<20} {:<24} {:>8} {:>9}",
        "timeframe",
        "level",
        "divergence",
        "signal",
        "total",
        "latest_type",
        "latest_subtype",
        "occurred",
        "available"
    )?;
    for timeframe in &report.timeframes {
        let latest = latest_event_columns(&timeframe.events);
        writeln!(
            out,
            "{:<10} {:>6} {:>11} {:>7} {:>6} {:<20} {:<24} {:>8} {:>9}",
            labels::timeframe_label(timeframe.timeframe_secs),
            timeframe.events.by_type[0],
            timeframe.events.by_type[1],
            timeframe.events.by_type[2],
            timeframe.events.total,
            latest.0,
            latest.1,
            latest.2,
            latest.3
        )?;
    }
    writeln!(out)
}

fn latest_event_columns(events: &FlatEventSeries) -> (String, String, String, String) {
    let Some(latest) = events.recent.last() else {
        return (
            String::from("-"),
            String::from("-"),
            String::from("-"),
            String::from("-"),
        );
    };
    (
        labels::event_type_label(latest.event_type_code).to_owned(),
        labels::event_subtype_label(latest.event_type_code, latest.subtype_code).to_owned(),
        latest.occurred_bar_index.to_string(),
        latest.available_bar_index.to_string(),
    )
}

fn write_latest(out: &mut impl Write, report: &FlatReport) -> io::Result<()> {
    writeln!(out, "[latest]")?;
    writeln!(
        out,
        "{:<10} {:<21} {:>11} {:>11} {:>11} {:>9} {:>9} {:>9} {:>9} {:>9} {:>4} {:>4} {:>4} {:>5} {:>6} {:>5} {:>6} {:>7}",
        "timeframe",
        "open_time",
        "ema_fast",
        "ema_mid",
        "ema_slow",
        "rsi",
        "atr",
        "rvol",
        "adx",
        "di_plus",
        "stack",
        "zone",
        "macd",
        "adx_r",
        "adx_s",
        "vol",
        "struct",
        "context"
    )?;
    for timeframe in &report.timeframes {
        let Some(latest) = &timeframe.latest else {
            writeln!(
                out,
                "{:<10} {:<21} {:>11} {:>11} {:>11} {:>9} {:>9} {:>9} {:>9} {:>9} {:>4} {:>4} {:>4} {:>5} {:>6} {:>5} {:>6} {:>7}",
                labels::timeframe_label(timeframe.timeframe_secs),
                "-",
                "-",
                "-",
                "-",
                "-",
                "-",
                "-",
                "-",
                "-",
                "-",
                "-",
                "-",
                "-",
                "-",
                "-",
                "-",
                "-"
            )?;
            continue;
        };
        writeln!(
            out,
            "{:<10} {:<21} {:>11} {:>11} {:>11} {:>9} {:>9} {:>9} {:>9} {:>9} {:>4} {:>4} {:>4} {:>5} {:>6} {:>5} {:>6} {:>7}",
            labels::timeframe_label(timeframe.timeframe_secs),
            format_utc_timestamp(latest.open_time),
            opt_float(latest.readings.ema_fast),
            opt_float(latest.readings.ema_mid),
            opt_float(latest.readings.ema_slow),
            opt_float(latest.readings.rsi),
            opt_float(latest.readings.atr),
            opt_float(latest.readings.rvol),
            opt_float(latest.readings.adx),
            opt_float(latest.readings.di_plus),
            opt_code(latest.states.ema_stack),
            opt_code(latest.states.rsi_zone),
            opt_code(latest.states.macd_state),
            opt_code(latest.states.adx_regime),
            opt_code(latest.states.adx_slope),
            opt_code(latest.states.vol_regime),
            opt_code(latest.states.structure_state),
            latest.context.context_code.map_or_else(
                || "-".to_owned(),
                |code| labels::context_label(code).to_owned()
            ),
        )?;
    }
    writeln!(out)
}

fn write_progression(out: &mut impl Write, report: &FlatReport) -> io::Result<()> {
    writeln!(out, "[progression]")?;
    writeln!(
        out,
        "{:<10} {:<8} {:>9} {:>9} {:>10} {:>7} {:>9} {:>12} {:>12} {:>10}",
        "timeframe",
        "scale",
        "direction",
        "lifecycle",
        "stage",
        "target",
        "progress",
        "origin",
        "target_price",
        "basis"
    )?;
    for timeframe in &report.timeframes {
        let Some(leg) = timeframe
            .progression
            .as_ref()
            .and_then(|progression| progression.published)
        else {
            continue;
        };
        writeln!(
            out,
            "{:<10} {:<8} {:>9} {:>9} {:>10} {:>7} {:>9} {:>12} {:>12} {:>10}",
            labels::timeframe_label(timeframe.timeframe_secs),
            labels::scale_label(leg.scale_code),
            labels::leg_direction_label(leg.direction_code),
            labels::lifecycle_label(leg.lifecycle_code),
            leg.stage_code.map_or_else(
                || "-".to_owned(),
                |code| labels::stage_label(code).to_owned()
            ),
            code_or_dash(leg.target_reached_code),
            opt_float(leg.progress),
            leg.origin_price.to_string(),
            opt_float(leg.target_price),
            leg.target_basis_code.map_or_else(
                || "-".to_owned(),
                |code| labels::target_basis_label(code).to_owned()
            ),
        )?;
    }
    writeln!(out)
}

fn opt_time(value: Option<i64>) -> String {
    value.map_or_else(|| String::from("-"), format_utc_timestamp)
}

fn opt_share(value: Option<f64>) -> String {
    value.map_or_else(|| String::from("-"), |value| format!("{value:.4}"))
}

fn opt_float(value: Option<f64>) -> String {
    value.map_or_else(|| String::from("-"), |value| format!("{value:.6}"))
}

fn opt_code(value: Option<i8>) -> String {
    value.map_or_else(|| String::from("-"), |value| value.to_string())
}

fn code_or_dash(value: Option<u8>) -> String {
    value.map_or_else(|| String::from("-"), |value| value.to_string())
}
