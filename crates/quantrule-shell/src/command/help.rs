//! The shell command reference.
//!
//! `help`, `help <command>`, and `help all` all render from [`COMMANDS`], so the
//! short map, the per-command contract, and the complete reference cannot drift.
//! The reference topics are static text. Nothing here reads a file, the clock, or
//! the terminal width, and the output has no color.

mod topics;

use std::io::Write;

use topics::{render_glossary, render_roles, render_tutorial};

use super::types::{HelpTopic, RunError};

/// One option accepted by a command.
struct OptionHelp {
    /// The option and its value placeholder, such as `--window <full|latest>`.
    flag: &'static str,
    /// One line describing the option.
    description: &'static str,
}

/// One command contract.
struct CommandHelp {
    /// The command name, including a two-word family prefix when needed.
    name: &'static str,
    /// The short usage line shown in the one-screen map.
    short_usage: &'static str,
    /// The short description shown in the one-screen map.
    short_summary: &'static str,
    /// The full usage line, including arguments and options.
    usage: &'static str,
    /// One line describing the command.
    summary: &'static str,
    /// The options the command accepts.
    options: &'static [OptionHelp],
    /// The shape of the command's output.
    output: &'static str,
    /// Short contract notes: caps, errors, and boundaries.
    notes: &'static [&'static str],
    /// The reference group.
    group: Group,
}

/// A group in the complete reference.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Group {
    Data,
    Model,
    Research,
    Strategy,
    Runs,
    Store,
    Sources,
    Shell,
}

impl Group {
    /// The groups in reference order.
    const ALL: [Self; 8] = [
        Self::Data,
        Self::Model,
        Self::Research,
        Self::Strategy,
        Self::Runs,
        Self::Store,
        Self::Sources,
        Self::Shell,
    ];

    /// The group heading.
    const fn label(self) -> &'static str {
        match self {
            Self::Data => "Data",
            Self::Model => "Model",
            Self::Research => "Research",
            Self::Strategy => "Strategy",
            Self::Runs => "Runs",
            Self::Store => "Store",
            Self::Sources => "Sources",
            Self::Shell => "Shell",
        }
    }
}

