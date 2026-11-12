//! Panel divergence and signal events.
//!
//! A panel divergence compares consecutive confirmed price pivots of one
//! instrument against a panel reading at the pivot timestamp: relative strength,
//! lead-lag, or breadth. A panel signal is a threshold crossing of one of the
//! five cross-asset readings. The events are keyed by instrument, because the
//! model event record does not carry an instrument field.

pub use crate::events::source_codes::{
    BREADTH_SOURCE_CODE, CO_MOVEMENT_SOURCE_CODE, DISPERSION_SOURCE_CODE, LEAD_LAG_SOURCE_CODE,
    RELATIVE_STRENGTH_SOURCE_CODE,
};
use std::collections::BTreeMap;

use crate::events::EventRecord;
use crate::events::EventType;
use crate::events::codes;
use crate::events::extended::divergence_subtype;
use crate::panel::{PanelInstrument, PanelInstrumentReadings, PanelReadings};
use crate::readings::{PivotKind, PivotScale, PivotSeries};

/// Signal subtype for a cross-asset signal.
const CROSS_ASSET_SIGNAL_SUBTYPE: u8 = 22;
/// Dispersion crossing threshold, in ATR units.
const DISPERSION_THRESHOLD: f64 = 1.0;

/// The panel divergence events, grouped by instrument.
#[derive(Debug, Clone, PartialEq)]
pub struct PanelEventSeries {
    instruments: BTreeMap<String, Vec<EventRecord>>,
}

