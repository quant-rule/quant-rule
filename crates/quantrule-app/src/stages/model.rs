//! The model report use case.
//!
//! One call loads the run configuration, computes the flat model over the
//! configured ladder or a model spec's ladder, and writes the model bundle. The
//! adapters pass paths and print the returned artifact names; they do not parse
//! a spec or assemble an artifact.

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use quantrule_data::{Candle, Timeframe};
use quantrule_model::panel::PanelInstrument;
use quantrule_model::run::ModelRun;
use quantrule_model::spec::v2::SpecRun;

use crate::artifact::model_bundle::{
    BundleOutput, ModelBundleError, SpecRunArtifacts, model_bundle_from, model_bundle_with_spec,
    panel_provenance, period_label, resolve_spec_path,
};
use crate::config::RunConfig;
use crate::config::spec;
use crate::config::spec::v2::ParsedModelSpecV2;
use crate::csv::SourceError;
use crate::pipeline::{self, PipelineError};

/// One model report request.
pub struct ModelReportRequest<'a> {
    /// The run configuration path.
    pub config: &'a Path,
    /// The optional model-spec path.
    pub spec: Option<&'a Path>,
    /// The output root; the run configuration's root when `None`.
    pub out: Option<&'a Path>,
}

/// The model report could not be produced.
#[derive(Debug)]
pub enum ModelReportError {
    /// The run configuration is invalid.
    Config(crate::config::ConfigError),
    /// The candle source or data engine failed.
    Source(SourceError),
    /// The spec file could not be read.
    SpecRead {
        /// The spec path.
        path: PathBuf,
        /// The underlying I/O error.
        source: io::Error,
    },
    /// The spec is invalid.
    Spec(String),
    /// A spec timeframe is not in the data ladder.
    SpecTimeframe {
        /// The requested timeframe in seconds.
        timeframe_secs: u64,
    },
    /// The model bundle could not be written.
    Model(ModelBundleError),
}

impl fmt::Display for ModelReportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(error) => error.fmt(formatter),
            Self::Source(error) => error.fmt(formatter),
            Self::SpecRead { path, source } => {
                write!(formatter, "cannot read {}: {source}", path.display())
            }
            Self::Spec(message) => formatter.write_str(message),
            Self::SpecTimeframe { timeframe_secs } => write!(
                formatter,
                "the spec timeframe {timeframe_secs}s is not in the run's data ladder"
            ),
            Self::Model(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for ModelReportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Config(error) => Some(error),
            Self::Source(error) => Some(error),
            Self::SpecRead { source, .. } => Some(source),
            Self::Model(error) => Some(error),
            Self::Spec(_) | Self::SpecTimeframe { .. } => None,
        }
    }
}

/// Runs the model report and writes its bundle.
///
/// # Errors
///
/// Returns [`ModelReportError`] when the configuration, the source, the model,
/// a spec, or an artifact fails.
pub fn run_report(request: &ModelReportRequest<'_>) -> Result<BundleOutput, ModelReportError> {
    let config = RunConfig::load(request.config).map_err(ModelReportError::Config)?;
    let out = request
        .out
        .map_or_else(|| config.output_root.clone(), Path::to_path_buf);

    if let Some(spec_path) = request.spec {
        return run_spec_report(&config, spec_path, &out);
    }

    let model = load_model(&config)?;
    model_bundle_from(&config, &out, &model).map_err(ModelReportError::Model)
}

