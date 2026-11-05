//! The config-driven `report model` command.
//!
//! The command loads the run configuration, computes the flat model over the
//! configured ladder, and writes the bundle through the shared writer. It does
//! not compute a reading, an event, or a progression value.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use quantrule_data::{Candle, Timeframe};
use quantrule_model::input::MarketStateConfig;
use quantrule_model::panel::PanelInstrument;
use quantrule_model::report::FlatReport;
use quantrule_model::run::ModelRun;
use quantrule_model::spec::v2::{ModelSpecV2, SpecRun};
use quantrule_model::spec::{ModelSpec, SpecEvaluation};
use serde_json::{Value, json};

mod model_artifacts;

use super::types::{ParseError, ReportArgs, ReportError, RunError};
use crate::config::RunConfig;
use crate::pipeline::{self, PipelineError};
use crate::render_report::{self, Provenance};

/// Parses the already-split arguments of `report`.
///
/// # Errors
///
/// Returns [`ParseError::Report`] for a missing `model` engine, a missing or
/// repeated `--config`/`--out`, an unknown option, or an extra positional
/// argument.
pub(crate) fn parse_report_arguments(arguments: &[String]) -> Result<ReportArgs, ParseError> {
    const USAGE: &str =
        "usage: report model --config <run.json> [--spec <spec.json>] [--out <dir>] [--json]";
    let mut engine = None;
    let mut config = None;
    let mut spec = None;
    let mut out = None;
    let mut json = false;
    let mut index = 0;

    while index < arguments.len() {
        match arguments[index].as_str() {
            "--config" => {
                if config.is_some() {
                    return Err(ParseError::Report(format!(
                        "--config was supplied twice\n{USAGE}"
                    )));
                }
                index += 1;
                let value = arguments
                    .get(index)
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| {
                        ParseError::Report(format!("--config requires a non-empty path\n{USAGE}"))
                    })?;
                config = Some(PathBuf::from(value));
            }
            "--spec" => {
                if spec.is_some() {
                    return Err(ParseError::Report(format!(
                        "--spec was supplied twice\n{USAGE}"
                    )));
                }
                index += 1;
                let value = arguments
                    .get(index)
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| {
                        ParseError::Report(format!("--spec requires a non-empty path\n{USAGE}"))
                    })?;
                spec = Some(PathBuf::from(value));
            }
            "--out" => {
                if out.is_some() {
                    return Err(ParseError::Report(format!(
                        "--out was supplied twice\n{USAGE}"
                    )));
                }
                index += 1;
                let value = arguments
                    .get(index)
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| {
                        ParseError::Report(format!("--out requires a non-empty path\n{USAGE}"))
                    })?;
                out = Some(PathBuf::from(value));
            }
            "--json" => {
                if json {
                    return Err(ParseError::Report(format!(
                        "--json was supplied twice\n{USAGE}"
                    )));
                }
                json = true;
            }
            option if option.starts_with("--") => {
                return Err(ParseError::Report(format!(
                    "unknown option '{option}'\n{USAGE}"
                )));
            }
            value => {
                if engine.is_some() {
                    return Err(ParseError::Report(format!(
                        "unexpected argument '{value}'\n{USAGE}"
                    )));
                }
                if value != "model" {
                    return Err(ParseError::Report(format!(
                        "unknown report engine '{value}'; use model\n{USAGE}"
                    )));
                }
                engine = Some(value);
            }
        }
        index += 1;
    }

    let engine =
        engine.ok_or_else(|| ParseError::Report(format!("missing report engine\n{USAGE}")))?;
    let config =
        config.ok_or_else(|| ParseError::Report(format!("--config is required\n{USAGE}")))?;
    let _ = engine;

    Ok(ReportArgs {
        config,
        spec,
        out,
        json,
    })
}

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
        let spec_hash = content_hash(text.as_bytes());
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

/// One written bundle and the bytes of its `report.json`.
pub(super) struct BundleOutput {
    pub(super) path: PathBuf,
    pub(super) names: Vec<String>,
    pub(super) json: Vec<u8>,
}

