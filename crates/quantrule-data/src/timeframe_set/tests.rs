use super::{PartialBucket, TimeframeError, TimeframeSet};
use crate::{
    Candle, CandleValues, DEFAULT_WINDOW, EXTENDED_SERIES, MAX_SERIES, RawCandle, Timeframe,
    WindowError,
};

const BASE: i64 = 1_735_689_600;

fn minute(offset: i64) -> i64 {
    BASE + offset * 60
}

fn values_at(seconds: i64) -> CandleValues {
    CandleValues {
        timestamp: seconds,
        open: 1.0,
        high: 2.0,
        low: 0.5,
        close: 1.5,
        volume: Some(10.0),
    }
}

fn values_with(
    seconds: i64,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: Option<f64>,
) -> CandleValues {
    CandleValues {
        timestamp: seconds,
        open,
        high,
        low,
        close,
        volume,
    }
}

fn push(set: &mut TimeframeSet, offset: i64) {
    set.push_values(values_at(minute(offset)))
        .expect("fixture value is valid");
}

fn push_range(set: &mut TimeframeSet, start: i64, end: i64) {
    for offset in start..end {
        push(set, offset);
    }
}

fn build_minutes(minutes: i64) -> TimeframeSet {
    let mut set = TimeframeSet::new();
    push_range(&mut set, 0, minutes);
    set.finish().expect("fixture buckets are valid");
    set
}

#[test]
fn a_complete_h4_bucket_is_aggregated_from_m1() {
    let set = build_minutes(240);
    let series = set.series(Timeframe::H4);

    assert_eq!(series.len(), 1);
    let candle = series.last().expect("the complete H4 bucket is published");
    assert_eq!(candle.timestamp(), BASE);
    assert_eq!(candle.open(), 1.0);
    assert_eq!(candle.high(), 2.0);
    assert_eq!(candle.low(), 0.5);
    assert_eq!(candle.close(), 1.5);
    assert_eq!(candle.volume(), Some(2_400.0));
}

#[test]
fn a_partial_h4_bucket_is_recorded_and_not_published() {
    let set = build_minutes(239);
    let series = set.series(Timeframe::H4);

    assert!(series.is_empty());
    assert_eq!(
        series.partial_buckets(),
        &[PartialBucket {
            bucket_start: BASE,
            present: 239,
            expected: 240,
        }]
    );
}

#[test]
fn a_h4_gap_is_recorded_between_complete_buckets() {
    let mut set = TimeframeSet::new();
    push_range(&mut set, 0, 240);
    push_range(&mut set, 480, 720);
    set.finish().expect("fixture buckets are valid");

    let series = set.series(Timeframe::H4);
    assert_eq!(series.len(), 2);
    assert_eq!(series.gaps().len(), 1);
    assert_eq!(series.gaps()[0].after, BASE);
    assert_eq!(series.gaps()[0].before, BASE + 28_800);
    assert_eq!(series.gaps()[0].missing, 1);
}

#[test]
fn a_new_set_has_five_empty_series_in_fixed_order() {
    let set = TimeframeSet::new();
    let series: Vec<Timeframe> = set.series_iter().map(|series| series.timeframe()).collect();

    assert_eq!(
        series,
        vec![
            Timeframe::M1,
            Timeframe::M5,
            Timeframe::M15,
            Timeframe::H1,
            Timeframe::H4
        ]
    );
    for timeframe in Timeframe::ALL {
        let series = set.series(timeframe);
        assert!(series.is_empty());
        assert_eq!(series.len(), 0);
        assert!(series.first().is_none());
        assert!(series.last().is_none());
        assert!(series.get(0).is_none());
        assert!(series.gaps().is_empty());
        assert!(series.partial_buckets().is_empty());
    }
    assert!(!set.is_finished());
}

#[test]
fn empty_and_finished_sets_are_stable() {
    let mut set = TimeframeSet::new();

    set.finish().expect("an empty set finishes");
    set.finish().expect("finish is idempotent");

    let mut expected = TimeframeSet::new();
    expected.finish().expect("an empty set finishes");

    assert!(set.is_finished());
    assert_eq!(set, expected);
}

