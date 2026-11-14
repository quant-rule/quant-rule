use super::*;

fn bar(index: u64, close: f64) -> Bar {
    Bar {
        bar: index,
        timestamp: 1_514_764_800 + index as i64 * 900,
        open: close,
        high: close + 0.5,
        low: close - 0.5,
        close,
        atr: Some(1.0),
        ema_stack: Some(1),
        rsi_zone: Some(0),
        rsi: Some(55.0),
        atr_ratio: Some(1.1),
        adx: Some(25.0),
        close_position: Some(0.6),
        upper_wick: Some(0.2),
        lower_wick: Some(0.3),
        body_ratio: Some(0.5),
    }
}

fn bars(count: usize) -> Vec<Bar> {
    (0..count)
        .map(|index| bar(index as u64, 100.0 + index as f64 * 0.5))
        .collect()
}

fn mark(chart: &str, moment: i64, bar: u64, side: Side) -> Mark {
    Mark {
        chart: chart.to_owned(),
        marked_time_utc: "2018-01-01T00:00:00Z".to_owned(),
        moment,
        bar,
        action: MarkAction::Entry,
        quality: MarkQuality::Good,
        side,
        note: String::new(),
    }
}

#[test]
fn the_forward_path_is_signed_by_the_side() {
    let bars = bars(12);
    let long = forward_path(&bars, 2, Side::Long, 5).expect("long");
    let short = forward_path(&bars, 2, Side::Short, 5).expect("short");
    assert!(long.return_50 > 0.0);
    assert!(short.return_50 < 0.0);
    assert!(long.mfe_50 > 0.0);
    assert!(long.mae_50 >= 0.0);
    assert!(short.mae_50 > 0.0);
}

#[test]
fn a_missing_atr_has_no_path() {
    let mut bars = bars(12);
    bars[2].atr = None;
    assert!(forward_path(&bars, 2, Side::Long, 5).is_none());
}

#[test]
fn a_single_bar_has_no_forward_path() {
    let bars = bars(1);
    assert!(forward_path(&bars, 0, Side::Long, 5).is_none());
}

#[test]
fn the_horizon_is_capped_by_the_sample() {
    let bars = bars(4);
    let path = forward_path(&bars, 2, Side::Long, 50).expect("path");
    assert!((path.return_50 - 0.5 / 101.0).abs() < 1e-12);
}

#[test]
fn the_nearest_bar_ties_to_the_earlier_bar() {
    let bars = bars(4);
    let moment = bars[1].timestamp + 450;
    assert_eq!(nearest_bar(&bars, moment, 450), Some(1));
}

#[test]
fn a_far_moment_has_no_bar() {
    let bars = bars(4);
    assert_eq!(nearest_bar(&bars, 0, 450), None);
    assert_eq!(nearest_bar(&[], 0, 450), None);
}

#[test]
fn atr_buckets_split_at_one() {
    assert_eq!(atr_bucket(None), "unknown");
    assert_eq!(atr_bucket(Some(0.99)), "lt1");
    assert_eq!(atr_bucket(Some(1.0)), "ge1");
}

#[test]
fn label_marks_labels_entries_and_skips_exits() {
    let bars = bars(80);
    let mut exit = mark("chart", bars[10].timestamp, 10, Side::Long);
    exit.action = MarkAction::Exit;
    let marks = [mark("chart", bars[10].timestamp, 10, Side::Long), exit];
    let (rows, unmatched) = label_marks(&bars, &marks, 50);
    assert_eq!(rows.len(), 1);
    assert_eq!(unmatched, 0);
    assert_eq!(rows[0].quality, MarkQuality::Good);
    assert_eq!(rows[0].ema_stack, "bull");
    assert_eq!(rows[0].atr_bucket, "ge1");
}

#[test]
fn a_mark_outside_the_max_distance_is_unmatched() {
    let bars = bars(80);
    let marks = [mark("chart", 0, 0, Side::Long)];
    let (rows, unmatched) = label_marks(&bars, &marks, 50);
    assert!(rows.is_empty());
    assert_eq!(unmatched, 1);
}