const WINDOW: OptionHelp = OptionHelp {
    flag: "--window <full|latest>",
    description: "Input window: full up to 100000 M1 candles (default), or the last 1000 per timeframe. Latest streams the file and bypasses the full-series cap.",
};
const JSON_DASHBOARD: OptionHelp = OptionHelp {
    flag: "--json",
    description: "Print the numeric dashboard state as one JSON object.",
};
const JSON_REPORT: OptionHelp = OptionHelp {
    flag: "--json",
    description: "Print report.json bytes to stdout.",
};
const JSON_STRATEGY: OptionHelp = OptionHelp {
    flag: "--json",
    description: "Print one provenance envelope instead of bare NDJSON.",
};
const JSON_BUNDLE: OptionHelp = OptionHelp {
    flag: "--json",
    description: "Print report.json bytes to stdout.",
};
const JSON_BACKTEST: OptionHelp = OptionHelp {
    flag: "--json",
    description: "Print a deterministic summary envelope.",
};
const DATA_VERSION: OptionHelp = OptionHelp {
    flag: "--data-version <id>",
    description: "Caller-supplied identity; required.",
};
const CONFIG: OptionHelp = OptionHelp {
    flag: "--config <path>",
    description: "The run configuration; required.",
};
const OPTIONAL_CONFIG: OptionHelp = OptionHelp {
    flag: "--config <path>",
    description: "The strategy blueprint run configuration; optional.",
};
const BUNDLE_OUT: OptionHelp = OptionHelp {
    flag: "--out <dir>",
    description: "Output root override (default: the run config).",
};
const REPORT_OUT: OptionHelp = OptionHelp {
    flag: "--out <dir>",
    description: "New or empty output directory; required.",
};
const DB: OptionHelp = OptionHelp {
    flag: "--db <path>",
    description: "Database path (default data/quantrule.db).",
};
const TIMEFRAME_FILTER: OptionHelp = OptionHelp {
    flag: "--timeframe <tf>",
    description: "Timeframe filter: 1m, 5m, 15m, or 1h.",
};
const BAR: OptionHelp = OptionHelp {
    flag: "--bar <n>",
    description: "Bar-index filter where the table has one.",
};
const LIMIT: OptionHelp = OptionHelp {
    flag: "--limit <n>",
    description: "Maximum rows, 1..1000 (default 100).",
};
const CANDLES: OptionHelp = OptionHelp {
    flag: "--candles <n>",
    description: "Last n bars, 1..1000 (default 1000).",
};
const FROM: OptionHelp = OptionHelp {
    flag: "--from <date>",
    description: "Inclusive UTC start date, YYYY-MM-DD.",
};
const TO: OptionHelp = OptionHelp {
    flag: "--to <date>",
    description: "Inclusive UTC end date, YYYY-MM-DD.",
};
const ROOT: OptionHelp = OptionHelp {
    flag: "--root <dir>",
    description: "Export root (default: data).",
};
const DATASET: OptionHelp = OptionHelp {
    flag: "--dataset <name>",
    description: "Dataset name; required.",
};
const ROLE: OptionHelp = OptionHelp {
    flag: "--role <role>",
    description: "researcher, developer, or validator; required.",
};
const CREATED: OptionHelp = OptionHelp {
    flag: "--created-utc <ts>",
    description: "Explicit UTC time, YYYY-MM-DDTHH:MM:SSZ; required.",
};
const OPERATOR: OptionHelp = OptionHelp {
    flag: "--operator <kind>",
    description: "human or agent (default human).",
};
const NOTES: OptionHelp = OptionHelp {
    flag: "--notes <path>",
    description: "Include role notes verbatim.",
};
const FORCE: OptionHelp = OptionHelp {
    flag: "--force",
    description: "Overwrite an existing summary.txt.",
};
const STRATEGY: OptionHelp = OptionHelp {
    flag: "--strategy <name>",
    description: "Strategy name; required (ladder-context).",
};
const PNG: OptionHelp = OptionHelp {
    flag: "--png <path>",
    description: "Required PNG target.",
};
const CHART_TIMEFRAME: OptionHelp = OptionHelp {
    flag: "--timeframe <tf>",
    description: "Chart timeframe: 1m, 5m, 15m, 1h, or 4h (default 1m).",
};
const MODEL_TIMEFRAME: OptionHelp = OptionHelp {
    flag: "--timeframe <tf>",
    description: "Restrict to 1m, 5m, 15m, 1h, or 4h (default: whole ladder).",
};
const BARS: OptionHelp = OptionHelp {
    flag: "--bars <n>",
    description: "Bars per timeframe, 1..1000 (default 1000).",
};
const FETCH_TIMEFRAME: OptionHelp = OptionHelp {
    flag: "--timeframe <tf>",
    description: "Native timeframe: 1m, 5m, 15m, or 1h (default 1m).",
};
const CSV: OptionHelp = OptionHelp {
    flag: "--csv <path>",
    description: "CSV target (default: data/<SYMBOL>_<M1|M5|M15|H1>_<start>_<end>.csv).",
};

const NO_OPTIONS: &[OptionHelp] = &[];
const WINDOW_ONLY: &[OptionHelp] = &[WINDOW];
const DASHBOARD_OPTIONS: &[OptionHelp] = &[WINDOW, JSON_DASHBOARD];
const CHART_OPTIONS: &[OptionHelp] = &[PNG, CHART_TIMEFRAME, CANDLES, WINDOW];
const MARKET_REPORT_OPTIONS: &[OptionHelp] = &[DATA_VERSION, FROM, TO, REPORT_OUT, JSON_REPORT];
const MODEL_SERIES_OPTIONS: &[OptionHelp] = &[MODEL_TIMEFRAME, BARS, WINDOW];
const RESEARCH_EXPORT_OPTIONS: &[OptionHelp] = &[DATA_VERSION, ROOT];
const STRATEGY_RUN_OPTIONS: &[OptionHelp] = &[
    STRATEGY,
    DATA_VERSION,
    OPTIONAL_CONFIG,
    FROM,
    TO,
    JSON_STRATEGY,
];
const REPORT_OPTIONS: &[OptionHelp] = &[CONFIG, BUNDLE_OUT, JSON_BUNDLE];
const BACKTEST_OPTIONS: &[OptionHelp] = &[CONFIG, BUNDLE_OUT, JSON_BACKTEST];
const LAB_SUMMARY_OPTIONS: &[OptionHelp] =
    &[ROLE, DATA_VERSION, CONFIG, CREATED, OPERATOR, NOTES, FORCE];