#[test]
fn a_single_candle_stays_on_m1_and_is_partial_everywhere_else() {
    let mut set = TimeframeSet::new();
    push(&mut set, 0);
    set.finish().expect("the partial bucket is recordable");

    assert_eq!(set.series(Timeframe::M1).len(), 1);
    assert_eq!(
        set.series(Timeframe::M1)
            .last()
            .expect("retained")
            .timestamp(),
        BASE
    );
    assert!(set.series(Timeframe::M5).is_empty());
    assert!(set.series(Timeframe::M15).is_empty());
    assert!(set.series(Timeframe::H1).is_empty());

    assert_eq!(
        set.series(Timeframe::M5).partial_buckets(),
        &[PartialBucket {
            bucket_start: BASE,
            present: 1,
            expected: 5,
        }]
    );
    assert_eq!(
        set.series(Timeframe::M15).partial_buckets(),
        &[PartialBucket {
            bucket_start: BASE,
            present: 1,
            expected: 15,
        }]
    );
    assert_eq!(
        set.series(Timeframe::H1).partial_buckets(),
        &[PartialBucket {
            bucket_start: BASE,
            present: 1,
            expected: 60,
        }]
    );
}

#[test]
fn a_complete_m5_bucket_is_aggregated_with_exact_ohlcv() {
    let mut set = TimeframeSet::new();
    set.push_values(values_with(minute(0), 1.0, 2.0, 0.5, 1.5, Some(1.0)))
        .expect("valid");
    set.push_values(values_with(minute(1), 1.5, 2.5, 0.75, 2.0, Some(2.0)))
        .expect("valid");
    set.push_values(values_with(minute(2), 2.0, 3.0, 1.0, 2.5, Some(3.0)))
        .expect("valid");
    set.push_values(values_with(minute(3), 2.5, 2.75, 0.25, 1.0, Some(4.0)))
        .expect("valid");
    set.push_values(values_with(minute(4), 1.0, 1.5, 0.5, 0.75, Some(5.0)))
        .expect("valid");
    set.finish().expect("the M5 bucket is complete");

    let series = set.series(Timeframe::M5);
    let candle = series.first().expect("the M5 bucket was published");
    assert_eq!(series.len(), 1);
    assert_eq!(candle.series_index(), 0);
    assert_eq!(candle.timestamp(), BASE);
    assert_eq!(candle.open(), 1.0);
    assert_eq!(candle.high(), 3.0);
    assert_eq!(candle.low(), 0.25);
    assert_eq!(candle.close(), 0.75);
    assert_eq!(candle.volume(), Some(15.0));
    assert!(series.partial_buckets().is_empty());
    assert!(set.series(Timeframe::M15).is_empty());
    assert!(set.series(Timeframe::H1).is_empty());
}

#[test]
fn a_partial_bucket_is_never_published() {
    let mut set = TimeframeSet::new();
    push_range(&mut set, 0, 4);
    push(&mut set, 5);
    set.finish().expect("partial buckets are recordable");

    let series = set.series(Timeframe::M5);
    assert!(series.is_empty());
    assert_eq!(
        series.partial_buckets(),
        &[
            PartialBucket {
                bucket_start: BASE,
                present: 4,
                expected: 5,
            },
            PartialBucket {
                bucket_start: BASE + 300,
                present: 1,
                expected: 5,
            }
        ]
    );
}

#[test]
fn leading_and_trailing_partials_are_counted_but_complete_buckets_are_kept() {
    let mut set = TimeframeSet::new();
    push_range(&mut set, 2, 5);
    push_range(&mut set, 5, 10);
    set.finish().expect("the leading partial is recordable");

    let series = set.series(Timeframe::M5);
    assert_eq!(series.len(), 1);
    assert_eq!(series.first().expect("retained").timestamp(), BASE + 300);
    assert_eq!(
        series.partial_buckets(),
        &[PartialBucket {
            bucket_start: BASE,
            present: 3,
            expected: 5,
        }]
    );
}

