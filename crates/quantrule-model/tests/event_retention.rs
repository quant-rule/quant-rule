//! End-to-end retention checks over a real model run.
//!
//! The unit tests in `retention.rs` prove the container. This test proves the
//! model actually feeds it: a long run keeps a larger full batch while the
//! published history stays at the newest 1,024 records.

use quantrule_data::Candle;
use quantrule_model::input::{MarketStateConfig, TimeframeInput};
use quantrule_model::retention::EVENT_SNAPSHOT_LIMIT;
use quantrule_model::timeframe::TimeframeModel;

const BARS: usize = 3_000;
const TIMEFRAME_SECS: u64 = 60;
const BASE_EPOCH: i64 = 1_735_689_600;

fn candle(index: usize) -> Candle {
    let phase = index as f64;
    let open = 100.0 + 4.0 * (phase / 5.0).sin() + 0.8 * (phase / 2.0).sin();
    let close = 100.0 + 4.0 * ((phase + 1.0) / 5.0).sin() + 0.8 * ((phase + 1.0) / 2.0).sin();
    let high = open.max(close) + 0.4 + 0.2 * (phase / 3.0).sin().abs();
    let low = open.min(close) - 0.4 - 0.2 * (phase / 3.0).cos().abs();
    Candle::new(
        index as u64,
        BASE_EPOCH + index as i64 * 60,
        open,
        high,
        low,
        close,
        Some(1_000.0),
    )
    .expect("the generated candle is valid")
}

#[test]
fn a_long_run_bounds_the_history_without_dropping_the_batch() {
    let candles: Vec<Candle> = (0..BARS).map(candle).collect();
    let model = TimeframeModel::compute(
        &TimeframeInput {
            timeframe_secs: TIMEFRAME_SECS,
            candles: &candles,
        },
        &MarketStateConfig::default(),
    )
    .expect("the generated series produces a model");

    let batch = model.events();
    let history = model.event_history();
    assert!(
        batch.len() > EVENT_SNAPSHOT_LIMIT,
        "the fixture must produce more than the snapshot limit, got {}",
        batch.len()
    );
    assert_eq!(history.len(), EVENT_SNAPSHOT_LIMIT);
    assert_eq!(
        history.as_slice(),
        &batch[batch.len() - EVENT_SNAPSHOT_LIMIT..],
        "the history is not the newest records in batch order"
    );

    let latest = candles.len() - 1;
    let cutoff = latest.saturating_sub(4);
    let recent = |events: &[quantrule_model::events::EventRecord]| {
        events
            .iter()
            .filter(|event| event.available_bar_index >= cutoff)
            .count()
    };
    let batch_recent = recent(batch);
    let history_recent = recent(history.as_slice());
    assert!(batch_recent > 0, "the fixture has no recent events");
    assert_eq!(
        history_recent, batch_recent,
        "the five-bar window differs between the full batch and the bounded history"
    );
}