impl PanelEventSeries {
    /// Computes the panel divergences for every instrument.
    ///
    /// The pivot bar must be a common panel timestamp; a pivot outside the
    /// common timestamps is skipped. The event is emitted at the pivot's
    /// confirmation bar.
    #[must_use]
    pub fn compute(
        panel: &[PanelInstrument<'_>],
        readings: &PanelReadings,
        timeframe_secs: u64,
    ) -> Self {
        let positions: BTreeMap<i64, usize> = readings
            .timestamps
            .iter()
            .enumerate()
            .map(|(index, timestamp)| (*timestamp, index))
            .collect();
        let mut instruments = BTreeMap::new();
        for instrument in panel {
            let Some(series) = readings.instruments.get(instrument.name) else {
                continue;
            };
            let pivots = PivotSeries::compute(instrument.candles);
            let mut events = Vec::new();
            for (source, metric) in [
                (RELATIVE_STRENGTH_SOURCE_CODE, &series.relative_strength),
                (LEAD_LAG_SOURCE_CODE, &series.lead_lag),
                (BREADTH_SOURCE_CODE, &series.breadth),
            ] {
                let mut previous_high: Option<(f64, f64, usize, usize)> = None;
                let mut previous_low: Option<(f64, f64, usize, usize)> = None;
                for pivot in pivots.all_confirmed(PivotScale::Major) {
                    let Some(timestamp) = instrument
                        .candles
                        .get(pivot.occurred_at)
                        .map(|candle| candle.timestamp())
                    else {
                        continue;
                    };
                    let Some(&position) = positions.get(&timestamp) else {
                        continue;
                    };
                    let Some(value) = metric.get(position).copied().flatten() else {
                        continue;
                    };
                    let state = (pivot.price, value, pivot.occurred_at, pivot.available_at);
                    let previous = match pivot.kind {
                        PivotKind::High => previous_high.replace(state),
                        PivotKind::Low => previous_low.replace(state),
                    };
                    let Some(previous) = previous else { continue };
                    let Some((subtype, direction)) = divergence_subtype(
                        pivot.kind,
                        pivot.price - previous.0,
                        value - previous.1,
                    ) else {
                        continue;
                    };
                    let event_code = match source {
                        RELATIVE_STRENGTH_SOURCE_CODE if subtype == 1 => {
                            codes::DIVERGENCE_RS_REGULAR_BULLISH
                        }
                        RELATIVE_STRENGTH_SOURCE_CODE => codes::DIVERGENCE_RS_REGULAR_BEARISH,
                        LEAD_LAG_SOURCE_CODE if subtype == 1 => codes::DIVERGENCE_LEAD_LAG_BULLISH,
                        LEAD_LAG_SOURCE_CODE => codes::DIVERGENCE_LEAD_LAG_BEARISH,
                        BREADTH_SOURCE_CODE if subtype == 1 => {
                            codes::DIVERGENCE_BREADTH_REGULAR_BULLISH
                        }
                        _ => codes::DIVERGENCE_BREADTH_REGULAR_BEARISH,
                    };
                    events.push(EventRecord {
                        event_code,
                        event_type: EventType::Divergence,
                        event_subtype_code: subtype,
                        source_code: source,
                        timeframe_secs,
                        occurred_bar_index: pivot.occurred_at,
                        available_bar_index: pivot.available_at,
                        direction_code: direction,
                        reference_code: None,
                        primary_value: Some(pivot.price),
                        reference_value: Some(value),
                    });
                }
            }
            let bars: BTreeMap<i64, usize> = instrument
                .candles
                .iter()
                .enumerate()
                .map(|(index, candle)| (candle.timestamp(), index))
                .collect();
            panel_signal_events(
                series,
                &readings.timestamps,
                &bars,
                timeframe_secs,
                &mut events,
            );
            events.sort_by_key(|event| (event.available_bar_index, event.occurred_bar_index));
            let mut seen = BTreeMap::new();
            events.retain(|event| {
                seen.insert(
                    (
                        event.event_code,
                        event.occurred_bar_index,
                        event.reference_code,
                    ),
                    true,
                )
                .is_none()
            });
            instruments.insert(instrument.name.to_string(), events);
        }
        Self { instruments }
    }

    /// The events for one instrument, or an empty slice when absent.
    #[must_use]
    pub fn events(&self, instrument: &str) -> &[EventRecord] {
        self.instruments.get(instrument).map_or(&[], Vec::as_slice)
    }

    /// The instrument keys.
    pub fn instruments(&self) -> impl Iterator<Item = &String> {
        self.instruments.keys()
    }
}

/// Emits the five cross-asset signal families at their threshold crossings.
///
/// The reading position is a common panel position; the bar index is mapped
/// back to the instrument's own candles so the record keeps the bar identity
/// of the instrument it is grouped under.
fn panel_signal_events(
    series: &PanelInstrumentReadings,
    timestamps: &[i64],
    bars: &BTreeMap<i64, usize>,
    timeframe_secs: u64,
    events: &mut Vec<EventRecord>,
) {
    for (position, timestamp) in timestamps.iter().enumerate() {
        let Some(&bar) = bars.get(timestamp) else {
            continue;
        };
        if crossed_up(&series.relative_strength, position, 0.0) {
            push_signal(
                events,
                codes::SIGNAL_RELATIVE_STRENGTH_CROSS,
                RELATIVE_STRENGTH_SOURCE_CODE,
                timeframe_secs,
                bar,
                1,
                series.relative_strength[position],
                0.0,
            );
        } else if crossed_down(&series.relative_strength, position, 0.0) {
            push_signal(
                events,
                codes::SIGNAL_RELATIVE_STRENGTH_CROSS,
                RELATIVE_STRENGTH_SOURCE_CODE,
                timeframe_secs,
                bar,
                -1,
                series.relative_strength[position],
                0.0,
            );
        }
        if crossed_up(&series.leader_return, position, 1.0) {
            push_signal(
                events,
                codes::SIGNAL_LEAD_LAG_CROSS,
                LEAD_LAG_SOURCE_CODE,
                timeframe_secs,
                bar,
                1,
                series.leader_return[position],
                1.0,
            );
        } else if crossed_down(&series.leader_return, position, -1.0) {
            push_signal(
                events,
                codes::SIGNAL_LEAD_LAG_CROSS,
                LEAD_LAG_SOURCE_CODE,
                timeframe_secs,
                bar,
                -1,
                series.leader_return[position],
                -1.0,
            );
        }
        if crossed_up(&series.breadth, position, 0.67) {
            push_signal(
                events,
                codes::SIGNAL_BREADTH_CROSS,
                BREADTH_SOURCE_CODE,
                timeframe_secs,
                bar,
                1,
                series.breadth[position],
                0.67,
            );
        } else if crossed_down(&series.breadth, position, 0.33) {
            push_signal(
                events,
                codes::SIGNAL_BREADTH_CROSS,
                BREADTH_SOURCE_CODE,
                timeframe_secs,
                bar,
                -1,
                series.breadth[position],
                0.33,
            );
        }
        if crossed_up(&series.co_movement, position, 0.7) {
            push_signal(
                events,
                codes::SIGNAL_CO_MOVEMENT_CROSS,
                CO_MOVEMENT_SOURCE_CODE,
                timeframe_secs,
                bar,
                1,
                series.co_movement[position],
                0.7,
            );
        } else if crossed_down(&series.co_movement, position, 0.3) {
            push_signal(
                events,
                codes::SIGNAL_CO_MOVEMENT_CROSS,
                CO_MOVEMENT_SOURCE_CODE,
                timeframe_secs,
                bar,
                -1,
                series.co_movement[position],
                0.3,
            );
        }
        if crossed_up(&series.dispersion, position, DISPERSION_THRESHOLD) {
            push_signal(
                events,
                codes::SIGNAL_DISPERSION_CROSS,
                DISPERSION_SOURCE_CODE,
                timeframe_secs,
                bar,
                1,
                series.dispersion[position],
                DISPERSION_THRESHOLD,
            );
        }
    }
}

fn crossed_up(series: &[Option<f64>], position: usize, level: f64) -> bool {
    if position == 0 {
        return false;
    }
    match (
        series.get(position).copied().flatten(),
        series.get(position - 1).copied().flatten(),
    ) {
        (Some(now), Some(previous)) => now > level && previous <= level,
        _ => false,
    }
}

fn crossed_down(series: &[Option<f64>], position: usize, level: f64) -> bool {
    if position == 0 {
        return false;
    }
    match (
        series.get(position).copied().flatten(),
        series.get(position - 1).copied().flatten(),
    ) {
        (Some(now), Some(previous)) => now < level && previous >= level,
        _ => false,
    }
}

// The arguments mirror the event-record fields one-for-one; grouping them into
// a synthetic struct would add an allocation or a second mapping without
// removing a real invariant.
#[allow(clippy::too_many_arguments)]
fn push_signal(
    events: &mut Vec<EventRecord>,
    event_code: u16,
    source: u16,
    timeframe_secs: u64,
    bar: usize,
    direction: i8,
    primary: Option<f64>,
    threshold: f64,
) {
    events.push(EventRecord {
        event_code,
        event_type: EventType::Signal,
        event_subtype_code: CROSS_ASSET_SIGNAL_SUBTYPE,
        source_code: source,
        timeframe_secs,
        occurred_bar_index: bar,
        available_bar_index: bar,
        direction_code: direction,
        reference_code: None,
        primary_value: primary,
        reference_value: Some(threshold),
    });
}

#[cfg(test)]
mod row_tests;

#[cfg(test)]
mod tests {
    use super::PanelEventSeries;
    use crate::events::EventType;
    use crate::panel::{PanelConfig, PanelInstrument, PanelReadings};
    use quantrule_data::Candle;

