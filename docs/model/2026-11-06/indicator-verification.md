# Indicator Verification Matrix

- Date: 2026-11-06 (UTC)
- Status: Generated from `docs/registry/indicators.json` and `docs/registry/validation.json`
- Claim type: construct and test checklist; no edge claim

The test columns are targets. `◐` means current evidence is family-level or partial; `☐` means the per-row evidence is still required; `—` means not applicable. The construct test is the registry validation claim, not proof of every test class.

| # | Indicator | Code | Area | L/D/S | Registry | Construct test | Predictive | F | C | D | G | E | A | N | P | M | Current | Target |
|---:|---|---:|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | `ema` | 1 | Base | 0/0/2 | ✅ | readings_golden::every_scalar_reading_matches_the_golden_values | pending | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ◐ | ☐ |
| 2 | `rsi` | 2 | Base | 0/4/2 | ✅ | readings_golden::every_scalar_reading_matches_the_golden_values | pending | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ◐ | ☐ |
| 3 | `atr` | 3 | Base | 0/0/0 | ✅ | readings_golden::every_scalar_reading_matches_the_golden_values | pending | ☐ | ☐ | ☐ | ☐ | ☐ | — | — | — | ☐ | ◐ | ☐ |
| 4 | `rvol` | 4 | 3 Participation | 0/2/3 | ✅ | rvol_reads_the_volume_ratio | pending | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ◐ | ☐ |
| 5 | `macd` | 5 | Base | 0/0/3 | ✅ | readings_golden::every_scalar_reading_matches_the_golden_values | pending | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ◐ | ☐ |
| 6 | `pivots` | 6 | 6 Structure | 3/0/0 | ✅ | readings_golden::the_fixture_covers_every_reading_and_both_pivot_scales | pending | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ◐ | ☐ |
| 7 | `context` | 7 | Base | 0/0/3 | ✅ | context_golden::the_context_matches_the_golden_fixture | pending | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ◐ | ☐ |
| 8 | `progression` | 8 | Base | 0/0/0 | ✅ | progression_golden::the_per_bar_fixture_matches_the_computed_series | pending | ☐ | ☐ | ☐ | ☐ | ☐ | — | — | — | ☐ | ◐ | ☐ |
| 9 | `adx` | 9 | Base | 0/0/1 | ✅ | readings_golden::every_scalar_reading_matches_the_golden_values | pending | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ◐ | ☐ |
| 10 | `dmi` | 10 | Base | 0/0/1 | ✅ | readings_golden::every_scalar_reading_matches_the_golden_values | pending | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ◐ | ☐ |
| 11 | `vwap` | 11 | 1 Volume-weighted price | 3/4/3 | ✅ | vwap_is_the_volume_weighted_typical_price | pending | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ◐ | ☐ |
| 12 | `volume_profile` | 12 | 2 Volume by price | 5/2/4 | ✅ | profile_poc_is_the_highest_volume_bin | pending | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ◐ | ☐ |
| 13 | `volume_climax` | 13 | 3 Participation | 0/2/2 | ✅ | climax_flags_a_high_volume_wide_bar | pending | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ◐ | ☐ |
| 14 | `absorption` | 14 | 3 Participation | 1/0/2 | ✅ | absorption_flags_a_small_range_near_vwap | pending | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ◐ | ☐ |
| 15 | `volume_oscillator` | 15 | 3 Participation | 0/0/1 | ✅ | volume_oscillator_reads_the_fast_slow_gap | pending | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ◐ | ☐ |
| 16 | `obv` | 16 | 4 Volume-price flow | 0/4/2 | ✅ | obv_accumulates_signed_volume | pending | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ◐ | ☐ |
| 17 | `cmf` | 17 | 4 Volume-price flow | 0/4/2 | ✅ | cmf_is_one_when_every_close_is_the_high | pending | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ◐ | ☐ |
| 18 | `mfi` | 18 | 4 Volume-price flow | 0/4/2 | ✅ | mfi_is_one_hundred_when_every_bar_rises | pending | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ◐ | ☐ |
| 19 | `ad_line` | 19 | 4 Volume-price flow | 0/2/1 | ✅ | ad_line_accumulates_money_flow | pending | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ◐ | ☐ |
| 20 | `atr_term` | 20 | 5 Volatility structure | 0/2/3 | ✅ | atr_term_is_available_after_the_slow_window | pending | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ◐ | ☐ |
| 21 | `range_expansion` | 21 | 5 Volatility structure | 0/0/2 | ✅ | range_and_mar_read_the_window | pending | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ◐ | ☐ |
| 22 | `mar` | 22 | 5 Volatility structure | 0/0/1 | ✅ | range_and_mar_read_the_window | not_applicable | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ◐ | ☐ |
| 23 | `keltner` | 23 | 5 Volatility structure | 2/0/2 | ✅ | keltner_and_donchian_publish_bands | pending | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ◐ | ☐ |
| 24 | `donchian` | 24 | 6 Structure | 2/0/2 | ✅ | keltner_and_donchian_publish_bands | pending | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ◐ | ☐ |
| 25 | `swing_structure` | 25 | 6 Structure | 1/0/1 | ✅ | structure_and_levels_publish_when_history_exists | pending | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ◐ | ☐ |
| 26 | `level_strength` | 26 | 6 Structure | 1/0/1 | ✅ | structure_and_levels_publish_when_history_exists | pending | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ◐ | ☐ |
| 27 | `aroon` | 27 | 6 Structure | 0/0/2 | ✅ | aroon_reads_a_rising_series | pending | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ◐ | ☐ |
| 28 | `bos_choch` | 28 | 6 Structure | 2/0/0 | ✅ | event_validation::bos_and_choch_are_classified | pending | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ◐ | ☐ |
| 29 | `candle_anatomy` | 29 | 7 Candle anatomy and gaps | 1/0/4 | ✅ | candle_anatomy_reads_the_bar | pending | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ◐ | ☐ |
| 30 | `candle_pattern` | 30 | 7 Candle anatomy and gaps | 0/0/7 | ✅ | candle_pattern_flags_an_engulfing_bar | pending | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ◐ | ☐ |
| 31 | `gap` | 31 | 7 Candle anatomy and gaps | 2/0/2 | ✅ | gap_reads_the_open_jump | pending | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ◐ | ☐ |
| 32 | `path_efficiency` | 32 | 8 Path and duration | 0/0/1 | ✅ | path_reads_efficiency_and_extremes | not_applicable | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ◐ | ☐ |
| 33 | `direction_changes` | 33 | 8 Path and duration | 0/0/1 | ✅ | direction_changes_counts_sign_flips | not_applicable | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ◐ | ☐ |
| 34 | `bars_since_extreme` | 34 | 8 Path and duration | 0/0/2 | ✅ | path_reads_efficiency_and_extremes | pending | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ◐ | ☐ |
| 35 | `drawdown_atr` | 35 | 8 Path and duration | 0/0/2 | ✅ | path_reads_efficiency_and_extremes | pending | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ◐ | ☐ |
| 36 | `range_position` | 36 | 8 Path and duration | 0/0/1 | ✅ | path_reads_efficiency_and_extremes | pending | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ◐ | ☐ |
| 37 | `relative_strength` | 37 | 9 Cross-asset | 0/2/1 | ✅ | relative_strength_ranks_the_faster_instrument | pending | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ◐ | ☐ |
| 38 | `lead_lag` | 38 | 9 Cross-asset | 0/2/1 | ✅ | lead_lag_reads_the_leader | pending | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ◐ | ☐ |
| 39 | `breadth` | 39 | 9 Cross-asset | 0/2/1 | ✅ | breadth_and_dispersion_are_common | pending | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ◐ | ☐ |
| 40 | `co_movement` | 40 | 9 Cross-asset | 0/0/1 | ✅ | co_movement_is_high_for_aligned_series | not_applicable | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ◐ | ☐ |
| 41 | `dispersion` | 41 | 9 Cross-asset | 0/0/1 | ✅ | breadth_and_dispersion_are_common | not_applicable | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ◐ | ☐ |
| 42 | `session` | 42 | 10 Calendar and session | 1/0/2 | ✅ | london_session_and_opening_range | pending | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ◐ | ☐ |
| 43 | `opening_range` | 43 | 10 Calendar and session | 2/0/2 | ✅ | london_session_and_opening_range | pending | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ◐ | ☐ |
| 44 | `overnight_intraday` | 44 | 10 Calendar and session | 0/0/2 | ✅ | overnight_and_intraday_returns_read_the_session | pending | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ◐ | ☐ |
| 45 | `session_range` | 45 | 10 Calendar and session | 0/0/2 | ✅ | session_range_reads_the_session_high_low | pending | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ◐ | ☐ |
| 46 | `calendar_bucket` | 46 | 10 Calendar and session | 0/0/0 | ✅ | london_session_and_opening_range | not_applicable | ☐ | ☐ | ☐ | ☐ | ☐ | — | — | — | ☐ | ◐ | ☐ |

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