const STORE_INIT_OPTIONS: &[OptionHelp] = &[DB];
const STORE_INGEST_OPTIONS: &[OptionHelp] = &[DATASET, DATA_VERSION, DB];
const STORE_VERIFY_OPTIONS: &[OptionHelp] = &[DATASET, DATA_VERSION, DB];
const STORE_LIST_OPTIONS: &[OptionHelp] = &[DB];
const STORE_SHOW_OPTIONS: &[OptionHelp] = &[TIMEFRAME_FILTER, BAR, LIMIT, DB];
const STORE_REMOVE_OPTIONS: &[OptionHelp] = &[DB];
const DB_SERIES_OPTIONS: &[OptionHelp] = &[TIMEFRAME_FILTER, LIMIT, DB];
const DB_BARS_OPTIONS: &[OptionHelp] = &[TIMEFRAME_FILTER, BAR, LIMIT, DB];
const DB_METRICS_OPTIONS: &[OptionHelp] = &[TIMEFRAME_FILTER, BAR, LIMIT, DB];
const DB_LAYER3_OPTIONS: &[OptionHelp] = &[BAR, LIMIT, DB];
const FETCH_OPTIONS: &[OptionHelp] = &[FETCH_TIMEFRAME, CSV, PNG, CANDLES];

/// Every command and its contract, in reference order.
const COMMANDS: &[CommandHelp] = &[
    CommandHelp {
        name: "candles",
        short_usage: "candles <csv-path>",
        short_summary: "Read strictly; last 1000 candles as NDJSON.",
        usage: "candles <csv-path>",
        summary: "Read a CSV strictly and print the retained candle window as NDJSON.",
        options: NO_OPTIONS,
        output: "NDJSON, one candle per line, oldest first; at most 1000 lines.",
        notes: &[
            "A malformed row, invalid OHLC, duplicate timestamp, or out-of-order timestamp aborts the command.",
            "The window is built before writing, so a late error produces no partial NDJSON.",
        ],
        group: Group::Data,
    },
    CommandHelp {
        name: "candles-tolerant",
        short_usage: "candles-tolerant <csv-path>",
        short_summary: "Skip bad rows; NDJSON plus quality on stderr.",
        usage: "candles-tolerant <csv-path>",
        summary: "Read a CSV, skip row-level failures, and report quality on stderr.",
        options: NO_OPTIONS,
        output: "NDJSON on stdout; one quality summary and one line per skipped row on stderr.",
        notes: &[
            "A missing file or bad header aborts; row-level failures are skipped and counted.",
            "A run with skipped rows, including a file where every row is rejected, exits 0.",
        ],
        group: Group::Data,
    },
    CommandHelp {
        name: "timeframes",
        short_usage: "timeframes <csv-path>",
        short_summary: "Build M1/M5/M15/H1/H4; NDJSON plus aggregation.",
        usage: "timeframes <csv-path>",
        summary: "Build M5, M15, H1, and H4 from an M1 CSV and print the five series.",
        options: NO_OPTIONS,
        output: "NDJSON grouped M1, M5, M15, H1, H4; aggregation, partial, and gap records on stderr.",
        notes: &[
            "M1 is the source; M5, M15, H1, and H4 are derived by the data engine.",
            "A source whose rows are never closer than 5m, 15m, 1h, or 4h is rejected; supply M1 rows.",
            "A complete trailing bucket is published; an incomplete bucket is dropped and counted.",
            "No derived CSV is written.",
        ],
        group: Group::Data,
    },
    CommandHelp {
        name: "market-state",
        short_usage: "market-state <csv-path>",
        short_summary: "Four-timeframe state as JSON.",
        usage: "market-state <csv-path> [--window full|latest]",
        summary: "Print the default-ladder market state as one JSON object.",
        options: WINDOW_ONLY,
        output: "One JSON object with ladder, timeframes, aggregates, and consensus.",
        notes: &[
            "The model publishes numbers; the shell maps stable codes to labels.",
            "The four axes are separate; agreement is a count, never an average.",
            "Full window cap: 100000 M1 candles; --window latest streams the file and keeps the last 1000 per timeframe.",
        ],
        group: Group::Model,
    },
    CommandHelp {
        name: "market-dashboard",
        short_usage: "market-dashboard <csv-path>",
        short_summary: "Four-axis dashboard, one row per timeframe.",
        usage: "market-dashboard <csv-path> [--window full|latest] [--json]",
        summary: "Print the four-axis dashboard, one row per timeframe.",
        options: DASHBOARD_OPTIONS,
        output: "Fixed-width four-track text, or one numeric JSON object with --json.",
        notes: &[
            "Full window cap: 100000 M1 candles; --window latest streams the file and keeps the last 1000 per timeframe.",
            "Missing value: an em dash with a reason code; a valid zero prints 0%.",
            "The view is descriptive; it is not a strategy, forecast, or recommendation.",
        ],
        group: Group::Model,
    },
    CommandHelp {
        name: "market-chart",
        short_usage: "market-chart <csv-path>",
        short_summary: "Deterministic PNG of candles and axes.",
        usage: "market-chart <csv-path> --png <path> [--timeframe <tf>] [--candles <n>] [--window full|latest]",
        summary: "Render the last candles and the four axis panels as a deterministic PNG.",
        options: CHART_OPTIONS,
        output: "A PNG; prints png=<path> timeframe=<tf> candles=<n> bytes=<n>.",
        notes: &[
            "Input cap: 100000 M1 candles; --window latest charts a larger fetch.",
            "The four axis panels share the [-1,+1] scale; a missing axis is a gap, never zero.",
        ],
        group: Group::Model,
    },
    CommandHelp {
        name: "market-report",
        short_usage: "market-report <csv-path>",
        short_summary: "Full-period report: four artifacts.",
        usage: "market-report <csv-path> --data-version <id> [--from <date>] [--to <date>] --out <dir> [--json]",
        summary: "Write the full-period market report and its charts.",
        options: MARKET_REPORT_OPTIONS,
        output: "Four artifacts in order: summary.txt, report.json, price-axes.png, events.png.",
        notes: &[
            "Input cap: 600000 M1 candles.",
            "Contains no order, fill, position, trade, equity, or P&L field.",
            "The output directory must be absent or empty; a failure removes the partial set.",
        ],
        group: Group::Model,
    },
    CommandHelp {
        name: "axis",
        short_usage: "axis <csv-path> <axis>",
        short_summary: "One axis per timeframe.",
        usage: "axis <csv-path> <axis> [--window full|latest]",
        summary: "Print one axis per timeframe: direction, momentum, structure, or progression.",
        options: WINDOW_ONLY,
        output: "One NDJSON line per timeframe.",
        notes: &[
            "Each line carries the numeric fields, compact code, readiness and reason codes, labels, and counters.",
            "Full window cap: 100000 M1 candles; --window latest keeps the last 1000 per timeframe.",
        ],
        group: Group::Model,
    },
    CommandHelp {
        name: "events",
        short_usage: "events <csv-path>",
        short_summary: "Every event in the analyzed window.",
        usage: "events <csv-path> [--window full|latest]",
        summary: "Print every event record in the analyzed window as NDJSON.",
        options: WINDOW_ONLY,
        output: "One NDJSON line per event, grouped by timeframe and ordered by availability.",
        notes: &[
            "Types: level, divergence, and signal.",
            "Every record carries occurrence and availability; an event never appears before it is knowable.",
            "Events are evidence; they do not alter an axis value.",
            "Full window cap: 100000 M1 candles; --window latest keeps the last 1000 per timeframe.",
        ],
        group: Group::Model,
    },
    CommandHelp {
        name: "legs",
        short_usage: "legs <csv-path>",
        short_summary: "Minor and major leg per timeframe.",
        usage: "legs <csv-path> [--window full|latest]",
        summary: "Print the minor and major leg per timeframe.",
        options: WINDOW_ONLY,
        output: "Two NDJSON lines per timeframe, one per pivot scale.",
        notes: &[
            "A leg's stage may be null while its origin is unconfirmed or its target is unstable.",
            "A target revision changes the target version without a price movement.",
            "Full window cap: 100000 M1 candles; --window latest keeps the last 1000 per timeframe.",
        ],
        group: Group::Model,
    },
    CommandHelp {
        name: "model-series",
        short_usage: "model-series <csv-path>",
        short_summary: "Last complete model bars.",
        usage: "model-series <csv-path> [--timeframe <tf>] [--bars <n>] [--window full|latest]",
        summary: "Print the last complete model bars as NDJSON.",
        options: MODEL_SERIES_OPTIONS,
        output: "NDJSON, one complete model bar per line, oldest first.",
        notes: &[
            "Each line carries the candle, six readings, the four axes with diagnostics, context, and consensus.",
            "Events stay a separate command because one bar can carry several events.",
            "Full window cap: 100000 M1 candles; --window latest keeps the last 1000 per timeframe.",
        ],
        group: Group::Model,
    },
    CommandHelp {
        name: "research export",
        short_usage: "research export <csv-path> <dataset>",
        short_summary: "Research dataset with layers and metadata.",
        usage: "research export <csv-path> <dataset> --data-version <id> [--root <dir>]",
        summary: "Write the research dataset: raw M1, three layers, manifest, dictionary, and codes.",
        options: RESEARCH_EXPORT_OPTIONS,
        output: "Writes the raw M1 file and the processed layer files; prints row counts and file count.",
        notes: &[
            "Input cap: 100000 M1 candles; an over-cap input fails rather than truncating.",
            "The export is deterministic; the manifest records the data version, configuration, row counts, and files.",
        ],
        group: Group::Research,
    },
    CommandHelp {
        name: "research summary",
        short_usage: "research summary <csv-path>",
        short_summary: "Counts, ranges, histograms, relations.",
        usage: "research summary <csv-path>",
        summary: "Print counts, ranges, ordinal histograms, and axis correlations.",
        options: NO_OPTIONS,
        output: "One line per timeframe and axis, then one line per timeframe and axis pair.",
        notes: &[
            "Ordinal codes are counted, never averaged; no line combines the axes.",
            "Input cap: 100000 M1 candles.",
        ],
        group: Group::Research,
    },
    CommandHelp {
        name: "strategy-run",
        short_usage: "strategy-run <csv-path>",
        short_summary: "Entries and exits; no fills or P&L.",
        usage: "strategy-run <csv-path> --strategy <name> --data-version <id> [--config <run.json>] [--from <date>] [--to <date>] [--json]",
        summary: "Print strategy entries and exits as NDJSON; no fills or P&L.",
        options: STRATEGY_RUN_OPTIONS,
        output: "NDJSON entries and exits, or one JSON envelope with --json.",
        notes: &[
            "Input cap: 600000 M1 candles; computes the active ladder and walks the execution timeframe.",
            "With --config, uses the active ladder, strategy, and model configuration from the run.",
            "The JSON envelope marks execution_state as virtual; each decision is assumed to fill immediately.",
            "Stops at the decision: no order, fill, position, equity, trade, or P&L.",
            "Model Signal events are not strategy entries or exits.",
        ],
        group: Group::Strategy,
    },
    CommandHelp {
        name: "report",
        short_usage: "report <engine> --config <run>",
        short_summary: "One config-driven report bundle.",
        usage: "report <engine> --config <run.json> [--out <dir>] [--json]",
        summary: "Write one config-driven report bundle under <out>/<engine>/.",
        options: REPORT_OPTIONS,
        output: "One bundle and its artifact paths, or report.json bytes with --json.",
        notes: &[
            "Engines: model, strategy, execution, portfolio, or analytical.",
            "Runs only the engines the bundle needs; a non-empty target is refused and a partial bundle is removed on failure.",
        ],
        group: Group::Runs,
    },
    CommandHelp {
        name: "backtest",
        short_usage: "backtest --config <run>",
        short_summary: "Five bundles from one full run.",
        usage: "backtest --config <run.json> [--out <dir>] [--json]",
        summary: "Run the full pipeline and write all five bundles.",
        options: BACKTEST_OPTIONS,
        output: "Five bundle paths, or one JSON summary envelope with --json.",
        notes: &[
            "Loads the candles once, runs the pipeline once, and computes the model once.",
            "The input cap comes from data.json.max_candles, at most 600000.",
            "A non-empty output root is refused before any write.",
        ],
        group: Group::Runs,
    },
    CommandHelp {
        name: "lab summary",
        short_usage: "lab summary <lab-dir>",
        short_summary: "Root summary.txt for a lab result folder.",
        usage: "lab summary <lab-dir> --role <role> --data-version <id> --config <run.json> --created-utc <ts> [--operator human|agent] [--notes <path>] [--force]",
        summary: "Write one deterministic summary.txt for a lab result folder.",
        options: LAB_SUMMARY_OPTIONS,
        output: "Writes <lab-dir>/summary.txt and prints wrote <path>.",
        notes: &[
            "Reads the five report bundles and copies published values; it never recomputes the model.",
            "The data version must match every report.json and provenance.json.",
            "Never reads the clock; --created-utc is explicit.",
            "Refuses an existing summary.txt unless --force is given.",
        ],
        group: Group::Runs,
    },
    CommandHelp {
        name: "store init",
        short_usage: "store init [--db <path>]",
        short_summary: "Create or open the SQLite store.",
        usage: "store init [--db <path>]",
        summary: "Create or open the SQLite store and apply the schema migrations.",
        options: STORE_INIT_OPTIONS,
        output: "Prints store <path> schema=<n>.",
        notes: &[
            "Reopening an existing store is idempotent; a version 1 database is migrated in place.",
            "A store whose schema is newer than the binary is rejected; a file that is not a database fails clearly.",
        ],
        group: Group::Store,
    },
    CommandHelp {
        name: "strategy explain",
        short_usage: "strategy explain --config <run>",
        short_summary: "Render the strategy in trader words.",
        usage: "strategy explain --config <run.json>",
        summary: "Print the validated strategy thesis, entry, exit, and risk in human trader words.",
        options: &[CONFIG],
        output: "Plain text: strategy name, execution and context ladder, thesis, entry, exit, and risk.",
        notes: &[
            "Reads the run configuration; it never runs the model or invents a rule.",
            "The rendered rule is a presentation of the structured strategy config.",
        ],
        group: Group::Strategy,
    },
    CommandHelp {
        name: "policy check",
        short_usage: "policy check --config <run>",
        short_summary: "Validate the policy and engine configs.",
        usage: "policy check --config <run.json>",
        summary: "Validate the policy blueprint against the strategy, data, execution, portfolio, and analytics.",
        options: &[CONFIG],
        output: "One deterministic policy summary line or a config error.",
        notes: &[
            "Reads no candles and writes no output files.",
            "The policy is the human-owned account, objective, risk, and universe envelope.",
        ],
        group: Group::Runs,
    },
    CommandHelp {
        name: "store ingest",
        short_usage: "store ingest <csv-path>",
        short_summary: "Ingest one M1 dataset.",
        usage: "store ingest <csv-path> --dataset <name> --data-version <id> [--db <path>]",
        summary: "Ingest one M1 dataset into the SQLite store.",
        options: STORE_INGEST_OPTIONS,
        output: "Prints ingested <name> bars=<n> timeframes=<n>.",
        notes: &[
            "Input cap: 100000 M1 candles.",
            "Writes one dataset, every ladder bar, consensus row, Layer 3 row, pivot, level, event, and retained leg in one transaction.",
            "A same-name dataset with a different identity is a conflict until it is removed.",
        ],
        group: Group::Store,
    },
    CommandHelp {
        name: "store verify",
        short_usage: "store verify <csv-path>",
        short_summary: "Compare a dataset with a fresh build.",
        usage: "store verify <csv-path> --dataset <name> --data-version <id> [--db <path>]",
        summary: "Verify a stored dataset against a fresh build.",
        options: STORE_VERIFY_OPTIONS,
        output: "Prints verified <name> bars=<n> tables=7, or the first mismatch.",
        notes: &[
            "Rebuilds the source, opens the store read-only, and compares every table; it never repairs.",
            "A dataset with layer3_materialized = 0 is incomplete; re-ingest into a fresh database.",
        ],
        group: Group::Store,
    },
    CommandHelp {
        name: "store list",
        short_usage: "store list [--db <path>]",
        short_summary: "List stored datasets as CSV.",
        usage: "store list [--db <path>]",
        summary: "List the stored datasets as CSV.",
        options: STORE_LIST_OPTIONS,
        output: "One CSV row per dataset, sorted by name, with identity, counts, and Layer 3 state.",
        notes: &["Read-only; a missing file is an error and is never created."],
        group: Group::Store,
    },
    CommandHelp {
        name: "store show",
        short_usage: "store show <dataset> <table>",
        short_summary: "Show one stored table.",
        usage: "store show <dataset> <table> [--timeframe <tf>] [--bar <n>] [--limit <n>] [--db <path>]",
        summary: "Show one stored table as CSV.",
        options: STORE_SHOW_OPTIONS,
        output: "CSV rows in DDL order and primary-key order.",
        notes: &[
            "Tables: bar, consensus, layer3, pivot, level, event, or leg.",
            "event.reference_code = 0 is rendered as NULL.",
            "Read-only; a missing file is an error and is never created.",
        ],
        group: Group::Store,
    },
    CommandHelp {
        name: "store remove",
        short_usage: "store remove <dataset>",
        short_summary: "Remove one dataset and its rows.",
        usage: "store remove <dataset> [--db <path>]",
        summary: "Remove one dataset and its cascading records.",
        options: STORE_REMOVE_OPTIONS,
        output: "Prints removed <name> bars=<n>; the database file remains.",
        notes: &[
            "Removing a missing dataset is an error.",
            "Other datasets remain; removing the same name with a different identity is a conflict until removed.",
        ],
        group: Group::Store,
    },
    CommandHelp {
        name: "db series",
        short_usage: "db series <dataset>",
        short_summary: "Stored candles as NDJSON.",
        usage: "db series <dataset> [--timeframe <tf>] [--limit <n>] [--db <path>]",
        summary: "Read stored candles as NDJSON.",
        options: DB_SERIES_OPTIONS,
        output: "NDJSON with dataset, timeframe_secs, bar_index, ISO open_time, OHLC, and nullable volume; oldest first.",
        notes: &["Read-only; a missing file is an error and is never created."],
        group: Group::Store,
    },
    CommandHelp {
        name: "db bars",
        short_usage: "db bars <dataset>",
        short_summary: "Full stored bar rows as CSV.",
        usage: "db bars <dataset> [--timeframe <tf>] [--bar <n>] [--limit <n>] [--db <path>]",
        summary: "Read the full stored bar table as CSV.",
        options: DB_BARS_OPTIONS,
        output: "CSV with 74 columns.",
        notes: &["Read-only; a missing file is an error and is never created."],
        group: Group::Store,
    },
    CommandHelp {
        name: "db metrics",
        short_usage: "db metrics <dataset>",
        short_summary: "Metrics and resolutions as CSV.",
        usage: "db metrics <dataset> [--timeframe <tf>] [--bar <n>] [--limit <n>] [--db <path>]",
        summary: "Read metrics and resolutions as CSV.",
        options: DB_METRICS_OPTIONS,
        output: "CSV with 61 columns: identity, 47 metric columns, and 12 resolution columns.",
        notes: &["Read-only; a missing file is an error and is never created."],
        group: Group::Store,
    },
    CommandHelp {
        name: "db layer3",
        short_usage: "db layer3 <dataset>",
        short_summary: "Materialized Layer 3 rows as CSV.",
        usage: "db layer3 <dataset> [--bar <n>] [--limit <n>] [--db <path>]",
        summary: "Read the materialized Layer 3 rows as CSV.",
        options: DB_LAYER3_OPTIONS,
        output: "CSV with 285 columns.",
        notes: &["Read-only; a missing file is an error and is never created."],
        group: Group::Store,
    },
    CommandHelp {
        name: "fetch",
        short_usage: "fetch <symbol> <start> <end>",
        short_summary: "Fetch BID candles; CSV and optional PNG.",
        usage: "fetch <symbol> <start> <end> [--timeframe <tf>] [--csv <path>] [--png <path>] [--candles <n>]",
        summary: "Fetch native BID candles; write a CSV and an optional PNG.",
        options: FETCH_OPTIONS,
        output: "Writes the CSV and PNG; prints symbol, timeframe, dates, row counts, first/last, csv, and png.",
        notes: &[
            "Reads Dukascopy BID candles; network access is required.",
            "Dates are inclusive UTC dates in YYYY-MM-DD form; no clock is read.",
            "Fetched higher intervals are source references; canonical M5/M15/H1/H4 come from timeframes.",
        ],
        group: Group::Sources,
    },
    CommandHelp {
        name: "help",
        short_usage: "help [topic]",
        short_summary: "This map, a command, or a reference topic.",
        usage: "help [<command>|all|glossary|tutorial|roles]",
        summary: "Show the short map, one command contract, or a reference topic.",
        options: NO_OPTIONS,
        output: "The short map with no topic, one command contract, or a reference topic.",
        notes: &[
            "A topic may be one or two words, such as 'store ingest' or 'lab summary'.",
            "An unknown topic is an error that points to 'help all'.",
            "Output is fixed-width plain text; it does not depend on terminal width and has no color.",
        ],
        group: Group::Shell,
    },
    CommandHelp {
        name: "quit",
        short_usage: "quit",
        short_summary: "Exit the shell.",
        usage: "quit",
        summary: "Exit the shell with status 0.",
        options: NO_OPTIONS,
        output: "No output.",
        notes: &["Interactive: exits the shell. Non-interactive: exits with status 0."],
        group: Group::Shell,
    },
];

