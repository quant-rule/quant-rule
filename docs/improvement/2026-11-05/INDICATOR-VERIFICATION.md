# Indicator Implementation Summary and Verification Checklist

- Date: 2026-11-05 (UTC)
- Status: **Approved — implementation contract**
- Epic: [#405](https://github.com/quant-rule/quant-rule/issues/405)
- Coverage: [`docs/model/2026-11-05/indicator-coverage.md`](../../model/2026-11-05/indicator-coverage.md)
- Scope: 46 indicators (10 existing, 36 new), 10 information areas
- All inputs: OHLCV and time only
- Claim type: registry and implementation contract; no edge claim

## 1. What will be implemented

| Phase | Work | Output |
|---|---|---|
| 0 | Registry schema extension, coverage matrix, codes, roles, claims | `docs/registry/` updated; event `formula` required |
| 1 | Read-only feature audit of the top 10 | `docs/improvement/2026-11-05/INDICATOR-AUDIT.md` |
| 2 | Tier 1 readings | VWAP, profile, ATR term, participation, lead-lag/breadth, Donchian |
| 3 | Event layer | Level, divergence, signal events; availability; precedence |
| 4 | Spec engine | `quantrule.model-spec/v1`, evaluator, `--spec`, registry, conformance |
| 5 | Tier 2 readings | Flow, path, cross-asset context, calendar |
| 6 | v6–v10 charters and swarm | Five charters, matrix, dev run |
| 7 | Validation and baseline | One-shot `model_validation`, Quant Validator, baseline |
| 8 | Closeout | Registry, index, baselines, cleanup |

## 2. Indicators and events

Notation: L = level, D = divergence, S = signal. Every event ID below is the
registry ID; the registry entry carries the exact formula.

### Area 1 — Volume-weighted price

| Code | Indicator | L events | D events | S events |
|---:|---|---|---|---|
| 11 | `vwap` | `level.vwap_test`, `level.vwap_cross`, `level.vwap_break` | `divergence.vwap_regular_bullish`, `.regular_bearish`, `.hidden_bullish`, `.hidden_bearish` | `signal.vwap_reclaim`, `signal.vwap_rejection`, `signal.vwap_slope` |

### Area 2 — Volume by price

| Code | Indicator | L events | D events | S events |
|---:|---|---|---|---|
| 12 | `volume_profile` | `level.profile_poc_test`, `level.profile_vah_test`, `level.profile_val_test`, `level.profile_va_break`, `level.profile_va_reentry` | `divergence.profile_poc_bullish`, `divergence.profile_poc_bearish` | `signal.profile_va_breakout`, `signal.profile_poc_cross`, `signal.profile_skew`, `signal.profile_concentration` |

### Area 3 — Participation

| Code | Indicator | L events | D events | S events |
|---:|---|---|---|---|
| 4 | `rvol` (extended) | none | `divergence.rvol_bullish`, `divergence.rvol_bearish` | `signal.rvol_expansion`, `signal.rvol_contraction`, `signal.rvol_ratio` |
| 13 | `volume_climax` | none | `divergence.climax_bullish`, `divergence.climax_bearish` | `signal.climax_reversal`, `signal.climax_continuation` |
| 14 | `absorption` | `level.absorption_test` | none | `signal.absorption_break`, `signal.absorption_rejection` |
| 15 | `volume_oscillator` | none | none | `signal.volume_oscillator_cross` |

### Area 4 — Volume-price flow

| Code | Indicator | L events | D events | S events |
|---:|---|---|---|---|
| 16 | `obv` | none | `divergence.obv_regular_bullish`, `.regular_bearish`, `.hidden_bullish`, `.hidden_bearish` | `signal.obv_cross`, `signal.obv_slope` |
| 17 | `cmf` | none | `divergence.cmf_regular_bullish`, `.regular_bearish`, `.hidden_bullish`, `.hidden_bearish` | `signal.cmf_cross`, `signal.cmf_threshold` |
| 18 | `mfi` | none | `divergence.mfi_regular_bullish`, `.regular_bearish`, `.hidden_bullish`, `.hidden_bearish` | `signal.mfi_threshold`, `signal.mfi_zone` |
| 19 | `ad_line` | none | `divergence.ad_regular_bullish`, `divergence.ad_regular_bearish` | `signal.ad_cross` |

### Area 5 — Volatility structure

| Code | Indicator | L events | D events | S events |
|---:|---|---|---|---|
| 20 | `atr_term` | none | `divergence.atr_bullish`, `divergence.atr_bearish` | `signal.atr_compression`, `signal.atr_expansion`, `signal.atr_regime` |
| 21 | `range_expansion` | none | none | `signal.range_expansion`, `signal.range_contraction` |
| 22 | `mar` | none | none | `signal.mar_regime` |
| 23 | `keltner` | `level.keltner_test`, `level.keltner_break` | none | `signal.keltner_breakout`, `signal.keltner_position` |

### Area 6 — Structure

| Code | Indicator | L events | D events | S events |
|---:|---|---|---|---|
| 6 | `pivots` (existing) | `level.test`, `level.cross`, `level.break` | RSI divergence at pivots | existing structure signals |
| 24 | `donchian` | `level.donchian_test`, `level.donchian_break` | none | `signal.donchian_breakout`, `signal.donchian_position` |
| 25 | `swing_structure` | `level.structure_break` | none | `signal.structure_change` |
| 26 | `level_strength` | `level.strength_test`, `level.strength_break` | none | `signal.strength_change` |
| 27 | `aroon` | none | none | `signal.aroon_cross`, `signal.aroon_osc` |
| 28 | `bos_choch` | `level.bos`, `level.choch` | none | `signal.structure_direction` |

### Area 7 — Candle anatomy and gaps

| Code | Indicator | L events | D events | S events |
|---:|---|---|---|---|
| 29 | `candle_anatomy` | `level.wick_test` | none | `signal.candle_momentum`, `signal.candle_rejection`, `signal.candle_doji`, `signal.candle_marubozu` |
| 30 | `candle_pattern` | none | none | `signal.pattern_engulfing`, `signal.pattern_inside`, `signal.pattern_outside`, `signal.pattern_hammer`, `signal.pattern_star`, `signal.pattern_three_soldiers`, `signal.pattern_three_crows` |
| 31 | `gap` | `level.gap_fill`, `level.gap_test` | none | `signal.gap_continuation`, `signal.gap_fade` |

### Area 8 — Path and duration

| Code | Indicator | L events | D events | S events |
|---:|---|---|---|---|
| 32 | `path_efficiency` | none | none | `signal.path_efficiency` |
| 33 | `direction_changes` | none | none | `signal.direction_changes` |
| 34 | `bars_since_extreme` | none | none | `signal.extreme_new`, `signal.extreme_stale` |
| 35 | `drawdown_atr` | none | none | `signal.drawdown_threshold`, `signal.drawdown_recovery` |
| 36 | `range_position` | none | none | `signal.range_position` |

### Area 9 — Cross-asset

| Code | Indicator | L events | D events | S events |
|---:|---|---|---|---|
| 37 | `relative_strength` | none | `divergence.rs_regular_bullish`, `divergence.rs_regular_bearish` | `signal.rs_cross`, `signal.rs_regime` |
| 38 | `lead_lag` | none | `divergence.lead_lag_bullish`, `divergence.lead_lag_bearish` | `signal.lead_signal`, `signal.lag_divergence` |
| 39 | `breadth` | none | `divergence.breadth_regular_bullish`, `divergence.breadth_regular_bearish` | `signal.breadth_cross` |
| 40 | `co_movement` | none | none | `signal.comovement_regime` |
| 41 | `dispersion` | none | none | `signal.dispersion` |

### Area 10 — Calendar and session

| Code | Indicator | L events | D events | S events |
|---:|---|---|---|---|
| 42 | `session` | `level.session_vwap_test` | none | `signal.session_open`, `signal.session_close` |
| 43 | `opening_range` | `level.opening_range_test`, `level.opening_range_break` | none | `signal.opening_range_breakout`, `signal.opening_range_fade` |
| 44 | `overnight_intraday` | none | none | `signal.overnight_continuation`, `signal.overnight_fade` |
| 45 | `session_range` | none | none | `signal.session_range_expansion`, `signal.session_range_contraction` |
| 46 | `calendar_bucket` | none | none | none |

## 3. Verification table A — build checklist

Each cell is a checkbox for the human. `Registry` = entry in
`docs/registry/indicators.json`. `Formula` = reading and every event formula
present. `Reading` = Rust reading module. `Events` = Rust event module.
`Wiring` = model run emits the values and events. `Spec` = usable from a model
spec. `Status` = planned → docs → code → tested → wired → verified.

| # | Indicator | Code | Area | Registry | Formula | Reading | Events | Wiring | Spec | Status |
|---:|---|---:|---:|---|---|---|---|---|---|---|
| 1 | vwap | 11 | 1 | ☑ | ☑ | ☑ | ☑ | ☑ | ☐ | tested |
| 2 | volume_profile | 12 | 2 | ☑ | ☑ | ☑ | ☑ | ☑ | ☐ | tested |
| 3 | rvol (extended) | 4 | 3 | ☑ | ☑ | ☑ | ☑ | ☑ | ☐ | tested |
| 4 | volume_climax | 13 | 3 | ☑ | ☑ | ☑ | ☑ | ☑ | ☐ | tested |
| 5 | absorption | 14 | 3 | ☑ | ☑ | ☑ | ☑ | ☑ | ☐ | tested |
| 6 | volume_oscillator | 15 | 3 | ☑ | ☑ | ☑ | ☑ | ☑ | ☐ | tested |
| 7 | obv | 16 | 4 | ☑ | ☑ | ☑ | ☑ | ☑ | ☐ | tested |
| 8 | cmf | 17 | 4 | ☑ | ☑ | ☑ | ☑ | ☑ | ☐ | tested |
| 9 | mfi | 18 | 4 | ☑ | ☑ | ☑ | ☑ | ☑ | ☐ | tested |
| 10 | ad_line | 19 | 4 | ☑ | ☑ | ☑ | ☑ | ☑ | ☐ | tested |
| 11 | atr_term | 20 | 5 | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | planned |
| 12 | range_expansion | 21 | 5 | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | planned |
| 13 | mar | 22 | 5 | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | planned |
| 14 | keltner | 23 | 5 | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | planned |
| 15 | donchian | 24 | 6 | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | planned |
| 16 | swing_structure | 25 | 6 | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | planned |
| 17 | level_strength | 26 | 6 | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | planned |
| 18 | aroon | 27 | 6 | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | planned |
| 19 | bos_choch | 28 | 6 | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | planned |
| 20 | candle_anatomy | 29 | 7 | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | planned |
| 21 | candle_pattern | 30 | 7 | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | planned |
| 22 | gap | 31 | 7 | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | planned |
| 23 | path_efficiency | 32 | 8 | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | planned |
| 24 | direction_changes | 33 | 8 | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | planned |
| 25 | bars_since_extreme | 34 | 8 | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | planned |
| 26 | drawdown_atr | 35 | 8 | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | planned |
| 27 | range_position | 36 | 8 | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | planned |
| 28 | relative_strength | 37 | 9 | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | planned |
| 29 | lead_lag | 38 | 9 | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | planned |
| 30 | breadth | 39 | 9 | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | planned |
| 31 | co_movement | 40 | 9 | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | planned |
| 32 | dispersion | 41 | 9 | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | planned |
| 33 | session | 42 | 10 | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | planned |
| 34 | opening_range | 43 | 10 | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | planned |
| 35 | overnight_intraday | 44 | 10 | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | planned |
| 36 | session_range | 45 | 10 | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | planned |
| 37 | calendar_bucket | 46 | 10 | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | planned |

## 4. Verification table B — test and math checklist

`F` formula unit test, `C` causality, `D` determinism, `G` golden fixture, `E`
edge cases, `A` event availability, `N` no double count, `P` precedence, `M`
math validity (allowed ops only, null/zero handling, no future data, no
statistics), `Cmd` verification command, `Sign` human/Validator sign-off.

| # | Indicator | F | C | D | G | E | A | N | P | M | Cmd | Sign |
|---:|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | vwap | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | `cargo test vwap` | ☐ |
| 2 | volume_profile | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | `cargo test volume_profile` | ☐ |
| 3 | rvol | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | `cargo test rvol` | ☐ |
| 4 | volume_climax | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | `cargo test volume_climax` | ☐ |
| 5 | absorption | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | `cargo test absorption` | ☐ |
| 6 | volume_oscillator | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | `cargo test volume_oscillator` | ☐ |
| 7 | obv | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | `cargo test obv` | ☐ |
| 8 | cmf | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | `cargo test cmf` | ☐ |
| 9 | mfi | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | `cargo test mfi` | ☐ |
| 10 | ad_line | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | `cargo test ad_line` | ☐ |
| 11 | atr_term | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | `cargo test atr_term` | ☐ |
| 12 | range_expansion | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | `cargo test range_expansion` | ☐ |
| 13 | mar | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | `cargo test mar` | ☐ |
| 14 | keltner | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | `cargo test keltner` | ☐ |
| 15 | donchian | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | `cargo test donchian` | ☐ |
| 16 | swing_structure | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | `cargo test swing_structure` | ☐ |
| 17 | level_strength | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | `cargo test level_strength` | ☐ |
| 18 | aroon | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | `cargo test aroon` | ☐ |
| 19 | bos_choch | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | `cargo test bos_choch` | ☐ |
| 20 | candle_anatomy | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | `cargo test candle_anatomy` | ☐ |
| 21 | candle_pattern | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | `cargo test candle_pattern` | ☐ |
| 22 | gap | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | `cargo test gap` | ☐ |
| 23 | path_efficiency | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | `cargo test path_efficiency` | ☐ |
| 24 | direction_changes | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | `cargo test direction_changes` | ☐ |
| 25 | bars_since_extreme | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | `cargo test bars_since_extreme` | ☐ |
| 26 | drawdown_atr | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | `cargo test drawdown_atr` | ☐ |
| 27 | range_position | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | `cargo test range_position` | ☐ |
| 28 | relative_strength | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | `cargo test relative_strength` | ☐ |
| 29 | lead_lag | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | `cargo test lead_lag` | ☐ |
| 30 | breadth | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | `cargo test breadth` | ☐ |
| 31 | co_movement | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | `cargo test co_movement` | ☐ |
| 32 | dispersion | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | `cargo test dispersion` | ☐ |
| 33 | session | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | `cargo test session` | ☐ |
| 34 | opening_range | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | `cargo test opening_range` | ☐ |
| 35 | overnight_intraday | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | `cargo test overnight_intraday` | ☐ |
| 36 | session_range | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | `cargo test session_range` | ☐ |
| 37 | calendar_bucket | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | `cargo test calendar_bucket` | ☐ |

## 5. Verification commands

Per indicator:

```bash
cargo test <indicator>_formula
cargo test <indicator>_causality
cargo test <indicator>_determinism
cargo test <indicator>_golden
cargo test <indicator>_edge
cargo test <indicator>_availability
cargo test <indicator>_no_double_count
cargo test <indicator>_precedence
```

Per PR:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
python3 tools/check_registry.py
python3 tools/check_links.py
python3 tools/check_terms.py
```

## 6. Status definitions

| Status | Meaning |
|---|---|
| `planned` | Defined in this table, no code |
| `docs` | Registry, formula, coverage, and claim committed |
| `code` | Reading and events implemented |
| `tested` | F/C/D/G/E/A/N/P all pass |
| `wired` | Model emits the reading and event codes in the artifacts |
| `spec` | Usable from a `quantrule.model-spec/v1` spec |
| `verified` | Human or Quant Validator has signed the row |
