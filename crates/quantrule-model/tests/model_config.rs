//! Per-timeframe model configuration tests.

use quantrule_data::{Candle, Timeframe};
use quantrule_model::input::{MarketStateConfig, TimeframeInput};
use quantrule_model::readings::ReadingsConfig;
use quantrule_model::timeframe::TimeframeModel;

fn candles() -> Vec<Candle> {
    (0..30)
        .map(|index| {
            let close = 100.0 + index as f64;
            Candle::new(
                index as u64,
                index as i64 * 60,
                close,
                close + 1.0,
                close - 1.0,
                close,
                Some(10.0),
            )
            .expect("the test candle is valid")
        })
        .collect()
}

#[test]
fn a_timeframe_override_changes_only_that_timeframe() {
    let candles = candles();
    let default = MarketStateConfig::default();
    let mut configured = MarketStateConfig::default();
    configured.set_readings(Timeframe::M5, ReadingsConfig::new(3, 5, 8, 2, 3, 4));

    let input_m5 = TimeframeInput {
        timeframe_secs: u64::from(Timeframe::M5.seconds()),
        candles: &candles,
    };
    let input_m15 = TimeframeInput {
        timeframe_secs: u64::from(Timeframe::M15.seconds()),
        candles: &candles,
    };

    let default_m5 = TimeframeModel::compute(&input_m5, &default).expect("M5 computes");
    let configured_m5 = TimeframeModel::compute(&input_m5, &configured).expect("M5 computes");
    let default_m15 = TimeframeModel::compute(&input_m15, &default).expect("M15 computes");
    let configured_m15 = TimeframeModel::compute(&input_m15, &configured).expect("M15 computes");

    assert_eq!(default_m5.bar(2).expect("bar 2").readings.ema20, None);
    assert!(
        configured_m5
            .bar(2)
            .expect("bar 2")
            .readings
            .ema20
            .is_some()
    );
    assert!(
        configured_m5
            .bar(2)
            .expect("bar 2")
            .readings
            .rsi14
            .is_some()
    );
    assert_eq!(configured_m15, default_m15);
}

#[test]
fn an_unconfigured_timeframe_uses_the_default_readings() {
    let config = MarketStateConfig::default();
    let readings = config.readings_for(u64::from(Timeframe::H4.seconds()));
    assert_eq!(readings, ReadingsConfig::default());
    assert!(!config.is_overridden(Timeframe::H4));
}