/// Returns whether `name` is a command with a help page.
pub(crate) fn is_command(name: &str) -> bool {
    find_command(name).is_some()
}

fn find_command(name: &str) -> Option<&'static CommandHelp> {
    COMMANDS.iter().find(|command| command.name == name)
}

/// Writes the help topic.
///
/// # Errors
///
/// Returns [`RunError::Help`] for a command that has no page, or
/// [`RunError::Output`] when the writer fails.
pub(crate) fn run_help(topic: &HelpTopic, writer: &mut impl Write) -> Result<(), RunError> {
    let text = match topic {
        HelpTopic::Short => render_short(),
        HelpTopic::All => render_all(),
        HelpTopic::Glossary => render_glossary(),
        HelpTopic::Tutorial => render_tutorial(),
        HelpTopic::Roles => render_roles(),
        HelpTopic::Command(name) => match find_command(name) {
            Some(command) => render_command(command),
            None => {
                return Err(RunError::Help(format!(
                    "unknown help topic '{name}'; type 'help all' for every command and reference topic"
                )));
            }
        },
    };
    writer
        .write_all(text.as_bytes())
        .map_err(RunError::Output)?;
    writer.flush().map_err(RunError::Output)
}

fn render_short() -> String {
    let mut out = String::from(
        "QuantRule shell\nINPUT > DATA > MODEL > OUTPUT; the shell wraps the pipeline.\n\n",
    );
    for group in Group::ALL {
        out.push_str(group.label());
        out.push('\n');
        for command in COMMANDS.iter().filter(|command| command.group == group) {
            out.push_str(&format!(
                "  {:<31} {}\n",
                command.short_usage, command.short_summary
            ));
        }
    }
    out.push('\n');
    out.push_str(
        "Reference  help <command> | help all | help glossary | help tutorial | help roles\n",
    );
    out
}

