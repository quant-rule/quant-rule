//! Model-bundle assembly shared by the report and backtest commands.
//!
//! The module computes the flat report, renders every artifact, and writes the
//! bundle. It does not parse arguments or choose the run input.

use std::path::{Path, PathBuf};

use quantrule_data::Timeframe;
use quantrule_model::input::MarketStateConfig;
use quantrule_model::report::FlatReport;
use quantrule_model::run::ModelRun;
use quantrule_model::spec::v2::{ModelSpecV2, SpecRun};
use quantrule_model::spec::{ModelSpec, SpecEvaluation};
use serde_json::{Value, json};

use super::error::ModelBundleError;
use super::provenance::{config_files, spec_provenance, spec_v2_provenance};
use super::{artifacts, period_label};
use crate::artifact::render_report::{self, Provenance};
use crate::config::RunConfig;
use crate::pipeline;

/// The paths and bytes of a written model bundle.
pub struct BundleOutput {
    /// The bundle root.
    pub path: PathBuf,
    /// The artifact names written under the root.
    pub names: Vec<String>,
    /// The result envelope for `--json` output.
    pub json: Vec<u8>,
}

/// Writes a model bundle from a completed model run.
///
/// # Errors
///
/// Returns [`ModelBundleError`] when the report or an artifact fails.
pub fn model_bundle_from(
    config: &RunConfig,
    out: &Path,
    model_run: &ModelRun,
) -> Result<BundleOutput, ModelBundleError> {
    let ladder = pipeline::ladder_secs_for(&config.data.available_timeframes);
    model_bundle_with(config, out, model_run, &ladder, &config.model, None)
}

/// The spec artifacts a model bundle publishes.
pub enum SpecRunArtifacts<'a> {
    /// v1 conditions only.
    V1 {
        /// The parsed v1 spec.
        spec: &'a ModelSpec,
        /// The per-condition evaluation.
        evaluation: &'a SpecEvaluation,
        /// The spec file name as written.
        file: String,
        /// The spec content fingerprint.
        hash: String,
    },
    /// v2 prediction and optional conditions.
    V2 {
        /// The parsed v2 spec.
        spec: &'a ModelSpecV2,
        /// The completed v2 run.
        run: &'a SpecRun,
        /// The spec file name as written.
        file: String,
        /// The spec content fingerprint.
        hash: String,
        /// The panel source identities, including the evaluated instrument.
        panel_sources: Vec<Value>,
    },
}

/// Writes the model bundle for a spec run.
///
/// # Errors
///
/// Returns a [`ModelBundleError`] when the report or an artifact fails.
pub fn model_bundle_with_spec(
    config: &RunConfig,
    out: &Path,
    model_run: &ModelRun,
    ladder: &[u64],
    effective_model: &MarketStateConfig,
    spec: &SpecRunArtifacts<'_>,
) -> Result<BundleOutput, ModelBundleError> {
    model_bundle_with(config, out, model_run, ladder, effective_model, Some(spec))
}

