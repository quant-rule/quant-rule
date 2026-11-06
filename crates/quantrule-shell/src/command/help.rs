//! The shell command reference.
//!
//! `help`, `help <command>`, and `help all` all render from [`COMMANDS`], so the
//! short map, the per-command contract, and the complete reference cannot drift.
//! The glossary is static text. Nothing here reads a file, the clock, or the
//! terminal width, and the output has no color.

mod topics;

use std::io::Write;

use topics::render_glossary;

use super::types::{HelpTopic, RunError};

/// One command contract.
struct CommandHelp {
    /// The command name, including a two-word family prefix when needed.
    name: &'static str,
    /// The full usage line.
    usage: &'static str,
    /// One line describing the command.
    summary: &'static str,
    /// The shape of the command's output.
    output: &'static str,
    /// Short contract notes: caps, errors, and boundaries.
    notes: &'static [&'static str],
}

/// The surviving command set.
const COMMANDS: &[CommandHelp] = &[
    CommandHelp {
        name: "candles",
        usage: "candles <csv-path>",
        summary: "Read one M1 CSV strictly and print its retained candles as NDJSON.",
        output: "one JSON object per candle: series_index, window_index, datetime, open, high, low, close, volume",
        notes: &[
            "The strict read rejects a malformed row.",
            "The window is the data engine's bounded retained tail.",
        ],
    },
    CommandHelp {
        name: "candles-tolerant",
        usage: "candles-tolerant <csv-path>",
        summary: "Read one M1 CSV tolerantly and print the retained candles plus a quality report.",
        output: "NDJSON candles, then rejected counts and gap records",
        notes: &["A malformed row is counted and skipped, not accepted."],
    },
    CommandHelp {
        name: "timeframes",
        usage: "timeframes <csv-path>",
        summary: "Build every supported timeframe from one M1 CSV and print every retained series.",
        output: "NDJSON candles per timeframe, then partial-bucket and gap records",
        notes: &[
            "The data engine owns aggregation, ordering, and gap detection.",
            "The supported data timeframes are 1m, 5m, 15m, 1h, and 4h; the model ceiling is 1h.",
        ],
    },
    CommandHelp {
        name: "events",
        usage: "events <csv-path> [--window full|latest]",
        summary: "Print the flat model's registered and extended event records.",
        output: "one JSON object per event, grouped by timeframe and availability order",
        notes: &[
            "The default model configuration is used.",
            "The model runs over the default 1m/5m/15m/1h ladder.",
        ],
    },
    CommandHelp {
        name: "report model",
        usage: "report model --config <run.json> [--spec <spec.json>] [--out <dir>] [--json]",
        summary: "Write the flat per-timeframe model bundle for one configured run.",
        output: "the artifact paths, or report.json with --json",
        notes: &[
            "The bundle is written under <out>/model and is refused if the directory is not empty.",
            "The bundle carries readings, states, levels, events, progression, and context; it carries no axis or cross-timeframe aggregate.",
            "A spec adds conditions.csv; a v2 spec can also add prediction.csv, panel.csv, and session.csv.",
        ],
    },
    CommandHelp {
        name: "policy check",
        usage: "policy check --config <run.json>",
        summary: "Validate the policy and every engine config against it.",
        output: "one `policy ok ...` line",
        notes: &[
            "The loader rejects a missing, malformed, or inconsistent config before any candle is read.",
        ],
    },
    CommandHelp {
        name: "strategy explain",
        usage: "strategy explain --model <model-bundle> --strategy <strategy.json> --bar <index> [--position flat|long|short]",
        summary: "Print one strategy decision and its as-of evidence from a model bundle.",
        output: "one JSON decision record with bar identity, position, intent, ATR, and evidence",
        notes: &[
            "The model bundle is the report model output directory.",
            "The position argument is the strategy's own state, not a market reading.",
            "The command supports the event-composite and compression-bracket engines.",
        ],
    },
    CommandHelp {
        name: "strategy run",
        usage: "strategy run --model <model-bundle> --strategy <strategy.json> [--out <dir>] [--json]",
        summary: "Write the strategy decisions bundle for one model bundle.",
        output: "the artifact paths, or a JSON envelope; <out>/strategy/ holds decisions.ndjson, summary.txt, and provenance.json",
        notes: &[
            "The standalone sequence supports the event-composite engine.",
            "A compression-bracket strategy needs the fill side from execution; use strategy explain for one decision.",
            "If --out is omitted, the bundle is written beside the model bundle.",
            "The bundle carries no fill, quantity, fee, cash, equity, or PnL.",
        ],
    },
    CommandHelp {
        name: "execution run",
        usage: "execution run --decisions <strategy-bundle> --config <run.json> [--out <dir>] [--json]",
        summary: "Turn a strategy decisions bundle into orders, fills, and rejections.",
        output: "the artifact paths, or a JSON envelope; <out>/execution/ holds orders.csv, fills.csv, rejections.csv, summary.txt, and provenance.json",
        notes: &[
            "The decisions bundle is the strategy run output directory.",
            "The run config supplies the fill timeframe, costs, and position sizing.",
            "If --out is omitted, the bundle is written beside the decisions bundle.",
            "A standalone run sizes entries from the run's initial cash; it does not track equity.",
            "The fill records carry no cash, equity, or PnL.",
        ],
    },
    CommandHelp {
        name: "portfolio replay",
        usage: "portfolio replay --fills <execution-bundle> --config <run.json> [--out <dir>] [--json]",
        summary: "Replay an execution fills bundle through portfolio accounting.",
        output: "the artifact paths, or a JSON envelope; <out>/portfolio/ holds trades.csv, equity.csv, funding.csv, risk_events.csv, metrics.json, summary.txt, and provenance.json",
        notes: &[
            "The fills bundle is the execution run output directory.",
            "The run config supplies the portfolio capital, sizing, costs, and risk rules.",
            "If --out is omitted, the bundle is written beside the fills bundle.",
            "The command marks the open position at each execution-timeframe close.",
            "Undefined metrics are null, never zero.",
        ],
    },
    CommandHelp {
        name: "backtest run",
        usage: "backtest run --config <run.json> --strategy <strategy.json> [--out <dir>] [--json]",
        summary: "Run the coupled model, strategy, execution, and portfolio stages.",
        output: "the manifest path and the stage bundle paths, or a JSON envelope; <out>/backtest/ holds model, strategy, execution, portfolio, manifest.json, and provenance.json",
        notes: &[
            "The strategy sees the position execution actually holds at each bar.",
            "The stage bundles are the same artifacts the standalone commands write.",
            "If --out is omitted, the run configuration's output_root is used.",
            "The manifest records every stage provenance and its content hash.",
        ],
    },
    CommandHelp {
        name: "fetch",
        usage: "fetch <symbol> <start> <end> [--timeframe <1m|5m|15m|1h>] [--csv <path>]",
        summary: "Fetch a Dukascopy range and write the accepted candles as CSV.",
        output: "rows, accepted/rejected counts, first/last timestamps, and the CSV path",
        notes: &[
            "The default path is data/sources/<SYMBOL>_<TF>_<start>_<end>.csv.",
            "The CSV is written atomically; an empty result leaves no file.",
        ],
    },
    CommandHelp {
        name: "help",
        usage: "help [<command>|all|glossary]",
        summary: "Print the command map, one command contract, or the glossary.",
        output: "plain text",
        notes: &[],
    },
];