fn render_all() -> String {
    let mut out = String::from(
        "QuantRule complete command reference\n====================================\n\n",
    );
    for group in Group::ALL {
        out.push_str(group.label());
        out.push('\n');
        out.push_str(&"-".repeat(group.label().len()));
        out.push_str("\n\n");
        for command in COMMANDS.iter().filter(|command| command.group == group) {
            out.push_str(&render_command(command));
            out.push('\n');
        }
    }
    out.push_str("Reference topics\n----------------\n");
    out.push_str("  help glossary  Precise terms and definitions.\n");
    out.push_str("  help tutorial  The full-pipeline tutorial map.\n");
    out.push_str("  help roles     The quant role map and handoffs.\n");
    out
}

fn render_command(command: &CommandHelp) -> String {
    let mut out = String::new();
    out.push_str(command.usage);
    out.push_str("\n\n");
    out.push_str(command.summary);
    out.push_str("\n\nOptions\n");
    if command.options.is_empty() {
        out.push_str("  none\n");
    } else {
        for option in command.options {
            out.push_str(&format!("  {:<28} {}\n", option.flag, option.description));
        }
    }
    out.push_str("\nOutput\n  ");
    out.push_str(command.output);
    out.push('\n');
    if !command.notes.is_empty() {
        out.push_str("\nNotes\n");
        for note in command.notes {
            out.push_str("  ");
            out.push_str(note);
            out.push('\n');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_command_has_a_page_and_appears_in_all() {
        let all = render_all();
        for command in COMMANDS {
            let page = render_command(command);
            assert!(!page.is_empty(), "{} has an empty page", command.name);
            assert!(
                all.contains(&page),
                "help all omits the {} page",
                command.name
            );
            for option in command.options {
                assert!(
                    all.contains(option.flag),
                    "help all omits {} for {}",
                    option.flag,
                    command.name
                );
            }
        }
    }

    #[test]
    fn command_names_are_unique() {
        for (index, command) in COMMANDS.iter().enumerate() {
            assert!(
                !COMMANDS[..index]
                    .iter()
                    .any(|earlier| earlier.name == command.name),
                "duplicate command {}",
                command.name
            );
        }
    }

    #[test]
    fn short_map_fits_one_screen_and_lists_every_command() {
        let short = render_short();
        assert!(
            short.lines().count() <= 50,
            "short map is {} lines",
            short.lines().count()
        );
        for command in COMMANDS {
            assert!(
                short.contains(command.name),
                "short map omits {}",
                command.name
            );
        }
    }

    #[test]
    fn help_topics_are_plain_text() {
        for text in [
            render_short(),
            render_all(),
            render_glossary(),
            render_tutorial(),
            render_roles(),
        ] {
            assert!(
                !text.contains('\u{1b}'),
                "help output contains an ANSI escape"
            );
        }
    }
}
