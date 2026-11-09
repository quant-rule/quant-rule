use super::{
    MAX_EXPRESSION_DEPTH, MAX_EXPRESSION_NODES, ModelSpec, SPEC_SCHEMA, SpecCondition, SpecError,
    SpecEvent, SpecExpression, SpecState, SpecStateField,
};
use crate::events::EventType;
use crate::input::{MarketStateConfig, TimeframeInput};
use crate::run::ModelRun;
use quantrule_data::Candle;

const BASE: i64 = 1_700_000_100;
const M5: u64 = 300;
const M15: u64 = 900;

fn candle(index: usize, timestamp: i64, open: f64, close: f64) -> Candle {
    Candle::new(
        index as u64,
        timestamp,
        open,
        open.max(close) + 0.4,
        open.min(close) - 0.4,
        close,
        Some(1_000.0),
    )
    .expect("the generated candle is valid")
}

fn m5_candles(count: usize) -> Vec<Candle> {
    (0..count)
        .map(|index| {
            let phase = index as f64;
            let open = 100.0 + (phase / 7.0).sin() * 3.0 + 0.02 * phase;
            let close = 100.0 + ((phase + 1.0) / 7.0).sin() * 3.0 + 0.02 * (phase + 1.0);
            candle(index, BASE + index as i64 * 300, open, close)
        })
        .collect()
}

fn aggregate(candles: &[Candle], factor: usize) -> Vec<Candle> {
    candles
        .chunks_exact(factor)
        .enumerate()
        .map(|(index, chunk)| {
            let open = chunk[0].open();
            let close = chunk[chunk.len() - 1].close();
            let high = chunk
                .iter()
                .map(Candle::high)
                .fold(f64::NEG_INFINITY, f64::max);
            let low = chunk.iter().map(Candle::low).fold(f64::INFINITY, f64::min);
            Candle::new(
                index as u64,
                chunk[0].timestamp(),
                open,
                high,
                low,
                close,
                Some(1_000.0 * chunk.len() as f64),
            )
            .expect("the aggregated candle is valid")
        })
        .collect()
}

fn run(candles: &[Candle]) -> ModelRun {
    let m15 = aggregate(candles, 3);
    ModelRun::compute(
        &[
            TimeframeInput {
                timeframe_secs: M5,
                candles,
            },
            TimeframeInput {
                timeframe_secs: M15,
                candles: &m15,
            },
        ],
        &MarketStateConfig::default(),
    )
}

fn spec(conditions: Vec<SpecCondition>) -> ModelSpec {
    ModelSpec {
        schema: SPEC_SCHEMA.to_string(),
        id: "test".to_string(),
        ladder: vec![M5, M15],
        evaluation_timeframe_secs: M5,
        conditions,
    }
}

fn event(source: u16, subtype: u8, timeframe_secs: u64) -> SpecExpression {
    let event_code = match (source, subtype) {
        (4, 1) => crate::events::codes::SIGNAL_RSI_THRESHOLD,
        (30, 16) => crate::events::codes::SIGNAL_MAR_REGIME,
        _ => 0,
    };
    SpecExpression::Event(SpecEvent {
        event_code,
        event_type: EventType::Signal,
        subtype,
        source,
        timeframe_secs,
        direction: None,
        within_bars: 0,
    })
}

fn condition(id: &str, expression: SpecExpression) -> SpecCondition {
    SpecCondition {
        id: id.to_string(),
        expression,
    }
}

#[test]
fn validate_accepts_a_bounded_spec() {
    let spec = spec(vec![
        condition("macd", event(4, 1, M5)),
        condition(
            "stack",
            SpecExpression::State(SpecState {
                state: SpecStateField::DiState,
                timeframe_secs: M15,
                equals: 1,
            }),
        ),
    ]);
    assert_eq!(spec.validate(), Ok(()));
}

#[test]
fn a_spec_can_read_the_volatility_and_structure_states() {
    let spec = spec(vec![condition(
        "regime",
        SpecExpression::All(vec![
            SpecExpression::State(SpecState {
                state: SpecStateField::VolRegime,
                timeframe_secs: M5,
                equals: 1,
            }),
            SpecExpression::State(SpecState {
                state: SpecStateField::StructureState,
                timeframe_secs: M5,
                equals: 0,
            }),
        ]),
    )]);
    assert_eq!(spec.validate(), Ok(()));

    let candles = m5_candles(400);
    let evaluation = spec.evaluate(&run(&candles)).expect("the spec evaluates");
    assert_eq!(evaluation.bars, candles.len());
}

