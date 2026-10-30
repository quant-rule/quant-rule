//! The whole-lab summary writer.
//!
//! `lab summary` reads the five report bundles a `backtest` wrote and writes one
//! `summary.txt` for the lab result folder. It copies published values from the
//! report JSON and never recomputes the model. Every input is explicit,
//! including `created_utc`, so the same lab and the same arguments produce the
//! same bytes.

use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use quantrule_data::format_utc_timestamp;
use serde_json::Value;

use crate::command::LabSummaryArgs;
use crate::export::{ExportError, write_atomic};
use crate::labels;

/// The five report bundles, in contract order.
const BUNDLES: [&str; 5] = ["model", "strategy", "execution", "portfolio", "analytical"];

/// One bundle read from a lab result folder.
struct Bundle {
    /// The bundle name, such as `portfolio`.
    name: &'static str,
    /// The parsed `report.json`.
    report: Value,
    /// The parsed `provenance.json`, when present.
    provenance: Option<Value>,
}

/// Reads the five bundles and writes `<lab-dir>/summary.txt`.
///
/// # Errors
///
/// Returns a [`LabSummaryError`] when the lab folder is missing, a bundle is
/// missing or unreadable, a report does not match `--data-version`, the notes
/// file is not UTF-8, an existing summary is not overwritten with `--force`, or
/// the summary cannot be written.
pub(crate) fn write(args: &LabSummaryArgs) -> Result<PathBuf, LabSummaryError> {
    if !args.lab_dir.is_dir() {
        return Err(LabSummaryError::NotDirectory {
            path: args.lab_dir.clone(),
        });
    }

    let summary_path = args.lab_dir.join("summary.txt");
    if summary_path.exists() && !args.force {
        return Err(LabSummaryError::SummaryExists { path: summary_path });
    }

    let bundles = read_bundles(args)?;
    let notes = match &args.notes {
        Some(path) => Some(read_notes(path)?),
        None => None,
    };
    let text = render(args, &bundles, notes.as_deref())?;

    write_atomic(&summary_path, &text).map_err(|source| LabSummaryError::Write {
        path: summary_path.clone(),
        source,
    })?;
    Ok(summary_path)
}

fn read_bundles(args: &LabSummaryArgs) -> Result<Vec<Bundle>, LabSummaryError> {
    let mut bundles = Vec::with_capacity(BUNDLES.len());
    for name in BUNDLES {
        let directory = args.lab_dir.join(name);
        let report_path = directory.join("report.json");
        if !report_path.is_file() {
            return Err(LabSummaryError::MissingBundle { path: report_path });
        }
        let report = read_json(&report_path)?;
        check_data_version(&report_path, &report, &args.data_version)?;

        let provenance_path = directory.join("provenance.json");
        let provenance = if provenance_path.is_file() {
            let provenance = read_json(&provenance_path)?;
            check_data_version(&provenance_path, &provenance, &args.data_version)?;
            Some(provenance)
        } else {
            None
        };

        bundles.push(Bundle {
            name,
            report,
            provenance,
        });
    }
    Ok(bundles)
}

