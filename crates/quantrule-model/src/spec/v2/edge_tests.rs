use super::{ModelSpecV2, SPEC_V2_SCHEMA, SpecRun, SpecV2Error};
use crate::input::{MarketStateConfig, TimeframeInput};
use crate::run::ModelRun;
use quantrule_data::Candle;

fn candle(index: usize) -> Candle {
    Candle::new(
        index as u64,
        index as i64 * 300,
        100.0,
        101.0,
        99.0,
        100.5,
        Some(1_000.0),
    )
    .expect("the edge candle is valid")
}

fn spec() -> ModelSpecV2 {
    let event = super::SpecEventCode {
        id: "signal.rsi_threshold".to_string(),
        code: crate::events::codes::SIGNAL_RSI_THRESHOLD,
        role: super::SpecEventRole::Signal,
        event_type: crate::events::EventType::Signal,
        subtype: 1,
        source: 4,
    };
    ModelSpecV2 {
        schema: SPEC_V2_SCHEMA.to_string(),
        id: "M2.edge".to_string(),
        family: "M2".to_string(),
        instrument: "XAU".to_string(),
        timeframes: vec![300],
        evaluation_timeframe_secs: 300,
        readings: MarketStateConfig::default(),
        active_events: vec![event.clone()],
        prediction: super::SpecPrediction {
            kind: super::PredictionKind::Vote,
            long: vec![event],
            short: Vec::new(),
            events: Vec::new(),
            min_votes: 1,
        },
        conditions: Vec::new(),
        session: None,
        panel_instruments: Vec::new(),
    }
}

#[test]
fn empty_single_and_short_series_are_safe() {
    let empty = ModelRun::compute(&[], &MarketStateConfig::default());
    assert!(matches!(
        SpecRun::compute(&spec(), empty, &[]),
        Err(SpecV2Error::MissingTimeframe(300))
    ));
    for count in [1_usize, 5, 20] {
        let candles: Vec<Candle> = (0..count).map(candle).collect();
        let run = ModelRun::compute(
            &[TimeframeInput {
                timeframe_secs: 300,
                candles: &candles,
            }],
            &MarketStateConfig::default(),
        );
        let result = SpecRun::compute(&spec(), run, &[]).expect("the short series runs");
        assert_eq!(result.bars(), count);
        assert_eq!(result.prediction().len(), count);
        assert!(
            result.prediction().values.iter().all(|value| *value == 0),
            "a short series produced a vote"
        );
    }
}