#[test]
fn a_spec_can_read_the_rvol_regime() {
    let spec = spec(vec![condition(
        "rvol",
        SpecExpression::State(SpecState {
            state: SpecStateField::RvolRegime,
            timeframe_secs: M5,
            equals: -1,
        }),
    )]);
    assert_eq!(spec.validate(), Ok(()));

    let candles = m5_candles(400);
    let evaluation = spec.evaluate(&run(&candles)).expect("the spec evaluates");
    assert_eq!(evaluation.bars, candles.len());
    assert_eq!(evaluation.conditions.len(), 1);
    assert_eq!(evaluation.conditions[0].values.len(), candles.len());
}

#[test]
fn evaluate_publishes_one_series_per_condition() {
    let candles = m5_candles(400);
    let run = run(&candles);
    let spec = spec(vec![
        condition("rsi", event(4, 1, M5)),
        condition("mar", event(30, 16, M5)),
        condition(
            "di",
            SpecExpression::State(SpecState {
                state: SpecStateField::DiState,
                timeframe_secs: M15,
                equals: 1,
            }),
        ),
    ]);
    let evaluation = spec.evaluate(&run).expect("the spec evaluates");
    assert_eq!(evaluation.bars, candles.len());
    assert_eq!(evaluation.conditions.len(), 3);
    for condition in &evaluation.conditions {
        assert_eq!(condition.values.len(), evaluation.bars);
        assert_eq!(
            condition.true_count,
            condition.values.iter().filter(|value| **value).count()
        );
    }
    assert!(evaluation.conditions[0].true_count > 0);
    assert!(
        evaluation.conditions[1].true_count > 0,
        "the extended event batch must feed the spec"
    );
    assert!(evaluation.conditions[2].true_count > 0);
}

#[test]
fn combinators_follow_boolean_logic() {
    let candles = m5_candles(400);
    let run = run(&candles);
    let left = event(4, 1, M5);
    let right = event(4, 1, M5);
    let spec = spec(vec![
        condition("left", left.clone()),
        condition("right", right.clone()),
        condition(
            "all",
            SpecExpression::All(vec![left.clone(), right.clone()]),
        ),
        condition("any", SpecExpression::Any(vec![left, right])),
        condition("not", SpecExpression::Not(Box::new(event(4, 1, M5)))),
    ]);
    let evaluation = spec.evaluate(&run).expect("the spec evaluates");
    let left = &evaluation.conditions[0].values;
    let right = &evaluation.conditions[1].values;
    let all = &evaluation.conditions[2].values;
    let any = &evaluation.conditions[3].values;
    let not = &evaluation.conditions[4].values;
    for bar in 0..evaluation.bars {
        assert_eq!(all[bar], left[bar] && right[bar]);
        assert_eq!(any[bar], left[bar] || right[bar]);
        assert_eq!(not[bar], !left[bar]);
    }
}

#[test]
fn a_window_widens_the_event_lookback() {
    let candles = m5_candles(400);
    let run = run(&candles);
    let mut narrow = event(4, 1, M5);
    let mut wide = narrow.clone();
    if let SpecExpression::Event(event) = &mut narrow {
        event.within_bars = 0;
    }
    if let SpecExpression::Event(event) = &mut wide {
        event.within_bars = 100;
    }
    let spec = spec(vec![condition("narrow", narrow), condition("wide", wide)]);
    let evaluation = spec.evaluate(&run).expect("the spec evaluates");
    for bar in 0..evaluation.bars {
        assert!(
            !evaluation.conditions[0].values[bar] || evaluation.conditions[1].values[bar],
            "the wide window lost a narrow match at bar {bar}"
        );
    }
    assert!(
        evaluation.conditions[1].true_count >= evaluation.conditions[0].true_count,
        "the wide window must not be narrower"
    );
}

#[test]
fn a_direction_filter_selects_one_side() {
    let candles = m5_candles(400);
    let run = run(&candles);
    let mut up = event(4, 1, M5);
    let mut down = up.clone();
    if let SpecExpression::Event(event) = &mut up {
        event.direction = Some(1);
    }
    if let SpecExpression::Event(event) = &mut down {
        event.direction = Some(-1);
    }
    let spec = spec(vec![condition("up", up), condition("down", down)]);
    let evaluation = spec.evaluate(&run).expect("the spec evaluates");
    for bar in 0..evaluation.bars {
        assert!(
            !(evaluation.conditions[0].values[bar] && evaluation.conditions[1].values[bar]),
            "one event matched both directions at bar {bar}"
        );
    }
}

#[test]
fn appending_bars_does_not_change_past_condition_values() {
    let candles = m5_candles(400);
    let full = run(&candles);
    let cutoff = 201;
    let prefix = run(&candles[..cutoff]);
    let spec = spec(vec![
        condition("macd", event(4, 1, M5)),
        condition(
            "stack",
            SpecExpression::State(SpecState {
                state: SpecStateField::DiState,
                timeframe_secs: M15,
                equals: 1,
            }),
        ),
    ]);
    let full_evaluation = spec.evaluate(&full).expect("the full run evaluates");
    let prefix_evaluation = spec.evaluate(&prefix).expect("the prefix run evaluates");
    for (full_condition, prefix_condition) in full_evaluation
        .conditions
        .iter()
        .zip(&prefix_evaluation.conditions)
    {
        assert_eq!(
            &full_condition.values[..cutoff],
            &prefix_condition.values[..],
            "the past values changed for {}",
            full_condition.id
        );
    }
}

