//! Rendering for the full-period market report.
//!
//! The shell copies the numeric view into `summary.txt` and `report.json`; it
//! computes no statistic. The text maps stable codes to the shell's fixed
//! labels, and the JSON carries codes and numbers only.

mod bundle;
mod json;
mod provenance;

pub(crate) use bundle::{BundleError, ensure_available, write_bundle};
pub(crate) use json::json_bytes;
pub(crate) use provenance::{Provenance, REPORT_VERSION, bytes as provenance_bytes};

use std::io::{self, Write};

use quantrule_data::format_utc_timestamp;
use quantrule_model::report::{AxisReport, LatestState, MarketReport, TimeframeReport};

use crate::labels;

/// The report view-contract version carried by the JSON envelope.
pub(crate) const VIEW_CONTRACT: &str = "2026-10-27";

/// The four artifact names, in the contract's fixed order.
const ARTIFACTS: [&str; 4] = ["summary.txt", "report.json", "price-axes.png", "events.png"];

/// Renders `summary.txt`.
///
/// The sections and their order are fixed by the report contract. Every value
/// comes from [`MarketReport`]; the shell only formats it.
///
/// # Errors
///
/// Returns the writer's error when a line cannot be written.
pub(crate) fn summary_bytes(
    report: &MarketReport,
    data_version: &str,
    requested_period: &str,
) -> io::Result<Vec<u8>> {
    let mut out = Vec::new();
    writeln!(out, "QuantRule market report")?;
    writeln!(out, "data_version: {data_version}")?;
    writeln!(out, "requested_period: {requested_period}")?;
    writeln!(
        out,
        "actual_period: {}..{}",
        opt_time(report.period.first_open_time),
        opt_time(report.period.last_open_time)
    )?;
    writeln!(out, "ladder: {}", ladder_labels(&report.ladder))?;
    writeln!(out)?;

    write_coverage(&mut out, report)?;
    write_axes(&mut out, report)?;
    write_histograms(&mut out, report)?;
    write_events(&mut out, report)?;
    write_latest(&mut out, report)?;
    write_artifacts(&mut out)?;

    Ok(out)
}

fn ladder_labels(ladder: &[u64]) -> String {
    if ladder.is_empty() {
        return String::from("-");
    }
    ladder
        .iter()
        .map(|seconds| labels::timeframe_label(*seconds))
        .collect::<Vec<_>>()
        .join(" ")
}

fn write_coverage(out: &mut impl Write, report: &MarketReport) -> io::Result<()> {
    writeln!(out, "[coverage]")?;
    writeln!(
        out,
        "{:<10} {:>8} {:>9} {:>8} {:>7} {:<21} last",
        "timeframe", "bars", "expected", "missing", "share", "first"
    )?;
    for timeframe in &report.timeframes {
        writeln!(
            out,
            "{:<10} {:>8} {:>9} {:>8} {:>7} {:<21} {}",
            labels::timeframe_label(timeframe.timeframe_secs),
            timeframe.bars,
            timeframe.expected_bars,
            timeframe.missing_bars,
            opt_share(timeframe.coverage_share),
            opt_time(timeframe.first_open_time),
            opt_time(timeframe.last_open_time)
        )?;
    }
    writeln!(out)
}

fn write_axes(out: &mut impl Write, report: &MarketReport) -> io::Result<()> {
    writeln!(out, "[axes]")?;
    writeln!(
        out,
        "{:<10} {:<12} {:>8} {:>6} {:>11} {:>11} {:>11}",
        "timeframe", "axis", "present", "null", "min", "max", "mean"
    )?;
    for timeframe in &report.timeframes {
        for axis in &timeframe.axes {
            writeln!(
                out,
                "{:<10} {:<12} {:>8} {:>6} {:>11} {:>11} {:>11}",
                labels::timeframe_label(timeframe.timeframe_secs),
                labels::axis_label(axis.axis.code()),
                axis.present_count,
                axis.null_count,
                opt_float(axis.min),
                opt_float(axis.max),
                opt_float(axis.mean)
            )?;
        }
    }
    writeln!(out)
}

fn write_histograms(out: &mut impl Write, report: &MarketReport) -> io::Result<()> {
    writeln!(out, "[histograms]")?;
    writeln!(
        out,
        "{:<10} {:<12} {:<34} {:>6}",
        "timeframe", "axis", "codes", "null"
    )?;
    for timeframe in &report.timeframes {
        for axis in &timeframe.axes {
            writeln!(
                out,
                "{:<10} {:<12} {:<34} {:>6}",
                labels::timeframe_label(timeframe.timeframe_secs),
                labels::axis_label(axis.axis.code()),
                histogram_codes(axis),
                axis.code_null_count
            )?;
        }
    }
    writeln!(out)
}

