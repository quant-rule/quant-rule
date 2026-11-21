//! Static evidence maps for the verification manifest.
//!
//! Each map names the test that proves one construct. The data is
//! reviewed evidence, not a heuristic: an entry is added only when the
//! named test exists and covers the row.

/// Indicator families in the generated matrix.
pub const AREAS: &[(&str, &[&str])] = &[
    (
        "Base",
        &[
            "ema",
            "rsi",
            "atr",
            "macd",
            "context",
            "progression",
            "adx",
            "dmi",
        ],
    ),
    ("1 Volume-weighted price", &["vwap"]),
    ("2 Volume by price", &["volume_profile"]),
    (
        "3 Participation",
        &["rvol", "volume_climax", "absorption", "volume_oscillator"],
    ),
    ("4 Volume-price flow", &["obv", "cmf", "mfi", "ad_line"]),
    (
        "5 Volatility structure",
        &["atr_term", "range_expansion", "mar", "keltner"],
    ),
    (
        "6 Structure",
        &[
            "pivots",
            "donchian",
            "swing_structure",
            "pivot_sequence",
            "fibonacci",
            "horizontal_band",
            "level_strength",
            "aroon",
            "bos_choch",
        ],
    ),
    (
        "7 Candle anatomy and gaps",
        &["candle_anatomy", "candle_pattern", "gap"],
    ),
    (
        "8 Path and duration",
        &[
            "path_efficiency",
            "direction_changes",
            "bars_since_extreme",
            "drawdown_atr",
            "range_position",
        ],
    ),
    (
        "9 Cross-asset",
        &[
            "relative_strength",
            "lead_lag",
            "breadth",
            "co_movement",
            "dispersion",
        ],
    ),
    (
        "10 Calendar and session",
        &[
            "session",
            "opening_range",
            "overnight_intraday",
            "session_range",
            "calendar_bucket",
        ],
    ),
];

pub const ROW_INDICATOR_CONTRACTS: &[&str] = &[
    "absorption",
    "ad_line",
    "aroon",
    "atr_term",
    "bars_since_extreme",
    "bos_choch",
    "candle_anatomy",
    "candle_pattern",
    "cmf",
    "direction_changes",
    "donchian",
    "drawdown_atr",
    "fibonacci",
    "gap",
    "horizontal_band",
    "keltner",
    "level_strength",
    "mar",
    "mfi",
    "obv",
    "path_efficiency",
    "pivot_sequence",
    "range_expansion",
    "range_position",
    "rvol",
    "swing_structure",
    "volume_climax",
    "volume_oscillator",
    "volume_profile",
    "vwap",
];

pub const ROW_PANEL_CONTRACTS: &[&str] = &[
    "breadth",
    "co_movement",
    "dispersion",
    "lead_lag",
    "relative_strength",
];

pub const ROW_SESSION_CONTRACTS: &[&str] = &[
    "calendar_bucket",
    "opening_range",
    "overnight_intraday",
    "session",
    "session_range",
];

pub fn row_base_contract(ident: &str) -> Option<&'static str> {
    match ident {
        "adx" => Some("per_row_base_remaining_indicators::adx_row_contract"),
        "dmi" => Some("per_row_base_remaining_indicators::dmi_row_contract"),
        "macd" => Some("per_row_base_remaining_indicators::macd_row_contract"),
        "pivots" => Some("per_row_base::pivots_row_contract"),
        _ => None,
    }
}

