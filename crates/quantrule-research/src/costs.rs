//! The cost model.
//!
//! Every reported result is net of what it would have cost to trade. A cost is
//! expressed in basis points of notional and converted to R by the risk the
//! trade carried, so a wide stop pays less per trade than a tight one.

use serde::{Deserialize, Serialize};

/// Why a cost could not be applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CostError {
    /// A cost component was negative or not finite.
    InvalidCost,
    /// The entry price was not positive and finite.
    InvalidPrice,
    /// The risk distance was not positive and finite.
    InvalidRisk,
}

impl std::fmt::Display for CostError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::InvalidCost => "a cost must be a finite, non-negative number of basis points",
            Self::InvalidPrice => "the entry price must be positive and finite",
            Self::InvalidRisk => "the risk distance must be positive and finite",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for CostError {}

/// The costs that apply to one side of a trade, in basis points.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CostModel {
    /// Half the bid-ask spread, paid on each side.
    pub spread_bps: f64,
    /// Commission, paid on each side.
    pub commission_bps: f64,
    /// Assumed slippage, paid on each side.
    pub slippage_bps: f64,
}

impl Default for CostModel {
    fn default() -> Self {
        Self {
            spread_bps: 0.0,
            commission_bps: 0.0,
            slippage_bps: 0.0,
        }
    }
}

impl CostModel {
    /// Build a model and validate it.
    ///
    /// # Errors
    ///
    /// Returns [`CostError::InvalidCost`] when a component is negative or not
    /// finite.
    pub fn new(spread_bps: f64, commission_bps: f64, slippage_bps: f64) -> Result<Self, CostError> {
        let model = Self {
            spread_bps,
            commission_bps,
            slippage_bps,
        };
        model.validate()?;
        Ok(model)
    }

    /// The round-trip cost, both sides, in basis points.
    ///
    /// # Errors
    ///
    /// Returns [`CostError::InvalidCost`] when a component is invalid.
    pub fn round_trip_bps(&self) -> Result<f64, CostError> {
        self.validate()?;
        Ok(2.0 * (self.spread_bps + self.commission_bps + self.slippage_bps))
    }

    /// The round-trip cost of one trade in R.
    ///
    /// `entry_price / risk_distance` converts notional basis points into R: a
    /// trade risking one point on a hundred-point instrument pays a hundredth of
    /// the notional cost per R.
    ///
    /// # Errors
    ///
    /// Returns [`CostError`] when the model, the price, or the risk is invalid.
    pub fn cost_r(&self, entry_price: f64, risk_distance: f64) -> Result<f64, CostError> {
        if !entry_price.is_finite() || entry_price <= 0.0 {
            return Err(CostError::InvalidPrice);
        }
        if !risk_distance.is_finite() || risk_distance <= 0.0 {
            return Err(CostError::InvalidRisk);
        }
        Ok(self.round_trip_bps()? / 10_000.0 * entry_price / risk_distance)
    }

    /// Subtract the round-trip cost from one gross result in R.
    ///
    /// # Errors
    ///
    /// Returns [`CostError`] when the model, the price, or the risk is invalid.
    pub fn net_r(
        &self,
        gross_r: f64,
        entry_price: f64,
        risk_distance: f64,
    ) -> Result<f64, CostError> {
        Ok(gross_r - self.cost_r(entry_price, risk_distance)?)
    }

    /// Subtract the round-trip cost from every result in a sample.
    ///
    /// # Errors
    ///
    /// Returns [`CostError`] when the model, the price, or the risk is invalid.
    pub fn net_series(
        &self,
        gross: &[f64],
        entry_price: f64,
        risk_distance: f64,
    ) -> Result<Vec<f64>, CostError> {
        let cost = self.cost_r(entry_price, risk_distance)?;
        Ok(gross.iter().map(|value| value - cost).collect())
    }

    fn validate(&self) -> Result<(), CostError> {
        let valid = |value: f64| value.is_finite() && value >= 0.0;
        if valid(self.spread_bps) && valid(self.commission_bps) && valid(self.slippage_bps) {
            Ok(())
        } else {
            Err(CostError::InvalidCost)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model() -> CostModel {
        CostModel::new(2.0, 1.0, 2.0).expect("model")
    }

    #[test]
    fn the_round_trip_is_both_sides() {
        assert!((model().round_trip_bps().expect("bps") - 10.0).abs() < 1e-12);
    }

    #[test]
    fn zero_costs_leave_the_gross_result_alone() {
        let stats = CostModel::default();
        assert!((stats.net_r(2.0, 100.0, 1.0).expect("net") - 2.0).abs() < 1e-12);
    }

    #[test]
    fn cost_in_r_depends_on_the_risk_distance() {
        // 10 bps round trip on a 100.0 instrument risking 1.0 point is 0.1R.
        let cost = model().cost_r(100.0, 1.0).expect("cost");
        assert!((cost - 0.1).abs() < 1e-12, "{cost}");
    }

    #[test]
    fn a_wider_stop_pays_less_per_r() {
        let wide = model().cost_r(100.0, 2.0).expect("cost");
        let tight = model().cost_r(100.0, 0.5).expect("cost");
        assert!(wide < tight);
    }

    #[test]
    fn a_series_is_shifted_by_the_same_cost() {
        let net = model()
            .net_series(&[3.0, -1.0], 100.0, 1.0)
            .expect("series");
        assert!((net[0] - 2.9).abs() < 1e-12);
        assert!((net[1] + 1.1).abs() < 1e-12);
    }

    #[test]
    fn invalid_inputs_are_refused() {
        assert_eq!(CostModel::new(-1.0, 0.0, 0.0), Err(CostError::InvalidCost));
        assert_eq!(model().cost_r(0.0, 1.0), Err(CostError::InvalidPrice));
        assert_eq!(model().cost_r(100.0, 0.0), Err(CostError::InvalidRisk));
    }

    #[test]
    fn a_model_round_trips_through_json() {
        let text = serde_json::to_string(&model()).expect("encode");
        let back: CostModel = serde_json::from_str(&text).expect("decode");
        assert_eq!(back, model());
    }
}