#[test]
fn a_missing_m1_run_is_a_gap_in_the_series_that_contains_it() {
    let mut set = TimeframeSet::new();
    push_range(&mut set, 0, 5);
    push_range(&mut set, 10, 15);
    set.finish().expect("all buckets are complete");

    let m1 = set.series(Timeframe::M1);
    assert_eq!(m1.gaps().len(), 1);
    assert_eq!(m1.gaps()[0].after, minute(4));
    assert_eq!(m1.gaps()[0].before, minute(10));
    assert_eq!(m1.gaps()[0].missing, 5);

    let m5 = set.series(Timeframe::M5);
    assert_eq!(m5.len(), 2);
    assert_eq!(m5.gaps().len(), 1);
    assert_eq!(m5.gaps()[0].after, BASE);
    assert_eq!(m5.gaps()[0].before, BASE + 600);
    assert_eq!(m5.gaps()[0].missing, 1);
    assert!(m5.partial_buckets().is_empty());
}

#[test]
fn a_dropped_partial_bucket_appears_as_a_gap_in_the_higher_series() {
    let mut set = TimeframeSet::new();
    push_range(&mut set, 0, 5);
    push_range(&mut set, 5, 9);
    push_range(&mut set, 10, 15);
    set.finish().expect("all buckets are complete");

    let m5 = set.series(Timeframe::M5);
    assert_eq!(m5.len(), 2);
    assert_eq!(m5.first().expect("retained").timestamp(), BASE);
    assert_eq!(m5.last().expect("retained").timestamp(), BASE + 600);
    assert_eq!(
        m5.partial_buckets(),
        &[PartialBucket {
            bucket_start: BASE + 300,
            present: 4,
            expected: 5,
        }]
    );
    assert_eq!(m5.gaps().len(), 1);
    assert_eq!(m5.gaps()[0].after, BASE);
    assert_eq!(m5.gaps()[0].before, BASE + 600);
    assert_eq!(m5.gaps()[0].missing, 1);
}

#[test]
fn complete_m15_and_h1_buckets_come_from_the_same_m1_stream() {
    let mut set = TimeframeSet::new();
    push_range(&mut set, 0, 60);
    push(&mut set, 60);
    set.finish().expect("the complete hour is publishable");

    assert_eq!(set.series(Timeframe::M15).len(), 4);
    assert_eq!(set.series(Timeframe::H1).len(), 1);

    let h1 = set.series(Timeframe::H1).first().expect("published H1");
    assert_eq!(h1.timestamp(), BASE);
    assert_eq!(h1.open(), 1.0);
    assert_eq!(h1.high(), 2.0);
    assert_eq!(h1.low(), 0.5);
    assert_eq!(h1.close(), 1.5);
    assert_eq!(h1.volume(), Some(600.0));

    let m15 = set.series(Timeframe::M15);
    assert_eq!(m15.first().expect("retained").timestamp(), BASE);
    assert_eq!(m15.last().expect("retained").timestamp(), BASE + 2_700);
    assert_eq!(m15.gaps().len(), 0);
}

#[test]
fn an_m15_gap_is_recorded_at_m15_resolution() {
    let mut set = TimeframeSet::new();
    push_range(&mut set, 0, 15);
    push_range(&mut set, 30, 45);
    set.finish().expect("the M15 buckets are complete");

    let m15 = set.series(Timeframe::M15);
    assert_eq!(m15.len(), 2);
    assert_eq!(m15.gaps().len(), 1);
    assert_eq!(m15.gaps()[0].after, BASE);
    assert_eq!(m15.gaps()[0].before, BASE + 1_800);
    assert_eq!(m15.gaps()[0].missing, 1);
}

#[test]
fn an_h1_gap_is_recorded_at_h1_resolution() {
    let mut set = TimeframeSet::new();
    push_range(&mut set, 0, 60);
    push_range(&mut set, 120, 180);
    set.finish().expect("the H1 buckets are complete");

    let h1 = set.series(Timeframe::H1);
    assert_eq!(h1.len(), 2);
    assert_eq!(h1.gaps().len(), 1);
    assert_eq!(h1.gaps()[0].after, BASE);
    assert_eq!(h1.gaps()[0].before, BASE + 7_200);
    assert_eq!(h1.gaps()[0].missing, 1);
}