fn run_spec_report(
    config: &RunConfig,
    spec_path: &Path,
    out: &Path,
) -> Result<BundleOutput, ModelReportError> {
    let text = fs::read_to_string(spec_path).map_err(|source| ModelReportError::SpecRead {
        path: spec_path.to_path_buf(),
        source,
    })?;
    let schema =
        spec::schema_of(&text).map_err(|error| ModelReportError::Spec(error.to_string()))?;
    let spec_hash = crate::fingerprint::fnv1a(text.as_bytes());
    let file = spec_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("spec.json")
        .to_owned();
    match schema.as_str() {
        "quantrule.model-spec/v1" => {
            let parsed = spec::parse_model_spec(&text)
                .map_err(|error| ModelReportError::Spec(error.to_string()))?;
            let timeframes = spec_timeframes(config, &parsed.ladder)?;
            let model = pipeline::run_model_with_ladder(config, &timeframes)
                .map_err(|error| pipeline_error(config, error))?;
            let evaluation = parsed
                .evaluate(&model)
                .map_err(|error| ModelReportError::Spec(error.to_string()))?;
            model_bundle_with_spec(
                config,
                out,
                &model,
                &parsed.ladder,
                &config.model,
                &SpecRunArtifacts::V1 {
                    spec: &parsed,
                    evaluation: &evaluation,
                    file,
                    hash: spec_hash,
                },
            )
            .map_err(ModelReportError::Model)
        }
        "quantrule.model-spec/v2" => {
            let parsed = spec::v2::parse_model_spec_v2(&text)
                .map_err(|error| ModelReportError::Spec(error.to_string()))?;
            let timeframes = spec_timeframes(config, &parsed.spec.timeframes)?;
            let mut spec_config = config.clone();
            spec_config.model = parsed.spec.readings;
            let model = pipeline::run_model_with_ladder(&spec_config, &timeframes)
                .map_err(|error| pipeline_error(config, error))?;
            let panel_series =
                load_panel_series(config, spec_path, &parsed, &model, timeframes[0])?;
            let panel: Vec<PanelInstrument<'_>> = panel_series
                .iter()
                .map(|(name, candles)| PanelInstrument { name, candles })
                .collect();
            let run = SpecRun::compute(&parsed.spec, model, &panel)
                .map_err(|error| ModelReportError::Spec(error.to_string()))?;
            let panel_sources = panel_provenance(spec_path, &parsed, &config.data.data_version);
            model_bundle_with_spec(
                config,
                out,
                run.base(),
                &parsed.spec.timeframes,
                &parsed.spec.readings,
                &SpecRunArtifacts::V2 {
                    spec: &parsed.spec,
                    run: &run,
                    file,
                    hash: spec_hash,
                    panel_sources,
                },
            )
            .map_err(ModelReportError::Model)
        }
        other => Err(ModelReportError::Spec(format!(
            "the schema {other} is not quantrule.model-spec/v1 or /v2"
        ))),
    }
}

fn load_panel_series(
    config: &RunConfig,
    spec_path: &Path,
    parsed: &ParsedModelSpecV2,
    model: &ModelRun,
    timeframe: Timeframe,
) -> Result<Vec<(String, Vec<Candle>)>, ModelReportError> {
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

fn spec_timeframes(config: &RunConfig, ladder: &[u64]) -> Result<Vec<Timeframe>, ModelReportError> {
    ladder
        .iter()
        .map(|timeframe_secs| {
            config
                .data
                .available_timeframes
                .iter()
                .copied()
                .find(|timeframe| u64::from(timeframe.seconds()) == *timeframe_secs)
                .ok_or(ModelReportError::SpecTimeframe {
                    timeframe_secs: *timeframe_secs,
                })
        })
        .collect()
}

fn load_model(config: &RunConfig) -> Result<ModelRun, ModelReportError> {
    pipeline::run_model(config).map_err(|error| pipeline_error(config, error))
}

fn pipeline_error(config: &RunConfig, error: PipelineError) -> ModelReportError {
    match error {
        PipelineError::Source(error) => ModelReportError::Source(error),
        PipelineError::EmptyPeriod => ModelReportError::Model(ModelBundleError::EmptyPeriod {
            period: period_label(config),
        }),
    }
}

fn spec_error(message: impl Into<String>) -> ModelReportError {
    ModelReportError::Spec(message.into())
}
