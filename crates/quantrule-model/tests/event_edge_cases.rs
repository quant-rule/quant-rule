//! Event-level edge cases.
//!
//! The reading edge suite proves the readings are finite or null on degenerate
//! input. This suite proves the event batches are also safe: no panic, every
//! occurrence stays inside the series, availability is never earlier than the
//! occurrence, and every published value is finite.
//!
//! The test constructs the model, session, and panel event families so an
//! event-edge claim cannot point at a readings-only test.

use quantrule_data::Candle;
use quantrule_model::events::EventRecord;
use quantrule_model::events::panel::PanelEventSeries;
use quantrule_model::events::session::SessionEventSeries;
use quantrule_model::input::{MarketStateConfig, TimeframeInput};
use quantrule_model::panel::{PanelConfig, PanelInstrument, PanelReadings};
use quantrule_model::readings::{Readings, ReadingsConfig};
use quantrule_model::sessions::{SessionConfig, SessionReadings};
use quantrule_model::timeframe::TimeframeModel;

const TIMEFRAME_SECS: u64 = 300;

fn candle(index: u64, high: f64, low: f64, close: f64, volume: Option<f64>) -> Candle {
    Candle::new(
        index,
        index as i64 * TIMEFRAME_SECS as i64,
        close,
        high,
        low,
        close,
        volume,
    )
    .expect("the edge candle is valid")
}

fn rising(count: u64, volume: Option<f64>) -> Vec<Candle> {
    (0..count)
        .map(|index| {
            let close = 100.0 + index as f64 * 0.1;
            candle(index, close + 0.5, close - 0.5, close, volume)
        })
        .collect()
}

fn flat(count: u64, volume: Option<f64>) -> Vec<Candle> {
    (0..count)
        .map(|index| candle(index, 100.0, 100.0, 100.0, volume))
        .collect()
}

fn gapped() -> Vec<Candle> {
    let mut candles = rising(40, Some(1.0));
    let close = candles[20].close() + 10.0;
    candles[20] = candle(20, close + 0.5, close - 0.5, close, Some(1.0));
    candles
}

fn assert_batch(events: &[EventRecord], bars: usize) {
    for event in events {
        assert!(
            event.occurred_bar_index < bars,
            "an event occurred on bar {} but the series has {bars} bars",
            event.occurred_bar_index
        );
        assert!(
            event.available_bar_index >= event.occurred_bar_index,
            "an event became available before it occurred: {event:?}"
        );
        assert!(
            event.available_bar_index < bars,
            "an event became available on bar {} but the series has {bars} bars",
            event.available_bar_index
        );
        assert!(event.timeframe_secs > 0, "an event has no timeframe");
        for value in [event.primary_value, event.reference_value]
            .into_iter()
            .flatten()
        {
            assert!(value.is_finite(), "an event published a non-finite value");
        }
    }
}

fn check_series(name: &str, candles: &[Candle]) {
    let bars = candles.len();
    if bars == 0 {
        assert!(
            TimeframeModel::compute(
                &TimeframeInput::new(TIMEFRAME_SECS, candles),
                &MarketStateConfig::default(),
            )
            .is_none(),
            "{name}: an empty series produced a model"
        );
    } else {
        let model = TimeframeModel::compute(
            &TimeframeInput::new(TIMEFRAME_SECS, candles),
            &MarketStateConfig::default(),
        )
        .unwrap_or_else(|| panic!("{name}: the model is absent"));
        assert_batch(model.events(), bars);
        assert_batch(model.extended_events().events(), bars);
    }

    let readings = Readings::compute_with_config(candles, &ReadingsConfig::DEFAULT);
    let sessions = SessionReadings::compute(candles, &readings, &SessionConfig::london());
    let session_events = SessionEventSeries::compute(
        candles,
        &readings,
        &sessions,
        &SessionConfig::london(),
        TIMEFRAME_SECS,
    );
    assert_batch(session_events.events(), bars);

    let panel = [
        PanelInstrument { name: "A", candles },
        PanelInstrument { name: "B", candles },
    ];
    let panel_readings = PanelReadings::compute(&panel, &PanelConfig::DEFAULT);
    let panel_events = PanelEventSeries::compute(&panel, &panel_readings, TIMEFRAME_SECS);
    for instrument in panel_readings.instruments.keys() {
        assert_batch(panel_events.events(instrument), bars);
    }
}

#[test]
fn event_batches_are_safe_on_edge_inputs() {
    check_series("empty", &[]);
    check_series("single", &rising(1, Some(1.0)));
    check_series("short", &rising(5, Some(1.0)));
    check_series("flat", &flat(40, Some(1.0)));
    check_series("zero_volume", &rising(40, Some(0.0)));
    check_series("missing_volume", &rising(40, None));
    check_series("equal_ties", &flat(40, Some(1.0)));
    check_series("gap", &gapped());
}
