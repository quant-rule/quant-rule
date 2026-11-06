//! The config-driven `report model` command.
//!
//! The command loads the run configuration, computes the flat model over the
//! configured ladder, and writes the bundle through the shared writer. It does
//! not compute a reading, an event, or a progression value.

use std::fs;
use std::io::Write;

use quantrule_model::panel::PanelInstrument;
use quantrule_model::spec::v2::SpecRun;

use super::types::{ReportError, RunError};
use crate::config::RunConfig;
use crate::pipeline;
use crate::request::ReportArgs;

mod bundle;
mod model_artifacts;
mod parse;
mod provenance;
mod spec;

pub(crate) use bundle::model_bundle_from;
use bundle::{BundleOutput, SpecRunArtifacts, model_bundle_with_spec};
pub(crate) use parse::parse_report_arguments;
use provenance::panel_provenance;
use spec::{load_model, load_panel_series, pipeline_error, spec_timeframes};

/// Loads the run configuration and writes the model bundle.
///
/// # Errors
///
/// Returns a [`RunError`] when the configuration, the source, the model, a spec,
/// or an artifact fails.
pub(crate) fn run_report(args: &ReportArgs, writer: &mut impl Write) -> Result<(), RunError> {
    let config = RunConfig::load(&args.config).map_err(RunError::Config)?;
    let out = args
        .out
        .clone()
        .unwrap_or_else(|| config.output_root.clone());

    let bundle = if let Some(spec_path) = &args.spec {
        let text = fs::read_to_string(spec_path).map_err(|source| {
            RunError::Report(ReportError::SpecRead {
                path: spec_path.clone(),
                source,
            })
        })?;
        let schema = crate::spec::schema_of(&text)
            .map_err(|error| RunError::Report(ReportError::Spec(error.to_string())))?;
        let spec_hash = crate::fingerprint::fnv1a(text.as_bytes());
        let file = spec_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("spec.json")
            .to_owned();
        match schema.as_str() {
            "quantrule.model-spec/v1" => {
                let spec = crate::spec::parse_model_spec(&text)
                    .map_err(|error| RunError::Report(ReportError::Spec(error.to_string())))?;
                let timeframes = spec_timeframes(&config, &spec.ladder)?;
                let model = pipeline::run_model_with_ladder(&config, &timeframes)
                    .map_err(|error| pipeline_error(&config, error))?;
                let evaluation = spec
                    .evaluate(&model)
                    .map_err(|error| RunError::Report(ReportError::Spec(error.to_string())))?;
                model_bundle_with_spec(
                    &config,
                    &out,
                    &model,
                    &spec.ladder,
                    &config.model,
                    &SpecRunArtifacts::V1 {
                        spec: &spec,
                        evaluation: &evaluation,
                        file,
                        hash: spec_hash,
                    },
                )?
            }
            "quantrule.model-spec/v2" => {
                let parsed = crate::spec::v2::parse_model_spec_v2(&text)
                    .map_err(|error| RunError::Report(ReportError::Spec(error.to_string())))?;
                let timeframes = spec_timeframes(&config, &parsed.spec.timeframes)?;
                let mut spec_config = config.clone();
                spec_config.model = parsed.spec.readings;
                let model = pipeline::run_model_with_ladder(&spec_config, &timeframes)
                    .map_err(|error| pipeline_error(&config, error))?;
                let panel_series =
                    load_panel_series(&config, spec_path, &parsed, &model, timeframes[0])?;
                let panel: Vec<PanelInstrument<'_>> = panel_series
                    .iter()
                    .map(|(name, candles)| PanelInstrument { name, candles })
                    .collect();
                let run = SpecRun::compute(&parsed.spec, model, &panel)
                    .map_err(|error| RunError::Report(ReportError::Spec(error.to_string())))?;
                let panel_sources = panel_provenance(spec_path, &parsed, &config.data.data_version);
                model_bundle_with_spec(
                    &config,
                    &out,
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
                )?
            }
            other => {
                return Err(RunError::Report(ReportError::Spec(format!(
                    "the schema {other} is not quantrule.model-spec/v1 or /v2"
                ))));
            }
        }
    } else {
        let model = load_model(&config)?;
        model_bundle_from(&config, &out, &model)?
    };
    emit(writer, args.json, &bundle)
}

pub(super) fn period_label(config: &RunConfig) -> String {
    match (&config.data.from, &config.data.to) {
        (Some(from), Some(to)) => format!("{from}..{to}"),
        (Some(from), None) => format!("{from}..open"),
        (None, Some(to)) => format!("open..{to}"),
        (None, None) => String::from("full file"),
    }
}

fn emit(writer: &mut impl Write, json_stdout: bool, bundle: &BundleOutput) -> Result<(), RunError> {
    if json_stdout {
        writer.write_all(&bundle.json).map_err(RunError::Output)?;
    } else {
        for name in &bundle.names {
            writeln!(writer, "{}", bundle.path.join(name).display()).map_err(RunError::Output)?;
        }
    }
    writer.flush().map_err(RunError::Output)
}