pub fn row_indicator_tests(
    ident: &str,
) -> Option<&'static [(&'static str, &'static [&'static str])]> {
    match ident {
        "adx" => Some(&[
            (
                "formula",
                &["readings::adx::tests::an_uptrend_has_positive_direction_and_strength"],
            ),
            ("causality", &["readings::adx::tests::the_series_is_causal"]),
            (
                "determinism",
                &["readings::adx::tests::the_series_is_deterministic"],
            ),
            (
                "edge",
                &["readings::adx::tests::a_flat_series_is_null_not_zero"],
            ),
        ]),
        "atr" => Some(&[
            ("formula", &["per_row_base::atr_row_contract"]),
            ("causality", &["per_row_base::atr_row_contract"]),
            ("determinism", &["per_row_base::atr_row_contract"]),
            ("golden", &["per_row_base::atr_row_contract"]),
            ("edge", &["per_row_base::atr_row_contract"]),
        ]),
        "context" => Some(&[
            (
                "formula",
                &["context_golden::the_context_matches_the_golden_fixture"],
            ),
            (
                "causality",
                &["context_golden::later_candles_do_not_change_earlier_context"],
            ),
            (
                "determinism",
                &["context_golden::the_context_is_deterministic"],
            ),
            (
                "golden",
                &["context_golden::the_context_matches_the_golden_fixture"],
            ),
            (
                "edge",
                &["context_golden::every_available_ratio_retains_a_value_when_the_code_is_null"],
            ),
        ]),
        "dmi" => Some(&[
            (
                "formula",
                &["events::dmi::tests::a_direction_change_emits_one_cross"],
            ),
            ("causality", &["events::dmi::tests::the_series_is_causal"]),
            ("edge", &["events::dmi::tests::a_flat_series_has_no_event"]),
        ]),
        "ema" => Some(&[
            ("formula", &["per_row_base::ema_row_contract"]),
            ("causality", &["per_row_base::ema_row_contract"]),
            ("determinism", &["per_row_base::ema_row_contract"]),
            ("golden", &["per_row_base::ema_row_contract"]),
            ("edge", &["per_row_base::ema_row_contract"]),
        ]),
        "macd" => Some(&[
            (
                "formula",
                &["readings::macd::tests::the_line_is_the_fast_minus_the_slow_ema"],
            ),
            (
                "causality",
                &["readings::macd::tests::appended_bars_do_not_change_earlier_output"],
            ),
            (
                "determinism",
                &["readings::macd::tests::repeated_runs_are_identical"],
            ),
            (
                "edge",
                &["readings::macd::tests::short_and_invalid_inputs_are_all_null"],
            ),
        ]),
        "progression" => Some(&[
            (
                "formula",
                &["progression::tests::the_series_length_matches_the_readings_length"],
            ),
            (
                "causality",
                &["progression::tests::later_bars_do_not_change_earlier_records"],
            ),
            (
                "determinism",
                &["progression::tests::repeated_runs_are_identical"],
            ),
            (
                "golden",
                &["progression_golden::the_per_bar_fixture_matches_the_computed_series"],
            ),
            (
                "edge",
                &["progression::tests::an_empty_series_has_no_bars_and_no_legs"],
            ),
        ]),
        "rsi" => Some(&[
            ("formula", &["per_row_base::rsi_row_contract"]),
            ("causality", &["per_row_base::rsi_row_contract"]),
            ("determinism", &["per_row_base::rsi_row_contract"]),
            ("golden", &["per_row_base::rsi_row_contract"]),
            ("edge", &["per_row_base::rsi_row_contract"]),
        ]),
        _ => None,
    }
}

