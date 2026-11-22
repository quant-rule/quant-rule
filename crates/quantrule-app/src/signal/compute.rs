//! Pure signal computation from a frozen model artifact and a strategy pack.
//!
//! The computation reads the published model bundle's execution timeframe and
//! the strategy pack's signal-mode payload. It never reads a candle file, a run
//! config, an execution pack, a portfolio pack, the clock, or a random source.
//! Execution and portfolio policies are deliberately absent: the same signal
//! bytes are the shared input to every policy.
//!
//! The model artifact id is supplied by the caller because a model artifact is
//! a file set whose canonical digest is computed over its manifest entries, not
//! over one document. The strategy service verifies that id against the frozen
//! manifest; [`SignalArtifact::verify_sources`] is the same check once the
//! artifact exists.

use quantrule_api::Digest;

use crate::pack::StrategyMode;
use crate::pack::StrategyPack;
use crate::stages::strategy::{BarRow, ModelBundle, validate_context};

use super::artifact::{SignalArtifact, SignalRow};
use super::error::SignalError;
use super::reference::ReferenceMachine;

/// One pure signal-computation request.
pub struct SignalComputeRequest<'a> {
    /// The frozen model bundle the signal reads.
    pub bundle: &'a ModelBundle,
    /// The frozen, signal-mode strategy pack.
    pub pack: &'a StrategyPack,
    /// The model artifact id the caller resolved for `bundle`.
    pub model_artifact_id: &'a Digest,
}

/// Computes one signal artifact.
///
/// The rows are produced in execution-bar order by the deterministic reference
/// position. A row only reads model values available at or before its own bar,
/// so appending or changing a later bar cannot change an earlier row.
///
/// # Errors
///
/// Returns [`SignalError`] when the pack is fold mode, the model does not
/// publish the strategy's timeframes, the execution timeframe has no bar, or a
/// published bar has no strategy snapshot.
pub fn compute(request: &SignalComputeRequest<'_>) -> Result<SignalArtifact, SignalError> {
    let payload = request.pack.payload();
    if payload.mode() != StrategyMode::Signal {
        return Err(SignalError::FoldMode);
    }
    let config = payload.strategy();
    let execution = u64::from(config.execution_timeframe.seconds());
    validate_context(config, None, |timeframe| {
        request.bundle.contains_timeframe(timeframe)
    })
    .map_err(SignalError::Context)?;
    let bars = request
        .bundle
        .bars(execution)
        .ok_or(SignalError::MissingTimeframe {
            timeframe_secs: execution,
        })?;
    if bars.is_empty() {
        return Err(SignalError::EmptyExecution {
            timeframe_secs: execution,
        });
    }
    let context: Vec<u64> = config
        .context_timeframes
        .iter()
        .map(|timeframe| u64::from(timeframe.seconds()))
        .collect();
    let window = config.event_window_bars();
    let mut machine = ReferenceMachine::new(config);
    let mut rows: Vec<SignalRow> = Vec::with_capacity(bars.len());
    for (position, row) in bars.iter().enumerate() {
        let contiguous = position
            .checked_sub(1)
            .and_then(|previous| bars.get(previous))
            .is_some_and(|previous| row.timestamp - previous.timestamp == execution as i64);
        let input = request
            .bundle
            .input(execution, row.bar, window, &context)
            .ok_or(SignalError::MissingBar { bar_index: row.bar })?;
        rows.push(machine.step(position, row, contiguous, &input)?);
    }
    let (period_from, period_to) = period(request.bundle, bars);
    Ok(SignalArtifact::new(
        request.pack.id().clone(),
        request.model_artifact_id.clone(),
        request.bundle.data_version.clone(),
        config.execution_timeframe,
        period_from,
        period_to,
        rows,
    ))
}

/// The artifact period: the model's requested period when it is recorded, and
/// the execution series' first and last UTC dates otherwise.
fn period(bundle: &ModelBundle, bars: &[BarRow]) -> (String, String) {
    let from = bundle
        .period_from
        .clone()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| date(bars.first().map_or(0, |row| row.timestamp)));
    let to = bundle
        .period_to
        .clone()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| date(bars.last().map_or(0, |row| row.timestamp)));
    (from, to)
}

fn date(seconds: i64) -> String {
    // `format_utc_timestamp` is pure and always returns the 20-character
    // `YYYY-MM-DDTHH:MM:SSZ` form, so the first ten characters are the UTC date.
    quantrule_data::format_utc_timestamp(seconds)[..10].to_owned()
}