    fn candle(index: usize, close: f64) -> Candle {
        Candle::new(
            index as u64,
            1_700_000_000 + index as i64 * 3_600,
            close,
            close + 0.5,
            close - 0.5,
            close,
            Some(10.0),
        )
        .expect("valid candle")
    }

    #[test]
    fn panel_events_are_grouped_and_ordered() {
        let btc: Vec<Candle> = (0..80)
            .map(|index| candle(index, 100.0 + (index as f64 * 0.2).sin() * 5.0))
            .collect();
        let eth: Vec<Candle> = (0..80)
            .map(|index| candle(index, 100.0 + (index as f64 * 0.25).sin() * 6.0))
            .collect();
        let xau: Vec<Candle> = (0..80)
            .map(|index| candle(index, 100.0 + (index as f64 * 0.15).sin() * 3.0))
            .collect();
        let panel = [
            PanelInstrument {
                name: "BTC",
                candles: &btc,
            },
            PanelInstrument {
                name: "ETH",
                candles: &eth,
            },
            PanelInstrument {
                name: "XAU",
                candles: &xau,
            },
        ];
        let readings = PanelReadings::compute(&panel, &PanelConfig::DEFAULT);
        let events = PanelEventSeries::compute(&panel, &readings, 3_600);
        assert!(events.instruments().count() == 3);
        for instrument in events.instruments() {
            assert!(
                events
                    .events(instrument)
                    .windows(2)
                    .all(|pair| pair[0].available_bar_index <= pair[1].available_bar_index)
            );
        }
    }