#[test]
fn the_cell_table_is_sorted_and_grouped() {
    let bars = bars(20);
    let marks = [
        mark("chart", bars[2].timestamp, 2, Side::Long),
        mark("chart", bars[3].timestamp, 3, Side::Long),
        mark("chart", bars[4].timestamp, 4, Side::Short),
    ];
    let (rows, _) = label_marks(&bars, &marks, 10);
    let cells = cell_table(&rows);
    assert_eq!(cells.len(), 2);
    assert_eq!(cells[0].side, Side::Long);
    assert_eq!(cells[0].n, 2);
    assert_eq!(cells[1].side, Side::Short);
}

#[test]
fn the_best_cell_obeys_the_minimum() {
    let cells = vec![
        Cell {
            side: Side::Long,
            ema_stack: "bull".to_owned(),
            atr_bucket: "ge1".to_owned(),
            n: 2,
            mean_return: 0.02,
            hit_rate: 1.0,
            mean_mfe: 2.0,
            mean_mae: 0.5,
        },
        Cell {
            side: Side::Short,
            ema_stack: "bear".to_owned(),
            atr_bucket: "lt1".to_owned(),
            n: 3,
            mean_return: -0.01,
            hit_rate: 0.0,
            mean_mfe: 1.0,
            mean_mae: 1.0,
        },
    ];
    assert!(best_cell(&cells, 3).is_some_and(|cell| cell.side == Side::Short));
    assert_eq!(best_cell(&cells, 4), None);
}

#[test]
fn the_check_filters_by_the_cell_context() {
    let bars = bars(80);
    let cell = Cell {
        side: Side::Long,
        ema_stack: "bull".to_owned(),
        atr_bucket: "ge1".to_owned(),
        n: 30,
        mean_return: 0.01,
        hit_rate: 0.6,
        mean_mfe: 2.0,
        mean_mae: 0.5,
    };
    let from = bars[0].timestamp;
    let to = bars[20].timestamp;
    let (filtered, control) = check_bars(&bars, &cell, from, to, 10);
    assert_eq!(filtered.len(), 21);
    assert_eq!(control.len(), 21);
}

#[test]
fn an_unmarked_check_set_is_underpowered() {
    let bars = bars(80);
    let cell = Cell {
        side: Side::Long,
        ema_stack: "bear".to_owned(),
        atr_bucket: "lt1".to_owned(),
        n: 30,
        mean_return: 0.01,
        hit_rate: 0.6,
        mean_mfe: 2.0,
        mean_mae: 0.5,
    };
    let check = evaluate(&bars, &cell, bars[0].timestamp, bars[70].timestamp).expect("check");
    assert_eq!(check.filtered.n, 0);
    assert_eq!(check.filtered.mean_net, None);
    assert!(check.underpowered);
    assert_eq!(check.verdict, Verdict::Underpowered);
}

#[test]
fn reordering_the_marks_does_not_change_the_result() {
    let bars = bars(80);
    let marks = [
        mark("chart", bars[2].timestamp, 2, Side::Long),
        mark("chart", bars[3].timestamp, 3, Side::Short),
        mark("chart", bars[4].timestamp, 4, Side::Long),
    ];
    let mut reversed = marks.to_vec();
    reversed.reverse();
    let config = Config {
        label_from: 0,
        label_to: i64::MAX,
        check_from: bars[0].timestamp,
        check_to: bars[70].timestamp,
        horizon: 10,
        min_cell: 1,
    };
    let first = build(&bars, &marks, &config).expect("first");
    let second = build(&bars, &reversed, &config).expect("second");
    assert_eq!(first.rows, second.rows);
    assert_eq!(first.cells, second.cells);
    assert_eq!(first.candidate, second.candidate);
}

#[test]
fn no_marks_means_no_candidate() {
    let bars = bars(80);
    let config = Config::new(0, i64::MAX, 0, i64::MAX, 50);
    let result = build(&bars, &[], &config).expect("build");
    assert!(result.rows.is_empty());
    assert_eq!(result.unmatched, 0);
    assert!(result.cells.is_empty());
    assert_eq!(result.candidate, None);
    assert_eq!(result.check, None);
}

#[test]
fn the_half_year_bucket_is_utc() {
    assert_eq!(half_year(1_514_764_800), "2018H1");
    assert_eq!(half_year(1_530_403_200), "2018H2");
    assert_eq!(half_year(0), "1970H1");
}

#[test]
fn a_non_finite_bar_is_refused() {
    let mut bars = bars(10);
    bars[0].close = f64::NAN;
    let config = Config::new(0, i64::MAX, 0, i64::MAX, 5);
    assert_eq!(build(&bars, &[], &config), Err(MetaError::NotFinite));
}
