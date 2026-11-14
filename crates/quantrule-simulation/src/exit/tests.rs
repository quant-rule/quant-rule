use super::*;
use crate::bundle::Bar;

fn bundle() -> Bundle {
    let rows = [
        (100.5, 99.5, 100.0, 100.0),
        (102.1, 99.9, 101.0, 100.5),
        (103.0, 101.0, 102.0, 101.0),
        (101.0, 100.0, 100.5, 101.5),
        (102.0, 100.0, 101.0, 101.0),
    ];
    let bars = rows
        .iter()
        .enumerate()
        .map(|(index, (high, low, close, ema))| Bar {
            atr: Some(1.0),
            ema_fast: Some(*ema),
            last_pivot_low: Some(99.5),
            last_pivot_high: Some(103.5),
            ..Bar::from_prices(
                index as u64,
                1_514_764_800 + index as i64 * 900,
                *close,
                *high,
                *low,
                *close,
            )
        })
        .collect();
    Bundle::new(900, bars, Vec::new())
}

fn config() -> ExitConfig {
    ExitConfig {
        run_id: "SIM-T".to_owned(),
        from: 0,
        to: i64::MAX,
        check_from: 0,
        check_to: i64::MAX,
        from_text: "2018-01-01".to_owned(),
        to_text: "2019-12-31".to_owned(),
        check_from_text: "2020-01-01".to_owned(),
        check_to_text: "2022-12-31".to_owned(),
        event_code: 149,
        horizon_bars: 4,
        target_atr: 2.0,
        stop_atr: 1.0,
        partial_atr: 1.0,
        cost_bps: 6.0,
    }
}

#[test]
fn the_fixed_bracket_and_partial_match_the_reference() {
    let results = simulate(&bundle(), 0, Side::Long, &config()).expect("simulate");
    assert!((results["fixed_2r"].return_pct - (0.02 - 0.0006)).abs() < 1e-12);
    assert!((results["partial_1r"].return_pct - (0.015 - 0.0006)).abs() < 1e-12);
}

#[test]
fn the_time_stop_is_the_last_close() {
    let mut config = config();
    config.cost_bps = 0.0;
    let results = simulate(&bundle(), 0, Side::Long, &config).expect("simulate");
    assert!((results["time"].return_pct - 0.01).abs() < 1e-12);
}

#[test]
fn the_non_overlapping_filter_keeps_both_sides() {
    let entries = vec![
        Entry {
            index: 0,
            side: Side::Long,
            source: EntrySource::Event,
        },
        Entry {
            index: 0,
            side: Side::Short,
            source: EntrySource::Event,
        },
        Entry {
            index: 3,
            side: Side::Long,
            source: EntrySource::Event,
        },
    ];
    let selected = non_overlapping(&entries, 5);
    assert_eq!(selected.len(), 2);
    assert_eq!(selected[0].side, Side::Long);
    assert_eq!(selected[1].side, Side::Short);
}

#[test]
fn a_missing_atr_is_skipped() {
    let mut bundle = bundle();
    bundle.bars[0].atr = None;
    assert!(simulate(&bundle, 0, Side::Long, &config()).is_none());
}

#[test]
fn the_aggregate_groups_by_side_and_rule() {
    let results = simulate(&bundle(), 0, Side::Long, &config()).expect("simulate");
    let rows = vec![EntryResult {
        side: Side::Long,
        results,
    }];
    let table = aggregate(&rows, "label");
    assert_eq!(table.len(), 5);
    assert!(table.iter().all(|row| row.n == 1 && row.set == "label"));
}
