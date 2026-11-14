use super::*;
use crate::bundle::Bar;

fn event_bar(wick: f64) -> Bar {
    Bar {
        atr: Some(1.0),
        rvol: Some(1.0),
        ema_stack: Some(0),
        rsi_zone: Some(0),
        atr_ratio: Some(1.0),
        close_position: Some(0.5),
        lower_wick: Some(wick),
        upper_wick: Some(wick),
        ..Bar::from_prices(0, 1_577_836_800, 100.0, 101.0, 99.0, 100.0)
    }
}

fn rows() -> Vec<Row> {
    let mut result = Vec::new();
    for index in 0..80_usize {
        let positive = index % 2 == 0;
        result.push(Row {
            event_bar: event_bar(if positive { 0.4 } else { 0.1 }),
            direction: Some(0),
            reference: None,
            confirmation_bar: None,
            entry_index: index * 2,
            entry_time: 1_577_836_800 + index as i64 * 10 * 86_400,
            exit_index: index * 2 + 1,
            entry_open: 100.0,
            exit_close: if positive { 101.0 } else { 99.0 },
        });
    }
    result
}

fn filter() -> SideFilter {
    SideFilter {
        wick_column: Some("lower_wick_ratio".to_owned()),
        wick_min: Some(0.3),
        ..SideFilter::default()
    }
}

#[test]
fn the_non_overlapping_filter_keeps_spaced_events() {
    assert_eq!(non_overlapping(&rows(), 1).len(), 80);
}

#[test]
fn a_filtered_set_beats_the_control() {
    let result = check_side(&rows(), "long", &filter(), 1, 0.0, None).expect("check");
    assert_eq!(result.filtered.n, 40);
    assert!((result.filtered.mean_net.expect("mean") - 0.01).abs() < 1e-12);
    assert_eq!(result.filtered.hit_rate, Some(1.0));
    assert_eq!(result.control.n, 80);
    assert!((result.control.mean_net.expect("mean")).abs() < 1e-12);
    assert_eq!(result.verdict, Verdict::Survives);
}

#[test]
fn no_filter_is_the_control() {
    let result = check_side(&rows(), "long", &SideFilter::default(), 1, 0.0, None).expect("check");
    assert_eq!(result.filtered.n, result.control.n);
    assert_eq!(result.verdict, Verdict::NoPattern);
}

#[test]
fn the_cost_is_subtracted() {
    let result = check_side(&rows(), "long", &filter(), 1, 6.0, None).expect("check");
    assert!((result.filtered.mean_net.expect("mean") - (0.01 - 0.0006)).abs() < 1e-12);
}

#[test]
fn the_direction_and_close_back_filters_apply() {
    let base = event_bar(0.2);
    let confirmation = Bar {
        close: 99.0,
        ..Bar::from_prices(1, base.timestamp + 900, 100.0, 101.0, 98.5, 99.0)
    };
    let row = Row {
        event_bar: base,
        direction: Some(1),
        reference: Some(100.0),
        confirmation_bar: Some(confirmation),
        entry_index: 2,
        entry_time: base.timestamp + 1_800,
        exit_index: 3,
        entry_open: 99.0,
        exit_close: 98.0,
    };
    assert!(passes_direction(
        &row,
        "long",
        &SideFilter {
            direction_only: Some(true),
            ..SideFilter::default()
        }
    ));
    assert!(!passes_direction(
        &row,
        "short",
        &SideFilter {
            direction_only: Some(true),
            ..SideFilter::default()
        }
    ));
    assert!(passes_filter(
        &row,
        "short",
        &SideFilter {
            opposite_direction: Some(true),
            close_back: Some(true),
            ..SideFilter::default()
        }
    ));
}

#[test]
fn a_thin_set_is_underpowered() {
    let result = build(
        &Bundle::new(900, Vec::new(), Vec::new()),
        &CheckConfig {
            run_id: "SIM-T".to_owned(),
            event_code: 149,
            from: 0,
            to: i64::MAX,
            from_text: "2020-01-01".to_owned(),
            to_text: "2022-12-31".to_owned(),
            horizon_bars: 20,
            confirmation_bars: 0,
            cost_bps: 6.0,
            control: Control::Event,
            sides: BTreeMap::from([("long".to_owned(), SideFilter::default())]),
        },
        &Catalog::new(),
    )
    .expect("build");
    assert_eq!(result.candidate_count, 0);
    assert_eq!(result.verdict, Verdict::Underpowered);
    assert_eq!(result.results[0].filtered.n, 0);
}

#[test]
fn the_filter_renders_like_the_reference() {
    let filter = SideFilter {
        direction_only: Some(true),
        rvol_min: Some(1.0),
        ..SideFilter::default()
    };
    assert_eq!(
        render_filter(&filter),
        "{'direction_only': True, 'rvol_min': 1.0}"
    );
}