#[test]
fn volume_is_summed_only_when_every_source_volume_is_known() {
    let mut set = TimeframeSet::new();
    for offset in 0..5 {
        set.push_values(values_with(minute(offset), 1.0, 2.0, 0.5, 1.5, Some(2.0)))
            .expect("valid");
    }
    set.finish().expect("complete");
    assert_eq!(
        set.series(Timeframe::M5)
            .first()
            .expect("published")
            .volume(),
        Some(10.0)
    );

    let mut set = TimeframeSet::new();
    for offset in 0..5 {
        let volume = if offset == 2 { None } else { Some(2.0) };
        set.push_values(values_with(minute(offset), 1.0, 2.0, 0.5, 1.5, volume))
            .expect("valid");
    }
    set.finish().expect("complete");
    assert_eq!(
        set.series(Timeframe::M5)
            .first()
            .expect("published")
            .volume(),
        None
    );
}

#[test]
fn a_real_zero_volume_survives_aggregation() {
    let mut set = TimeframeSet::new();
    for offset in 0..5 {
        set.push_values(values_with(minute(offset), 1.0, 1.0, 1.0, 1.0, Some(0.0)))
            .expect("valid");
    }
    set.finish().expect("complete");

    let candle = set.series(Timeframe::M5).first().expect("published");
    assert_eq!(candle.high(), 1.0);
    assert_eq!(candle.low(), 1.0);
    assert_eq!(candle.volume(), Some(0.0));
}

#[test]
fn volume_overflow_is_reported_at_the_bucket() {
    let mut set = TimeframeSet::new();
    set.push_values(values_with(minute(0), 1.0, 2.0, 0.5, 1.5, Some(1e308)))
        .expect("the first candle is valid");
    let result = set.push_values(values_with(minute(1), 1.0, 2.0, 0.5, 1.5, Some(1e308)));

    assert_eq!(
        result,
        Err(TimeframeError::VolumeOverflow { bucket_start: BASE })
    );
}

#[test]
fn push_values_rejects_an_out_of_range_timestamp() {
    let mut set = TimeframeSet::full();
    let error = set
        .push_values(values_with(i64::MAX, 1.0, 2.0, 0.5, 1.5, None))
        .expect_err("an out-of-range timestamp is rejected");
    assert!(matches!(error, TimeframeError::Window(_)));
    assert!(set.series(Timeframe::M1).is_empty());
}

#[test]
fn misaligned_m1_timestamps_are_rejected_without_changing_the_set() {
    let mut set = TimeframeSet::new();
    let misaligned = minute(0) + 30;

    assert_eq!(
        set.push_values(values_at(misaligned)),
        Err(TimeframeError::MisalignedM1Timestamp {
            timestamp: misaligned
        })
    );
    assert!(set.series(Timeframe::M1).is_empty());
    assert_eq!(
        set.push_values(values_at(minute(0)))
            .expect("the aligned value is accepted"),
        0
    );
}

#[test]
fn raw_misaligned_timestamps_are_rejected_before_acceptance() {
    let mut set = TimeframeSet::new();
    let result = set.push(RawCandle {
        date: "2025-01-01 00:00:30+00:00",
        open: "1",
        high: "2",
        low: "0.5",
        close: "1.5",
        volume: Some("10"),
    });

    assert_eq!(
        result,
        Err(TimeframeError::MisalignedM1Timestamp {
            timestamp: BASE + 30
        })
    );
    assert!(set.series(Timeframe::M1).is_empty());
}

#[test]
fn duplicate_and_out_of_order_values_do_not_disturb_the_buckets() {
    let mut set = TimeframeSet::new();
    push_range(&mut set, 0, 5);

    assert!(matches!(
        set.push_values(values_at(minute(0))),
        Err(TimeframeError::Window(
            crate::WindowError::OutOfOrderTimestamp { .. }
        ))
    ));
    assert!(matches!(
        set.push_values(values_at(minute(4))),
        Err(TimeframeError::Window(
            crate::WindowError::DuplicateTimestamp { .. }
        ))
    ));

    push(&mut set, 5);
    set.finish().expect("the first bucket is complete");
    assert_eq!(set.series(Timeframe::M5).len(), 1);
}

#[test]
fn finish_is_idempotent_and_rejects_later_pushes() {
    let mut set = TimeframeSet::new();
    push_range(&mut set, 0, 5);
    set.finish().expect("the bucket is complete");
    set.finish().expect("finish is idempotent");

    assert_eq!(set.series(Timeframe::M5).len(), 1);
    assert_eq!(
        set.push_values(values_at(minute(5))),
        Err(TimeframeError::Finished)
    );
    assert_eq!(set.series(Timeframe::M1).len(), 5);
    assert!(set.is_finished());
}