fn histogram_codes(axis: &AxisReport) -> String {
    let codes: &[i8] = match axis.axis.code() {
        1 | 2 => &[-1, 0, 1],
        3 => &[-2, -1, 0, 1, 2],
        4 => &[0, 1, 2, 3, 4, 5],
        _ => &[],
    };
    codes
        .iter()
        .zip(&axis.code_counts)
        .map(|(code, count)| format!("{code}:{count}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn write_events(out: &mut impl Write, report: &MarketReport) -> io::Result<()> {
    writeln!(out, "[events]")?;
    writeln!(
        out,
        "{:<10} {:>6} {:>11} {:>7} {:>6} {:<20} {:<20} {:>9} {:>10}",
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
    for timeframe in &report.events.timeframes {
        let latest = latest_event_columns(timeframe);
        writeln!(
            out,
            "{:<10} {:>6} {:>11} {:>7} {:>6} {:<20} {:<20} {:>9} {:>10}",
            labels::timeframe_label(timeframe.timeframe_secs),
            timeframe.by_type.level,
            timeframe.by_type.divergence,
            timeframe.by_type.signal,
            timeframe.total,
            latest.0,
            latest.1,
            latest.2,
            latest.3
        )?;
    }
    writeln!(out)
}

fn latest_event_columns(
    timeframe: &quantrule_model::report::TimeframeEventReport,
) -> (String, String, String, String) {
    let Some(latest) = &timeframe.latest else {
        return (
            String::from("-"),
            String::from("-"),
            String::from("-"),
            String::from("-"),
        );
    };
    (
        labels::event_type_label(latest.event_type_code).to_owned(),
        labels::event_subtype_label(latest.event_type_code, latest.event_subtype_code).to_owned(),
        latest.occurred_bar_index.to_string(),
        latest.available_bar_index.to_string(),
    )
}

fn write_latest(out: &mut impl Write, report: &MarketReport) -> io::Result<()> {
    writeln!(out, "[latest]")?;
    writeln!(
        out,
        "{:<10} {:<21} {:>10} {:>9} {:>10} {:>9} {:>10} {:>9} {:>11} {:>10} {:>7} {:>7} {:>7} {:>7}",
        "timeframe",
        "open_time",
        "direction",
        "dir_ready",
        "momentum",
        "mom_ready",
        "structure",
        "str_ready",
        "progression",
        "prog_ready",
        "context",
        "bullish",
        "bearish",
        "neutral"
    )?;
    for timeframe in &report.timeframes {
        write_latest_row(out, timeframe)?;
    }
    writeln!(out)
}

fn write_latest_row(out: &mut impl Write, timeframe: &TimeframeReport) -> io::Result<()> {
    let Some(latest) = &timeframe.latest else {
        return writeln!(
            out,
            "{:<10} {:<21} {:>10} {:>9} {:>10} {:>9} {:>10} {:>9} {:>11} {:>10} {:>7} {:>7} {:>7} {:>7}",
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
            "-"
        );
    };

    let LatestState {
        open_time,
        direction,
        momentum,
        structure,
        progression,
        context,
        consensus,
        ..
    } = latest;
    writeln!(
        out,
        "{:<10} {:<21} {:>10} {:>9} {:>10} {:>9} {:>10} {:>9} {:>11} {:>10} {:>7} {:>7} {:>7} {:>7}",
        labels::timeframe_label(timeframe.timeframe_secs),
        format_utc_timestamp(*open_time),
        opt_float(direction.direction),
        direction.readiness_code,
        opt_float(momentum.force),
        momentum.readiness_code,
        opt_float(structure.score),
        structure.readiness_code,
        opt_float(progression.progress),
        progression.readiness_code,
        opt_u8(context.context_code),
        consensus.bullish_axis_count,
        consensus.bearish_axis_count,
        consensus.neutral_axis_count
    )
}

fn write_artifacts(out: &mut impl Write) -> io::Result<()> {
    writeln!(out, "[artifacts]")?;
    for name in ARTIFACTS {
        writeln!(out, "{name}")?;
    }
    Ok(())
}

fn opt_float(value: Option<f64>) -> String {
    value.map_or_else(|| String::from("-"), |value| format!("{value:.6}"))
}

fn opt_share(value: Option<f64>) -> String {
    value.map_or_else(|| String::from("-"), |value| format!("{value:.4}"))
}

fn opt_time(value: Option<i64>) -> String {
    value.map_or_else(|| String::from("-"), format_utc_timestamp)
}

fn opt_u8(value: Option<u8>) -> String {
    value.map_or_else(|| String::from("-"), |value| value.to_string())
}

#[cfg(test)]
mod tests {
    use quantrule_model::input::MarketStateConfig;
    use quantrule_model::report::MarketReport;
    use quantrule_model::run::ModelRun;

    use super::summary_bytes;

    #[test]
    fn an_empty_report_still_renders_every_section() {
        let run = ModelRun::compute(&[], &MarketStateConfig::default());
        let report = MarketReport::compute(&run, &[60, 300, 900, 3_600]);

        let bytes = summary_bytes(&report, "v1", "full file").expect("the summary renders");
        let text = String::from_utf8(bytes).expect("the summary is UTF-8");

        assert!(text.starts_with("QuantRule market report\n"));
        assert!(text.contains("actual_period: -..-"));
        assert!(text.contains("[coverage]\n"));
        assert!(text.contains("[axes]\n"));
        assert!(text.contains("[histograms]\n"));
        assert!(text.contains("[events]\n"));
        assert!(text.contains("[latest]\n"));
        assert!(
            text.contains("[artifacts]\nsummary.txt\nreport.json\nprice-axes.png\nevents.png\n")
        );
        assert!(text.ends_with('\n'));
    }
}