fn model_bundle_with(
    config: &RunConfig,
    out: &Path,
    model_run: &ModelRun,
    ladder: &[u64],
    effective_model: &MarketStateConfig,
    spec: Option<&SpecRunArtifacts<'_>>,
) -> Result<BundleOutput, ModelBundleError> {
    let bundle = out.join("model");
    render_report::ensure_available(&bundle).map_err(ModelBundleError::Bundle)?;

    let report = FlatReport::compute_with_session(model_run, ladder, &config.data.session);
    if report
        .timeframes
        .iter()
        .all(|timeframe| timeframe.bars == 0)
    {
        return Err(ModelBundleError::EmptyPeriod {
            period: period_label(config),
        });
    }
    let first_open_time = report
        .timeframes
        .iter()
        .filter_map(|timeframe| timeframe.first_open_time)
        .min();
    let last_open_time = report
        .timeframes
        .iter()
        .filter_map(|timeframe| timeframe.last_open_time)
        .max();

    let summary =
        render_report::summary_bytes(&report, &config.data.data_version, &period_label(config))
            .map_err(ModelBundleError::Output)?;
    let json = render_report::json_bytes(
        &report,
        &config.data.data_version,
        config.data.from.as_deref(),
        config.data.to.as_deref(),
    )
    .map_err(ModelBundleError::Output)?;
    let result = render_report::model_result_bytes(
        &report,
        ladder,
        &config.data.data_version,
        config.data.from.as_deref(),
        config.data.to.as_deref(),
        &crate::config::effective_json(effective_model),
    )
    .map_err(ModelBundleError::Output)?;
    let readings_csv = artifacts::readings_csv(model_run, ladder);
    let events_csv = match spec {
        Some(SpecRunArtifacts::V2 { run, .. }) => {
            artifacts::spec_events_csv(run, run.active_events())
        }
        _ => artifacts::events_csv(model_run, ladder),
    };
    let levels_csv = artifacts::levels_csv(model_run, ladder);
    let conditions_csv = match spec {
        Some(SpecRunArtifacts::V1 { evaluation, .. }) => {
            Some(artifacts::conditions_csv(evaluation, model_run))
        }
        Some(SpecRunArtifacts::V2 { run, .. }) => run
            .conditions()
            .map(|evaluation| artifacts::conditions_csv(evaluation, model_run)),
        None => None,
    };
    let prediction_csv = match spec {
        Some(SpecRunArtifacts::V2 { run, .. }) => Some(artifacts::prediction_csv(run)),
        _ => None,
    };
    let panel_csv = match spec {
        Some(SpecRunArtifacts::V2 { spec, run, .. }) => run.panel_readings().map(|readings| {
            artifacts::panel_csv(
                readings,
                spec.panel_instruments.first().map_or("", String::as_str),
            )
        }),
        _ => None,
    };
    let session_csv = match spec {
        Some(SpecRunArtifacts::V2 { run, .. }) => run.session_readings().map(|readings| {
            let candles = run
                .base()
                .timeframe(run.evaluation_timeframe_secs())
                .map_or(&[][..], |timeframe| timeframe.candles());
            artifacts::session_csv(candles, readings)
        }),
        _ => None,
    };
    let legs_csv = artifacts::legs_csv(model_run, ladder);

    let mut name_list = vec![
        "summary.txt",
        "report.json",
        "result.json",
        "readings.csv",
        "events.csv",
        "levels.csv",
        "legs.csv",
        "provenance.json",
    ];
    if conditions_csv.is_some() {
        name_list.push("conditions.csv");
    }
    if prediction_csv.is_some() {
        name_list.push("prediction.csv");
    }
    if panel_csv.is_some() {
        name_list.push("panel.csv");
    }
    if session_csv.is_some() {
        name_list.push("session.csv");
    }
    let names = artifact_names(&name_list);
    let files = config_files(config);
    let spec_value = spec.map(|spec| match spec {
        SpecRunArtifacts::V1 {
            spec,
            evaluation,
            file,
            hash,
        } => spec_provenance(spec, evaluation, file, hash),
        SpecRunArtifacts::V2 {
            spec,
            run,
            file,
            hash,
            panel_sources,
        } => spec_v2_provenance(spec, run, file, hash, panel_sources),
    });
    let provenance = render_report::provenance_bytes(&Provenance {
        report: "model",
        data_version: &config.data.data_version,
        config_hash: &config.config_hash,
        from: config.data.from.as_deref(),
        to: config.data.to.as_deref(),
        first_open_time,
        last_open_time,
        config_files: &files,
        config: json!({
            "model_id": config.model_id,
            "execution_id": config.execution_id,
            "portfolio_id": config.portfolio_id,
            "model": crate::config::effective_json(effective_model),
            "ladder": ladder.iter().map(|seconds| timeframe_label(*seconds)).collect::<Vec<_>>(),
        }),
        spec: spec_value,
        artifacts: &names,
    })
    .map_err(ModelBundleError::Output)?;

    let mut artifacts = vec![
        ("summary.txt", summary),
        ("report.json", json.clone()),
        ("result.json", result),
        ("readings.csv", readings_csv.into_bytes()),
        ("events.csv", events_csv.into_bytes()),
        ("levels.csv", levels_csv.into_bytes()),
        ("legs.csv", legs_csv.into_bytes()),
    ];
    if let Some(conditions) = conditions_csv {
        artifacts.push(("conditions.csv", conditions.into_bytes()));
    }
    if let Some(prediction) = prediction_csv {
        artifacts.push(("prediction.csv", prediction.into_bytes()));
    }
    if let Some(panel) = panel_csv {
        artifacts.push(("panel.csv", panel.into_bytes()));
    }
    if let Some(session) = session_csv {
        artifacts.push(("session.csv", session.into_bytes()));
    }
    render_report::write_bundle(&bundle, &artifacts, &provenance)
        .map_err(ModelBundleError::Bundle)?;
    Ok(BundleOutput {
        path: bundle,
        names,
        json,
    })
}

fn artifact_names(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

fn timeframe_label(timeframe_secs: u64) -> &'static str {
    Timeframe::ALL
        .iter()
        .find(|timeframe| u64::from(timeframe.seconds()) == timeframe_secs)
        .map_or("15m", |timeframe| timeframe.code())
}
