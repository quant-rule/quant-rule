# Construct Coverage — P1b-2

- Date: 2026-11-05 (UTC)
- Status: **54 of 54 covered**; the session-range defect found during this work is fixed in [#482](https://github.com/quant-rule/quant-rule/issues/482)
- Parent: [#469](https://github.com/quant-rule/quant-rule/issues/469), [#480](https://github.com/quant-rule/quant-rule/issues/480), [#481](https://github.com/quant-rule/quant-rule/issues/481)
- Plan: [`469-validation-claims.md`](../execution-plans/2026-11-05/469-validation-claims.md)
- Claim type: construct-test coverage; no edge claim

## 1. Rule

A case counts only when it reaches the detector with inputs that satisfy
the event formula and asserts the published event code. The tests are
unit tests beside the detectors, so they can call the private family
functions without exposing a test-only public API.

## 2. Coverage

| Code | Event | Role | Construct test | Status |
|---:|---|---|---|---|
| 4 | `divergence.rsi_regular_bullish` | signal | `divergence::tests::a_lower_low_with_a_higher_rsi_low_is_regular_bullish` | covered |
| 6 | `divergence.rsi_hidden_bullish` | signal | `divergence::tests::a_higher_low_with_a_lower_rsi_low_is_hidden_bullish` | covered |
| 10 | `signal.ema_stack_change` | signal | `signal::tests::ema_stack_and_rsi_zone_changes_are_published` | covered |
| 17 | `signal.compression_breakout` | signal | `compression::tests::a_coiling_window_followed_by_a_breakout_emits_one_event` | covered |
| 23 | `divergence.vwap_regular_bullish` | signal | `extended::tests::divergence_sources_cover_their_events` | covered |
| 25 | `divergence.vwap_hidden_bullish` | signal | `extended::tests::divergence_sources_cover_their_events` | covered |
| 40 | `signal.profile_concentration` | context | `extended::tests::absorption_wick_and_profile_events_are_exercised` | covered |
| 48 | `signal.climax_reversal` | signal | `event_validation::extended_and_session_events_match_their_fixtures` | covered |
| 49 | `signal.climax_continuation` | signal | `event_validation::extended_and_session_events_match_their_fixtures` | covered |
| 50 | `level.absorption_test` | context | `extended::tests::absorption_wick_and_profile_events_are_exercised` | covered |
| 51 | `signal.absorption_break` | signal | `extended::tests::absorption_wick_and_profile_events_are_exercised` | covered |
| 52 | `signal.absorption_rejection` | signal | `extended::tests::absorption_wick_and_profile_events_are_exercised` | covered |
| 54 | `divergence.obv_regular_bullish` | signal | `extended::tests::divergence_sources_cover_their_events` | covered |
| 57 | `divergence.obv_hidden_bearish` | signal | `extended::tests::divergence_sources_cover_their_events` | covered |
| 62 | `divergence.cmf_hidden_bullish` | signal | `extended::tests::divergence_sources_cover_their_events` | covered |
| 66 | `divergence.mfi_regular_bullish` | signal | `extended::tests::divergence_sources_cover_their_events` | covered |
| 67 | `divergence.mfi_regular_bearish` | signal | `extended::tests::divergence_sources_cover_their_events` | covered |
| 68 | `divergence.mfi_hidden_bullish` | signal | `extended::tests::divergence_sources_cover_their_events` | covered |
| 72 | `divergence.ad_regular_bullish` | signal | `extended::tests::divergence_sources_cover_their_events` | covered |
| 81 | `level.wick_test` | context | `extended::tests::absorption_wick_and_profile_events_are_exercised` | covered |
| 85 | `divergence.atr_bearish` | signal | `extended::tests::divergence_sources_cover_their_events` | covered |
| 86 | `divergence.rs_regular_bullish` | signal | `panel::tests::panel_divergences_and_crosses_are_exercised` | covered |
| 87 | `divergence.rs_regular_bearish` | signal | `panel::tests::panel_divergences_and_crosses_are_exercised` | covered |
| 88 | `divergence.lead_lag_bullish` | signal | `panel::tests::panel_divergences_and_crosses_are_exercised` | covered |
| 89 | `divergence.lead_lag_bearish` | signal | `panel::tests::panel_divergences_and_crosses_are_exercised` | covered |
| 90 | `divergence.breadth_regular_bullish` | signal | `panel::tests::panel_divergences_and_crosses_are_exercised` | covered |
| 91 | `divergence.breadth_regular_bearish` | signal | `panel::tests::panel_divergences_and_crosses_are_exercised` | covered |
| 92 | `signal.atr_compression` | context | `signals::tests::atr_candle_pattern_and_gap_events_are_exercised` | covered |
| 93 | `signal.atr_expansion` | context | `signals::tests::atr_candle_pattern_and_gap_events_are_exercised` | covered |
| 94 | `signal.atr_regime` | context | `signals::tests::atr_candle_pattern_and_gap_events_are_exercised` | covered |
| 106 | `signal.candle_momentum` | context | `signals::tests::atr_candle_pattern_and_gap_events_are_exercised` | covered |
| 107 | `signal.candle_rejection` | context | `signals::tests::atr_candle_pattern_and_gap_events_are_exercised` | covered |
| 109 | `signal.candle_marubozu` | context | `signals::tests::atr_candle_pattern_and_gap_events_are_exercised` | covered |
| 113 | `signal.pattern_hammer` | context | `signals::tests::atr_candle_pattern_and_gap_events_are_exercised` | covered |
| 114 | `signal.pattern_star` | context | `signals::tests::atr_candle_pattern_and_gap_events_are_exercised` | covered |
| 115 | `signal.pattern_three_soldiers` | context | `signals::tests::atr_candle_pattern_and_gap_events_are_exercised` | covered |
| 116 | `signal.pattern_three_crows` | context | `signals::tests::atr_candle_pattern_and_gap_events_are_exercised` | covered |
| 117 | `signal.gap_continuation` | context | `signals::tests::atr_candle_pattern_and_gap_events_are_exercised` | covered |
| 118 | `signal.gap_fade` | context | `signals::tests::atr_candle_pattern_and_gap_events_are_exercised` | covered |
| 126 | `signal.relative_strength_cross` | context | `panel::tests::panel_divergences_and_crosses_are_exercised` | covered |
| 127 | `signal.lead_lag_cross` | context | `panel::tests::panel_divergences_and_crosses_are_exercised` | covered |
| 128 | `signal.breadth_cross` | context | `panel::tests::panel_divergences_and_crosses_are_exercised` | covered |
| 129 | `signal.co_movement_cross` | diagnostic | `panel::tests::panel_divergences_and_crosses_are_exercised` | covered |
| 130 | `signal.dispersion_cross` | diagnostic | `panel::tests::panel_divergences_and_crosses_are_exercised` | covered |
| 131 | `signal.session_open` | context | `session::tests::session_open_and_close_are_published` | covered |
| 132 | `signal.session_close` | context | `session::tests::session_open_and_close_are_published` | covered |
| 133 | `signal.opening_range_breakout` | context | `session::tests::opening_range_breakout_and_overnight_continuation_are_published` | covered |
| 134 | `signal.opening_range_fade` | context | `session::tests::session_fade_and_range_events_are_exercised` | covered |
| 135 | `signal.overnight_continuation` | signal | `session::tests::opening_range_breakout_and_overnight_continuation_are_published` | covered |
| 136 | `signal.overnight_fade` | signal | `session::tests::session_fade_and_range_events_are_exercised` | covered |
| 137 | `signal.session_range_expansion` | context | `session::tests::session_fade_and_range_events_are_exercised` | covered |
| 138 | `signal.session_range_contraction` | context | `session::tests::session_fade_and_range_events_are_exercised` | covered |
| 141 | `level.session_vwap_test` | context | `session::tests::session_level_events_are_published` | covered |
| 142 | `level.opening_range_test` | context | `session::tests::the_opening_range_test_fires_on_a_touch` | covered |
| 143 | `level.opening_range_break` | context | `session::tests::session_level_events_are_published` | covered |

## 3. Defect found and fixed

`signal.session_range_contraction` (code 138) was active but unreachable.
The range cross was guarded by `previous_inside`, and `session_range_atr` is
cumulative within a session, so the cross-down was never evaluated. The fix
compares the first range of a new session with the prior session's last
inside range, and keeps the within-session comparison for later bars. The
construct test `session::tests::session_fade_and_range_events_are_exercised`
now asserts code 138. This is a model behavior change; the Quant Developer
implemented it and the Quant Validator pass is recorded in the plan and AI
transparency log. Human acceptance remains required.

## 4. Verification

```bash
cargo test -p quantrule-model --lib events::extended::tests::divergence_sources_cover_their_events
cargo test -p quantrule-model --lib events::extended::tests::absorption_wick_and_profile_events_are_exercised
cargo test -p quantrule-model --lib events::signals::tests::atr_candle_pattern_and_gap_events_are_exercised
cargo test -p quantrule-model --lib events::session::tests::session_fade_and_range_events_are_exercised
cargo test -p quantrule-model --lib events::panel::tests::panel_divergences_and_crosses_are_exercised
cargo test -p quantrule-model
python3 tools/check_validation.py
```