fn read_json(path: &Path) -> Result<Value, LabSummaryError> {
    let bytes = fs::read(path).map_err(|source| LabSummaryError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    serde_json::from_slice(&bytes).map_err(|source| LabSummaryError::Parse {
        path: path.to_path_buf(),
        source,
    })
}

fn check_data_version(path: &Path, value: &Value, expected: &str) -> Result<(), LabSummaryError> {
    let actual = string_at(value, path, "data_version")?;
    if actual != expected {
        return Err(LabSummaryError::DataVersionMismatch {
            path: path.to_path_buf(),
            expected: expected.to_owned(),
            actual: actual.to_owned(),
        });
    }
    Ok(())
}

fn read_notes(path: &Path) -> Result<String, LabSummaryError> {
    let bytes = fs::read(path).map_err(|source| LabSummaryError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    String::from_utf8(bytes).map_err(|_| LabSummaryError::NotesNotUtf8 {
        path: path.to_path_buf(),
    })
}

fn render(
    args: &LabSummaryArgs,
    bundles: &[Bundle],
    notes: Option<&str>,
) -> Result<Vec<u8>, LabSummaryError> {
    let portfolio = bundle(bundles, "portfolio")?;
    let execution = bundle(bundles, "execution")?;
    let strategy = bundle(bundles, "strategy")?;
    let analytical = bundle(bundles, "analytical")?;
    let model = bundle(bundles, "model")?;

    let mut out = Vec::new();
    writeln!(out, "QuantRule lab result")?;
    writeln!(out, "====================")?;
    writeln!(out)?;

    writeln!(out, "Identity")?;
    writeln!(out, "  created_utc: {}", args.created_utc)?;
    writeln!(out, "  role: {}", args.role.label())?;
    writeln!(out, "  operator: {}", args.operator.code())?;
    writeln!(out, "  data_version: {}", args.data_version)?;
    writeln!(out, "  config: {}", args.config.display())?;
    writeln!(out)?;

    write_artifacts(&mut out, bundles)?;
    write_observed(&mut out, portfolio, execution, strategy, analytical, model)?;
    write_checks(&mut out, bundles)?;
    write_notes(&mut out, notes)?;

    writeln!(out, "Limitations")?;
    writeln!(
        out,
        "  - In-sample only unless an out-of-sample run is recorded in the role notes."
    )?;
    writeln!(
        out,
        "  - Costs are the configured assumptions; no live fills."
    )?;
    writeln!(out, "  - No profitability claim.")?;
    writeln!(out)?;

    writeln!(out, "Next")?;
    writeln!(out, "  {}", args.role.handoff())?;

    Ok(out)
}

fn bundle<'a>(bundles: &'a [Bundle], name: &str) -> Result<&'a Bundle, LabSummaryError> {
    bundles
        .iter()
        .find(|bundle| bundle.name == name)
        .ok_or_else(|| LabSummaryError::MissingBundle {
            path: PathBuf::from(name),
        })
}

fn write_artifacts(out: &mut Vec<u8>, bundles: &[Bundle]) -> Result<(), LabSummaryError> {
    writeln!(out, "Artifacts")?;
    for bundle in bundles {
        writeln!(
            out,
            "  {:<30} present",
            format!("{}/report.json", bundle.name)
        )?;
        let provenance = if bundle.provenance.is_some() {
            "present"
        } else {
            "missing"
        };
        writeln!(
            out,
            "  {:<30} {provenance}",
            format!("{}/provenance.json", bundle.name)
        )?;
    }
    writeln!(out)?;
    Ok(())
}