#[test]
fn repeated_evaluation_is_identical() {
    let candles = m5_candles(400);
    let run = run(&candles);
    let spec = spec(vec![condition("macd", event(4, 1, M5))]);
    assert_eq!(
        spec.evaluate(&run).expect("the first evaluation"),
        spec.evaluate(&run).expect("the second evaluation")
    );
}

#[test]
fn a_missing_run_timeframe_is_rejected() {
    let candles = m5_candles(200);
    let run = run(&candles);
    let spec = spec(vec![condition("hour", event(4, 1, 3_600))]);
    let mut spec = spec;
    spec.ladder.push(3_600);
    assert_eq!(spec.evaluate(&run), Err(SpecError::MissingTimeframe(3_600)));
}

#[test]
fn invalid_specs_are_rejected() {
    let mut bad_schema = spec(vec![condition("c", event(4, 1, M5))]);
    bad_schema.schema = "quantrule.model-spec/v2".to_string();
    assert_eq!(bad_schema.validate(), Err(SpecError::UnsupportedSchema));

    let mut empty_ladder = spec(vec![condition("c", event(4, 1, M5))]);
    empty_ladder.ladder.clear();
    assert_eq!(empty_ladder.validate(), Err(SpecError::EmptyLadder));

    let mut unsorted = spec(vec![condition("c", event(4, 1, M5))]);
    unsorted.ladder = vec![M15, M5];
    assert_eq!(
        unsorted.validate(),
        Err(SpecError::LadderNotAscending {
            previous: M15,
            current: M5,
        })
    );

    let mut zero = spec(vec![condition("c", event(4, 1, M5))]);
    zero.ladder = vec![0, M5];
    assert_eq!(zero.validate(), Err(SpecError::ZeroTimeframe));

    let mut wrong_evaluation = spec(vec![condition("c", event(4, 1, M5))]);
    wrong_evaluation.evaluation_timeframe_secs = 60;
    assert_eq!(
        wrong_evaluation.validate(),
        Err(SpecError::UnknownEvaluationTimeframe(60))
    );

    let no_conditions = spec(Vec::new());
    assert_eq!(no_conditions.validate(), Err(SpecError::EmptyConditions));

    let duplicate = spec(vec![
        condition("c", event(4, 1, M5)),
        condition("c", event(4, 1, M5)),
    ]);
    assert_eq!(
        duplicate.validate(),
        Err(SpecError::DuplicateConditionId("c".to_string()))
    );

    let empty_combinator = spec(vec![condition("c", SpecExpression::All(Vec::new()))]);
    assert_eq!(empty_combinator.validate(), Err(SpecError::EmptyCombinator));

    let unknown_timeframe = spec(vec![condition("c", event(4, 1, 60))]);
    assert_eq!(
        unknown_timeframe.validate(),
        Err(SpecError::UnknownTimeframe(60))
    );

    let mut bad_direction = event(4, 1, M5);
    if let SpecExpression::Event(event) = &mut bad_direction {
        event.direction = Some(2);
    }
    assert_eq!(
        spec(vec![condition("c", bad_direction)]).validate(),
        Err(SpecError::InvalidDirection(2))
    );

    let mut too_wide = event(4, 1, M5);
    if let SpecExpression::Event(event) = &mut too_wide {
        event.within_bars = super::MAX_WITHIN_BARS + 1;
    }
    assert_eq!(
        spec(vec![condition("c", too_wide)]).validate(),
        Err(SpecError::WithinBarsTooLarge(super::MAX_WITHIN_BARS + 1))
    );

    let bad_state = SpecExpression::State(SpecState {
        state: SpecStateField::DiState,
        timeframe_secs: M5,
        equals: 2,
    });
    assert_eq!(
        spec(vec![condition("c", bad_state)]).validate(),
        Err(SpecError::InvalidStateCode {
            state: SpecStateField::DiState,
            equals: 2,
        })
    );

    let mut deep = event(4, 1, M5);
    for _ in 0..=MAX_EXPRESSION_DEPTH {
        deep = SpecExpression::Not(Box::new(deep));
    }
    assert_eq!(
        spec(vec![condition("c", deep)]).validate(),
        Err(SpecError::ExpressionTooDeep)
    );

    let wide = SpecExpression::All(vec![event(4, 1, M5); MAX_EXPRESSION_NODES]);
    assert_eq!(
        spec(vec![condition("c", wide)]).validate(),
        Err(SpecError::ExpressionTooLarge)
    );
}
