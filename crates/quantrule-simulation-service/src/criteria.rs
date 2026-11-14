//! The pre-registered criteria a run records before the first mark.
//!
//! The create request carries the researcher's criteria as text. When that
//! text is a JSON object in this module's shape, the service reads the window
//! selection, the oracle criterion, the check thresholds, and the indicator
//! panel from it. Any other text is recorded verbatim and the documented
//! defaults apply, because a human writes the premise and the criterion in
//! prose before a run exists.
//!
//! Every resolved value is recorded on the run and covered by the inputs
//! digest, so the same bundle, seed, and criteria select the same windows.

use std::collections::BTreeMap;

use quantrule_simulation::{
    CheckConfig, Control, OracleSpec, SideFilter, WindowSpec, parse_utc, utc_from_seconds,
};
use serde::{Deserialize, Serialize};

use crate::bundle::LoadedBundle;

/// The default indicator-panel code: RSI.
pub const DEFAULT_PANEL_CODE: u32 = 2;
/// The default window length, in bars.
pub const DEFAULT_WINDOW_BARS: usize = 400;
/// The default candidate zone inside a window.
pub const DEFAULT_CANDIDATE_ZONE: (usize, usize) = (80, 120);
/// The default maximum window count.
pub const DEFAULT_MAX_WINDOWS: usize = 48;
/// The default minimum window count.
pub const DEFAULT_MIN_WINDOWS: usize = 30;
/// The default overlap allowance.
pub const DEFAULT_OVERLAP_BARS: usize = 40;
/// The default check horizon, in bars.
pub const DEFAULT_CHECK_HORIZON: usize = 20;
/// The default confirmation delay, in bars.
pub const DEFAULT_CONFIRMATION_BARS: usize = 1;
/// The default round-trip cost proxy, in basis points.
pub const DEFAULT_COST_BPS: f64 = 6.0;

/// Why a criteria document or a resolved parameter is unusable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CriteriaError {
    /// The JSON is malformed or names an unknown field.
    Json(String),
    /// A window parameter cannot produce a selection.
    Window(String),
    /// The oracle criterion is impossible.
    Oracle(String),
    /// A check threshold is unusable.
    Check(String),
    /// A date is not an ISO UTC date.
    Date(String),
    /// The bundle publishes no event code to select on.
    NoEvents,
    /// The named indicator panel is unknown.
    Panel(String),
}

impl std::fmt::Display for CriteriaError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Json(message) => formatter.write_str(message),
            Self::Window(message) => formatter.write_str(message),
            Self::Oracle(message) => formatter.write_str(message),
            Self::Check(message) => formatter.write_str(message),
            Self::Date(message) => formatter.write_str(message),
            Self::NoEvents => formatter.write_str("the bundle publishes no event code"),
            Self::Panel(value) => write!(formatter, "the panel {value:?} is not rsi, rvol, or adx"),
        }
    }
}

impl std::error::Error for CriteriaError {}

/// The window-selection parameters a criteria document may override.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct WindowCriteria {
    /// The event codes a candidate bar must publish.
    pub event_codes: Option<Vec<u32>>,
    /// Bars in one window.
    pub window_bars: Option<usize>,
    /// The inclusive lower and exclusive upper candidate position.
    pub candidate_zone: Option<(usize, usize)>,
    /// The most windows to select.
    pub max_windows: Option<usize>,
    /// The fewest windows a run needs.
    pub min_windows: Option<usize>,
    /// The overlap allowance.
    pub overlap_bars: Option<usize>,
    /// The first candidate date, ISO UTC.
    pub from: Option<String>,
    /// The last candidate date, ISO UTC.
    pub to: Option<String>,
}

/// The check thresholds a criteria document may override.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct CheckCriteria {
    /// The event code the rule reads.
    pub event_code: Option<u32>,
    /// The first event-bar date, ISO UTC.
    pub from: Option<String>,
    /// The last event-bar date, ISO UTC.
    pub to: Option<String>,
    /// Bars held after entry.
    pub horizon_bars: Option<usize>,
    /// Bars between the event and the entry.
    pub confirmation_bars: Option<usize>,
    /// The round-trip cost proxy, in basis points.
    pub cost_bps: Option<f64>,
    /// `event` or `bars`.
    pub control: Option<String>,
    /// The filters per side, keyed by side name.
    pub sides: Option<BTreeMap<String, SideFilter>>,
}