pub(super) fn model_bundle_from(
    config: &RunConfig,
    out: &Path,
    model_run: &ModelRun,
) -> Result<BundleOutput, RunError> {
    let ladder = pipeline::ladder_secs_for(&config.data.available_timeframes);
    model_bundle_with(config, out, model_run, &ladder, &config.model, None)
}

/// The spec artifacts a model bundle publishes.
pub(super) enum SpecRunArtifacts<'a> {
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
    },
}

/// Writes the model bundle for a spec run.
///
/// # Errors
///
/// Returns a [`RunError`] when the report or an artifact fails.
pub(super) fn model_bundle_with_spec(
    config: &RunConfig,
    out: &Path,
    model_run: &ModelRun,
    ladder: &[u64],
    effective_model: &MarketStateConfig,
    spec: &SpecRunArtifacts<'_>,
) -> Result<BundleOutput, RunError> {
    model_bundle_with(config, out, model_run, ladder, effective_model, Some(spec))
}

fn model_bundle_with(
    config: &RunConfig,
    out: &Path,
    model_run: &ModelRun,
    ladder: &[u64],
    effective_model: &MarketStateConfig,
    spec: Option<&SpecRunArtifacts<'_>>,
) -> Result<BundleOutput, RunError> {
    let bundle = out.join("model");
    render_report::ensure_available(&bundle).map_err(RunError::Bundle)?;

    let report = FlatReport::compute(model_run, ladder);
    if report
        .timeframes
        .iter()
        .all(|timeframe| timeframe.bars == 0)
    {
        return Err(RunError::Report(ReportError::EmptyPeriod {
            period: period_label(config),
        }));
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
            .map_err(RunError::Output)?;
    let json = render_report::json_bytes(
        &report,
        &config.data.data_version,
        config.data.from.as_deref(),
        config.data.to.as_deref(),
    )
    .map_err(RunError::Output)?;
    let result = render_report::model_result_bytes(
        &report,
        ladder,
        &config.data.data_version,
        config.data.from.as_deref(),
        config.data.to.as_deref(),
        &crate::config::effective_json(effective_model),
    )
    .map_err(RunError::Output)?;
    let readings_csv = model_artifacts::readings_csv(model_run, ladder);
    let events_csv = match spec {
        Some(SpecRunArtifacts::V2 { run, .. }) => {
            model_artifacts::spec_events_csv(run, run.active_events())
        }
        _ => model_artifacts::events_csv(model_run, ladder),
    };
    let levels_csv = model_artifacts::levels_csv(model_run, ladder);
    let conditions_csv = match spec {
        Some(SpecRunArtifacts::V1 { evaluation, .. }) => {
            Some(model_artifacts::conditions_csv(evaluation, model_run))
        }
        Some(SpecRunArtifacts::V2 { run, .. }) => run
            .conditions()
            .map(|evaluation| model_artifacts::conditions_csv(evaluation, model_run)),
        None => None,
    };
    let prediction_csv = match spec {
        Some(SpecRunArtifacts::V2 { run, .. }) => Some(model_artifacts::prediction_csv(run)),
        _ => None,
    };
    let panel_csv = match spec {
        Some(SpecRunArtifacts::V2 { spec, run, .. }) => run.panel_readings().map(|readings| {
            model_artifacts::panel_csv(
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
            model_artifacts::session_csv(candles, readings)
        }),
        _ => None,
    };
    let legs_csv = model_artifacts::legs_csv(model_run, ladder);

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
        } => spec_v2_provenance(spec, run, file, hash),
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
            "policy_id": config.policy_id,
            "execution_id": config.execution_id,
            "portfolio_id": config.portfolio_id,
            "model": crate::config::effective_json(effective_model),
            "ladder": ladder.iter().map(|seconds| timeframe_label(*seconds)).collect::<Vec<_>>(),
            "policy": policy_value(config),
        }),
        spec: spec_value,
        artifacts: &names,
    })
    .map_err(RunError::Output)?;

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
    render_report::write_bundle(&bundle, &artifacts, &provenance).map_err(RunError::Bundle)?;
    Ok(BundleOutput {
        path: bundle,
        names,
        json,
    })
}