pub fn row_event_tests(ident: &str) -> Option<&'static [&'static str]> {
    match ident {
        "divergence.ad_regular_bullish" => {
            Some(&["events::extended::row_tests::divergence_ad_regular_bullish_row_contract"])
        }
        "divergence.atr_bearish" => {
            Some(&["events::extended::row_tests::divergence_atr_bearish_row_contract"])
        }
        "divergence.breadth_regular_bearish" => {
            Some(&["events::panel::row_tests::divergence_breadth_regular_bearish_row_contract"])
        }
        "divergence.breadth_regular_bullish" => {
            Some(&["events::panel::row_tests::divergence_breadth_regular_bullish_row_contract"])
        }
        "divergence.cmf_hidden_bullish" => {
            Some(&["events::extended::row_tests::divergence_cmf_hidden_bullish_row_contract"])
        }
        "divergence.lead_lag_bearish" => {
            Some(&["events::panel::row_tests::divergence_lead_lag_bearish_row_contract"])
        }
        "divergence.lead_lag_bullish" => {
            Some(&["events::panel::row_tests::divergence_lead_lag_bullish_row_contract"])
        }
        "divergence.mfi_hidden_bullish" => {
            Some(&["events::extended::row_tests::divergence_mfi_hidden_bullish_row_contract"])
        }
        "divergence.mfi_regular_bearish" => {
            Some(&["events::extended::row_tests::divergence_mfi_regular_bearish_row_contract"])
        }
        "divergence.mfi_regular_bullish" => {
            Some(&["events::extended::row_tests::divergence_mfi_regular_bullish_row_contract"])
        }
        "divergence.obv_hidden_bearish" => {
            Some(&["events::extended::row_tests::divergence_obv_hidden_bearish_row_contract"])
        }
        "divergence.obv_regular_bullish" => {
            Some(&["events::extended::row_tests::divergence_obv_regular_bullish_row_contract"])
        }
        "divergence.rs_regular_bearish" => {
            Some(&["events::panel::row_tests::divergence_rs_regular_bearish_row_contract"])
        }
        "divergence.rs_regular_bullish" => {
            Some(&["events::panel::row_tests::divergence_rs_regular_bullish_row_contract"])
        }
        "divergence.rsi_hidden_bearish" => {
            Some(&["per_row_base_events::divergence_rsi_hidden_bearish_row_contract"])
        }
        "divergence.rsi_hidden_bullish" => {
            Some(&["per_row_base_events::divergence_rsi_hidden_bullish_row_contract"])
        }
        "divergence.rsi_regular_bearish" => {
            Some(&["per_row_base_events::divergence_rsi_regular_bearish_row_contract"])
        }
        "divergence.rsi_regular_bullish" => {
            Some(&["per_row_base_events::divergence_rsi_regular_bullish_row_contract"])
        }
        "divergence.vwap_hidden_bullish" => {
            Some(&["events::extended::row_tests::divergence_vwap_hidden_bullish_row_contract"])
        }
        "divergence.vwap_regular_bullish" => {
            Some(&["events::extended::row_tests::divergence_vwap_regular_bullish_row_contract"])
        }
        "level.absorption_test" => {
            Some(&["events::extended::row_tests::level_absorption_test_row_contract"])
        }
        "level.band_test" => {
            Some(&["per_row_extended_session_events::level_band_test_row_contract"])
        }
        "level.break" => Some(&["per_row_base_events::level_break_row_contract"]),
        "level.cross" => Some(&["per_row_base_events::level_cross_row_contract"]),
        "level.fib_test" => Some(&["per_row_extended_session_events::level_fib_test_row_contract"]),
        "level.test" => Some(&["per_row_base_events::level_test_row_contract"]),
        "level.wick_test" => {
            Some(&["events::extended::row_signal_tests::level_wick_test_row_contract"])
        }
        "signal.absorption_break" => {
            Some(&["events::extended::row_tests::signal_absorption_break_row_contract"])
        }
        "signal.absorption_rejection" => {
            Some(&["events::extended::row_tests::signal_absorption_rejection_row_contract"])
        }
        "signal.atr_compression" => {
            Some(&["events::extended::row_signal_tests::signal_atr_compression_row_contract"])
        }
        "signal.atr_expansion" => {
            Some(&["events::extended::row_signal_tests::signal_atr_expansion_row_contract"])
        }
        "signal.atr_regime" => {
            Some(&["events::extended::row_signal_tests::signal_atr_regime_row_contract"])
        }
        "signal.breadth_cross" => {
            Some(&["events::panel::row_tests::signal_breadth_cross_row_contract"])
        }
        "signal.candle_marubozu" => {
            Some(&["events::extended::row_signal_tests::signal_candle_marubozu_row_contract"])
        }
        "signal.candle_momentum" => {
            Some(&["events::extended::row_signal_tests::signal_candle_momentum_row_contract"])
        }
        "signal.candle_rejection" => {
            Some(&["events::extended::row_signal_tests::signal_candle_rejection_row_contract"])
        }
        "signal.climax_continuation" => {
            Some(&["events::extended::row_tests::signal_climax_continuation_row_contract"])
        }
        "signal.climax_reversal" => {
            Some(&["events::extended::row_tests::signal_climax_reversal_row_contract"])
        }
        "signal.co_movement_cross" => {
            Some(&["events::panel::row_tests::signal_co_movement_cross_row_contract"])
        }
        "signal.compression_breakout" => {
            Some(&["per_row_event_compression::signal_compression_breakout_row_contract"])
        }
        "signal.compression_range" => {
            Some(&["per_row_event_compression::signal_compression_range_row_contract"])
        }
        "signal.compression_range_onset" => {
            Some(&["per_row_event_compression::signal_compression_range_onset_row_contract"])
        }
        "signal.di_cross" => Some(&["per_row_base_events::di_cross_row_contract"]),
        "signal.dispersion_cross" => {
            Some(&["events::panel::row_tests::signal_dispersion_cross_row_contract"])
        }
        "signal.ema_pair_cross" => Some(&["per_row_base_events::ema_pair_cross_row_contract"]),
        "signal.ema_stack_change" => Some(&["per_row_base_events::ema_stack_change_row_contract"]),
        "signal.gap_continuation" => {
            Some(&["events::extended::row_signal_tests::signal_gap_continuation_row_contract"])
        }
        "signal.gap_fade" => {
            Some(&["events::extended::row_signal_tests::signal_gap_fade_row_contract"])
        }
        "signal.lead_lag_cross" => {
            Some(&["events::panel::row_tests::signal_lead_lag_cross_row_contract"])
        }
        "signal.lower_high_sequence" => {
            Some(&["per_row_extended_session_events::signal_lower_high_sequence_row_contract"])
        }
        "signal.lower_low_sequence" => {
            Some(&["per_row_extended_session_events::signal_lower_low_sequence_row_contract"])
        }
        "signal.macd_histogram_slope_flip" => {
            Some(&["per_row_base_events::macd_histogram_slope_flip_row_contract"])
        }
        "signal.macd_line_signal_cross" => {
            Some(&["per_row_base_events::macd_line_signal_cross_row_contract"])
        }
        "signal.macd_zero_cross" => Some(&["per_row_base_events::macd_zero_cross_row_contract"]),
        "signal.opening_range_breakout" => {
            Some(&["events::session::row_tests::signal_opening_range_breakout_row_contract"])
        }
        "signal.opening_range_sweep" => {
            Some(&["events::session::row_tests::signal_opening_range_sweep_row_contract"])
        }
        "signal.opening_range_sweep_control" => {
            Some(&["events::session::row_tests::signal_opening_range_sweep_control_row_contract"])
        }
        "signal.overnight_continuation" => {
            Some(&["events::session::row_tests::signal_overnight_continuation_row_contract"])
        }
        "signal.overnight_fade" => {
            Some(&["events::session::row_tests::signal_overnight_fade_row_contract"])
        }
        "signal.pattern_hammer" => {
            Some(&["events::extended::row_signal_tests::signal_pattern_hammer_row_contract"])
        }
        "signal.pattern_star" => {
            Some(&["events::extended::row_signal_tests::signal_pattern_star_row_contract"])
        }
        "signal.pattern_three_crows" => {
            Some(&["events::extended::row_signal_tests::signal_pattern_three_crows_row_contract"])
        }
        "signal.pattern_three_soldiers" => Some(&[
            "events::extended::row_signal_tests::signal_pattern_three_soldiers_row_contract",
        ]),
        "signal.profile_concentration" => {
            Some(&["events::extended::row_tests::signal_profile_concentration_row_contract"])
        }
        "signal.relative_strength_cross" => {
            Some(&["events::panel::row_tests::signal_relative_strength_cross_row_contract"])
        }
        "signal.rsi_threshold" => Some(&["per_row_base_events::rsi_threshold_row_contract"]),
        "signal.rsi_zone_change" => Some(&["per_row_base_events::rsi_zone_change_row_contract"]),
        "signal.session_range_contraction" => {
            Some(&["events::session::row_tests::signal_session_range_contraction_row_contract"])
        }
        "signal.trend_continuation" => {
            Some(&["per_row_base_events::trend_continuation_row_contract"])
        }
        _ => None,
    }
}

