//! Multi-instrument panel readings.
//!
//! The panel aligns instruments on their common closed-bar timestamps and
//! computes the five cross-asset readings from past bars only. There is no
//! forward fill: a timestamp absent from any instrument is absent from the
//! panel. The module is pure and deterministic; no I/O, clock, or randomness.

use std::collections::{BTreeMap, BTreeSet};

use quantrule_data::Candle;

use crate::readings::atr;

/// One instrument in a panel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PanelInstrument<'a> {
    /// The instrument label.
    pub name: &'a str,
    /// Its closed 1h candles, in timestamp order.
    pub candles: &'a [Candle],
}

/// The panel configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PanelConfig {
    /// Return window in common bars.
    pub return_window: usize,
    /// Lead-lag in common bars.
    pub lead_lag: usize,
    /// Co-movement window in common bars.
    pub co_movement_window: usize,
    /// ATR period for the return normalization.
    pub atr_period: usize,
}

impl PanelConfig {
    /// The default: 24-bar returns, 2-bar lead-lag, 20-bar co-movement, ATR 14.
    pub const DEFAULT: Self = Self {
        return_window: 24,
        lead_lag: 2,
        co_movement_window: 20,
        atr_period: 14,
    };
}

/// The five cross-asset readings for one instrument.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PanelInstrumentReadings {
    /// `ret_i - mean(ret_j)` in ATR units.
    pub relative_strength: Vec<Option<f64>>,
    /// The leader return at `t - k`.
    pub leader_return: Vec<Option<f64>>,
    /// The leader return at `t - k` minus this instrument's return.
    pub lead_lag: Vec<Option<f64>>,
    /// The share of instruments with a positive return.
    pub breadth: Vec<Option<f64>>,
    /// The share of same-sign returns against the basket over the window.
    pub co_movement: Vec<Option<f64>>,
    /// The cross-sectional return range.
    pub dispersion: Vec<Option<f64>>,
}

/// The aligned panel readings.
#[derive(Debug, Clone, PartialEq)]
pub struct PanelReadings {
    /// The common timestamps, sorted.
    pub timestamps: Vec<i64>,
    /// The per-instrument readings, keyed by instrument name.
    pub instruments: BTreeMap<String, PanelInstrumentReadings>,
}

