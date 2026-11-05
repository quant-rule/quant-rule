//! Panel divergence events.
//!
//! A panel divergence compares consecutive confirmed price pivots of one
//! instrument against a panel reading at the pivot timestamp: relative strength,
//! lead-lag, or breadth. The event is keyed by instrument, because the model
//! event record does not carry an instrument field.

use std::collections::BTreeMap;

use crate::events::EventRecord;
use crate::events::EventType;
use crate::events::extended::divergence_subtype;
use crate::panel::{PanelInstrument, PanelReadings};
use crate::readings::{PivotKind, PivotScale, PivotSeries};

/// Source code for relative strength.
pub const RELATIVE_STRENGTH_SOURCE_CODE: u16 = 45;
/// Source code for lead-lag.
pub const LEAD_LAG_SOURCE_CODE: u16 = 46;
/// Source code for breadth.
pub const BREADTH_SOURCE_CODE: u16 = 47;

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

#[cfg(test)]
mod tests {
    use super::PanelEventSeries;
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
