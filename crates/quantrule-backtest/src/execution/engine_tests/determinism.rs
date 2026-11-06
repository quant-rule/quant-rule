//! Determinism engine tests.

use super::*;

#[test]
fn repeated_runs_are_identical() {
    let bars = [
        candle(0, 0, 100.0, 101.0, 99.0, 100.0, Some(10.0)),
        candle(1, 900, 100.0, 101.0, 99.5, 100.5, Some(10.0)),
        candle(2, 1800, 100.0, 104.0, 97.0, 100.0, Some(10.0)),
        candle(3, 2700, 100.0, 101.0, 99.0, 100.0, Some(10.0)),
    ];
    let atrs = [Some(1.0); 4];
    let intents = [
        enter(0, 0, Side::Long),
        OrderIntent::Hold,
        OrderIntent::Hold,
        OrderIntent::Hold,
    ];

    let first = run(&bars, &atrs, &intents);
    for _ in 0..10 {
        assert_eq!(run(&bars, &atrs, &intents), first);
    }
}

#[test]
fn later_bars_do_not_change_earlier_fills() {
    let bars = [
        candle(0, 0, 100.0, 101.0, 99.0, 100.0, Some(10.0)),
        candle(1, 900, 100.0, 101.0, 99.5, 100.5, Some(10.0)),
        candle(2, 1800, 100.0, 100.5, 99.0, 100.0, Some(10.0)),
        candle(3, 2700, 100.0, 100.5, 99.0, 100.0, Some(10.0)),
        candle(4, 3600, 101.0, 101.5, 100.5, 101.0, Some(10.0)),
        candle(5, 4500, 101.0, 101.5, 100.5, 101.0, Some(10.0)),
    ];
    let atrs = [Some(1.0); 6];
    let intents = [
        enter(0, 0, Side::Long),
        OrderIntent::Hold,
        OrderIntent::Hold,
        exit(3, 2700, Side::Long),
        OrderIntent::Hold,
        OrderIntent::Hold,
    ];

    let prefix = 4;
    let (prefix_fills, _, _) = run(&bars[..prefix], &atrs[..prefix], &intents[..prefix]);
    let (full_fills, _, _) = run(&bars, &atrs, &intents);

    let prefix_early: Vec<Fill> = prefix_fills
        .iter()
        .copied()
        .filter(|fill| fill.bar < prefix)
        .collect();
    let full_early: Vec<Fill> = full_fills
        .iter()
        .copied()
        .filter(|fill| fill.bar < prefix)
        .collect();
    assert_eq!(prefix_early, full_early);

    let mut changed = bars;
    changed[4] = candle(4, 3600, 200.0, 200.5, 199.0, 200.0, Some(10.0));
    let (changed_fills, _, _) = run(&changed, &atrs, &intents);
    let changed_early: Vec<Fill> = changed_fills
        .iter()
        .copied()
        .filter(|fill| fill.bar < prefix)
        .collect();
    assert_eq!(changed_early, full_early);
}
