//! Golden-fixture test for the full-period market report.
//!
//! The expected file is an independent Python replay of the report contract
//! over the checked-in readings, Direction, Momentum, Structure, Progression,
//! context, and event fixtures. It never runs the Rust code under test. The
//! divergence rows are restricted to the major scale because that is the
//! run's default structure scale.
//!
//! The render is a deterministic, line-oriented projection of every published
//! report field. An empty optional value is `-`, a measured zero is
//! `0.000000000`, and every float is printed with nine decimals.

use quantrule_data::Candle;
use quantrule_model::report::MarketReport;
use quantrule_model::run::ModelRun;
use quantrule_model::state::{MarketStateConfig, TimeframeInput};

const READINGS_FIXTURE: &str = include_str!("fixtures/readings_golden.csv");
const REPORT_FIXTURE: &str = include_str!("fixtures/market_report_golden.txt");
const READINGS_HEADER: &str = "bar,timestamp,open,high,low,close,volume,ema20,ema50,ema200,rsi14,atr14,rvol20,minor_candidate_high,minor_candidate_low,minor_pivot_high,minor_pivot_low,minor_pivot_occurred_at,major_candidate_high,major_candidate_low,major_pivot_high,major_pivot_low,major_pivot_occurred_at";
const TIMEFRAME_SECS: u64 = 300;

fn number(field: &str) -> Option<f64> {
    if field.is_empty() {
        None
    } else {
        Some(
            field
                .parse::<f64>()
                .unwrap_or_else(|error| panic!("invalid number {field:?}: {error}")),
        )
    }
}

fn load_candles() -> Vec<Candle> {
    let mut lines = READINGS_FIXTURE.lines();
    assert_eq!(lines.next(), Some(READINGS_HEADER));
    let mut candles = Vec::new();
    for line in lines {
        let fields: Vec<&str> = line.split(',').collect();
        assert_eq!(fields.len(), 23);
        candles.push(
            Candle::new(
                fields[0].parse().expect("bar"),
                fields[1].parse().expect("timestamp"),
                number(fields[2]).expect("open"),
                number(fields[3]).expect("high"),
                number(fields[4]).expect("low"),
                number(fields[5]).expect("close"),
                number(fields[6]),
            )
            .expect("valid fixture candle"),
        );
    }
    candles
}

fn float_text(value: f64) -> String {
    format!("{value:.9}")
}

fn optional(value: Option<f64>) -> String {
    value.map_or_else(|| "-".to_owned(), float_text)
}

fn optional_integer<T: std::fmt::Display>(value: Option<T>) -> String {
    value.map_or_else(|| "-".to_owned(), |value| value.to_string())
}

fn counts(values: &[u32]) -> String {
    values
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join("/")
}