fn spec_provenance(spec: &ModelSpec, evaluation: &SpecEvaluation, file: &str, hash: &str) -> Value {
    json!({
        "schema": spec.schema,
        "id": spec.id,
        "file": file,
        "file_hash": hash,
        "registry_version": quantrule_model::REGISTRY_VERSION,
        "ladder": spec.ladder,
        "evaluation_timeframe": spec.evaluation_timeframe_secs,
        "conditions": evaluation
            .conditions
            .iter()
            .map(|condition| json!({
                "id": condition.id,
                "bars": evaluation.bars,
                "true_count": condition.true_count,
            }))
            .collect::<Vec<Value>>(),
    })
}

fn spec_v2_provenance(spec: &ModelSpecV2, run: &SpecRun, file: &str, hash: &str) -> Value {
    let extended = spec.readings.extended();
    let prediction = run.prediction();
    let long = prediction
        .long_votes
        .iter()
        .filter(|votes| **votes > 0)
        .count();
    let short = prediction
        .short_votes
        .iter()
        .filter(|votes| **votes > 0)
        .count();
    json!({
        "schema": spec.schema,
        "id": spec.id,
        "family": spec.family,
        "instrument": spec.instrument,
        "file": file,
        "file_hash": hash,
        "registry_version": quantrule_model::REGISTRY_VERSION,
        "timeframes": spec.timeframes,
        "evaluation_timeframe": spec.evaluation_timeframe_secs,
        "readings": crate::config::effective_json(&spec.readings),
        "extended": {
            "vwap_window": extended.vwap_window,
            "vwap_band_mad": extended.vwap_band_mad,
            "profile_window": extended.profile_window,
            "profile_bins": extended.profile_bins,
            "profile_value_area": extended.profile_value_area,
            "rvol_fast": extended.rvol_fast,
            "rvol_slow": extended.rvol_slow,
            "volume_oscillator": extended.volume_oscillator,
            "climax_volume": extended.climax_volume,
            "climax_range_atr": extended.climax_range_atr,
            "absorption_volume": extended.absorption_volume,
            "absorption_range_atr": extended.absorption_range_atr,
            "absorption_level_atr": extended.absorption_level_atr,
            "flow_window": extended.flow_window,
            "mfi_window": extended.mfi_window,
            "cmf_threshold": extended.cmf_threshold,
            "atr_slow_window": extended.atr_slow_window,
            "atr_slope_window": extended.atr_slope_window,
            "range_window": extended.range_window,
            "mar_fast": extended.mar_fast,
            "mar_slow": extended.mar_slow,
            "keltner_window": extended.keltner_window,
            "keltner_atr": extended.keltner_atr,
            "donchian_window": extended.donchian_window,
            "level_lookback": extended.level_lookback,
            "level_atr": extended.level_atr,
            "aroon_window": extended.aroon_window,
            "path_window": extended.path_window,
            "candle_body": extended.candle_body,
            "candle_wick": extended.candle_wick,
            "candle_close": extended.candle_close,
            "gap_atr": extended.gap_atr,
        },
        "active_events": spec.active_events.iter().map(|event| event.id.clone()).collect::<Vec<_>>(),
        "prediction": {
            "kind": "vote",
            "long": spec.prediction.long.iter().map(|event| event.id.clone()).collect::<Vec<_>>(),
            "short": spec.prediction.short.iter().map(|event| event.id.clone()).collect::<Vec<_>>(),
            "min_votes": spec.prediction.min_votes,
            "bars": prediction.len(),
            "long_bars": long,
            "short_bars": short,
            "flat_bars": prediction.values.iter().filter(|value| **value == 0).count(),
        },
        "panel": spec.panel_instruments,
        "session": spec.session.map(|session| json!({
            "zone": session_zone_label(session.zone),
            "start_hour": session.start_hour,
            "end_hour": session.end_hour,
            "opening_range_bars": session.opening_range_bars,
        })),
        "session_events": run.session_events().len(),
        "panel_events": run.panel_events().len(),
        "conditions": run.conditions().map(|evaluation| evaluation
            .conditions
            .iter()
            .map(|condition| json!({
                "id": condition.id,
                "bars": evaluation.bars,
                "true_count": condition.true_count,
            }))
            .collect::<Vec<Value>>()),
    })
}