/// The criteria document a run may record.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Criteria {
    /// The window-selection overrides.
    pub window: Option<WindowCriteria>,
    /// The oracle criterion.
    pub oracle: Option<OracleSpec>,
    /// The check overrides.
    pub check: Option<CheckCriteria>,
    /// `rsi`, `rvol`, or `adx`.
    pub panel: Option<String>,
}

impl Criteria {
    /// Parse the criteria text.
    ///
    /// A blank value or prose that does not begin with `{` records no override
    /// and uses the defaults. A document that begins with `{` must parse
    /// strictly; a typo is refused rather than silently defaulted.
    ///
    /// # Errors
    ///
    /// Returns [`CriteriaError::Json`] when the text begins with `{` and does
    /// not decode.
    pub fn parse(text: &str) -> Result<Self, CriteriaError> {
        let trimmed = text.trim();
        if trimmed.is_empty() || !trimmed.starts_with('{') {
            return Ok(Self::default());
        }
        serde_json::from_str(trimmed).map_err(|error| CriteriaError::Json(error.to_string()))
    }

    /// Resolve the window-selection parameters for one bundle and seed.
    ///
    /// # Errors
    ///
    /// Returns [`CriteriaError::Window`] when the parameters cannot produce a
    /// selection, [`CriteriaError::Date`] when a date is malformed, and
    /// [`CriteriaError::NoEvents`] when neither the document nor the bundle
    /// names an event code.
    pub fn window_spec(
        &self,
        bundle: &LoadedBundle,
        seed: u64,
    ) -> Result<WindowSpec, CriteriaError> {
        let window = self.window.clone().unwrap_or_default();
        let mut codes: Vec<u32> = match window.event_codes {
            Some(codes) => codes,
            None => {
                let mut codes: Vec<u32> = bundle
                    .bundle
                    .events
                    .iter()
                    .map(|event| event.code)
                    .collect();
                codes.sort_unstable();
                codes.dedup();
                codes
            }
        };
        codes.sort_unstable();
        codes.dedup();
        if codes.is_empty() {
            return Err(CriteriaError::NoEvents);
        }
        let from_ts = window.from.as_deref().map(parse_date).transpose()?;
        let to_ts = window.to.as_deref().map(parse_date).transpose()?;
        let spec = WindowSpec {
            event_codes: codes,
            window_bars: window.window_bars.unwrap_or(DEFAULT_WINDOW_BARS),
            candidate_zone: window.candidate_zone.unwrap_or(DEFAULT_CANDIDATE_ZONE),
            max_windows: window.max_windows.unwrap_or(DEFAULT_MAX_WINDOWS),
            min_windows: window.min_windows.unwrap_or(DEFAULT_MIN_WINDOWS),
            seed,
            overlap_bars: window.overlap_bars.unwrap_or(DEFAULT_OVERLAP_BARS),
            from_ts,
            to_ts,
        };
        spec.validate()
            .map_err(|error| CriteriaError::Window(error.to_string()))?;
        Ok(spec)
    }

    /// Resolve the oracle criterion, or its documented default.
    ///
    /// # Errors
    ///
    /// Returns [`CriteriaError::Oracle`] when the criterion is impossible.
    pub fn oracle_spec(&self) -> Result<OracleSpec, CriteriaError> {
        let spec = self.oracle.unwrap_or(OracleSpec {
            horizon_bars: 60,
            target_atr: 2.0,
            stop_atr: 1.0,
            bad_stop_atr: 1.5,
            bad_target_atr: 1.0,
            entry_offset_max: 2,
        });
        spec.validate()
            .map_err(|error| CriteriaError::Oracle(error.to_string()))?;
        Ok(spec)
    }