#[test]
fn every_series_retains_at_most_one_thousand_candles() {
    let mut set = TimeframeSet::new();
    push_range(&mut set, 0, 5_005);
    set.finish().expect("all buckets are valid");

    for timeframe in Timeframe::ALL {
        assert!(
            set.series(timeframe).len() <= DEFAULT_WINDOW,
            "{} exceeded the window",
            timeframe.code()
        );
    }
    assert_eq!(set.series(Timeframe::M5).len(), DEFAULT_WINDOW);
    assert_eq!(
        set.series(Timeframe::M5)
            .first()
            .expect("retained")
            .series_index(),
        1
    );
    assert_eq!(
        set.series(Timeframe::M5)
            .last()
            .expect("retained")
            .series_index(),
        1_000
    );
}

#[test]
fn each_timeframe_owns_its_series_index() {
    let mut set = TimeframeSet::new();
    push_range(&mut set, 0, 6);
    set.finish().expect("the first M5 bucket is complete");

    let m5 = set.series(Timeframe::M5);
    assert_eq!(m5.first().expect("retained").series_index(), 0);
    assert_eq!(
        set.series(Timeframe::M1)
            .first()
            .expect("retained")
            .series_index(),
        0
    );
}

#[test]
fn appending_later_candles_does_not_change_earlier_output() {
    let mut set = TimeframeSet::new();
    push_range(&mut set, 0, 15);
    push(&mut set, 15);

    let earlier: Vec<_> = set.series(Timeframe::M5).iter().copied().collect();
    let earlier_gaps = set.series(Timeframe::M5).gaps().to_vec();
    let earlier_partials = set.series(Timeframe::M5).partial_buckets().to_vec();

    push_range(&mut set, 16, 25);
    set.finish().expect("all buckets are valid");

    for (index, candle) in earlier.iter().enumerate() {
        assert_eq!(
            set.series(Timeframe::M5).get(index),
            Some(candle),
            "candle {index} changed"
        );
    }
    assert_eq!(set.series(Timeframe::M5).gaps(), earlier_gaps);
    assert_eq!(
        set.series(Timeframe::M5).partial_buckets(),
        earlier_partials
    );
}

#[test]
fn repeated_runs_are_identical() {
    let first = build_minutes(125);
    let second = build_minutes(125);

    assert_eq!(first, second);
    assert_eq!(
        first.series_iter().collect::<Vec<_>>(),
        second.series_iter().collect::<Vec<_>>()
    );
}

fn build_in_chunks(chunk_size: i64) -> TimeframeSet {
    let mut set = TimeframeSet::new();
    let mut offset = 0;
    while offset < 125 {
        let end = (offset + chunk_size).min(125);
        push_range(&mut set, offset, end);
        offset = end;
    }
    set.finish().expect("fixture buckets are valid");
    set
}

#[test]
fn a_different_split_of_the_input_produces_the_same_set() {
    assert_eq!(build_in_chunks(1), build_in_chunks(7));
    assert_eq!(build_in_chunks(7), build_in_chunks(125));
}

#[test]
fn the_full_and_extended_sets_use_their_own_caps() {
    let full = TimeframeSet::full();
    let extended = TimeframeSet::extended();

    for timeframe in Timeframe::ALL {
        assert_eq!(full.series(timeframe).capacity(), MAX_SERIES);
        assert_eq!(extended.series(timeframe).capacity(), EXTENDED_SERIES);
    }
}

