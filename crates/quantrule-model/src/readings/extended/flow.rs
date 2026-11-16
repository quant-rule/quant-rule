//! Volume-flow readings.

use super::shared::volume;
use super::{Candle, ExtendedConfig};

pub(super) struct FlowSeries {
    pub(super) obv: Vec<Option<f64>>,
    pub(super) obv_ma: Vec<Option<f64>>,
    pub(super) obv_slope: Vec<Option<f64>>,
    pub(super) cmf: Vec<Option<f64>>,
    pub(super) cmf_slope: Vec<Option<f64>>,
    pub(super) mfi: Vec<Option<f64>>,
    pub(super) ad_line: Vec<Option<f64>>,
    pub(super) ad_ma: Vec<Option<f64>>,
}

fn money_flow(candle: &Candle) -> Option<(f64, f64)> {
    let volume = volume(candle)?;
    let span = candle.high() - candle.low();
    let multiplier = if span <= 0.0 {
        0.0
    } else {
        ((candle.close() - candle.low()) - (candle.high() - candle.close())) / span
    };
    Some((multiplier * volume, volume))
}

fn money_flow_volume(candle: &Candle) -> Option<f64> {
    money_flow(candle).map(|(flow, _)| flow)
}

pub(super) fn flow_series(candles: &[Candle], config: &ExtendedConfig) -> FlowSeries {
    let len = candles.len();
    let mut result = FlowSeries {
        obv: vec![None; len],
        obv_ma: vec![None; len],
        obv_slope: vec![None; len],
        cmf: vec![None; len],
        cmf_slope: vec![None; len],
        mfi: vec![None; len],
        ad_line: vec![None; len],
        ad_ma: vec![None; len],
    };
    if len == 0 {
        return result;
    }
    // The cumulative lines are seeded at zero only when the first bar carries a
    // measured volume. Once a volume is missing, the cumulative value cannot be
    // reconstructed, so every later bar stays unavailable rather than restarting
    // from a fabricated baseline.
    let mut obv = candles
        .first()
        .and_then(|candle| candle.volume())
        .map(|_| 0.0);
    let mut ad = obv;
    result.obv[0] = obv;
    result.ad_line[0] = ad;
    for bar in 1..len {
        let change = candles[bar].close() - candles[bar - 1].close();
        obv = match (obv, candles[bar].volume()) {
            (Some(previous), Some(volume)) => Some(previous + change.signum() * volume),
            _ => None,
        };
        ad = match (ad, money_flow_volume(&candles[bar])) {
            (Some(previous), Some(flow)) => Some(previous + flow),
            _ => None,
        };
        result.obv[bar] = obv;
        result.ad_line[bar] = ad;
    }
    let window = config.flow_window;
    if window > 0 {
        for bar in (window - 1)..len {
            let start = bar + 1 - window;
            let obv_values: Vec<f64> = result.obv[start..=bar]
                .iter()
                .filter_map(|value| *value)
                .collect();
            if obv_values.len() == window {
                result.obv_ma[bar] = Some(obv_values.iter().sum::<f64>() / window as f64);
            }
            let ad_values: Vec<f64> = result.ad_line[start..=bar]
                .iter()
                .filter_map(|value| *value)
                .collect();
            if ad_values.len() == window {
                result.ad_ma[bar] = Some(ad_values.iter().sum::<f64>() / window as f64);
            }
            let mut flow = 0.0;
            let mut total_volume = 0.0;
            let mut complete = true;
            for candle in &candles[start..=bar] {
                let Some((value, volume)) = money_flow(candle) else {
                    complete = false;
                    break;
                };
                flow += value;
                total_volume += volume;
            }
            if complete && total_volume > 0.0 {
                result.cmf[bar] = Some(flow / total_volume);
            }
            if bar >= 2 * window - 1 {
                let prior = bar - window;
                if let Some(previous) = result.cmf[prior] {
                    result.cmf_slope[bar] = result.cmf[bar].map(|value| value - previous);
                }
            }
            if bar + 1 >= 2 * window {
                let prior = bar - window;
                if let Some(previous) = result.obv[prior] {
                    result.obv_slope[bar] = result.obv[bar].map(|value| value - previous);
                }
            }
        }
    }
    let mfi_window = config.mfi_window;
    if mfi_window > 0 {
        for bar in mfi_window..len {
            let start = bar + 1 - mfi_window;
            let mut positive = 0.0;
            let mut negative = 0.0;
            let mut complete = true;
            for index in start..=bar {
                let Some(volume) = volume(&candles[index]) else {
                    complete = false;
                    break;
                };
                let typical =
                    (candles[index].high() + candles[index].low() + candles[index].close()) / 3.0;
                let previous = if index > 0 {
                    (candles[index - 1].high()
                        + candles[index - 1].low()
                        + candles[index - 1].close())
                        / 3.0
                } else {
                    typical
                };
                let flow = typical * volume;
                if typical > previous {
                    positive += flow;
                } else if typical < previous {
                    negative += flow;
                }
            }
            if complete && positive + negative > 0.0 {
                result.mfi[bar] = Some(if negative > 0.0 {
                    100.0 - 100.0 / (1.0 + positive / negative)
                } else {
                    100.0
                });
            }
        }
    }
    result
}
