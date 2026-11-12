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

fn money_flow_volume(candle: &Candle) -> f64 {
    let span = candle.high() - candle.low();
    if span <= 0.0 {
        return 0.0;
    }
    (((candle.close() - candle.low()) - (candle.high() - candle.close())) / span) * volume(candle)
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
    let mut obv = 0.0;
    let mut ad = 0.0;
    result.obv[0] = Some(0.0);
    result.ad_line[0] = Some(0.0);
    for bar in 1..len {
        let change = candles[bar].close() - candles[bar - 1].close();
        obv += change.signum() * volume(&candles[bar]);
        ad += money_flow_volume(&candles[bar]);
        result.obv[bar] = Some(obv);
        result.ad_line[bar] = Some(ad);
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
            for candle in &candles[start..=bar] {
                flow += money_flow_volume(candle);
                total_volume += volume(candle);
            }
            if total_volume > 0.0 {
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
            for index in start..=bar {
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
                let flow = typical * volume(&candles[index]);
                if typical > previous {
                    positive += flow;
                } else if typical < previous {
                    negative += flow;
                }
            }
            if positive + negative > 0.0 {
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
