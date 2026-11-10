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
| 4 | `rvol` | 4 | 3 Participation | 0/2/3 | ✅ | per_row_extended_indicators::rvol_row_contract | pending | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — | ✅ | partial (family) |
| 5 | `macd` | 5 | Base | 0/0/3 | ✅ | per_row_base_remaining_indicators::macd_row_contract | pending | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — | ✅ | partial (family) |
| 6 | `pivots` | 6 | 6 Structure | 3/0/0 | ✅ | per_row_base::pivots_row_contract | pending | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — | ✅ | partial (family) |
| 7 | `context` | 7 | Base | 0/0/4 | ✅ | context_golden::the_context_matches_the_golden_fixture | pending | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — | ✅ | partial (family) |
| 8 | `progression` | 8 | Base | 0/0/0 | ✅ | progression::tests::the_series_length_matches_the_readings_length | pending | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — | ✅ | covered |
| 9 | `adx` | 9 | Base | 0/0/1 | ✅ | per_row_base_remaining_indicators::adx_row_contract | pending | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — | ✅ | partial (family) |
| 10 | `dmi` | 10 | Base | 0/0/1 | ✅ | per_row_base_remaining_indicators::dmi_row_contract | pending | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — | ✅ | partial (family) |
| 11 | `vwap` | 11 | 1 Volume-weighted price | 3/4/3 | ✅ | per_row_extended_indicators::vwap_row_contract | pending | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — | ✅ | partial (family) |
| 12 | `volume_profile` | 12 | 2 Volume by price | 5/2/4 | ✅ | per_row_extended_indicators::volume_profile_row_contract | pending | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — | ✅ | partial (family) |
| 13 | `volume_climax` | 13 | 3 Participation | 0/2/2 | ✅ | per_row_extended_indicators::volume_climax_row_contract | pending | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — | ✅ | partial (family) |
| 14 | `absorption` | 14 | 3 Participation | 1/0/2 | ✅ | per_row_extended_indicators::absorption_row_contract | pending | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — | ✅ | partial (family) |
| 15 | `volume_oscillator` | 15 | 3 Participation | 0/0/1 | ✅ | per_row_extended_indicators::volume_oscillator_row_contract | pending | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — | ✅ | partial (family) |
| 16 | `obv` | 16 | 4 Volume-price flow | 0/4/2 | ✅ | per_row_extended_indicators::obv_row_contract | pending | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — | ✅ | partial (family) |
| 17 | `cmf` | 17 | 4 Volume-price flow | 0/4/2 | ✅ | per_row_extended_indicators::cmf_row_contract | pending | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — | ✅ | partial (family) |
| 18 | `mfi` | 18 | 4 Volume-price flow | 0/4/2 | ✅ | per_row_extended_indicators::mfi_row_contract | pending | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — | ✅ | partial (family) |
| 19 | `ad_line` | 19 | 4 Volume-price flow | 0/2/1 | ✅ | per_row_extended_indicators::ad_line_row_contract | pending | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — | ✅ | partial (family) |
| 20 | `atr_term` | 20 | 5 Volatility structure | 0/2/3 | ✅ | per_row_extended_indicators::atr_term_row_contract | pending | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — | ✅ | partial (family) |
| 21 | `range_expansion` | 21 | 5 Volatility structure | 0/0/2 | ✅ | per_row_extended_indicators::range_expansion_row_contract | pending | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — | ✅ | partial (family) |
| 22 | `mar` | 22 | 5 Volatility structure | 0/0/1 | ✅ | per_row_extended_indicators::mar_row_contract | not_applicable | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — | ✅ | partial (family) |
| 23 | `keltner` | 23 | 5 Volatility structure | 2/0/2 | ✅ | per_row_extended_indicators::keltner_row_contract | pending | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — | ✅ | partial (family) |
| 24 | `donchian` | 24 | 6 Structure | 2/0/2 | ✅ | per_row_extended_indicators::donchian_row_contract | pending | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — | ✅ | partial (family) |
| 25 | `swing_structure` | 25 | 6 Structure | 1/0/1 | ✅ | per_row_extended_indicators::swing_structure_row_contract | pending | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — | ✅ | partial (family) |
| 26 | `level_strength` | 26 | 6 Structure | 1/0/1 | ✅ | per_row_extended_indicators::level_strength_row_contract | pending | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — | ✅ | partial (family) |
| 27 | `aroon` | 27 | 6 Structure | 0/0/2 | ✅ | per_row_extended_indicators::aroon_row_contract | pending | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — | ✅ | partial (family) |
| 28 | `bos_choch` | 28 | 6 Structure | 2/0/0 | ✅ | per_row_extended_indicators::bos_choch_row_contract | pending | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — | ✅ | partial (family) |
| 29 | `candle_anatomy` | 29 | 7 Candle anatomy and gaps | 1/0/4 | ✅ | per_row_extended_indicators::candle_anatomy_row_contract | pending | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — | ✅ | partial (family) |
| 30 | `candle_pattern` | 30 | 7 Candle anatomy and gaps | 0/0/7 | ✅ | per_row_extended_indicators::candle_pattern_row_contract | pending | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — | ✅ | partial (family) |
| 31 | `gap` | 31 | 7 Candle anatomy and gaps | 2/0/2 | ✅ | per_row_extended_indicators::gap_row_contract | pending | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — | ✅ | partial (family) |
| 32 | `path_efficiency` | 32 | 8 Path and duration | 0/0/1 | ✅ | per_row_extended_indicators::path_efficiency_row_contract | not_applicable | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — | ✅ | partial (family) |
| 33 | `direction_changes` | 33 | 8 Path and duration | 0/0/1 | ✅ | per_row_extended_indicators::direction_changes_row_contract | not_applicable | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — | ✅ | partial (family) |
| 34 | `bars_since_extreme` | 34 | 8 Path and duration | 0/0/2 | ✅ | per_row_extended_indicators::bars_since_extreme_row_contract | pending | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — | ✅ | partial (family) |
| 35 | `drawdown_atr` | 35 | 8 Path and duration | 0/0/2 | ✅ | per_row_extended_indicators::drawdown_atr_row_contract | pending | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — | ✅ | partial (family) |
| 36 | `range_position` | 36 | 8 Path and duration | 0/0/1 | ✅ | per_row_extended_indicators::range_position_row_contract | pending | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — | ✅ | partial (family) |
| 37 | `relative_strength` | 37 | 9 Cross-asset | 0/2/1 | ✅ | per_row_panel_indicators::relative_strength_row_contract | pending | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — | ✅ | partial (family) |
| 38 | `lead_lag` | 38 | 9 Cross-asset | 0/2/1 | ✅ | per_row_panel_indicators::lead_lag_row_contract | pending | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — | ✅ | partial (family) |
| 39 | `breadth` | 39 | 9 Cross-asset | 0/2/1 | ✅ | per_row_panel_indicators::breadth_row_contract | pending | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — | ✅ | partial (family) |
| 40 | `co_movement` | 40 | 9 Cross-asset | 0/0/1 | ✅ | per_row_panel_indicators::co_movement_row_contract | not_applicable | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — | ✅ | partial (family) |
| 41 | `dispersion` | 41 | 9 Cross-asset | 0/0/1 | ✅ | per_row_panel_indicators::dispersion_row_contract | not_applicable | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — | ✅ | partial (family) |
| 42 | `session` | 42 | 10 Calendar and session | 1/0/2 | ✅ | per_row_session_indicators::session_row_contract | pending | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — | ✅ | partial (family) |
| 43 | `opening_range` | 43 | 10 Calendar and session | 2/0/4 | ✅ | per_row_session_indicators::opening_range_row_contract | pending | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — | ✅ | partial (family) |
| 44 | `overnight_intraday` | 44 | 10 Calendar and session | 0/0/2 | ✅ | per_row_session_indicators::overnight_intraday_row_contract | pending | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — | ✅ | partial (family) |
| 45 | `session_range` | 45 | 10 Calendar and session | 0/0/2 | ✅ | per_row_session_indicators::session_range_row_contract | pending | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — | ✅ | partial (family) |
| 46 | `calendar_bucket` | 46 | 10 Calendar and session | 0/0/0 | ✅ | per_row_session_indicators::calendar_bucket_row_contract | not_applicable | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | — | ✅ | covered |
| 47 | `pivot_sequence` | 47 | 6 Structure | 0/0/2 | ✅ | per_row_extended_indicators::pivot_sequence_row_contract | pending | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | ✅ | partial (family) |
| 48 | `fibonacci` | 48 | 6 Structure | 1/0/0 | ✅ | per_row_extended_indicators::fibonacci_row_contract, readings::extended::tests::fibonacci_reads_the_confirmed_major_leg | pending | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | ✅ | partial (family) |
| 49 | `horizontal_band` | 49 | 6 Structure | 1/0/0 | ✅ | per_row_extended_indicators::horizontal_band_row_contract, readings::extended::tests::horizontal_band_selects_the_largest_nearest_cluster, readings::extended::band::tests::horizontal_band_largest_cluster_wins, readings::extended::band::tests::horizontal_band_ties_go_to_nearest_close, readings::extended::band::tests::horizontal_band_equal_centers_go_to_lower | pending | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | — | — | ✅ | partial (family) |

## Area totals

| Area | Indicators | L | D | S | Total |
|---|---:|---:|---:|---:|---:|
| Base | 8 | 0 | 4 | 13 | 17 |
| 1 Volume-weighted price | 1 | 3 | 4 | 3 | 10 |
| 2 Volume by price | 1 | 5 | 2 | 4 | 11 |
| 3 Participation | 4 | 1 | 4 | 8 | 13 |
| 4 Volume-price flow | 4 | 0 | 14 | 7 | 21 |
| 5 Volatility structure | 4 | 2 | 2 | 8 | 12 |
| 6 Structure | 9 | 11 | 0 | 8 | 19 |
| 7 Candle anatomy and gaps | 3 | 3 | 0 | 13 | 16 |
| 8 Path and duration | 5 | 0 | 0 | 7 | 7 |
| 9 Cross-asset | 5 | 0 | 6 | 5 | 11 |
| 10 Calendar and session | 5 | 3 | 0 | 10 | 13 |
| **Total** | **49** | **28** | **36** | **86** | **150** |