impl PanelReadings {
    /// Computes the panel readings from the common timestamps.
    ///
    /// The first instrument is the lead-lag leader. A reading is `None` until
    /// its window is available; a valid zero is a value.
    #[must_use]
    pub fn compute(panel: &[PanelInstrument<'_>], config: &PanelConfig) -> Self {
        if panel.len() < 2 {
            return Self {
                timestamps: Vec::new(),
                instruments: BTreeMap::new(),
            };
        }
        let maps: Vec<BTreeMap<i64, usize>> = panel
            .iter()
            .map(|instrument| {
                instrument
                    .candles
                    .iter()
                    .enumerate()
                    .map(|(index, candle)| (candle.timestamp(), index))
                    .collect()
            })
            .collect();
        let mut common: BTreeSet<i64> = maps[0].keys().copied().collect();
        for map in maps.iter().skip(1) {
            common.retain(|timestamp| map.contains_key(timestamp));
        }
        let timestamps: Vec<i64> = common.into_iter().collect();
        if timestamps.is_empty() {
            return Self {
                timestamps,
                instruments: BTreeMap::new(),
            };
        }
        let atrs: Vec<Vec<Option<f64>>> = panel
            .iter()
            .map(|instrument| atr(instrument.candles, config.atr_period).values().to_vec())
            .collect();
        let mut returns: Vec<Vec<Option<f64>>> = vec![vec![None; timestamps.len()]; panel.len()];
        for (instrument_index, instrument) in panel.iter().enumerate() {
            for (position, timestamp) in timestamps.iter().enumerate() {
                if position < config.return_window {
                    continue;
                }
                let Some(&bar) = maps[instrument_index].get(timestamp) else {
                    continue;
                };
                let Some(&prior) =
                    maps[instrument_index].get(&timestamps[position - config.return_window])
                else {
                    continue;
                };
                let Some(Some(atr)) = atrs[instrument_index].get(bar) else {
                    continue;
                };
                if *atr <= 0.0 {
                    continue;
                }
                returns[instrument_index][position] = Some(
                    (instrument.candles[bar].close() - instrument.candles[prior].close()) / atr,
                );
            }
        }
        let mut basket = vec![None; timestamps.len()];
        for position in 0..timestamps.len() {
            let values: Vec<f64> = returns
                .iter()
                .filter_map(|series| series[position])
                .collect();
            if !values.is_empty() {
                basket[position] = Some(values.iter().sum::<f64>() / values.len() as f64);
            }
        }
        let mut instruments = BTreeMap::new();
        for (instrument_index, instrument) in panel.iter().enumerate() {
            let mut readings = PanelInstrumentReadings {
                relative_strength: vec![None; timestamps.len()],
                leader_return: vec![None; timestamps.len()],
                lead_lag: vec![None; timestamps.len()],
                breadth: vec![None; timestamps.len()],
                co_movement: vec![None; timestamps.len()],
                dispersion: vec![None; timestamps.len()],
            };
            for position in 0..timestamps.len() {
                let own = returns[instrument_index][position];
                let others: Vec<f64> = returns
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| *index != instrument_index)
                    .filter_map(|(_, series)| series[position])
                    .collect();
                if let Some(own) = own {
                    if !others.is_empty() {
                        readings.relative_strength[position] =
                            Some(own - others.iter().sum::<f64>() / others.len() as f64);
                    }
                    let positives = returns
                        .iter()
                        .filter_map(|series| series[position])
                        .filter(|value| *value > 0.0)
                        .count();
                    let total = returns
                        .iter()
                        .filter(|series| series[position].is_some())
                        .count();
                    if total > 0 {
                        readings.breadth[position] = Some(positives as f64 / total as f64);
                    }
                }
                if position >= config.co_movement_window {
                    let start = position + 1 - config.co_movement_window;
                    let mut same = 0_u32;
                    let mut total = 0_u32;
                    for index in start..=position {
                        if let (Some(value), Some(basket_value)) =
                            (returns[instrument_index][index], basket[index])
                        {
                            if value.signum() == basket_value.signum() {
                                same += 1;
                            }
                            total += 1;
                        }
                    }
                    if total > 0 {
                        readings.co_movement[position] = Some(same as f64 / total as f64);
                    }
                }
                let values: Vec<f64> = returns
                    .iter()
                    .filter_map(|series| series[position])
                    .collect();
                if values.len() >= 2 {
                    let high = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                    let low = values.iter().copied().fold(f64::INFINITY, f64::min);
                    readings.dispersion[position] = Some(high - low);
                }
                if position >= config.lead_lag {
                    let leader = returns[0][position - config.lead_lag];
                    readings.leader_return[position] = leader;
                    if instrument_index > 0
                        && let (Some(leader), Some(own)) = (leader, own)
                    {
                        readings.lead_lag[position] = Some(leader - own);
                    }
                }
            }
            instruments.insert(instrument.name.to_string(), readings);
        }
        Self {
            timestamps,
            instruments,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{PanelConfig, PanelInstrument, PanelReadings};
    use quantrule_data::Candle;

    const START: i64 = 1_700_000_000;

    fn candle(index: usize, close: f64) -> Candle {
        Candle::new(
            index as u64,
            START + index as i64 * 3600,
            close,
            close + 1.0,
            close - 1.0,
            close,
            Some(10.0),
        )
        .expect("valid candle")
    }

    fn panel() -> Vec<Vec<Candle>> {
        let btc: Vec<Candle> = (0..30)
            .map(|index| candle(index, 100.0 + index as f64))
            .collect();
        let eth: Vec<Candle> = (0..30)
            .map(|index| candle(index, 100.0 + index as f64 * 0.5))
            .collect();
        let xau: Vec<Candle> = (0..30)
            .map(|index| candle(index, 100.0 + index as f64 * 0.2))
            .collect();
        vec![btc, eth, xau]
    }

    fn readings() -> PanelReadings {
        let data = panel();
        let instruments = [
            PanelInstrument {
                name: "BTC",
                candles: &data[0],
            },
            PanelInstrument {
                name: "ETH",
                candles: &data[1],
            },
            PanelInstrument {
                name: "XAU",
                candles: &data[2],
            },
        ];
        PanelReadings::compute(&instruments, &PanelConfig::DEFAULT)
    }

    #[test]
    fn relative_strength_ranks_the_faster_instrument() {
        let readings = readings();
        let btc = readings.instruments.get("BTC").expect("btc");
        let xau = readings.instruments.get("XAU").expect("xau");
        assert!(btc.relative_strength[29].expect("btc rs") > 0.0);
        assert!(xau.relative_strength[29].expect("xau rs") < 0.0);
    }

    #[test]
    fn breadth_and_dispersion_are_common() {
        let readings = readings();
        let btc = readings.instruments.get("BTC").expect("btc");
        assert_eq!(btc.breadth[29], Some(1.0));
        assert!(btc.dispersion[29].expect("dispersion") > 0.0);
    }

    #[test]
    fn breadth_is_the_positive_return_share() {
        let make = |close_at_end: f64| -> Vec<Candle> {
            (0..25)
                .map(|index| {
                    let close = if index == 24 { close_at_end } else { 100.0 };
                    Candle::new(
                        index as u64,
                        START + index as i64 * 3_600,
                        close,
                        close + 0.5,
                        close - 0.5,
                        close,
                        Some(10.0),
                    )
                    .expect("valid candle")
                })
                .collect()
        };
        let btc_series = make(110.0);
        let eth_series = make(90.0);
        let xau_series = make(110.0);
        let instruments = [
            PanelInstrument {
                name: "BTC",
                candles: &btc_series,
            },
            PanelInstrument {
                name: "ETH",
                candles: &eth_series,
            },
            PanelInstrument {
                name: "XAU",
                candles: &xau_series,
            },
        ];
        let readings = PanelReadings::compute(&instruments, &PanelConfig::DEFAULT);
        let btc = readings.instruments.get("BTC").expect("btc");
        let eth = readings.instruments.get("ETH").expect("eth");
        assert_eq!(btc.breadth[24], Some(2.0 / 3.0));
        assert_eq!(eth.breadth[24], Some(2.0 / 3.0));
        assert!(btc.relative_strength[24].expect("btc rs") > 0.0);
        assert!(eth.relative_strength[24].expect("eth rs") < 0.0);
        assert!(btc.dispersion[24].expect("dispersion") > 0.0);
    }

    #[test]
    fn co_movement_is_high_for_aligned_series() {
        let readings = readings();
        let btc = readings.instruments.get("BTC").expect("btc");
        assert_eq!(btc.co_movement[29], Some(1.0));
    }

    #[test]
    fn lead_lag_reads_the_leader() {
        let readings = readings();
        let eth = readings.instruments.get("ETH").expect("eth");
        assert!(eth.lead_lag[29].is_some());
    }

    #[test]
    fn the_panel_intersects_timestamps() {
        let btc: Vec<Candle> = (0..10).map(|index| candle(index, 100.0)).collect();
        let eth: Vec<Candle> = (0..8).map(|index| candle(index + 2, 100.0)).collect();
        let instruments = [
            PanelInstrument {
                name: "BTC",
                candles: &btc,
            },
            PanelInstrument {
                name: "ETH",
                candles: &eth,
            },
        ];
        let readings = PanelReadings::compute(&instruments, &PanelConfig::DEFAULT);
        assert_eq!(readings.timestamps.len(), 8);
    }

    #[test]
    fn appending_a_future_bar_does_not_change_past_readings() {
        let data = panel();
        let full = readings();
        let prefix = {
            let instruments = [
                PanelInstrument {
                    name: "BTC",
                    candles: &data[0][..20],
                },
                PanelInstrument {
                    name: "ETH",
                    candles: &data[1][..20],
                },
                PanelInstrument {
                    name: "XAU",
                    candles: &data[2][..20],
                },
            ];
            PanelReadings::compute(&instruments, &PanelConfig::DEFAULT)
        };
        let btc_full = full.instruments.get("BTC").expect("btc");
        let btc_prefix = prefix.instruments.get("BTC").expect("btc");
        for index in 0..20 {
            assert_eq!(
                btc_full.relative_strength[index],
                btc_prefix.relative_strength[index]
            );
            assert_eq!(btc_full.breadth[index], btc_prefix.breadth[index]);
        }
    }
}