pub fn row_event_golden(ident: &str) -> bool {
    matches!(
        ident,
        "divergence.ad_regular_bullish"
            | "divergence.atr_bearish"
            | "divergence.breadth_regular_bearish"
            | "divergence.breadth_regular_bullish"
            | "divergence.cmf_hidden_bullish"
            | "divergence.lead_lag_bearish"
            | "divergence.lead_lag_bullish"
            | "divergence.mfi_hidden_bullish"
            | "divergence.mfi_regular_bearish"
            | "divergence.mfi_regular_bullish"
            | "divergence.obv_hidden_bearish"
            | "divergence.obv_regular_bullish"
            | "divergence.rs_regular_bearish"
            | "divergence.rs_regular_bullish"
            | "divergence.rsi_hidden_bearish"
            | "divergence.rsi_hidden_bullish"
            | "divergence.rsi_regular_bearish"
            | "divergence.rsi_regular_bullish"
            | "divergence.vwap_hidden_bullish"
            | "divergence.vwap_regular_bullish"
            | "level.absorption_test"
            | "level.band_test"
            | "level.break"
            | "level.cross"
            | "level.fib_test"
            | "level.test"
            | "level.wick_test"
            | "signal.absorption_break"
            | "signal.absorption_rejection"
            | "signal.atr_compression"
            | "signal.atr_expansion"
            | "signal.atr_regime"
            | "signal.breadth_cross"
            | "signal.candle_marubozu"
            | "signal.candle_momentum"
            | "signal.candle_rejection"
            | "signal.climax_continuation"
            | "signal.climax_reversal"
            | "signal.co_movement_cross"
            | "signal.compression_breakout"
            | "signal.compression_range"
            | "signal.compression_range_onset"
            | "signal.di_cross"
            | "signal.dispersion_cross"
            | "signal.ema_pair_cross"
            | "signal.ema_stack_change"
            | "signal.gap_continuation"
            | "signal.gap_fade"
            | "signal.lead_lag_cross"
            | "signal.lower_high_sequence"
            | "signal.lower_low_sequence"
            | "signal.macd_histogram_slope_flip"
            | "signal.macd_line_signal_cross"
            | "signal.macd_zero_cross"
            | "signal.opening_range_breakout"
            | "signal.opening_range_sweep"
            | "signal.opening_range_sweep_control"
            | "signal.overnight_continuation"
            | "signal.overnight_fade"
            | "signal.pattern_hammer"
            | "signal.pattern_star"
            | "signal.pattern_three_crows"
            | "signal.pattern_three_soldiers"
            | "signal.profile_concentration"
            | "signal.relative_strength_cross"
            | "signal.rsi_threshold"
            | "signal.rsi_zone_change"
            | "signal.session_range_contraction"
            | "signal.trend_continuation"
    )
}

