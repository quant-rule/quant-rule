#!/usr/bin/env python3
"""Build the verification manifest and the indicator/event matrices.

The manifest is the evidence map. Scope values are:

- ``row``: the test names the row's indicator or event;
- ``family``: the test exercises the family that contains the row;
- ``tool``: a repository gate covers every row;
- ``n/a``: the class does not apply.

Usage:
    python3 tools/build_verification.py --write
"""

from __future__ import annotations

import argparse
import json
import re
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
REGISTRY = ROOT / "docs/registry/indicators.json"
VALIDATION = ROOT / "docs/registry/validation.json"
MANIFEST = ROOT / "docs/registry/verification.json"
INDICATORS_DOC = ROOT / "docs/model/2026-11-06/indicator-verification.md"
EVENTS_DOC = ROOT / "docs/model/2026-11-06/event-verification.md"

AREAS = {
    "Base": [
        "ema",
        "rsi",
        "atr",
        "macd",
        "context",
        "progression",
        "adx",
        "dmi",
    ],
    "1 Volume-weighted price": ["vwap"],
    "2 Volume by price": ["volume_profile"],
    "3 Participation": ["rvol", "volume_climax", "absorption", "volume_oscillator"],
    "4 Volume-price flow": ["obv", "cmf", "mfi", "ad_line"],
    "5 Volatility structure": ["atr_term", "range_expansion", "mar", "keltner"],
    "6 Structure": [
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
    "7 Candle anatomy and gaps": ["candle_anatomy", "candle_pattern", "gap"],
    "8 Path and duration": [
        "path_efficiency",
        "direction_changes",
        "bars_since_extreme",
        "drawdown_atr",
        "range_position",
    ],
    "9 Cross-asset": [
        "relative_strength",
        "lead_lag",
        "breadth",
        "co_movement",
        "dispersion",
    ],
    "10 Calendar and session": [
        "session",
        "opening_range",
        "overnight_intraday",
        "session_range",
        "calendar_bucket",
    ],
}
AREA_OF = {indicator: area for area, ids in AREAS.items() for indicator in ids}

PANEL = {"relative_strength", "lead_lag", "breadth", "co_movement", "dispersion"}
SESSION = {"session", "opening_range", "overnight_intraday", "session_range", "calendar_bucket"}
BASE = set(AREAS["Base"])

CLASSES = [
    "formula",
    "causality",
    "determinism",
    "golden",
    "edge",
    "availability",
    "no_double_count",
    "precedence",
    "math",
    "spec",
    "emitter",
]



ROW_INDICATOR_TESTS = {
    "ema": {
        "formula": ("per_row_base::ema_row_contract",),
        "causality": ("per_row_base::ema_row_contract",),
        "determinism": ("per_row_base::ema_row_contract",),
        "golden": ("per_row_base::ema_row_contract",),
        "edge": ("per_row_base::ema_row_contract",),
    },
    "rsi": {
        "formula": ("per_row_base::rsi_row_contract",),
        "causality": ("per_row_base::rsi_row_contract",),
        "determinism": ("per_row_base::rsi_row_contract",),
        "golden": ("per_row_base::rsi_row_contract",),
        "edge": ("per_row_base::rsi_row_contract",),
    },
    "atr": {
        "formula": ("per_row_base::atr_row_contract",),
        "causality": ("per_row_base::atr_row_contract",),
        "determinism": ("per_row_base::atr_row_contract",),
        "golden": ("per_row_base::atr_row_contract",),
        "edge": ("per_row_base::atr_row_contract",),
    },
    "macd": {
        "formula": ("readings::macd::tests::the_line_is_the_fast_minus_the_slow_ema",),
        "causality": ("readings::macd::tests::appended_bars_do_not_change_earlier_output",),
        "determinism": ("readings::macd::tests::repeated_runs_are_identical",),
        "edge": ("readings::macd::tests::short_and_invalid_inputs_are_all_null",),
    },
    "adx": {
        "formula": ("readings::adx::tests::an_uptrend_has_positive_direction_and_strength",),
        "causality": ("readings::adx::tests::the_series_is_causal",),
        "determinism": ("readings::adx::tests::the_series_is_deterministic",),
        "edge": ("readings::adx::tests::a_flat_series_is_null_not_zero",),
    },
    "dmi": {
        "formula": ("events::dmi::tests::a_direction_change_emits_one_cross",),
        "causality": ("events::dmi::tests::the_series_is_causal",),
        "edge": ("events::dmi::tests::a_flat_series_has_no_event",),
    },
    "context": {
        "formula": ("context_golden::the_context_matches_the_golden_fixture",),
        "causality": ("context_golden::later_candles_do_not_change_earlier_context",),
        "determinism": ("context_golden::the_context_is_deterministic",),
        "golden": ("context_golden::the_context_matches_the_golden_fixture",),
        "edge": ("context_golden::every_available_ratio_retains_a_value_when_the_code_is_null",),
    },
    "progression": {
        "formula": ("progression::tests::the_series_length_matches_the_readings_length",),
        "causality": ("progression::tests::later_bars_do_not_change_earlier_records",),
        "determinism": ("progression::tests::repeated_runs_are_identical",),
        "golden": ("progression_golden::the_per_bar_fixture_matches_the_computed_series",),
        "edge": ("progression::tests::an_empty_series_has_no_bars_and_no_legs",),
    },
}

ROW_INDICATOR_CONTRACTS = {
    "vwap",
    "volume_profile",
    "rvol",
    "volume_climax",
    "absorption",
    "volume_oscillator",
    "obv",
    "cmf",
    "mfi",
    "ad_line",
    "atr_term",
    "range_expansion",
    "mar",
    "keltner",
    "donchian",
    "swing_structure",
    "pivot_sequence",
    "fibonacci",
    "horizontal_band",
    "level_strength",
    "aroon",
    "bos_choch",
    "candle_anatomy",
    "candle_pattern",
    "gap",
    "path_efficiency",
    "direction_changes",
    "bars_since_extreme",
    "drawdown_atr",
    "range_position",
}
ROW_PANEL_CONTRACTS = {
    "relative_strength",
    "lead_lag",
    "breadth",
    "co_movement",
    "dispersion",
}
ROW_SESSION_CONTRACTS = {
    "session",
    "opening_range",
    "overnight_intraday",
    "session_range",
    "calendar_bucket",
}
ROW_BASE_CONTRACTS = {
    "macd": "per_row_base_remaining_indicators::macd_row_contract",
    "adx": "per_row_base_remaining_indicators::adx_row_contract",
    "dmi": "per_row_base_remaining_indicators::dmi_row_contract",
    "pivots": "per_row_base::pivots_row_contract",
}
ROW_INDICATOR_CLASSES = ("formula", "causality", "determinism", "golden", "edge")


def indicator_row_test(ident: str) -> tuple[str, ...]:
    if ident in ROW_INDICATOR_CONTRACTS:
        return (f"per_row_extended_indicators::{ident}_row_contract",)
    if ident in ROW_PANEL_CONTRACTS:
        return (f"per_row_panel_indicators::{ident}_row_contract",)
    if ident in ROW_SESSION_CONTRACTS:
        return (f"per_row_session_indicators::{ident}_row_contract",)
    if ident in ROW_BASE_CONTRACTS:
        return (ROW_BASE_CONTRACTS[ident],)
    return ()


ROW_EVENT_TESTS = {
    "level.test": ("per_row_base_events::level_test_row_contract",),
    "level.cross": ("per_row_base_events::level_cross_row_contract",),
    "level.break": ("per_row_base_events::level_break_row_contract",),
    "divergence.rsi_regular_bullish": (
        "per_row_base_events::divergence_rsi_regular_bullish_row_contract",
    ),
    "divergence.rsi_regular_bearish": (
        "per_row_base_events::divergence_rsi_regular_bearish_row_contract",
    ),
    "divergence.rsi_hidden_bullish": (
        "per_row_base_events::divergence_rsi_hidden_bullish_row_contract",
    ),
    "divergence.rsi_hidden_bearish": (
        "per_row_base_events::divergence_rsi_hidden_bearish_row_contract",
    ),
    "signal.rsi_threshold": ("per_row_base_events::rsi_threshold_row_contract",),
    "signal.ema_pair_cross": ("per_row_base_events::ema_pair_cross_row_contract",),
    "signal.ema_stack_change": ("per_row_base_events::ema_stack_change_row_contract",),
    "signal.rsi_zone_change": ("per_row_base_events::rsi_zone_change_row_contract",),
    "signal.macd_line_signal_cross": (
        "per_row_base_events::macd_line_signal_cross_row_contract",
    ),
    "signal.macd_histogram_slope_flip": (
        "per_row_base_events::macd_histogram_slope_flip_row_contract",
    ),
    "signal.macd_zero_cross": ("per_row_base_events::macd_zero_cross_row_contract",),
    "signal.di_cross": ("per_row_base_events::di_cross_row_contract",),
    "signal.trend_continuation": (
        "per_row_base_events::trend_continuation_row_contract",
    ),
    "signal.compression_breakout": (
        "per_row_event_compression::signal_compression_breakout_row_contract",
    ),
    "signal.compression_range": (
        "per_row_event_compression::signal_compression_range_row_contract",
    ),
    "signal.compression_range_onset": (
        "per_row_event_compression::signal_compression_range_onset_row_contract",
    ),
    "divergence.vwap_regular_bullish": (
        "events::extended::row_tests::divergence_vwap_regular_bullish_row_contract",
    ),
    "divergence.vwap_hidden_bullish": (
        "events::extended::row_tests::divergence_vwap_hidden_bullish_row_contract",
    ),
    "signal.profile_concentration": (
        "events::extended::row_tests::signal_profile_concentration_row_contract",
    ),
    "signal.climax_reversal": (
        "events::extended::row_tests::signal_climax_reversal_row_contract",
    ),
    "signal.climax_continuation": (
        "events::extended::row_tests::signal_climax_continuation_row_contract",
    ),
    "level.absorption_test": (
        "events::extended::row_tests::level_absorption_test_row_contract",
    ),
    "signal.absorption_break": (
        "events::extended::row_tests::signal_absorption_break_row_contract",
    ),
    "signal.absorption_rejection": (
        "events::extended::row_tests::signal_absorption_rejection_row_contract",
    ),
    "divergence.obv_regular_bullish": (
        "events::extended::row_tests::divergence_obv_regular_bullish_row_contract",
    ),
    "divergence.obv_hidden_bearish": (
        "events::extended::row_tests::divergence_obv_hidden_bearish_row_contract",
    ),
    "divergence.cmf_hidden_bullish": (
        "events::extended::row_tests::divergence_cmf_hidden_bullish_row_contract",
    ),
    "divergence.mfi_regular_bullish": (
        "events::extended::row_tests::divergence_mfi_regular_bullish_row_contract",
    ),
    "divergence.mfi_regular_bearish": (
        "events::extended::row_tests::divergence_mfi_regular_bearish_row_contract",
    ),
    "divergence.mfi_hidden_bullish": (
        "events::extended::row_tests::divergence_mfi_hidden_bullish_row_contract",
    ),
    "divergence.ad_regular_bullish": (
        "events::extended::row_tests::divergence_ad_regular_bullish_row_contract",
    ),
    "divergence.atr_bearish": (
        "events::extended::row_tests::divergence_atr_bearish_row_contract",
    ),
    "divergence.rs_regular_bullish": (
        "events::panel::row_tests::divergence_rs_regular_bullish_row_contract",
    ),
    "divergence.rs_regular_bearish": (
        "events::panel::row_tests::divergence_rs_regular_bearish_row_contract",
    ),
    "divergence.lead_lag_bullish": (
        "events::panel::row_tests::divergence_lead_lag_bullish_row_contract",
    ),
    "divergence.lead_lag_bearish": (
        "events::panel::row_tests::divergence_lead_lag_bearish_row_contract",
    ),
    "divergence.breadth_regular_bullish": (
        "events::panel::row_tests::divergence_breadth_regular_bullish_row_contract",
    ),
    "divergence.breadth_regular_bearish": (
        "events::panel::row_tests::divergence_breadth_regular_bearish_row_contract",
    ),
    "level.wick_test": (
        "events::extended::row_signal_tests::level_wick_test_row_contract",
    ),
    "signal.atr_compression": (
        "events::extended::row_signal_tests::signal_atr_compression_row_contract",
    ),
    "signal.atr_expansion": (
        "events::extended::row_signal_tests::signal_atr_expansion_row_contract",
    ),
    "signal.atr_regime": (
        "events::extended::row_signal_tests::signal_atr_regime_row_contract",
    ),
    "signal.candle_momentum": (
        "events::extended::row_signal_tests::signal_candle_momentum_row_contract",
    ),
    "signal.candle_rejection": (
        "events::extended::row_signal_tests::signal_candle_rejection_row_contract",
    ),
    "signal.candle_marubozu": (
        "events::extended::row_signal_tests::signal_candle_marubozu_row_contract",
    ),
    "signal.pattern_hammer": (
        "events::extended::row_signal_tests::signal_pattern_hammer_row_contract",
    ),
    "signal.pattern_star": (
        "events::extended::row_signal_tests::signal_pattern_star_row_contract",
    ),
    "signal.pattern_three_soldiers": (
        "events::extended::row_signal_tests::signal_pattern_three_soldiers_row_contract",
    ),
    "signal.pattern_three_crows": (
        "events::extended::row_signal_tests::signal_pattern_three_crows_row_contract",
    ),
    "signal.gap_continuation": (
        "events::extended::row_signal_tests::signal_gap_continuation_row_contract",
    ),
    "signal.gap_fade": (
        "events::extended::row_signal_tests::signal_gap_fade_row_contract",
    ),
    "signal.relative_strength_cross": (
        "events::panel::row_tests::signal_relative_strength_cross_row_contract",
    ),
    "signal.lead_lag_cross": (
        "events::panel::row_tests::signal_lead_lag_cross_row_contract",
    ),
    "signal.breadth_cross": (
        "events::panel::row_tests::signal_breadth_cross_row_contract",
    ),
    "signal.co_movement_cross": (
        "events::panel::row_tests::signal_co_movement_cross_row_contract",
    ),
    "signal.dispersion_cross": (
        "events::panel::row_tests::signal_dispersion_cross_row_contract",
    ),
    "signal.opening_range_breakout": (
        "events::session::row_tests::signal_opening_range_breakout_row_contract",
    ),
    "signal.opening_range_sweep": (
        "events::session::row_tests::signal_opening_range_sweep_row_contract",
    ),
    "signal.opening_range_sweep_control": (
        "events::session::row_tests::signal_opening_range_sweep_control_row_contract",
    ),
    "signal.overnight_continuation": (
        "events::session::row_tests::signal_overnight_continuation_row_contract",
    ),
    "signal.overnight_fade": (
        "events::session::row_tests::signal_overnight_fade_row_contract",
    ),
    "signal.session_range_contraction": (
        "events::session::row_tests::signal_session_range_contraction_row_contract",
    ),
    "signal.lower_high_sequence": (
        "per_row_extended_session_events::signal_lower_high_sequence_row_contract",
    ),
    "signal.lower_low_sequence": (
        "per_row_extended_session_events::signal_lower_low_sequence_row_contract",
    ),
    "level.fib_test": (
        "per_row_extended_session_events::level_fib_test_row_contract",
    ),
    "level.band_test": (
        "per_row_extended_session_events::level_band_test_row_contract",
    ),
}
ROW_EVENT_CLASSES = (
    "formula",
    "causality",
    "determinism",
    "availability",
    "no_double_count",
)
ROW_EVENT_GOLDEN = {
    "signal.compression_breakout",
    "signal.compression_range",
    "signal.compression_range_onset",
    "divergence.vwap_regular_bullish",
    "divergence.vwap_hidden_bullish",
    "signal.profile_concentration",
    "signal.climax_reversal",
    "signal.climax_continuation",
    "level.absorption_test",
    "signal.absorption_break",
    "signal.absorption_rejection",
    "divergence.obv_regular_bullish",
    "divergence.obv_hidden_bearish",
    "divergence.cmf_hidden_bullish",
    "divergence.mfi_regular_bullish",
    "divergence.mfi_regular_bearish",
    "divergence.mfi_hidden_bullish",
    "divergence.ad_regular_bullish",
    "divergence.atr_bearish",
    "divergence.rs_regular_bullish",
    "divergence.rs_regular_bearish",
    "divergence.lead_lag_bullish",
    "divergence.lead_lag_bearish",
    "divergence.breadth_regular_bullish",
    "divergence.breadth_regular_bearish",
    "level.wick_test",
    "signal.atr_compression",
    "signal.atr_expansion",
    "signal.atr_regime",
    "signal.candle_momentum",
    "signal.candle_rejection",
    "signal.candle_marubozu",
    "signal.pattern_hammer",
    "signal.pattern_star",
    "signal.pattern_three_soldiers",
    "signal.pattern_three_crows",
    "signal.gap_continuation",
    "signal.gap_fade",
    "signal.relative_strength_cross",
    "signal.lead_lag_cross",
    "signal.breadth_cross",
    "signal.co_movement_cross",
    "signal.dispersion_cross",
    "signal.opening_range_breakout",
    "signal.opening_range_sweep",
    "signal.opening_range_sweep_control",
    "signal.overnight_continuation",
    "signal.overnight_fade",
    "signal.session_range_contraction",
    "level.test",
    "level.cross",
    "level.break",
    "divergence.rsi_regular_bullish",
    "divergence.rsi_regular_bearish",
    "divergence.rsi_hidden_bullish",
    "divergence.rsi_hidden_bearish",
    "signal.rsi_threshold",
    "signal.ema_pair_cross",
    "signal.ema_stack_change",
    "signal.rsi_zone_change",
    "signal.macd_line_signal_cross",
    "signal.macd_histogram_slope_flip",
    "signal.macd_zero_cross",
    "signal.di_cross",
    "signal.trend_continuation",
    "signal.lower_high_sequence",
    "signal.lower_low_sequence",
    "level.fib_test",
    "level.band_test",
}
ROW_EVENT_PRECEDENCE = {
    "level.test": "per_row_base_events::level_test_row_contract",
    "level.cross": "per_row_base_events::level_cross_row_contract",
    "level.break": "per_row_base_events::level_break_row_contract",
}
ROW_FIXTURE_EVENT_CLASSES = (
    "formula",
    "causality",
    "determinism",
    "golden",
    "availability",
    "no_double_count",
)
EVENT_FIXTURES = (
    ROOT / "crates/quantrule-model/tests/fixtures/extended_events_golden.csv",
    ROOT / "crates/quantrule-model/tests/fixtures/session_events_golden.csv",
)


def fixture_event_tests(events: list[dict[str, object]]) -> dict[str, tuple[str, ...]]:
    by_code = {int(event["code"]): str(event["id"]) for event in events}
    tests: dict[str, tuple[str, ...]] = {}
    for path in EVENT_FIXTURES:
        for line in path.read_text(encoding="utf-8").splitlines():
            if not line or line.startswith("#") or line.startswith("bar,"):
                continue
            ident = by_code[int(line.split(",")[1])]
            tests[ident] = (
                f"per_row_extended_session_events::{ident.replace('.', '_')}_row_contract",
            )
    return tests


def evidence(scope: str, *tests: str) -> dict[str, object]:
    return {"scope": scope, "tests": list(tests)}


def indicator_family(indicator: str) -> str:
    if indicator in PANEL:
        return "panel"
    if indicator in SESSION:
        return "session"
    if indicator == "progression":
        return "progression"
    if indicator == "context":
        return "context"
    if indicator in BASE:
        return "base"
    return "extended"


def indicator_tests(indicator: str) -> dict[str, dict[str, object]]:
    family = indicator_family(indicator)
    family_tests = {
        "base": {
            "causality": ("hardening::every_prefix_keeps_the_output_lengths_and_ranges",),
            "determinism": ("hardening::the_model_run_is_deterministic_across_repeated_runs",),
            "golden": ("readings_golden::every_scalar_reading_matches_the_golden_values",),
            "edge": ("hardening::every_prefix_keeps_the_output_lengths_and_ranges",),
        },
        "extended": {
            "causality": ("extended_readings_golden::appending_bars_does_not_change_past_readings",),
            "determinism": ("extended_readings_golden::repeated_computation_is_identical",),
            "golden": ("extended_readings_golden::the_extended_readings_match_the_golden_fixture",),
            "edge": ("edge_cases::an_empty_series_publishes_no_readings",),
        },
        "panel": {
            "causality": ("panel_readings_golden::appending_bars_does_not_change_past_panel_readings",),
            "determinism": ("panel_readings_golden::repeated_panel_computation_is_identical",),
            "golden": ("panel_readings_golden::the_panel_readings_match_the_golden_fixture",),
            "edge": ("edge_cases::a_one_bar_panel_and_session_are_safe",),
        },
        "session": {
            "causality": ("session_readings_golden::appending_bars_does_not_change_past_session_readings",),
            "determinism": ("session_readings_golden::repeated_session_computation_is_identical",),
            "golden": ("session_readings_golden::the_session_readings_match_the_golden_fixture",),
            "edge": ("edge_cases::a_one_bar_panel_and_session_are_safe",),
        },
        "progression": {
            "causality": ("progression_golden::every_prefix_matches_the_full_series",),
            "determinism": ("progression_golden::the_per_bar_fixture_matches_the_computed_series",),
            "golden": ("progression_golden::the_per_bar_fixture_matches_the_computed_series",),
            "edge": ("edge_cases::an_empty_series_publishes_no_readings",),
        },
        "context": {
            "causality": ("context_golden::later_candles_do_not_change_earlier_context",),
            "determinism": ("context_golden::the_context_is_deterministic",),
            "golden": ("context_golden::the_context_matches_the_golden_fixture",),
            "edge": ("edge_cases::an_empty_series_publishes_no_readings",),
        },
    }[family]
    return family_tests


def event_golden(event: dict[str, object]) -> tuple[str, ...]:
    event_type = int(event["event_type"])
    source = int(event["source"])
    if event_type == 1:
        return ("level_events_golden::the_level_events_match_the_golden_fixture",)
    if event_type == 2:
        return ("divergence_events_golden::the_divergence_events_match_the_golden_fixture_at_both_scales",)
    if source >= 19:
        return ("event_validation::extended_and_session_events_match_their_fixtures",)
    return ("signal_events_golden::the_signal_events_match_the_golden_fixture",)


def row_scope(test: str, ident: str) -> str:
    suffix = ident.split(".")[-1].replace("_", "").lower()
    return "row" if suffix and suffix in test.replace("_", "").lower() else "family"


def indicator_entry(indicator: dict[str, object], claims: dict[str, dict[str, object]]) -> dict[str, object]:
    ident = str(indicator["id"])
    claim = claims.get(ident, {})
    tests = list(claim.get("construct_tests", []))
    formula_scope = row_scope(tests[0], ident) if tests else "family"
    family_tests = indicator_tests(ident)
    has_events = any(event["indicator"] == ident for event in event_registry)
    evidence_map = {
        "formula": evidence(formula_scope, *tests) if tests else evidence("n/a"),
        "causality": evidence("family", *family_tests["causality"]),
        "determinism": evidence("family", *family_tests["determinism"]),
        "golden": evidence("family", *family_tests["golden"]),
        "edge": evidence("family", *family_tests["edge"]),
        "availability": evidence("n/a"),
        "no_double_count": evidence("n/a"),
        "precedence": evidence("n/a"),
        "math": evidence("tool", "check_registry.py", "check_validation.py"),
        "spec": evidence("family", "report_command::two_model_specs_with_disjoint_active_events_stay_disjoint")
        if has_events
        else evidence("n/a"),
        "emitter": evidence("n/a"),
    }
    for name, tests in ROW_INDICATOR_TESTS.get(ident, {}).items():
        evidence_map[name] = evidence("row", *tests)
    row_test = indicator_row_test(ident)
    if row_test:
        for name in ROW_INDICATOR_CLASSES:
            evidence_map[name] = evidence("row", *row_test)
    return {"id": ident, "kind": "indicator", "evidence": evidence_map}


def event_entry(
    event: dict[str, object],
    claims: dict[str, dict[str, object]],
    fixture_tests: dict[str, tuple[str, ...]],
) -> dict[str, object]:
    ident = str(event["id"])
    claim = claims.get(ident, {})
    tests = list(claim.get("construct_tests", []))
    formula_scope = row_scope(tests[0], ident) if tests else "family"
    evidence_map = {
        "formula": evidence(formula_scope, *tests) if tests else evidence("n/a"),
        "causality": evidence("family", "event_validation::appending_bars_does_not_change_past_events"),
        "determinism": evidence("family", "event_validation::repeated_model_runs_publish_the_same_events"),
        "golden": evidence("family", *event_golden(event)),
        "edge": evidence("family", "edge_cases::an_empty_series_publishes_no_readings"),
        "availability": evidence("family", "event_validation::every_batch_respects_availability_and_order"),
        "no_double_count": evidence("family", "event_validation::one_occurrence_is_recorded_once"),
        "precedence": evidence("row", ROW_EVENT_PRECEDENCE[ident])
        if ident in ROW_EVENT_PRECEDENCE
        else evidence("n/a"),
        "math": evidence("tool", "check_registry.py"),
        "spec": evidence("family", "report_command::two_model_specs_with_disjoint_active_events_stay_disjoint"),
        "emitter": evidence("tool", "check_event_emitters.py"),
    }
    for name in ROW_EVENT_CLASSES:
        if ident in ROW_EVENT_TESTS:
            evidence_map[name] = evidence("row", *ROW_EVENT_TESTS[ident])
    if ident in ROW_EVENT_GOLDEN:
        evidence_map["golden"] = evidence("row", *ROW_EVENT_TESTS[ident])
    if ident in fixture_tests:
        for name in ROW_FIXTURE_EVENT_CLASSES:
            evidence_map[name] = evidence("row", *fixture_tests[ident])
    return {"id": ident, "kind": "event", "evidence": evidence_map}


def cell(entry: dict[str, object], name: str) -> str:
    scope = entry["evidence"][name]["scope"]  # type: ignore[index]
    return {"row": "✅", "family": "◐", "tool": "✅", "n/a": "—"}.get(scope, "☐")


def write_indicators(manifest: dict[str, object]) -> None:
    entries = {entry["id"]: entry for entry in manifest["indicators"]}  # type: ignore[index]
    counts = defaultdict(lambda: [0, 0, 0])
    for event in event_registry:
        counts[event["indicator"]][int(event["event_type"]) - 1] += 1
    lines = [
        "# Indicator Verification Matrix",
        "",
        "- Date: 2026-11-06 (UTC)",
        "- Status: Generated from `docs/registry/verification.json`",
        "- Claim type: evidence map; no edge claim",
        "",
        "`✅` row-level or tool gate; `◐` family-level evidence; `—` not applicable.",
        "",
        "| # | Indicator | Code | Area | L/D/S | Registry | Construct test | Predictive | F | C | D | G | E | A | N | P | M | Status |",
        "|---:|---|---:|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|",
    ]
    for number, indicator in enumerate(sorted(indicator_registry, key=lambda item: item["code"]), 1):
        ident = indicator["id"]
        entry = entries[ident]
        claim = claims.get(ident, {})
        l, d, s = counts[ident]
        formula_tests = entry["evidence"]["formula"]["tests"]
        tests = ", ".join(formula_tests) if formula_tests else "—"
        lines.append(
            "| "
            + " | ".join(
                [
                    str(number),
                    f"`{ident}`",
                    str(indicator["code"]),
                    AREA_OF.get(ident, "—"),
                    f"{l}/{d}/{s}",
                    "✅",
                    tests.replace("|", "\\|"),
                    str(claim.get("predictive_status", "—")),
                    cell(entry, "formula"),
                    cell(entry, "causality"),
                    cell(entry, "determinism"),
                    cell(entry, "golden"),
                    cell(entry, "edge"),
                    cell(entry, "availability"),
                    cell(entry, "no_double_count"),
                    cell(entry, "precedence"),
                    cell(entry, "math"),
                    "partial (family)" if any(cell(entry, c) == "◐" for c in CLASSES if c not in {"formula", "math", "emitter"}) else "covered",
                ]
            )
            + " |"
        )
    lines += ["", "## Area totals", "", "| Area | Indicators | L | D | S | Total |", "|---|---:|---:|---:|---:|---:|"]
    for area, ids in AREAS.items():
        l = d = s = 0
        for ident in ids:
            l += counts[ident][0]
            d += counts[ident][1]
            s += counts[ident][2]
        lines.append(f"| {area} | {len(ids)} | {l} | {d} | {s} | {l + d + s} |")
    lines.append(
        f"| **Total** | **{len(indicator_registry)}** | **{sum(v[0] for v in counts.values())}** | **{sum(v[1] for v in counts.values())}** | **{sum(v[2] for v in counts.values())}** | **{len(event_registry)}** |"
    )
    INDICATORS_DOC.write_text("\n".join(lines) + "\n")


def write_events(manifest: dict[str, object]) -> None:
    entries = {entry["id"]: entry for entry in manifest["events"]}  # type: ignore[index]
    lines = [
        "# Event Verification Matrix",
        "",
        "- Date: 2026-11-06 (UTC)",
        "- Status: Generated from `docs/registry/verification.json`",
        "- Claim type: evidence map; no edge claim",
        "",
        "`✅` row-level or tool gate; `◐` family-level evidence; `—` not applicable. `event_code` is the identity column.",
        "",
        "| Event ID | Code | Type | Subtype | Sources | Role | Formula | Indicator | Construct test | Predictive | Emitter | Causality | Determinism | Availability | Dedupe | Precedence | Spec | Status |",
        "|---|---:|---:|---:|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|",
    ]
    for event in sorted(event_registry, key=lambda item: item["code"]):
        ident = event["id"]
        entry = entries[ident]
        claim = claims.get(ident, {})
        formula_tests = entry["evidence"]["formula"]["tests"]
        tests = ", ".join(formula_tests) if formula_tests else "—"
        sources = ",".join(str(value) for value in event.get("sources", [event.get("source")]))
        lines.append(
            "| "
            + " | ".join(
                [
                    f"`{ident}`",
                    str(event["code"]),
                    str(event["event_type"]),
                    str(event["subtype"]),
                    sources,
                    str(event["role"]),
                    str(event.get("formula", "")).replace("|", "\\|"),
                    f"`{event['indicator']}`",
                    tests.replace("|", "\\|"),
                    str(claim.get("predictive_status", "—")),
                    cell(entry, "emitter"),
                    cell(entry, "causality"),
                    cell(entry, "determinism"),
                    cell(entry, "availability"),
                    cell(entry, "no_double_count"),
                    cell(entry, "precedence"),
                    cell(entry, "spec"),
                    "partial (family)" if any(cell(entry, c) == "◐" for c in CLASSES if c not in {"formula", "math", "emitter"}) else "covered",
                ]
            )
            + " |"
        )
    EVENTS_DOC.write_text("\n".join(lines) + "\n")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true", help="write the manifest and matrices")
    args = parser.parse_args()
    if not args.write:
        parser.print_help()
        return 0

    registry = json.loads(REGISTRY.read_text(encoding="utf-8"))
    validation = json.loads(VALIDATION.read_text(encoding="utf-8"))
    global indicator_registry, event_registry, claims
    indicator_registry = registry["indicators"]
    event_registry = registry["events"]
    claims = {claim["id"]: claim for claim in validation["claims"]}
    fixture_tests = fixture_event_tests(event_registry)
    manifest = {
        "schema": "quantrule.verification/v1",
        "version": "2026-11-06.2",
        "indicators": [indicator_entry(indicator, claims) for indicator in indicator_registry],
        "events": [
            event_entry(event, claims, fixture_tests) for event in event_registry
        ],
    }
    MANIFEST.write_text(json.dumps(manifest, indent=2) + "\n")
    write_indicators(manifest)
    write_events(manifest)
    print(
        f"wrote verification: {len(manifest['indicators'])} indicators, "
        f"{len(manifest['events'])} events"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