    #[test]
    fn panel_signals_cross_their_thresholds() {
        let btc: Vec<Candle> = (0..100)
            .map(|index| {
                let close = if index < 30 {
                    100.0 + index as f64
                } else {
                    130.0 - (index - 30) as f64 * 0.5
                };
                candle(index, close)
            })
            .collect();
        let eth: Vec<Candle> = (0..100)
            .map(|index| {
                let close = if index < 30 {
                    100.0 + index as f64
                } else {
                    130.0 + (index - 30) as f64
                };
                candle(index, close)
            })
            .collect();
        let xau: Vec<Candle> = (0..100)
            .map(|index| {
                let close = if index < 30 {
                    100.0 + index as f64
                } else {
                    130.0 + (index - 30) as f64 * 0.2
                };
                candle(index, close)
            })
            .collect();
        let panel = [
            PanelInstrument {
                name: "BTC",
                candles: &btc,
            },
            PanelInstrument {
                name: "ETH",
                candles: &eth,
            },
            PanelInstrument {
                name: "XAU",
                candles: &xau,
            },
        ];
        let readings = PanelReadings::compute(&panel, &PanelConfig::DEFAULT);
        let events = PanelEventSeries::compute(&panel, &readings, 3_600);
        let btc_events = events.events("BTC");
        assert!(
            btc_events
                .iter()
                .any(|event| event.source_code == 45 && event.event_type == EventType::Signal)
        );
        assert!(btc_events.iter().any(|event| event.source_code == 45
            && event.event_type == EventType::Signal
            && event.direction_code == -1));
        assert!(
            btc_events
                .iter()
                .any(|event| event.source_code == 48 && event.event_type == EventType::Signal)
        );
        assert!(
            btc_events
                .iter()
                .any(|event| event.source_code == 49 && event.event_type == EventType::Signal)
        );
        assert!(
            btc_events
                .iter()
                .all(|event| event.available_bar_index >= event.occurred_bar_index)
        );
    }