fn write_observed(
    out: &mut Vec<u8>,
    portfolio: &Bundle,
    execution: &Bundle,
    strategy: &Bundle,
    analytical: &Bundle,
    model: &Bundle,
) -> Result<(), LabSummaryError> {
    let portfolio_path = Path::new("portfolio/report.json");
    let execution_path = Path::new("execution/report.json");
    let strategy_path = Path::new("strategy/report.json");
    let analytical_path = Path::new("analytical/report.json");
    let model_path = Path::new("model/report.json");

    let requested_period = requested_period(&portfolio.report, portfolio_path)?;
    let actual_period = actual_period(&portfolio.report, portfolio_path)?;
    let final_equity = f64_at(&portfolio.report, portfolio_path, "summary.final_equity")?;
    let total_return = f64_at(&portfolio.report, portfolio_path, "summary.total_return")?;
    let net_pnl = f64_at(&portfolio.report, portfolio_path, "summary.net_pnl")?;
    let max_drawdown = f64_at(&portfolio.report, portfolio_path, "summary.max_drawdown")?;
    let trades = u64_at(&portfolio.report, portfolio_path, "summary.trades")?;
    let halted = bool_at(&portfolio.report, portfolio_path, "summary.halted")?;
    let commission = f64_at(&portfolio.report, portfolio_path, "summary.commission")?;
    let funding_paid = f64_at(&portfolio.report, portfolio_path, "summary.funding_paid")?;
    let funding_received = f64_at(
        &portfolio.report,
        portfolio_path,
        "summary.funding_received",
    )?;

    let execution_fees = f64_at(&execution.report, execution_path, "summary.fees.total")?;
    let slippage = f64_at(&execution.report, execution_path, "summary.slippage.total")?;
    let strategy_entries = u64_at(&strategy.report, strategy_path, "summary.entries")?;
    let strategy_exits = u64_at(&strategy.report, strategy_path, "summary.exits")?;
    let layer3_rows = u64_at(&analytical.report, analytical_path, "summary.layer3_rows")?;

    let ladder = array_at(&model.report, model_path, "market_report.ladder")?;
    let ladder_labels = ladder
        .iter()
        .map(|value| {
            value
                .as_u64()
                .map(labels::timeframe_label)
                .ok_or_else(|| invalid(model_path, "market_report.ladder", "expected integers"))
        })
        .collect::<Result<Vec<_>, _>>()?
        .join(" ");

    let level = u64_at(
        &model.report,
        model_path,
        "market_report.events.by_type.level",
    )?;
    let divergence = u64_at(
        &model.report,
        model_path,
        "market_report.events.by_type.divergence",
    )?;
    let signal = u64_at(
        &model.report,
        model_path,
        "market_report.events.by_type.signal",
    )?;
    let event_total = level
        .checked_add(divergence)
        .and_then(|total| total.checked_add(signal))
        .ok_or_else(|| {
            invalid(
                model_path,
                "market_report.events.by_type",
                "event count overflow",
            )
        })?;

    writeln!(out, "Observed result")?;
    writeln!(out, "  requested_period: {requested_period}")?;
    writeln!(out, "  actual_period: {actual_period}")?;
    writeln!(out, "  final_equity: {:.6}", number(final_equity))?;
    writeln!(out, "  total_return: {:.6}", number(total_return))?;
    writeln!(out, "  net_pnl: {:.6}", number(net_pnl))?;
    writeln!(out, "  max_drawdown: {:.6}", number(max_drawdown))?;
    writeln!(out, "  trades: {trades}")?;
    writeln!(out, "  halted: {halted}")?;
    writeln!(out, "  commission: {:.6}", number(commission))?;
    writeln!(out, "  execution_fees: {:.6}", number(execution_fees))?;
    writeln!(out, "  slippage: {:.6}", number(slippage))?;
    writeln!(out, "  funding_paid: {:.6}", number(funding_paid))?;
    writeln!(out, "  funding_received: {:.6}", number(funding_received))?;
    writeln!(out, "  strategy_entries: {strategy_entries}")?;
    writeln!(out, "  strategy_exits: {strategy_exits}")?;
    writeln!(out, "  analytical_layer3_rows: {layer3_rows}")?;
    writeln!(out, "  model_ladder: {ladder_labels}")?;
    writeln!(
        out,
        "  model_events: level={level} divergence={divergence} signal={signal} total={event_total}"
    )?;
    writeln!(out)?;
    Ok(())
}

fn write_checks(out: &mut Vec<u8>, bundles: &[Bundle]) -> Result<(), LabSummaryError> {
    let provenance_count = bundles
        .iter()
        .filter(|bundle| bundle.provenance.is_some())
        .count();
    let version_count = bundles
        .iter()
        .filter(|bundle| {
            bundle.report.get("report_version").is_some()
                || bundle.report.get("view_contract").is_some()
        })
        .count();

    writeln!(out, "Checks")?;
    writeln!(out, "  [x] five report bundles present")?;
    writeln!(
        out,
        "  [{}] provenance.json present ({provenance_count}/5)",
        check_mark(provenance_count == 5)
    )?;
    writeln!(out, "  [x] data version matches report.json")?;
    writeln!(
        out,
        "  [{}] data version matches provenance.json ({provenance_count}/5)",
        check_mark(provenance_count == 5)
    )?;
    writeln!(
        out,
        "  [{}] report versions present ({version_count}/5)",
        check_mark(version_count == 5)
    )?;
    writeln!(out, "  [ ] out-of-sample evidence")?;
    writeln!(out, "  [ ] independent rerun")?;
    writeln!(out)?;
    Ok(())
}

