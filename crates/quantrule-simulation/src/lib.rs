//! The research plane.
//!
//! Hindsight by design: a window of history is rendered, a human marks good and
//! bad entries, and a deterministic oracle marks the same window. Nothing here
//! reaches the signal path, and nothing here is a run.
//!
//! This crate holds the deterministic parts of the simulation domain: the bundle
//! types, window selection, the mark schema, the hindsight oracle, the path
//! label, the exit lab, the review, the check, and the meta-label and context
//! split. The stages are a faithful port of the retired Python harness; the golden
//! fixture `docs/port-parity/PORT-001.golden.json` compares every numeric value
//! to `1e-9`.
//!
//! The crate is pure: no I/O, no clock, no randomness beyond the seeded,
//! bit-exact Mersenne Twister the reference selection uses, and no dependency on
//! a filesystem or a live market.

#![forbid(unsafe_code)]

pub mod bundle;
pub mod check;
pub mod exit;
pub mod marks;
pub mod meta;
pub mod oracle;
pub mod path;
pub mod review;
pub mod window;

pub use bundle::{
    Bar, Bundle, Catalog, Event, TimestampError, civil_from_days, days_from_civil, fsum, half_year,
    mean, parse_utc, utc_from_seconds, utc_year,
};
pub use check::{
    CheckConfig, CheckError, CheckRecord, Control, Rows as CheckRows, SideFilter, Stats,
    Verdict as CheckVerdict, build as build_check, build_from_rows as build_check_from_rows,
    check_side, event_rows,
};
pub use exit::{
    Aggregate, Entry, EntryResult, EntrySource, ExitConfig, ExitError, ExitRecord, RuleResult,
    build as build_exit, build_with_entries as build_exit_with_entries, simulate,
};
pub use marks::{
    Chart, ChartBar, Mark, MarkAction, MarkError, MarkQuality, RawMark, Side, annotation_key,
    count_by_quality, delete, read_marks, upsert, validate_mark, validate_marks, write_marks,
};
pub use meta::{
    Cell, Check as MetaCheck, Config as MetaConfig, ForwardPath, MetaError, MetaResult,
    Row as MetaRow, Verdict as MetaVerdict, build as build_meta, evaluate as evaluate_meta,
};
pub use oracle::{Detail, Label, OracleError, OracleSpec, PathResult, label};
pub use path::{FirstTouch, Path, PathError, classify};
pub use review::{
    Context, ContextRow, Pair, Review, ReviewError, TableRow, TradeStats, review, trade_stats,
};
pub use window::{Selection, WindowError, WindowEvent, WindowRecord, WindowSpec, select};