fn content_hash(bytes: &[u8]) -> String {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = OFFSET;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(PRIME);
    }
    format!("{hash:016x}")
}

fn session_zone_label(zone: quantrule_model::sessions::SessionZone) -> &'static str {
    match zone {
        quantrule_model::sessions::SessionZone::Utc => "utc",
        quantrule_model::sessions::SessionZone::London => "london",
        quantrule_model::sessions::SessionZone::NewYork => "new_york",
    }
}

fn load_panel_series(
    config: &RunConfig,
    spec_path: &Path,
    parsed: &crate::spec::v2::ParsedModelSpecV2,
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

fn resolve_spec_path(spec_path: &Path, source: &str) -> PathBuf {
    let path = PathBuf::from(source);
    if path.is_absolute() {
        path
    } else {
        spec_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(path)
    }
}

fn spec_error(message: impl Into<String>) -> RunError {
    RunError::Report(ReportError::Spec(message.into()))
}

fn spec_timeframes(config: &RunConfig, ladder: &[u64]) -> Result<Vec<Timeframe>, RunError> {
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

fn load_model(config: &RunConfig) -> Result<ModelRun, RunError> {
    pipeline::run_model(config).map_err(|error| pipeline_error(config, error))
}

fn pipeline_error(config: &RunConfig, error: PipelineError) -> RunError {
    match error {
        PipelineError::Source(error) => RunError::Source(error),
        PipelineError::EmptyPeriod => RunError::Report(ReportError::EmptyPeriod {
            period: period_label(config),
        }),
    }
}

fn policy_value(config: &RunConfig) -> Value {
    json!({
        "account": {
            "currency": config.policy.account.currency,
            "initial_cash": config.policy.account.initial_cash,
        },
        "objective": {
            "performance_target": config.policy.objective.performance_target,
            "evaluation_days": config.policy.objective.evaluation_days,
            "max_drawdown": config.policy.objective.max_drawdown,
        },
        "risk": {
            "max_position_fraction": config.policy.risk.max_position_fraction,
            "max_leverage": config.policy.risk.max_leverage,
            "max_open_positions": config.policy.risk.max_open_positions,
        },
        "universe": {
            "tickets": config.policy.universe.tickets,
            "timeframes": config.policy.universe.timeframes.iter().map(|timeframe| timeframe.code()).collect::<Vec<_>>(),
        },
    })
}

fn period_label(config: &RunConfig) -> String {
    match (&config.data.from, &config.data.to) {
        (Some(from), Some(to)) => format!("{from}..{to}"),
        (Some(from), None) => format!("{from}..open"),
        (None, Some(to)) => format!("open..{to}"),
        (None, None) => String::from("full file"),
    }
}

fn config_files(config: &RunConfig) -> Vec<String> {
    [
        &config.paths.run,
        &config.paths.policy,
        &config.paths.data,
        &config.paths.model,
        &config.paths.execution,
        &config.paths.portfolio,
    ]
    .into_iter()
    .map(|path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("config.json")
            .to_owned()
    })
    .collect()
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

#[cfg(test)]
mod tests {
    use super::parse_report_arguments;

    fn arguments(tokens: &[&str]) -> Vec<String> {
        tokens.iter().map(|token| (*token).to_owned()).collect()
    }

    #[test]
    fn parses_the_model_shape() {
        let parsed = parse_report_arguments(&arguments(&[
            "model",
            "--config",
            "run.json",
            "--out",
            "target/report",
        ]))
        .expect("the shape is valid");
        assert_eq!(parsed.config.to_str(), Some("run.json"));
        assert_eq!(
            parsed.out.as_deref().and_then(|path| path.to_str()),
            Some("target/report")
        );
    }

    #[test]
    fn rejects_other_engines_and_missing_config() {
        assert!(parse_report_arguments(&arguments(&["strategy", "--config", "run.json"])).is_err());
        assert!(parse_report_arguments(&arguments(&["model"])).is_err());
        assert!(parse_report_arguments(&arguments(&["model", "--config"])).is_err());
    }
}