fn render(report: &MarketReport) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "ladder {}",
        report
            .ladder
            .iter()
            .map(u64::to_string)
            .collect::<Vec<_>>()
            .join("/")
    ));
    lines.push(format!(
        "period first={} last={}",
        optional_integer(report.period.first_open_time),
        optional_integer(report.period.last_open_time)
    ));

    for timeframe in &report.timeframes {
        lines.push(format!(
            "tf {} bars={} expected={} missing={} coverage={} first={} last={}",
            timeframe.timeframe_secs,
            timeframe.bars,
            timeframe.expected_bars,
            timeframe.missing_bars,
            optional(timeframe.coverage_share),
            optional_integer(timeframe.first_open_time),
            optional_integer(timeframe.last_open_time)
        ));
        for axis in &timeframe.axes {
            lines.push(format!(
                "axis {} {} present={} null={} min={} max={} mean={} codes={} code_null={}",
                timeframe.timeframe_secs,
                axis.axis.code(),
                axis.present_count,
                axis.null_count,
                optional(axis.min),
                optional(axis.max),
                optional(axis.mean),
                counts(&axis.code_counts),
                axis.code_null_count
            ));
        }
    }

    let timeframe = &report.timeframes[0];
    let latest = timeframe.latest.as_ref().expect("the fixture has a bar");
    lines.push(format!(
        "latest {} bar={} open_time={}",
        timeframe.timeframe_secs, latest.bar_index, latest.open_time
    ));
    lines.push(format!(
        "readings ema20={} ema50={} ema200={} rsi14={} atr14={} rvol20={}",
        optional(latest.readings.ema20),
        optional(latest.readings.ema50),
        optional(latest.readings.ema200),
        optional(latest.readings.rsi14),
        optional(latest.readings.atr14),
        optional(latest.readings.rvol20)
    ));
    lines.push(format!(
        "direction value={} state={} size={} readiness={} reason={} age={} changes={}",
        optional(latest.direction.direction),
        optional_integer(latest.direction.state_code),
        optional(latest.direction.size_percentile),
        latest.direction.readiness_code,
        optional_integer(latest.direction.reason_code),
        latest.direction.state_age_bars,
        latest.direction.state_changes_50
    ));
    lines.push(format!(
        "momentum force={} acceleration={} intensification={} state={} size={} readiness={} reason={} age={} changes={}",
        optional(latest.momentum.force),
        optional(latest.momentum.acceleration),
        optional(latest.momentum.intensification),
        optional_integer(latest.momentum.state_code),
        optional(latest.momentum.size_percentile),
        latest.momentum.readiness_code,
        optional_integer(latest.momentum.reason_code),
        latest.momentum.state_age_bars,
        latest.momentum.state_changes_50
    ));
    lines.push(format!(
        "structure high_relation={} low_relation={} score={} state={} scale={} ema_distance_20={} ema_distance_50={} ema_distance_200={} ema_cross_20={} ema_cross_50={} ema_cross_200={} ema_retest_20={} ema_retest_50={} ema_retest_200={} size={} readiness={} reason={} age={} changes={}",
        optional_integer(latest.structure.high_relation_code),
        optional_integer(latest.structure.low_relation_code),
        optional(latest.structure.score),
        optional_integer(latest.structure.state_code),
        latest.structure.scale_code,
        optional(latest.structure.ema_distance_atr_20),
        optional(latest.structure.ema_distance_atr_50),
        optional(latest.structure.ema_distance_atr_200),
        optional_integer(latest.structure.ema_cross_code_20),
        optional_integer(latest.structure.ema_cross_code_50),
        optional_integer(latest.structure.ema_cross_code_200),
        optional_integer(latest.structure.ema_retest_code_20),
        optional_integer(latest.structure.ema_retest_code_50),
        optional_integer(latest.structure.ema_retest_code_200),
        optional(latest.structure.size_percentile),
        latest.structure.readiness_code,
        optional_integer(latest.structure.reason_code),
        latest.structure.state_age_bars,
        latest.structure.state_changes_50
    ));
    lines.push(format!(
        "progression progress={} stage={} leg_direction={} leg_scale={} size={} readiness={} reason={} age={} changes={}",
        optional(latest.progression.progress),
        optional_integer(latest.progression.stage_code),
        optional_integer(latest.progression.leg_direction),
        optional_integer(latest.progression.leg_scale_code),
        optional(latest.progression.size_percentile),
        latest.progression.readiness_code,
        optional_integer(latest.progression.reason_code),
        latest.progression.state_age_bars,
        latest.progression.state_changes_50
    ));
    match latest.progression.leg {
        None => lines.push("leg -".to_owned()),
        Some(leg) => lines.push(format!(
            "leg id={} timeframe={} scale={} origin_price={} origin_occurred={} origin_available={} origin_confirmed={} direction={} target_price={} target_basis={} target_version={} target_stable={} progress={} displacement_price={} displacement_atr={} lifecycle={} stage={} target_reached={}",
            leg.leg_id,
            optional_integer(leg.timeframe_secs),
            leg.scale_code,
            float_text(leg.origin_price),
            leg.origin_occurred_at,
            optional_integer(leg.origin_available_at),
            leg.origin_confirmed_code,
            leg.direction_code,
            optional(leg.target_price),
            optional_integer(leg.target_basis_code),
            leg.target_version,
            leg.target_stable_code,
            optional(leg.progress),
            optional(leg.displacement_price),
            optional(leg.displacement_atr),
            leg.lifecycle_code,
            optional_integer(leg.stage_code),
            optional_integer(leg.target_reached_code)
        )),
    }
    lines.push(format!(
        "context atr_ratio={} rvol_ratio={} code={}",
        optional(latest.context.atr_ratio),
        optional(latest.context.rvol_ratio),
        optional_integer(latest.context.context_code)
    ));
    lines.push(format!(
        "consensus bullish={} bearish={} neutral={} weakest={}",
        latest.consensus.bullish_axis_count,
        latest.consensus.bearish_axis_count,
        latest.consensus.neutral_axis_count,
        optional_integer(latest.consensus.weakest_directional_axis_code)
    ));

    lines.push(format!("events total={}", report.events.total));
    lines.push(format!(
        "events_type level={} divergence={} signal={}",
        report.events.by_type.level, report.events.by_type.divergence, report.events.by_type.signal
    ));
    lines.push(format!(
        "events_subtype level={} divergence={} signal={}",
        counts(&report.events.by_subtype.level),
        counts(&report.events.by_subtype.divergence),
        counts(&report.events.by_subtype.signal)
    ));
    let timeframe_events = &report.events.timeframes[0];
    let latest_event = timeframe_events.latest.map_or_else(
        || "-".to_owned(),
        |event| {
            format!(
                "{}:{}:{}:{}:{}:{}:{}:{}:{}",
                event.event_type_code,
                event.event_subtype_code,
                event.available_bar_index,
                event.occurred_bar_index,
                event.source_code,
                event.direction_code,
                optional_integer(event.reference_code),
                optional(event.primary_value),
                optional(event.reference_value)
            )
        },
    );
    lines.push(format!(
        "events_tf {} total={} level={} divergence={} signal={} latest={}",
        timeframe_events.timeframe_secs,
        timeframe_events.total,
        timeframe_events.by_type.level,
        timeframe_events.by_type.divergence,
        timeframe_events.by_type.signal,
        latest_event
    ));

    lines.push(format!(
        "chart timeframe={} source_bars={} bucket_size={}",
        optional_integer(report.charts.timeframe_secs),
        report.charts.source_bars,
        report.charts.bucket_size
    ));
    for bar in &report.charts.bars {
        lines.push(format!(
            "chart_bar index={} first={} last={} open_time={} last_open_time={} open={} high={} low={} close={} volume={} direction={} momentum={} structure={} stage={}",
            bar.first_bar_index / report.charts.bucket_size,
            bar.first_bar_index,
            bar.last_bar_index,
            bar.open_time,
            bar.last_open_time,
            float_text(bar.open),
            float_text(bar.high),
            float_text(bar.low),
            float_text(bar.close),
            optional(bar.volume),
            optional(bar.direction),
            optional(bar.momentum),
            optional(bar.structure),
            optional_integer(bar.progression_stage_code)
        ));
    }
    for event in &report.charts.events {
        lines.push(format!(
            "chart_event bucket={} available={} occurred={} type={} subtype={} direction={}",
            event.bucket_index,
            event.available_bar_index,
            event.occurred_bar_index,
            event.event_type_code,
            event.event_subtype_code,
            event.direction_code
        ));
    }

    lines.join("\n") + "\n"
}

#[test]
fn the_market_report_matches_the_independent_golden_fixture() {
    let candles = load_candles();
    let run = ModelRun::compute(
        &[TimeframeInput {
            timeframe_secs: TIMEFRAME_SECS,
            candles: &candles,
        }],
        &MarketStateConfig::default(),
    );
    let report = MarketReport::compute(&run, &[TIMEFRAME_SECS]);
    let actual = render(&report);

    assert_eq!(
        actual, REPORT_FIXTURE,
        "the market report differs from the independent fixture"
    );
}

#[test]
fn the_render_is_stable_across_repeated_runs() {
    let candles = load_candles();
    let inputs = [TimeframeInput {
        timeframe_secs: TIMEFRAME_SECS,
        candles: &candles,
    }];
    let first = MarketReport::compute(
        &ModelRun::compute(&inputs, &MarketStateConfig::default()),
        &[TIMEFRAME_SECS],
    );
    let second = MarketReport::compute(
        &ModelRun::compute(&inputs, &MarketStateConfig::default()),
        &[TIMEFRAME_SECS],
    );

    assert_eq!(render(&first), render(&second));
}
