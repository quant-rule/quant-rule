# Indicator Verification Matrix

- Date: 2026-11-06 (UTC)
- Status: Generated from `docs/registry/verification.json`
- Claim type: evidence map; no edge claim

`✅` row-level or tool gate; `◐` family-level evidence; `—` not applicable.

| # | Indicator | Code | Area | L/D/S | Registry | Construct test | Predictive | F | C | D | G | E | A | N | P | M | Status |
|---:|---|---:|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | `ema` | 1 | Base | 0/0/2 | ✅ | per_row_base::ema_row_contract | pending | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — | ✅ | partial (family) |
| 2 | `rsi` | 2 | Base | 0/4/2 | ✅ | per_row_base::rsi_row_contract | pending | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — | ✅ | partial (family) |
| 3 | `atr` | 3 | Base | 0/0/0 | ✅ | per_row_base::atr_row_contract | pending | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — | ✅ | covered |
| 4 | `rvol` | 4 | 3 Participation | 0/2/3 | ✅ | rvol_reads_the_volume_ratio | pending | ✅ | ◐ | ◐ | ◐ | ◐ | — | — | — | ✅ | partial (family) |
| 5 | `macd` | 5 | Base | 0/0/3 | ✅ | readings::macd::tests::the_line_is_the_fast_minus_the_slow_ema | pending | ✅ | ✅ | ✅ | ◐ | ✅ | — | — | — | ✅ | partial (family) |
| 6 | `pivots` | 6 | 6 Structure | 3/0/0 | ✅ | readings_golden::the_fixture_covers_every_reading_and_both_pivot_scales | pending | ✅ | ◐ | ◐ | ◐ | ◐ | — | — | — | ✅ | partial (family) |
| 7 | `context` | 7 | Base | 0/0/3 | ✅ | context_golden::the_context_matches_the_golden_fixture | pending | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — | ✅ | partial (family) |
| 8 | `progression` | 8 | Base | 0/0/0 | ✅ | progression::tests::the_series_length_matches_the_readings_length | pending | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — | ✅ | covered |
| 9 | `adx` | 9 | Base | 0/0/1 | ✅ | readings::adx::tests::an_uptrend_has_positive_direction_and_strength | pending | ✅ | ✅ | ✅ | ◐ | ✅ | — | — | — | ✅ | partial (family) |
| 10 | `dmi` | 10 | Base | 0/0/1 | ✅ | events::dmi::tests::a_direction_change_emits_one_cross | pending | ✅ | ✅ | ◐ | ◐ | ✅ | — | — | — | ✅ | partial (family) |
| 11 | `vwap` | 11 | 1 Volume-weighted price | 3/4/3 | ✅ | vwap_is_the_volume_weighted_typical_price | pending | ✅ | ◐ | ◐ | ◐ | ◐ | — | — | — | ✅ | partial (family) |
| 12 | `volume_profile` | 12 | 2 Volume by price | 5/2/4 | ✅ | profile_poc_is_the_highest_volume_bin | pending | ◐ | ◐ | ◐ | ◐ | ◐ | — | — | — | ✅ | partial (family) |
| 13 | `volume_climax` | 13 | 3 Participation | 0/2/2 | ✅ | climax_flags_a_high_volume_wide_bar | pending | ◐ | ◐ | ◐ | ◐ | ◐ | — | — | — | ✅ | partial (family) |
| 14 | `absorption` | 14 | 3 Participation | 1/0/2 | ✅ | absorption_flags_a_small_range_near_vwap | pending | ✅ | ◐ | ◐ | ◐ | ◐ | — | — | — | ✅ | partial (family) |
| 15 | `volume_oscillator` | 15 | 3 Participation | 0/0/1 | ✅ | volume_oscillator_reads_the_fast_slow_gap | pending | ✅ | ◐ | ◐ | ◐ | ◐ | — | — | — | ✅ | partial (family) |
| 16 | `obv` | 16 | 4 Volume-price flow | 0/4/2 | ✅ | obv_accumulates_signed_volume | pending | ✅ | ◐ | ◐ | ◐ | ◐ | — | — | — | ✅ | partial (family) |
| 17 | `cmf` | 17 | 4 Volume-price flow | 0/4/2 | ✅ | cmf_is_one_when_every_close_is_the_high | pending | ✅ | ◐ | ◐ | ◐ | ◐ | — | — | — | ✅ | partial (family) |
| 18 | `mfi` | 18 | 4 Volume-price flow | 0/4/2 | ✅ | mfi_is_one_hundred_when_every_bar_rises | pending | ✅ | ◐ | ◐ | ◐ | ◐ | — | — | — | ✅ | partial (family) |
| 19 | `ad_line` | 19 | 4 Volume-price flow | 0/2/1 | ✅ | ad_line_accumulates_money_flow | pending | ✅ | ◐ | ◐ | ◐ | ◐ | — | — | — | ✅ | partial (family) |
| 20 | `atr_term` | 20 | 5 Volatility structure | 0/2/3 | ✅ | atr_term_is_available_after_the_slow_window | pending | ✅ | ◐ | ◐ | ◐ | ◐ | — | — | — | ✅ | partial (family) |
| 21 | `range_expansion` | 21 | 5 Volatility structure | 0/0/2 | ✅ | range_and_mar_read_the_window | pending | ◐ | ◐ | ◐ | ◐ | ◐ | — | — | — | ✅ | partial (family) |
| 22 | `mar` | 22 | 5 Volatility structure | 0/0/1 | ✅ | range_and_mar_read_the_window | not_applicable | ✅ | ◐ | ◐ | ◐ | ◐ | — | — | — | ✅ | partial (family) |
| 23 | `keltner` | 23 | 5 Volatility structure | 2/0/2 | ✅ | keltner_and_donchian_publish_bands | pending | ✅ | ◐ | ◐ | ◐ | ◐ | — | — | — | ✅ | partial (family) |
| 24 | `donchian` | 24 | 6 Structure | 2/0/2 | ✅ | keltner_and_donchian_publish_bands | pending | ✅ | ◐ | ◐ | ◐ | ◐ | — | — | — | ✅ | partial (family) |
| 25 | `swing_structure` | 25 | 6 Structure | 1/0/1 | ✅ | structure_and_levels_publish_when_history_exists | pending | ◐ | ◐ | ◐ | ◐ | ◐ | — | — | — | ✅ | partial (family) |
| 26 | `level_strength` | 26 | 6 Structure | 1/0/1 | ✅ | structure_and_levels_publish_when_history_exists | pending | ◐ | ◐ | ◐ | ◐ | ◐ | — | — | — | ✅ | partial (family) |
| 27 | `aroon` | 27 | 6 Structure | 0/0/2 | ✅ | aroon_reads_a_rising_series | pending | ✅ | ◐ | ◐ | ◐ | ◐ | — | — | — | ✅ | partial (family) |
| 28 | `bos_choch` | 28 | 6 Structure | 2/0/0 | ✅ | event_validation::bos_and_choch_are_classified | pending | ◐ | ◐ | ◐ | ◐ | ◐ | — | — | — | ✅ | partial (family) |
| 29 | `candle_anatomy` | 29 | 7 Candle anatomy and gaps | 1/0/4 | ✅ | candle_anatomy_reads_the_bar | pending | ✅ | ◐ | ◐ | ◐ | ◐ | — | — | — | ✅ | partial (family) |
| 30 | `candle_pattern` | 30 | 7 Candle anatomy and gaps | 0/0/7 | ✅ | candle_pattern_flags_an_engulfing_bar | pending | ✅ | ◐ | ◐ | ◐ | ◐ | — | — | — | ✅ | partial (family) |
| 31 | `gap` | 31 | 7 Candle anatomy and gaps | 2/0/2 | ✅ | gap_reads_the_open_jump | pending | ✅ | ◐ | ◐ | ◐ | ◐ | — | — | — | ✅ | partial (family) |
| 32 | `path_efficiency` | 32 | 8 Path and duration | 0/0/1 | ✅ | path_reads_efficiency_and_extremes | not_applicable | ◐ | ◐ | ◐ | ◐ | ◐ | — | — | — | ✅ | partial (family) |
| 33 | `direction_changes` | 33 | 8 Path and duration | 0/0/1 | ✅ | direction_changes_counts_sign_flips | not_applicable | ✅ | ◐ | ◐ | ◐ | ◐ | — | — | — | ✅ | partial (family) |
| 34 | `bars_since_extreme` | 34 | 8 Path and duration | 0/0/2 | ✅ | path_reads_efficiency_and_extremes | pending | ◐ | ◐ | ◐ | ◐ | ◐ | — | — | — | ✅ | partial (family) |
| 35 | `drawdown_atr` | 35 | 8 Path and duration | 0/0/2 | ✅ | path_reads_efficiency_and_extremes | pending | ◐ | ◐ | ◐ | ◐ | ◐ | — | — | — | ✅ | partial (family) |
| 36 | `range_position` | 36 | 8 Path and duration | 0/0/1 | ✅ | path_reads_efficiency_and_extremes | pending | ◐ | ◐ | ◐ | ◐ | ◐ | — | — | — | ✅ | partial (family) |
| 37 | `relative_strength` | 37 | 9 Cross-asset | 0/2/1 | ✅ | relative_strength_ranks_the_faster_instrument | pending | ✅ | ◐ | ◐ | ◐ | ◐ | — | — | — | ✅ | partial (family) |
| 38 | `lead_lag` | 38 | 9 Cross-asset | 0/2/1 | ✅ | lead_lag_reads_the_leader | pending | ✅ | ◐ | ◐ | ◐ | ◐ | — | — | — | ✅ | partial (family) |
| 39 | `breadth` | 39 | 9 Cross-asset | 0/2/1 | ✅ | breadth_and_dispersion_are_common | pending | ✅ | ◐ | ◐ | ◐ | ◐ | — | — | — | ✅ | partial (family) |
| 40 | `co_movement` | 40 | 9 Cross-asset | 0/0/1 | ✅ | co_movement_is_high_for_aligned_series | not_applicable | ✅ | ◐ | ◐ | ◐ | ◐ | — | — | — | ✅ | partial (family) |
| 41 | `dispersion` | 41 | 9 Cross-asset | 0/0/1 | ✅ | breadth_and_dispersion_are_common | not_applicable | ✅ | ◐ | ◐ | ◐ | ◐ | — | — | — | ✅ | partial (family) |
| 42 | `session` | 42 | 10 Calendar and session | 1/0/2 | ✅ | london_session_and_opening_range | pending | ✅ | ◐ | ◐ | ◐ | ◐ | — | — | — | ✅ | partial (family) |
| 43 | `opening_range` | 43 | 10 Calendar and session | 2/0/2 | ✅ | london_session_and_opening_range | pending | ✅ | ◐ | ◐ | ◐ | ◐ | — | — | — | ✅ | partial (family) |
| 44 | `overnight_intraday` | 44 | 10 Calendar and session | 0/0/2 | ✅ | overnight_and_intraday_returns_read_the_session | pending | ◐ | ◐ | ◐ | ◐ | ◐ | — | — | — | ✅ | partial (family) |
| 45 | `session_range` | 45 | 10 Calendar and session | 0/0/2 | ✅ | session_range_reads_the_session_high_low | pending | ✅ | ◐ | ◐ | ◐ | ◐ | — | — | — | ✅ | partial (family) |
| 46 | `calendar_bucket` | 46 | 10 Calendar and session | 0/0/0 | ✅ | london_session_and_opening_range | not_applicable | ◐ | ◐ | ◐ | ◐ | ◐ | — | — | — | ✅ | partial (family) |

## Area totals

| Area | Indicators | L | D | S | Total |
|---|---:|---:|---:|---:|---:|
| Base | 8 | 0 | 4 | 12 | 16 |
| 1 Volume-weighted price | 1 | 3 | 4 | 3 | 10 |
| 2 Volume by price | 1 | 5 | 2 | 4 | 11 |
| 3 Participation | 4 | 1 | 4 | 8 | 13 |
| 4 Volume-price flow | 4 | 0 | 14 | 7 | 21 |
| 5 Volatility structure | 4 | 2 | 2 | 8 | 12 |
| 6 Structure | 6 | 9 | 0 | 6 | 15 |
| 7 Candle anatomy and gaps | 3 | 3 | 0 | 13 | 16 |
| 8 Path and duration | 5 | 0 | 0 | 7 | 7 |
| 9 Cross-asset | 5 | 0 | 6 | 5 | 11 |
| 10 Calendar and session | 5 | 3 | 0 | 8 | 11 |
| **Total** | **46** | **26** | **36** | **81** | **143** |