pub fn row_event_precedence(ident: &str) -> Option<&'static str> {
    match ident {
        "level.break" => Some("per_row_base_events::level_break_row_contract"),
        "level.cross" => Some("per_row_base_events::level_cross_row_contract"),
        "level.test" => Some("per_row_base_events::level_test_row_contract"),
        _ => None,
    }
}

pub fn indicator_formula_extra(ident: &str) -> Option<&'static [&'static str]> {
    match ident {
        "fibonacci" => {
            Some(&["readings::extended::tests::fibonacci_reads_the_confirmed_major_leg"])
        }
        "horizontal_band" => Some(&[
            "readings::extended::tests::horizontal_band_selects_the_largest_nearest_cluster",
            "readings::extended::band::tests::horizontal_band_largest_cluster_wins",
            "readings::extended::band::tests::horizontal_band_ties_go_to_nearest_close",
            "readings::extended::band::tests::horizontal_band_equal_centers_go_to_lower",
        ]),
        _ => None,
    }
}

pub fn indicator_edge_extra(ident: &str) -> Option<&'static [&'static str]> {
    match ident {
        "fibonacci" => Some(&[
            "readings::extended::tests::fibonacci_levels_publish_without_atr_but_distance_is_null",
            "readings::extended::tests::fibonacci_nulls_a_zero_span_leg",
        ]),
        "horizontal_band" => Some(&[
            "readings::extended::tests::horizontal_band_is_null_without_two_levels_or_atr",
            "readings::extended::band::tests::horizontal_band_requires_two_levels",
            "readings::extended::band::tests::horizontal_band_equal_levels_form_a_zero_width_cluster",
        ]),
        _ => None,
    }
}

