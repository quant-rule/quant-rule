//! Determinism under a reordered ladder.
//!
//! The model publishes one block per timeframe. Reordering the input slice
//! must not change a reading or an event for either timeframe: the output is
//! keyed by the timeframe, not by the position in the caller's vector.

use quantrule_data::Candle;
use quantrule_model::input::{MarketStateConfig, TimeframeInput};
use quantrule_model::run::ModelRun;

fn candles() -> Vec<Candle> {
    (0..160)
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
fn the_ladder_order_does_not_change_readings_or_events() {
    let candles = candles();
    let config = MarketStateConfig::default();
    let forward = ModelRun::compute(
        &[
            TimeframeInput::new(300, &candles),
            TimeframeInput::new(900, &candles),
        ],
        &config,
    );
    let reverse = ModelRun::compute(
        &[
            TimeframeInput::new(900, &candles),
            TimeframeInput::new(300, &candles),
        ],
        &config,
    );

    for seconds in [300, 900] {
        let left = forward.timeframe(seconds).expect("the forward timeframe");
        let right = reverse.timeframe(seconds).expect("the reverse timeframe");
        assert_eq!(left.readings().ema20, right.readings().ema20);
        assert_eq!(left.readings().rsi14, right.readings().rsi14);
        assert_eq!(left.all_events().len(), right.all_events().len());
        for (left, right) in left.all_events().iter().zip(right.all_events()) {
            assert_eq!(left.event_code, right.event_code);
            assert_eq!(left.occurred_bar_index, right.occurred_bar_index);
            assert_eq!(left.available_bar_index, right.available_bar_index);
        }
    }
}