fn write_notes(out: &mut Vec<u8>, notes: Option<&str>) -> Result<(), LabSummaryError> {
    writeln!(out, "Role notes")?;
    match notes {
        None => writeln!(out, "  (none)")?,
        Some("") => writeln!(out, "  (empty)")?,
        Some(text) => {
            for line in text.lines() {
                if line.is_empty() {
                    writeln!(out)?;
                } else {
                    writeln!(out, "  {line}")?;
                }
            }
        }
    }
    writeln!(out)?;
    Ok(())
}

fn requested_period(value: &Value, path: &Path) -> Result<String, LabSummaryError> {
    let from = optional_string_at(value, path, "requested_period.from")?;
    let to = optional_string_at(value, path, "requested_period.to")?;
    Ok(match (from, to) {
        (Some(from), Some(to)) => format!("{from}..{to}"),
        (Some(from), None) => format!("{from}..open"),
        (None, Some(to)) => format!("open..{to}"),
        (None, None) => String::from("full file"),
    })
}

fn actual_period(value: &Value, path: &Path) -> Result<String, LabSummaryError> {
    let first = optional_i64_at(value, path, "actual_period.first_open_time")?;
    let last = optional_i64_at(value, path, "actual_period.last_open_time")?;
    Ok(format!("{}..{}", opt_time(first), opt_time(last)))
}

fn opt_time(value: Option<i64>) -> String {
    value.map_or_else(|| String::from("-"), format_utc_timestamp)
}

fn number(value: f64) -> f64 {
    if value == 0.0 { 0.0 } else { value }
}

const fn check_mark(present: bool) -> &'static str {
    if present { "x" } else { " " }
}

fn get<'a>(value: &'a Value, path: &Path, field: &str) -> Result<&'a Value, LabSummaryError> {
    let mut current = value;
    for part in field.split('.') {
        current = current
            .get(part)
            .ok_or_else(|| invalid(path, field, "missing"))?;
    }
    Ok(current)
}

fn string_at<'a>(value: &'a Value, path: &Path, field: &str) -> Result<&'a str, LabSummaryError> {
    get(value, path, field)?
        .as_str()
        .ok_or_else(|| invalid(path, field, "expected a string"))
}

fn optional_string_at<'a>(
    value: &'a Value,
    path: &Path,
    field: &str,
) -> Result<Option<&'a str>, LabSummaryError> {
    match get(value, path, field)? {
        Value::Null => Ok(None),
        Value::String(text) => Ok(Some(text)),
        _ => Err(invalid(path, field, "expected a string or null")),
    }
}

fn f64_at(value: &Value, path: &Path, field: &str) -> Result<f64, LabSummaryError> {
    get(value, path, field)?
        .as_f64()
        .ok_or_else(|| invalid(path, field, "expected a number"))
}

fn u64_at(value: &Value, path: &Path, field: &str) -> Result<u64, LabSummaryError> {
    get(value, path, field)?
        .as_u64()
        .ok_or_else(|| invalid(path, field, "expected a non-negative integer"))
}

fn optional_i64_at(
    value: &Value,
    path: &Path,
    field: &str,
) -> Result<Option<i64>, LabSummaryError> {
    match get(value, path, field)? {
        Value::Null => Ok(None),
        value => value
            .as_i64()
            .map(Some)
            .ok_or_else(|| invalid(path, field, "expected an integer or null")),
    }
}

fn bool_at(value: &Value, path: &Path, field: &str) -> Result<bool, LabSummaryError> {
    get(value, path, field)?
        .as_bool()
        .ok_or_else(|| invalid(path, field, "expected a boolean"))
}

