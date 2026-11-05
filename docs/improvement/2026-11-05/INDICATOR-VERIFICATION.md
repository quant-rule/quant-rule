# Indicator Implementation Summary and Verification Checklist

- Date: 2026-11-05 (UTC)
- Status: **Approved — implementation contract**
- Epic: [#405](https://github.com/quant-rule/quant-rule/issues/405)
- Coverage: [`docs/model/2026-11-05/indicator-coverage.md`](../../model/2026-11-05/indicator-coverage.md)
- Scope: 46 indicators (10 existing, 36 new), 10 information areas
- All inputs: OHLCV and time only
- Claim type: registry and implementation contract; no edge claim
- Reconciliation: 2026-11-05; phases 0–5 implemented, phase 1 audit not delivered, phases 6–8 open

## 1. What will be implemented

| Phase | Work | Output | State |
|---|---|---|---|
| 0 | Registry schema extension, coverage matrix, codes, roles, claims | `docs/registry/` updated; event `formula` required | Done |
| 1 | Read-only feature audit of the top 10 | `docs/improvement/2026-11-05/INDICATOR-AUDIT.md` | **Not delivered** |
| 2 | Tier 1 readings | VWAP, profile, ATR term, participation, lead-lag/breadth, Donchian | Done |
| 3 | Event layer | Level, divergence, signal events; availability; precedence | Done; §5 records the remaining audit and phase gaps |
| 4 | Spec engine | `quantrule.model-spec/v1`, evaluator, `--spec`, registry, conformance | Done |
| 5 | Tier 2 readings | Flow, path, cross-asset context, calendar | Done |
| 6 | v6–v10 charters and swarm | Five charters, matrix, dev run | Open — #426–#428 |
| 7 | Validation and baseline | One-shot `model_validation`, Quant Validator, baseline | Open — #429 |
| 8 | Closeout | Registry, index, baselines, cleanup | Open — #430 |

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
| 26 | `level_strength` | `level.strength_test` (the plan's `level.strength_break` is the same test event; no separate break) | none | `signal.strength_change` |
| 27 | `aroon` | none | none | `signal.aroon_cross`, `signal.aroon_osc` |
| 28 | `bos_choch` | `level.bos`, `level.choch` | none | `signal.structure_change` (shared with `swing_structure`) |

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
| 37 | `relative_strength` | none | `divergence.rs_regular_bullish`, `divergence.rs_regular_bearish` | `signal.relative_strength_cross` |
| 38 | `lead_lag` | none | `divergence.lead_lag_bullish`, `divergence.lead_lag_bearish` | `signal.lead_lag_cross` |
| 39 | `breadth` | none | `divergence.breadth_regular_bullish`, `divergence.breadth_regular_bearish` | `signal.breadth_cross` |
| 40 | `co_movement` | none | none | `signal.co_movement_cross` |
| 41 | `dispersion` | none | none | `signal.dispersion_cross` |

The registry implements one cross per panel reading. The earlier plan named two
signals for `relative_strength` and `lead_lag`; `signal.rs_regime` and
`signal.lag_divergence` are **not registered**.

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
| 1 | vwap | 11 | 1 | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | code |
| 2 | volume_profile | 12 | 2 | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | code |
| 3 | rvol (extended) | 4 | 3 | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | code |
| 4 | volume_climax | 13 | 3 | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | code |
| 5 | absorption | 14 | 3 | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | code |
| 6 | volume_oscillator | 15 | 3 | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | code |
| 7 | obv | 16 | 4 | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | code |
| 8 | cmf | 17 | 4 | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | code |
| 9 | mfi | 18 | 4 | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | code |
| 10 | ad_line | 19 | 4 | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | code |
| 11 | atr_term | 20 | 5 | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | code |
| 12 | range_expansion | 21 | 5 | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | code |
| 13 | mar | 22 | 5 | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | code |
| 14 | keltner | 23 | 5 | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | code |
| 15 | donchian | 24 | 6 | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | code |
| 16 | swing_structure | 25 | 6 | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | code |
| 17 | level_strength | 26 | 6 | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | code |
| 18 | aroon | 27 | 6 | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | code |
| 19 | bos_choch | 28 | 6 | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | code |
| 20 | candle_anatomy | 29 | 7 | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | code |
| 21 | candle_pattern | 30 | 7 | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | code |
| 22 | gap | 31 | 7 | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | code |
| 23 | path_efficiency | 32 | 8 | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | code |
| 24 | direction_changes | 33 | 8 | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | code |
| 25 | bars_since_extreme | 34 | 8 | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | code |
| 26 | drawdown_atr | 35 | 8 | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | code |
| 27 | range_position | 36 | 8 | ☑ | ☑ | ☑ | ☑ | ☑ | ☑ | code |
| 28 | relative_strength | 37 | 9 | ☑ | ☑ | ☑ | ☑ | ☐ | ☐ | code |
| 29 | lead_lag | 38 | 9 | ☑ | ☑ | ☑ | ☑ | ☐ | ☐ | code |
| 30 | breadth | 39 | 9 | ☑ | ☑ | ☑ | ☑ | ☐ | ☐ | code |
| 31 | co_movement | 40 | 9 | ☑ | ☑ | ☑ | ☑ | ☐ | ☐ | code |
| 32 | dispersion | 41 | 9 | ☑ | ☑ | ☑ | ☑ | ☐ | ☐ | code |
| 33 | session | 42 | 10 | ☑ | ☑ | ☑ | ☑ | ☐ | ☐ | code |
| 34 | opening_range | 43 | 10 | ☑ | ☑ | ☑ | ☑ | ☐ | ☐ | code |
| 35 | overnight_intraday | 44 | 10 | ☑ | ☑ | ☑ | ☑ | ☐ | ☐ | code |
| 36 | session_range | 45 | 10 | ☑ | ☑ | ☑ | ☑ | ☐ | ☐ | code |
| 37 | calendar_bucket | 46 | 10 | ☑ | ☑ | ☑ | ☑ | ☐ | ☐ | code |

## 4. Verification table B — test and math checklist

`F` formula unit test, `C` causality, `D` determinism, `G` golden fixture, `E`
edge cases, `A` event availability, `N` no double count, `P` precedence, `M`
math validity (allowed ops only, null/zero handling, no future data, no
statistics), `Cmd` verification command, `Sign` human/Validator sign-off.

| # | Indicator | F | C | D | G | E | A | N | P | M | Cmd | Sign |
|---:|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | vwap | ☑ | ☑ | ☑ | ☐ | ☑ | ☑ | ☑ | ☐ | ☑ | cargo test vwap_is_the_volume_weighted_typical_price | ☐ |
| 2 | volume_profile | ☑ | ☑ | ☑ | ☐ | ☑ | ☑ | ☑ | ☐ | ☑ | cargo test profile_poc_is_the_highest_volume_bin | ☐ |
| 3 | rvol (extended) | ☑ | ☑ | ☑ | ☐ | ☐ | ☑ | ☑ | ☐ | ☑ | cargo test rvol_reads_the_volume_ratio | ☐ |
| 4 | volume_climax | ☑ | ☑ | ☑ | ☐ | ☐ | ☑ | ☑ | ☐ | ☑ | cargo test climax_flags_a_high_volume_wide_bar | ☐ |
| 5 | absorption | ☑ | ☑ | ☑ | ☐ | ☐ | ☑ | ☑ | ☐ | ☑ | cargo test absorption_flags_a_small_range_near_vwap | ☐ |
| 6 | volume_oscillator | ☐ | ☑ | ☑ | ☐ | ☐ | ☑ | ☑ | ☐ | ☑ | — | ☐ |
| 7 | obv | ☑ | ☑ | ☑ | ☐ | ☐ | ☑ | ☑ | ☐ | ☑ | cargo test obv_accumulates_signed_volume | ☐ |
| 8 | cmf | ☑ | ☑ | ☑ | ☐ | ☐ | ☑ | ☑ | ☐ | ☑ | cargo test cmf_is_one_when_every_close_is_the_high | ☐ |
| 9 | mfi | ☑ | ☑ | ☑ | ☐ | ☐ | ☑ | ☑ | ☐ | ☑ | cargo test mfi_is_one_hundred_when_every_bar_rises | ☐ |
| 10 | ad_line | ☐ | ☑ | ☑ | ☐ | ☐ | ☑ | ☑ | ☐ | ☑ | — | ☐ |
| 11 | atr_term | ☑ | ☑ | ☑ | ☐ | ☐ | ☑ | ☑ | ☐ | ☑ | cargo test atr_term_is_available_after_the_slow_window | ☐ |
| 12 | range_expansion | ☑ | ☑ | ☑ | ☐ | ☐ | ☑ | ☑ | ☐ | ☑ | cargo test range_and_mar_read_the_window | ☐ |
| 13 | mar | ☑ | ☑ | ☑ | ☐ | ☐ | ☑ | ☑ | ☐ | ☑ | cargo test range_and_mar_read_the_window | ☐ |
| 14 | keltner | ☑ | ☑ | ☑ | ☐ | ☐ | ☑ | ☑ | ☐ | ☑ | cargo test keltner_and_donchian_publish_bands | ☐ |
| 15 | donchian | ☑ | ☑ | ☑ | ☐ | ☐ | ☑ | ☑ | ☐ | ☑ | cargo test keltner_and_donchian_publish_bands | ☐ |
| 16 | swing_structure | ☑ | ☑ | ☑ | ☐ | ☐ | ☑ | ☑ | ☐ | ☑ | cargo test structure_and_levels_publish_when_history_exists | ☐ |
| 17 | level_strength | ☑ | ☑ | ☑ | ☐ | ☐ | ☑ | ☑ | ☐ | ☑ | cargo test structure_and_levels_publish_when_history_exists | ☐ |
| 18 | aroon | ☑ | ☑ | ☑ | ☐ | ☐ | ☑ | ☑ | ☐ | ☑ | cargo test aroon_reads_a_rising_series | ☐ |
| 19 | bos_choch | ☐ | ☑ | ☑ | ☐ | ☐ | ☑ | ☑ | ☐ | ☑ | cargo test -p quantrule-model --test event_validation | ☐ |
| 20 | candle_anatomy | ☑ | ☑ | ☑ | ☐ | ☐ | ☑ | ☑ | ☐ | ☑ | cargo test candle_anatomy_reads_the_bar | ☐ |
| 21 | candle_pattern | ☑ | ☑ | ☑ | ☐ | ☐ | ☑ | ☑ | ☐ | ☑ | cargo test candle_pattern_flags_an_engulfing_bar | ☐ |
| 22 | gap | ☑ | ☑ | ☑ | ☐ | ☐ | ☑ | ☑ | ☐ | ☑ | cargo test gap_reads_the_open_jump | ☐ |
| 23 | path_efficiency | ☑ | ☑ | ☑ | ☐ | ☐ | ☑ | ☑ | ☐ | ☑ | cargo test path_reads_efficiency_and_extremes | ☐ |
| 24 | direction_changes | ☐ | ☑ | ☑ | ☐ | ☐ | ☑ | ☑ | ☐ | ☑ | — | ☐ |
| 25 | bars_since_extreme | ☑ | ☑ | ☑ | ☐ | ☐ | ☑ | ☑ | ☐ | ☑ | cargo test path_reads_efficiency_and_extremes | ☐ |
| 26 | drawdown_atr | ☐ | ☑ | ☑ | ☐ | ☐ | ☑ | ☑ | ☐ | ☑ | — | ☐ |
| 27 | range_position | ☐ | ☑ | ☑ | ☐ | ☐ | ☑ | ☑ | ☐ | ☑ | — | ☐ |
| 28 | relative_strength | ☑ | ☑ | ☑ | ☐ | ☐ | ☑ | ☑ | ☐ | ☑ | cargo test relative_strength_ranks_the_faster_instrument | ☐ |
| 29 | lead_lag | ☑ | ☑ | ☑ | ☐ | ☐ | ☑ | ☑ | ☐ | ☑ | cargo test lead_lag_reads_the_leader | ☐ |
| 30 | breadth | ☑ | ☑ | ☑ | ☐ | ☐ | ☑ | ☑ | ☐ | ☑ | cargo test breadth_and_dispersion_are_common | ☐ |
| 31 | co_movement | ☑ | ☑ | ☑ | ☐ | ☐ | ☑ | ☑ | ☐ | ☑ | cargo test co_movement_is_high_for_aligned_series | ☐ |
| 32 | dispersion | ☑ | ☑ | ☑ | ☐ | ☐ | ☑ | ☑ | ☐ | ☑ | cargo test breadth_and_dispersion_are_common | ☐ |
| 33 | session | ☑ | ☑ | ☑ | ☐ | ☐ | ☑ | ☑ | ☐ | ☑ | cargo test london_session_and_opening_range | ☐ |
| 34 | opening_range | ☑ | ☑ | ☑ | ☐ | ☐ | ☑ | ☑ | ☐ | ☑ | cargo test london_session_and_opening_range | ☐ |
| 35 | overnight_intraday | ☐ | ☑ | ☑ | ☐ | ☐ | ☑ | ☑ | ☐ | ☑ | — | ☐ |
| 36 | session_range | ☐ | ☑ | ☑ | ☐ | ☐ | ☑ | ☑ | ☐ | ☑ | — | ☐ |
| 37 | calendar_bucket | ☑ | ☑ | ☑ | ☐ | ☑ | — | — | ☐ | ☑ | cargo test london_session_and_opening_range | ☐ |

## 5. Known gaps

The missing-event batch closed the BOS/CHOCH, session-VWAP, opening-range, and
strength-name gaps. `level.strength_break` is the same test event as
`level.strength_test`; the panel's extra `rs_regime` and `lag_divergence` names
are reconciled to one cross per reading. `signal.structure_change` is shared by
`swing_structure` and `bos_choch`.

| Gap | Evidence | Next step |
|---|---|---|
| `INDICATOR-AUDIT.md` | The file is absent from `docs/improvement/2026-11-05/` | Human decides whether to require the audit retroactively |
| Per-indicator golden fixtures | The batch fixtures cover the event families, not each indicator's reading | Add per-indicator fixtures before any row becomes `tested` |
| Phases 6–8 | #426–#430 are Todo | Approve the charters, run the swarm, validate, close out |

## 6. Event validation suite

`crates/quantrule-model/tests/event_validation.rs` proves, over the checked-in
260-bar fixture:

| Test | Coverage |
|---|---|
| Availability and order | v4, extended, and session batches |
| One occurrence | Extended occurrence key, full-record uniqueness, v4/extended disjointness |
| Level precedence | One event per reference per bar; fixed reference order |
| Golden fixtures | Extended and session regression fixtures |
| Causality | Prefix runs at bars 61, 131, and 201 |
| Determinism | Repeated model runs |
| Panel | Availability and occurrence keys on a three-instrument panel |

Table B's `C` and `D` cells are family-level: the extended readings, panel, and
session suites each have one causality and one determinism test. `A` and `N`
are checked for indicators whose plan events are all registered; they remain
open where an event is missing. `P` stays open for the extended families: the
precedence test covers the v4 level references, and the extended batch proves
one occurrence per key rather than a per-indicator reference order.

## 7. Verification commands

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

## 8. Status definitions

| Status | Meaning |
|---|---|
| `planned` | Defined in this table, no code |
| `docs` | Registry, formula, coverage, and claim committed |
| `code` | Reading and events implemented |
| `tested` | F/C/D/G/E/A/N/P all pass |
| `wired` | Model emits the reading and event codes in the artifacts |
| `spec` | Usable from a `quantrule.model-spec/v1` spec |
| `verified` | Human or Quant Validator has signed the row |