    #[test]
    fn panel_divergences_and_crosses_are_exercised() {
        use crate::events::codes;
        use crate::panel::PanelInstrumentReadings;
        use std::collections::BTreeMap;

        fn pivot_btc(kind: crate::readings::PivotKind, first: f64, second: f64) -> Vec<Candle> {
            (0..30)
                .map(|index| {
                    let index = index as u64;
                    match kind {
                        crate::readings::PivotKind::Low => {
                            let low = if index == 5 {
                                first
                            } else if index == 15 {
                                second
                            } else {
                                101.0
                            };
                            Candle::new(
                                index,
                                1_700_000_000 + index as i64 * 3_600,
                                101.0,
                                101.5,
                                low,
                                101.0,
                                Some(10.0),
                            )
                            .expect("valid candle")
                        }
                        crate::readings::PivotKind::High => {
                            let high = if index == 5 {
                                first
                            } else if index == 15 {
                                second
                            } else {
                                100.0
                            };
                            Candle::new(
                                index,
                                1_700_000_000 + index as i64 * 3_600,
                                100.0,
                                high,
                                99.5,
                                100.0,
                                Some(10.0),
                            )
                            .expect("valid candle")
                        }
                    }
                })
                .collect()
        }

        fn panel(
            btc: &[Candle],
            rs: Vec<Option<f64>>,
            lead: Vec<Option<f64>>,
            breadth: Vec<Option<f64>>,
            leader_return: Vec<Option<f64>>,
            co_movement: Vec<Option<f64>>,
            dispersion: Vec<Option<f64>>,
        ) -> crate::panel::PanelReadings {
            let timestamps: Vec<i64> = btc.iter().map(Candle::timestamp).collect();
            let mut instruments = BTreeMap::new();
            instruments.insert(
                "BTC".to_string(),
                PanelInstrumentReadings {
                    relative_strength: rs,
                    leader_return,
                    lead_lag: lead,
                    breadth,
                    co_movement,
                    dispersion,
                },
            );
            crate::panel::PanelReadings {
                timestamps,
                instruments,
            }
        }

        fn empty(len: usize) -> Vec<Option<f64>> {
            vec![None; len]
        }

        let len = 30;
        let mut rs = empty(len);
        let mut lead = empty(len);
        let mut breadth = empty(len);
        rs[5] = Some(0.5);
        rs[15] = Some(1.5);
        lead[5] = Some(0.5);
        lead[15] = Some(1.5);
        breadth[5] = Some(0.5);
        breadth[15] = Some(1.5);
        let bullish = pivot_btc(crate::readings::PivotKind::Low, 99.0, 98.0);
        let p = panel(
            &bullish,
            rs,
            lead,
            breadth,
            empty(len),
            empty(len),
            empty(len),
        );
        let series = PanelEventSeries::compute(
            &[PanelInstrument {
                name: "BTC",
                candles: &bullish,
            }],
            &p,
            3_600,
        );
        for code in [
            codes::DIVERGENCE_RS_REGULAR_BULLISH,
            codes::DIVERGENCE_LEAD_LAG_BULLISH,
            codes::DIVERGENCE_BREADTH_REGULAR_BULLISH,
        ] {
            assert!(
                series
                    .events("BTC")
                    .iter()
                    .any(|event| event.event_code == code),
                "divergence {code} did not emit"
            );
        }

        let mut rs = empty(len);
        let mut lead = empty(len);
        let mut breadth = empty(len);
        rs[5] = Some(1.5);
        rs[15] = Some(0.5);
        lead[5] = Some(1.5);
        lead[15] = Some(0.5);
        breadth[5] = Some(1.5);
        breadth[15] = Some(0.5);
        let bearish = pivot_btc(crate::readings::PivotKind::High, 101.0, 102.0);
        let p = panel(
            &bearish,
            rs,
            lead,
            breadth,
            empty(len),
            empty(len),
            empty(len),
        );
        let series = PanelEventSeries::compute(
            &[PanelInstrument {
                name: "BTC",
                candles: &bearish,
            }],
            &p,
            3_600,
        );
        for code in [
            codes::DIVERGENCE_RS_REGULAR_BEARISH,
            codes::DIVERGENCE_LEAD_LAG_BEARISH,
            codes::DIVERGENCE_BREADTH_REGULAR_BEARISH,
        ] {
            assert!(
                series
                    .events("BTC")
                    .iter()
                    .any(|event| event.event_code == code),
                "divergence {code} did not emit"
            );
        }

        let base = (0..len)
            .map(|index| candle(index, 100.0))
            .collect::<Vec<_>>();
        let mut rs = empty(len);
        let mut leader = empty(len);
        let mut breadth = empty(len);
        let mut co = empty(len);
        let mut dispersion = empty(len);
        rs[28] = Some(0.0);
        rs[29] = Some(0.1);
        leader[28] = Some(1.0);
        leader[29] = Some(2.0);
        breadth[28] = Some(0.5);
        breadth[29] = Some(0.7);
        co[28] = Some(0.5);
        co[29] = Some(0.8);
        dispersion[28] = Some(0.5);
        dispersion[29] = Some(1.5);
        let p = panel(&base, rs, empty(len), breadth, leader, co, dispersion);
        let series = PanelEventSeries::compute(
            &[PanelInstrument {
                name: "BTC",
                candles: &base,
            }],
            &p,
            3_600,
        );
        for code in [
            codes::SIGNAL_RELATIVE_STRENGTH_CROSS,
            codes::SIGNAL_LEAD_LAG_CROSS,
            codes::SIGNAL_BREADTH_CROSS,
            codes::SIGNAL_CO_MOVEMENT_CROSS,
            codes::SIGNAL_DISPERSION_CROSS,
        ] {
            assert!(
                series
                    .events("BTC")
                    .iter()
                    .any(|event| event.event_code == code),
                "cross {code} did not emit"
            );
        }
    }

    #[test]
    fn repeated_panel_event_computation_is_identical() {
        let btc: Vec<Candle> = (0..60)
            .map(|index| candle(index, 100.0 + index as f64 * 0.1))
            .collect();
        let eth: Vec<Candle> = (0..60)
            .map(|index| candle(index, 100.0 + index as f64 * 0.2))
            .collect();
        let panel = [
            PanelInstrument {
                name: "BTC",
                candles: &btc,
            },
            PanelInstrument {
                name: "ETH",
                candles: &eth,
            },
        ];
        let readings = PanelReadings::compute(&panel, &PanelConfig::DEFAULT);
        let first = PanelEventSeries::compute(&panel, &readings, 3_600);
        let second = PanelEventSeries::compute(&panel, &readings, 3_600);
        assert_eq!(first, second);
    }
}