fn array_at<'a>(
    value: &'a Value,
    path: &Path,
    field: &str,
) -> Result<&'a Vec<Value>, LabSummaryError> {
    get(value, path, field)?
        .as_array()
        .ok_or_else(|| invalid(path, field, "expected an array"))
}

fn invalid(path: &Path, field: &str, message: &str) -> LabSummaryError {
    LabSummaryError::InvalidReport {
        path: path.to_path_buf(),
        field: field.to_owned(),
        message: message.to_owned(),
    }
}

/// A lab summary could not be read or written.
#[derive(Debug)]
pub(crate) enum LabSummaryError {
    /// The lab result folder is missing or is not a directory.
    NotDirectory {
        /// The supplied path.
        path: PathBuf,
    },
    /// A bundle's `report.json` is missing.
    MissingBundle {
        /// The missing report path.
        path: PathBuf,
    },
    /// A report or notes file could not be read.
    Read {
        /// The file path.
        path: PathBuf,
        /// The underlying I/O error.
        source: io::Error,
    },
    /// A report file is not valid JSON.
    Parse {
        /// The file path.
        path: PathBuf,
        /// The JSON error.
        source: serde_json::Error,
    },
    /// A report does not carry a field the summary needs.
    InvalidReport {
        /// The report path.
        path: PathBuf,
        /// The JSON field path.
        field: String,
        /// What was wrong.
        message: String,
    },
    /// A bundle's data version differs from `--data-version`.
    DataVersionMismatch {
        /// The report or provenance path.
        path: PathBuf,
        /// The supplied data version.
        expected: String,
        /// The version found in the file.
        actual: String,
    },
    /// A summary already exists and `--force` was not supplied.
    SummaryExists {
        /// The existing summary path.
        path: PathBuf,
    },
    /// The notes file is not valid UTF-8.
    NotesNotUtf8 {
        /// The notes path.
        path: PathBuf,
    },
    /// The summary could not be rendered.
    Render {
        /// The underlying I/O error.
        source: io::Error,
    },
    /// The summary file could not be written.
    Write {
        /// The summary path.
        path: PathBuf,
        /// The underlying export error.
        source: ExportError,
    },
}

impl fmt::Display for LabSummaryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotDirectory { path } => {
                write!(
                    formatter,
                    "lab result folder {} is not a directory",
                    path.display()
                )
            }
            Self::MissingBundle { path } => {
                write!(formatter, "missing report bundle: {}", path.display())
            }
            Self::Read { path, source } => {
                write!(formatter, "cannot read {}: {source}", path.display())
            }
            Self::Parse { path, source } => {
                write!(formatter, "cannot parse {}: {source}", path.display())
            }
            Self::InvalidReport {
                path,
                field,
                message,
            } => write!(formatter, "{}: field {field}: {message}", path.display()),
            Self::DataVersionMismatch {
                path,
                expected,
                actual,
            } => write!(
                formatter,
                "{}: data version mismatch: expected {expected}, found {actual}",
                path.display()
            ),
            Self::SummaryExists { path } => write!(
                formatter,
                "{} already exists; use --force to overwrite it",
                path.display()
            ),
            Self::NotesNotUtf8 { path } => {
                write!(
                    formatter,
                    "notes file {} is not valid UTF-8",
                    path.display()
                )
            }
            Self::Render { source } => {
                write!(formatter, "cannot render the lab summary: {source}")
            }
            Self::Write { path, source } => {
                write!(formatter, "cannot write {}: {source}", path.display())
            }
        }
    }
}

impl std::error::Error for LabSummaryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read { source, .. } => Some(source),
            Self::Parse { source, .. } => Some(source),
            Self::Render { source } => Some(source),
            Self::Write { source, .. } => Some(source),
            Self::NotDirectory { .. }
            | Self::MissingBundle { .. }
            | Self::InvalidReport { .. }
            | Self::DataVersionMismatch { .. }
            | Self::SummaryExists { .. }
            | Self::NotesNotUtf8 { .. } => None,
        }
    }
}

impl From<io::Error> for LabSummaryError {
    fn from(source: io::Error) -> Self {
        Self::Render { source }
    }
}
