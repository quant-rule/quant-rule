//! The research-plane bundle derived from a completed model run.
//!
//! This is a projection, not a second model. It carries the bars, events,
//! confirmed levels, and event catalog the simulation service reads. A
//! timeframe with a missing candle volume is omitted: the simulation bar has a
//! numeric volume, and a zero-fill would fabricate a missing input.

use std::collections::BTreeMap;

use quantrule_api::simulation::BUNDLE_SCHEMA;
use quantrule_model::readings::{PivotKind, PivotScale};
use quantrule_model::run::ModelRun;
use serde_json::{Value, json};

use super::artifacts::{ema_stack, event_completeness, rsi_zone};
use crate::artifact::labels::event_name;

/// Build one simulation bundle for a ladder timeframe, or `None` when the
/// timeframe is absent or a bar carries no volume.
pub(super) fn simulation_json(
    run: &ModelRun,
    timeframe_secs: u64,
    data_version: &str,
    config_hash: &str,
    model_id: Option<&str>,
) -> Option<String> {
    let timeframe = run.timeframe(timeframe_secs)?;
    let readings = timeframe.readings();
    let extended = timeframe.extended();
    let adx = timeframe.adx();
    let mut bars = Vec::with_capacity(timeframe.len());
    for bar in 0..timeframe.len() {
        let candle = timeframe.candle(bar)?;
        let ema_fast = readings.ema20.get(bar);
        let ema_mid = readings.ema50.get(bar);
        let ema_slow = readings.ema200.get(bar);
        let rsi = readings.rsi14.get(bar);
        let context = timeframe.context().get(bar);
        bars.push(json!({
            "bar": bar,
            "timestamp": candle.timestamp(),
            "open": candle.open(),
            "high": candle.high(),
            "low": candle.low(),
            "close": candle.close(),
            "volume": candle.volume()?,
            "atr": readings.atr14.get(bar),
            "ema_fast": ema_fast,
            "ema_mid": ema_mid,
            "ema_slow": ema_slow,
            "rsi": rsi,
            "rvol": readings.rvol20.get(bar),
            "adx": adx.adx.get(bar),
            "ema_stack": ema_stack(ema_fast, ema_mid, ema_slow).map(i32::from),
            "rsi_zone": rsi.map(rsi_zone).map(i32::from),
            "atr_ratio": context.and_then(|context| context.atr_ratio),
            "close_position": extended.close_position.get(bar),
            "upper_wick": extended.upper_wick_ratio.get(bar),
            "lower_wick": extended.lower_wick_ratio.get(bar),
            "body_ratio": extended.body_ratio.get(bar),
            "last_pivot_low": extended.last_pivot_low.get(bar),
            "last_pivot_high": extended.last_pivot_high.get(bar),
        }));
    }
    let events: Vec<Value> = timeframe
        .all_events()
        .iter()
        .enumerate()
        .map(|(index, event)| {
            json!({
                "event_index": index,
                "code": event.event_code,
                "type_code": event.event_type.code(),
                "subtype_code": event.event_subtype_code,
                "source_code": event.source_code,
                "occurred_bar": event.occurred_bar_index,
                "available_bar": event.available_bar_index,
                "direction": event.direction_code,
                "reference_code": event.reference_code,
                "primary": event.primary_value,
                "reference": event.reference_value,
                "completeness": event_completeness(event),
            })
        })
        .collect();
    let levels: Vec<Value> = timeframe
        .levels()
        .iter()
        .map(|level| {
            let distance = timeframe.candle(level.available_at).and_then(|candle| {
                readings
                    .atr14
                    .get(level.available_at)
                    .filter(|atr| *atr > 0.0)
                    .map(|atr| (candle.close() - level.price) / atr)
            });
            let kind = match level.kind {
                PivotKind::High => 1,
                PivotKind::Low => -1,
            };
            let scale = match level.scale {
                PivotScale::Minor => 1,
                PivotScale::Major => 2,
            };
            json!({
                "reference_code": level.reference_code,
                "scale_code": scale,
                "kind_code": kind,
                "price": level.price,
                "occurred_bar": level.occurred_at,
                "available_bar": level.available_at,
                "distance_atr": distance,
            })
        })
        .collect();
    let mut catalog = BTreeMap::new();
    for event in timeframe.all_events() {
        catalog.insert(event.event_code, event_name(event.event_code));
    }
    let provenance = BTreeMap::from([
        ("data_version".to_owned(), data_version.to_owned()),
        ("config_hash".to_owned(), config_hash.to_owned()),
        (
            "model_version".to_owned(),
            model_id.unwrap_or("unknown").to_owned(),
        ),
        ("timeframe".to_owned(), timeframe_secs.to_string()),
    ]);
    serde_json::to_string(&json!({
        "schema": BUNDLE_SCHEMA,
        "timeframe_secs": timeframe_secs,
        "bars": bars,
        "events": events,
        "levels": levels,
        "catalog": catalog,
        "provenance": provenance,
    }))
    .ok()
}

#[cfg(test)]
mod tests {
    use quantrule_data::Candle;
    use quantrule_model::input::{MarketStateConfig, TimeframeInput};

    use super::*;

    fn candles() -> Vec<Candle> {
        (0..90)
            .map(|index| {
                let close = 100.0 + (index as f64 * 0.37).sin() * 3.0;
                Candle::new(
                    index as u64,
                    index as i64 * 300,
                    close,
                    close + 1.0,
                    close - 1.0,
                    close,
                    Some(1.0 + index as f64),
                )
                .expect("the test candle is valid")
            })
            .collect()
    }

    #[test]
    fn the_bundle_carries_bars_events_and_the_catalog() {
        let candles = candles();
        let run = ModelRun::compute(
            &[TimeframeInput::new(300, &candles)],
            &MarketStateConfig::default(),
        );
        let text = simulation_json(&run, 300, "sha256:data", "sha256:config", Some("M1.3"))
            .expect("the bundle is produced");
        let value: Value = serde_json::from_str(&text).expect("the bundle is JSON");
        assert_eq!(value["schema"], BUNDLE_SCHEMA);
        assert_eq!(value["timeframe_secs"], 300);
        assert_eq!(value["bars"].as_array().expect("bars").len(), 90);
        assert!(value["bars"][0].get("atr").is_some());
        assert!(value["catalog"].is_object());
        assert_eq!(value["provenance"]["model_version"], "M1.3");
    }

    #[test]
    fn a_missing_volume_omits_the_timeframe() {
        let mut candles = candles();
        for candle in &mut candles {
            *candle = Candle::new(
                candle.series_index(),
                candle.timestamp(),
                candle.open(),
                candle.high(),
                candle.low(),
                candle.close(),
                None,
            )
            .expect("the test candle is valid");
        }
        let run = ModelRun::compute(
            &[TimeframeInput::new(300, &candles)],
            &MarketStateConfig::default(),
        );
        assert!(
            simulation_json(&run, 300, "sha256:data", "sha256:config", None).is_none(),
            "a missing volume must not become zero"
        );
    }
}
