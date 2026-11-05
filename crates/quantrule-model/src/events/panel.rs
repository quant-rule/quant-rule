//! Panel divergence and signal events.
//!
//! A panel divergence compares consecutive confirmed price pivots of one
//! instrument against a panel reading at the pivot timestamp: relative strength,
//! lead-lag, or breadth. A panel signal is a threshold crossing of one of the
//! five cross-asset readings. The events are keyed by instrument, because the
//! model event record does not carry an instrument field.

use std::collections::BTreeMap;

use crate::events::EventRecord;
use crate::events::EventType;
use crate::events::extended::divergence_subtype;
use crate::panel::{PanelInstrument, PanelInstrumentReadings, PanelReadings};
use crate::readings::{PivotKind, PivotScale, PivotSeries};

/// Source code for relative strength.
pub const RELATIVE_STRENGTH_SOURCE_CODE: u16 = 45;
/// Source code for lead-lag.
pub const LEAD_LAG_SOURCE_CODE: u16 = 46;
/// Source code for breadth.
pub const BREADTH_SOURCE_CODE: u16 = 47;
/// Source code for co-movement.
pub const CO_MOVEMENT_SOURCE_CODE: u16 = 48;
/// Source code for dispersion.
pub const DISPERSION_SOURCE_CODE: u16 = 49;

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
                    events.push(EventRecord {
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
                        event.event_type.code(),
                        event.event_subtype_code,
                        event.source_code,
                        event.occurred_bar_index,
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

fn push_signal(
    events: &mut Vec<EventRecord>,
    source: u16,
    timeframe_secs: u64,
    bar: usize,
    direction: i8,
    primary: Option<f64>,
    threshold: f64,
) {
    events.push(EventRecord {
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
