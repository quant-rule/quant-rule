//! Provenance assembly for the model bundle.
//!
//! The module turns the run configuration and the selected spec into the
//! provenance record. It hashes panel sources but does not write the bundle.

use std::fs;
use std::path::{Path, PathBuf};

use quantrule_model::spec::v2::{ModelSpecV2, PredictionKind, SpecRun};
use quantrule_model::spec::{ModelSpec, SpecEvaluation};
use serde_json::{Value, json};

use quantrule_app::config::RunConfig;

pub(super) fn spec_provenance(
    spec: &ModelSpec,
    evaluation: &SpecEvaluation,
    file: &str,
    hash: &str,
) -> Value {
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

pub(super) fn spec_v2_provenance(
    spec: &ModelSpecV2,
    run: &SpecRun,
    file: &str,
    hash: &str,
    panel_sources: &[Value],
) -> Value {
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
    let (kind, long_events, short_events, direction_events) = match spec.prediction.kind {
        PredictionKind::Vote => (
            "vote",
            spec.prediction
                .long
                .iter()
                .map(|event| event.id.clone())
                .collect::<Vec<_>>(),
            spec.prediction
                .short
                .iter()
                .map(|event| event.id.clone())
                .collect::<Vec<_>>(),
            Vec::new(),
        ),
        PredictionKind::Direction => (
            "direction",
            Vec::new(),
            Vec::new(),
            spec.prediction
                .events
                .iter()
                .map(|event| event.id.clone())
                .collect::<Vec<_>>(),
        ),
    };
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
        "readings": quantrule_app::config::effective_json(&spec.readings),
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
            "fib_test_atr": extended.fib_test_atr,
            "band_atr": extended.band_atr,
            "candle_body": extended.candle_body,
            "candle_wick": extended.candle_wick,
            "candle_close": extended.candle_close,
            "gap_atr": extended.gap_atr,
        },
        "active_events": spec.active_events.iter().map(|event| event.id.clone()).collect::<Vec<_>>(),
        "prediction": {
            "kind": kind,
            "long": long_events,
            "short": short_events,
            "events": direction_events,
            "min_votes": spec.prediction.min_votes,
            "bars": prediction.len(),
            "long_bars": long,
            "short_bars": short,
            "flat_bars": prediction.values.iter().filter(|value| **value == 0).count(),
        },
        "panel": spec.panel_instruments,
        "panel_sources": panel_sources,
        "session": spec.session.map(|session| json!({
            "zone": session_zone_label(session.zone),
            "start_hour": session.start_hour,
            "end_hour": session.end_hour,
            "opening_range_bars": session.opening_range_bars,
            "sweep_atr": session.sweep_atr,
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

fn session_zone_label(zone: quantrule_model::sessions::SessionZone) -> &'static str {
    match zone {
        quantrule_model::sessions::SessionZone::Utc => "utc",
        quantrule_model::sessions::SessionZone::London => "london",
        quantrule_model::sessions::SessionZone::NewYork => "new_york",
    }
}

pub(crate) fn panel_provenance(
    spec_path: &Path,
    parsed: &crate::spec::v2::ParsedModelSpecV2,
    data_version: &str,
) -> Vec<Value> {
    parsed
        .spec
        .panel_instruments
        .iter()
        .map(|name| {
            if name == &parsed.spec.instrument {
                json!({
                    "instrument": name,
                    "role": "evaluated_instrument",
                    "source": "run-config",
                    "data_version": data_version,
                    "content_hash": null,
                })
            } else {
                let source = parsed.panel_sources.get(name).map_or("", String::as_str);
                let path = resolve_spec_path(spec_path, source);
                let hash = fs::read(&path)
                    .map(|bytes| quantrule_app::fingerprint::fnv1a(&bytes))
                    .unwrap_or_default();
                json!({
                    "instrument": name,
                    "role": "panel",
                    "path": source,
                    "data_version": format!("content:{hash}"),
                    "content_hash": hash,
                })
            }
        })
        .collect()
}

pub(crate) fn resolve_spec_path(spec_path: &Path, source: &str) -> PathBuf {
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

pub(super) fn config_files(config: &RunConfig) -> Vec<String> {
    [
        &config.paths.run,
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