    /// Resolve the check thresholds for one run.
    ///
    /// # Errors
    ///
    /// Returns [`CriteriaError::Check`] when a threshold is unusable and
    /// [`CriteriaError::Date`] when a date is malformed.
    pub fn check_config(
        &self,
        run_id: &str,
        bundle: &LoadedBundle,
        window_spec: &WindowSpec,
    ) -> Result<CheckConfig, CriteriaError> {
        let check = self.check.clone().unwrap_or_default();
        let event_code = check
            .event_code
            .or_else(|| window_spec.event_codes.first().copied())
            .ok_or(CriteriaError::NoEvents)?;
        let from = check
            .from
            .as_deref()
            .map(parse_date)
            .transpose()?
            .unwrap_or_else(|| bundle.first_timestamp());
        let to = check
            .to
            .as_deref()
            .map(parse_date)
            .transpose()?
            .unwrap_or_else(|| bundle.last_timestamp());
        if from > to {
            return Err(CriteriaError::Check(
                "the check window ends before it starts".to_owned(),
            ));
        }
        let control = match check.control.as_deref().unwrap_or("event") {
            "event" => Control::Event,
            "bars" => Control::Bars,
            other => {
                return Err(CriteriaError::Check(format!(
                    "the control {other:?} is not event or bars"
                )));
            }
        };
        let mut sides = check.sides.unwrap_or_else(|| {
            BTreeMap::from([
                ("long".to_owned(), SideFilter::default()),
                ("short".to_owned(), SideFilter::default()),
            ])
        });
        sides.retain(|side, _| side == "long" || side == "short");
        if sides.is_empty() {
            return Err(CriteriaError::Check(
                "the check must name long or short".to_owned(),
            ));
        }
        let cost_bps = check.cost_bps.unwrap_or(DEFAULT_COST_BPS);
        if !cost_bps.is_finite() || cost_bps < 0.0 {
            return Err(CriteriaError::Check(
                "the cost proxy must be a finite, non-negative number".to_owned(),
            ));
        }
        Ok(CheckConfig {
            run_id: run_id.to_owned(),
            event_code,
            from,
            to,
            from_text: utc_from_seconds(from),
            to_text: utc_from_seconds(to),
            horizon_bars: check.horizon_bars.unwrap_or(DEFAULT_CHECK_HORIZON),
            confirmation_bars: check.confirmation_bars.unwrap_or(DEFAULT_CONFIRMATION_BARS),
            cost_bps,
            control,
            sides,
        })
    }

    /// The indicator panel code and the reading name it publishes.
    ///
    /// # Errors
    ///
    /// Returns [`CriteriaError::Panel`] when the panel is not one of the three
    /// implemented readings.
    pub fn panel(&self) -> Result<(u32, &'static str), CriteriaError> {
        match self.panel.as_deref().unwrap_or("rsi") {
            "rsi" => Ok((2, "rsi")),
            "rvol" => Ok((4, "rvol")),
            "adx" => Ok((9, "adx")),
            other => Err(CriteriaError::Panel(other.to_owned())),
        }
    }
}

fn parse_date(value: &str) -> Result<i64, CriteriaError> {
    parse_utc(value).map_err(|_| CriteriaError::Date(value.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prose_is_recorded_and_defaults_apply() {
        let criteria = Criteria::parse("min_events=30; mean net > 0").expect("criteria");
        assert_eq!(criteria, Criteria::default());
    }

    #[test]
    fn a_json_document_overrides_the_window_and_panel() {
        let text = serde_json::json!({
            "window": {"event_codes": [149], "window_bars": 60, "candidate_zone": [10, 20]},
            "check": {"event_code": 149},
            "panel": "rvol"
        })
        .to_string();
        let criteria = Criteria::parse(&text).expect("criteria");
        assert_eq!(criteria.panel().expect("panel"), (4, "rvol"));
        assert_eq!(
            criteria.window.expect("window").event_codes,
            Some(vec![149])
        );
    }

    #[test]
    fn a_malformed_document_is_refused() {
        assert!(matches!(
            Criteria::parse("{\"window\": "),
            Err(CriteriaError::Json(_))
        ));
        assert!(matches!(
            Criteria::parse("{\"unknown\": 1}"),
            Err(CriteriaError::Json(_))
        ));
    }

    #[test]
    fn an_unknown_panel_is_refused() {
        let criteria = Criteria {
            panel: Some("macd".to_owned()),
            ..Criteria::default()
        };
        assert!(matches!(criteria.panel(), Err(CriteriaError::Panel(_))));
    }
}