/// Whether `name` names a documented command.
pub(crate) fn is_command(name: &str) -> bool {
    COMMANDS.iter().any(|command| command.name == name)
}

/// Renders the selected help topic.
///
/// # Errors
///
/// Returns [`RunError::Help`] for an unknown command, and [`RunError::Output`]
/// when the writer fails.
pub(crate) fn run_help(topic: &HelpTopic, writer: &mut impl Write) -> Result<(), RunError> {
    match topic {
        HelpTopic::Short => write_short(writer),
        HelpTopic::All => write_all(writer),
        HelpTopic::Glossary => render_glossary(writer),
        HelpTopic::Command(name) => {
            let command = COMMANDS
                .iter()
                .find(|command| command.name == name)
                .ok_or_else(|| RunError::Help(format!("unknown command '{name}'")))?;
            write_command(command, writer)
        }
    }
    .map_err(RunError::Output)?;
    writer.flush().map_err(RunError::Output)
}

fn write_short(writer: &mut impl Write) -> std::io::Result<()> {
    writeln!(writer, "QuantRule shell commands")?;
    writeln!(writer)?;
    for command in COMMANDS {
        writeln!(writer, "  {:<58} {}", command.usage, command.summary)?;
    }
    writeln!(writer)?;
    writeln!(
        writer,
        "Type help <command> for the full contract, help all for every contract,"
    )?;
    writeln!(writer, "or help glossary for the precise-term glossary.")
}

fn write_all(writer: &mut impl Write) -> std::io::Result<()> {
    for (index, command) in COMMANDS.iter().enumerate() {
        if index > 0 {
            writeln!(writer)?;
        }
        write_command(command, writer)?;
    }
    Ok(())
}

fn write_command(command: &CommandHelp, writer: &mut impl Write) -> std::io::Result<()> {
    writeln!(writer, "{}", command.name)?;
    writeln!(writer, "  usage:  {}", command.usage)?;
    writeln!(writer, "  does:   {}", command.summary)?;
    writeln!(writer, "  output: {}", command.output)?;
    for note in command.notes {
        writeln!(writer, "  note:   {note}")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{COMMANDS, is_command};

    #[test]
    fn the_surviving_command_set_is_documented() {
        for name in [
            "candles",
            "candles-tolerant",
            "timeframes",
            "events",
            "report model",
            "policy check",
            "strategy explain",
            "strategy run",
            "execution run",
            "portfolio replay",
            "backtest run",
            "fetch",
            "help",
        ] {
            assert!(is_command(name), "{name} must be documented");
        }
        assert_eq!(COMMANDS.len(), 13);
    }

    #[test]
    fn a_deleted_command_is_not_documented() {
        for name in [
            "market-state",
            "market-dashboard",
            "market-chart",
            "market-report",
            "axis",
            "legs",
            "model-series",
            "backtest",
            "strategy",
            "store",
            "db",
            "research",
            "lab",
        ] {
            assert!(!is_command(name), "{name} must not be documented");
        }
    }
}