#[test]
fn a_full_set_keeps_one_hundred_thousand_m1_candles() {
    let started = std::time::Instant::now();
    let mut set = TimeframeSet::full();
    for offset in 0..100_000_i64 {
        set.push_values(values_at(minute(offset)))
            .expect("the fixture candle is valid");
    }
    set.finish().expect("the fixture buckets are valid");
    let elapsed = started.elapsed();

    let m1 = set.series(Timeframe::M1);
    assert_eq!(m1.len(), 100_000);
    assert_eq!(m1.first().expect("not empty").series_index(), 0);
    assert_eq!(m1.last().expect("not empty").series_index(), 99_999);
    for (position, candle) in m1.iter().enumerate() {
        assert_eq!(candle.series_index(), position as u64);
    }
    assert_eq!(set.series(Timeframe::M5).len(), 20_000);
    assert_eq!(set.series(Timeframe::M15).len(), 6_666);
    assert_eq!(set.series(Timeframe::H1).len(), 1_666);

    let candles: usize = Timeframe::ALL
        .iter()
        .map(|timeframe| set.series(*timeframe).len())
        .sum();
    eprintln!(
        "full set: {candles} candles, {} bytes of candle storage, {:.3} s",
        candles * std::mem::size_of::<Candle>(),
        elapsed.as_secs_f64()
    );
}

#[test]
fn a_full_set_rejects_the_candle_over_its_cap() {
    let mut set = TimeframeSet::with_limit(2);
    set.push_values(values_at(minute(0)))
        .expect("the first candle is valid");
    set.push_values(values_at(minute(1)))
        .expect("the second candle is valid");

    assert_eq!(
        set.push_values(values_at(minute(2))),
        Err(TimeframeError::Window(WindowError::CapacityExceeded {
            limit: 2
        }))
    );
    assert_eq!(set.series(Timeframe::M1).len(), 2);
}

#[test]
fn a_full_set_rejects_a_duplicate_and_an_out_of_order_candle() {
    let mut set = TimeframeSet::full();
    set.push_values(values_at(minute(0)))
        .expect("the first candle is valid");

    assert_eq!(
        set.push_values(values_at(minute(0))),
        Err(TimeframeError::Window(WindowError::DuplicateTimestamp {
            timestamp: minute(0)
        }))
    );
    assert_eq!(
        set.push_values(values_at(minute(-1))),
        Err(TimeframeError::Window(WindowError::OutOfOrderTimestamp {
            previous: minute(0),
            current: minute(-1)
        }))
    );
}

#[test]
fn a_full_set_rejects_an_invalid_candle() {
    let mut set = TimeframeSet::full();
    let mut invalid = values_at(minute(0));
    invalid.high = 0.1;

    assert!(matches!(
        set.push_values(invalid),
        Err(TimeframeError::Window(WindowError::Candle(_)))
    ));
    assert!(set.series(Timeframe::M1).is_empty());
}

#[test]
fn a_full_set_records_a_gap_and_preserves_zero_volume() {
    let mut set = TimeframeSet::full();
    set.push_values(values_at(minute(0)))
        .expect("the first candle is valid");
    let mut zero_volume = values_at(minute(1));
    zero_volume.volume = Some(0.0);
    set.push_values(zero_volume)
        .expect("the second candle is valid");
    set.push_values(values_at(minute(3)))
        .expect("the candle after the gap is valid");
    set.finish().expect("the fixture buckets are valid");

    let m1 = set.series(Timeframe::M1);
    assert_eq!(m1.len(), 3);
    assert_eq!(m1.get(1).expect("the second candle").volume(), Some(0.0));
    assert_eq!(m1.gaps().len(), 1);
    assert_eq!(m1.gaps()[0].missing, 1);
}

#[test]
fn a_full_set_derives_the_same_buckets_as_a_bounded_set() {
    let mut bounded = TimeframeSet::new();
    let mut full = TimeframeSet::full();
    for offset in 0..125_i64 {
        let values = values_at(minute(offset));
        bounded.push_values(values).expect("the candle is valid");
        full.push_values(values).expect("the candle is valid");
    }
    bounded.finish().expect("the buckets are valid");
    full.finish().expect("the buckets are valid");

    for timeframe in Timeframe::ALL {
        let full_series = full.series(timeframe);
        let bounded_series = bounded.series(timeframe);
        assert_eq!(
            full_series.len(),
            bounded_series.len(),
            "{timeframe:?} length"
        );
        assert_eq!(
            full_series.iter().collect::<Vec<_>>(),
            bounded_series.iter().collect::<Vec<_>>(),
            "{timeframe:?} candles"
        );
        assert_eq!(
            full_series.gaps(),
            bounded_series.gaps(),
            "{timeframe:?} gaps"
        );
    }
}