pub fn indicator_availability(ident: &str) -> Option<&'static [&'static str]> {
    match ident {
        "fibonacci" => Some(&[
            "per_row_extended_indicators::fibonacci_availability_starts_at_both_confirmed_anchors",
        ]),
        "horizontal_band" => Some(&[
            "per_row_extended_indicators::horizontal_band_availability_starts_at_two_confirmed_levels",
        ]),
        "pivot_sequence" => Some(&[
            "per_row_extended_indicators::pivot_sequence_availability_starts_at_two_confirmed_pivots",
        ]),
        _ => None,
    }
}

pub fn event_formula_extra(ident: &str) -> Option<&'static [&'static str]> {
    match ident {
        "level.fib_test" => Some(&[
            "events::extended::tests::a_fibonacci_level_inside_the_bar_range_is_tested",
            "events::extended::tests::every_tested_fibonacci_level_emits_an_event",
        ]),
        _ => None,
    }
}

pub fn event_edge_extra(ident: &str) -> Option<&'static [&'static str]> {
    match ident {
        "level.fib_test" => Some(&[
            "events::extended::tests::a_fibonacci_level_outside_the_bar_range_is_not_tested",
        ]),
        _ => None,
    }
}

pub fn event_golden_override(ident: &str) -> Option<&'static [&'static str]> {
    match ident {
        "signal.breakout_retest_hold" => {
            Some(&["retest_events_golden::the_retest_events_match_the_golden_fixture"])
        }
        _ => None,
    }
}

pub fn event_availability_override(ident: &str) -> Option<&'static [&'static str]> {
    match ident {
        "signal.lower_high_sequence" => Some(&[
            "event_validation::sequence_events_record_the_pivot_occurrence_and_availability",
        ]),
        "signal.lower_low_sequence" => Some(&[
            "event_validation::sequence_events_record_the_pivot_occurrence_and_availability",
        ]),
        _ => None,
    }
}

pub const EXTENDED_EDGE_TESTS: &[&str] = &[
    "edge_cases::a_flat_series_does_not_divide_by_zero",
    "edge_cases::zero_volume_publishes_no_volume_weighted_readings",
    "edge_cases::missing_volume_publishes_no_volume_weighted_readings",
    "edge_cases::equal_highs_and_lows_are_handled",
];
pub const EVENT_EDGE_TESTS: &[&str] = &["event_edge_cases::event_batches_are_safe_on_edge_inputs"];

pub const EVENT_FIXTURES: &[&str] = &[
    "crates/quantrule-model/tests/fixtures/extended_events_golden.csv",
    "crates/quantrule-model/tests/fixtures/session_events_golden.csv",
];
