//! Spec and panel loading for the `report model` command.
//!
//! The module resolves the configured timeframes, loads the model run, and
//! loads panel instruments for a v2 spec. It does not assemble the bundle.

use std::path::Path;

use crate::spec::v2::ParsedModelSpecV2;
use quantrule_data::{Candle, Timeframe};
use quantrule_model::run::ModelRun;

use crate::command::types::{ReportError, RunError};
use crate::model_bundle::{ModelBundleError, period_label, resolve_spec_path};
use quantrule_app::config::RunConfig;
use quantrule_app::pipeline::{self, PipelineError};

pub(super) fn load_panel_series(
    config: &RunConfig,
    spec_path: &Path,
    parsed: &ParsedModelSpecV2,
    model: &ModelRun,
    timeframe: Timeframe,
) -> Result<Vec<(String, Vec<Candle>)>, RunError> {
    let mut series = Vec::with_capacity(parsed.spec.panel_instruments.len());
    for name in &parsed.spec.panel_instruments {
        let candles = if name == &parsed.spec.instrument {
            model
                .timeframe(parsed.spec.evaluation_timeframe_secs)
                .map(|timeframe| timeframe.candles().to_vec())
                .ok_or_else(|| {
                    spec_error(format!(
                        "the evaluated timeframe {} is absent from the model run",
                        parsed.spec.evaluation_timeframe_secs
                    ))
                })?
        } else {
            let source = parsed
                .panel_sources
                .get(name)
                .ok_or_else(|| spec_error(format!("the panel source {name} is not declared")))?;
            let path = resolve_spec_path(spec_path, source);
            let set =
                pipeline::load_set_with_limit(&path, config.data.bounds, config.data.max_candles)
                    .map_err(|error| pipeline_error(config, PipelineError::Source(error)))?;
            set.series(timeframe).iter().copied().collect()
        };
        if candles.is_empty() {
            return Err(spec_error(format!(
                "the panel instrument {name} has no {} candles",
                timeframe.code()
            )));
        }
        series.push((name.clone(), candles));
    }
    Ok(series)
}

fn spec_error(message: impl Into<String>) -> RunError {
    RunError::Report(ReportError::Spec(message.into()))
}

pub(super) fn spec_timeframes(
    config: &RunConfig,
    ladder: &[u64],
) -> Result<Vec<Timeframe>, RunError> {
    ladder
        .iter()
        .map(|timeframe_secs| {
            config
                .data
                .available_timeframes
                .iter()
                .copied()
                .find(|timeframe| u64::from(timeframe.seconds()) == *timeframe_secs)
                .ok_or(RunError::Report(ReportError::SpecTimeframe {
                    timeframe_secs: *timeframe_secs,
                }))
        })
        .collect()
}

pub(super) fn load_model(config: &RunConfig) -> Result<ModelRun, RunError> {
    pipeline::run_model(config).map_err(|error| pipeline_error(config, error))
}

pub(super) fn pipeline_error(config: &RunConfig, error: PipelineError) -> RunError {
    match error {
        PipelineError::Source(error) => RunError::Source(error),
        PipelineError::EmptyPeriod => RunError::Model(ModelBundleError::EmptyPeriod {
            period: period_label(config),
        }),
    }
}
