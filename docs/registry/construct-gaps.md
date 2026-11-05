# Construct Coverage Gaps — P1b-2

- Date: 2026-11-05 (UTC)
- Status: Open worklist for [#481](https://github.com/quant-rule/quant-rule/issues/481)
- Parent: [#469](https://github.com/quant-rule/quant-rule/issues/469), [#480](https://github.com/quant-rule/quant-rule/issues/480)
- Plan: [`469-validation-claims.md`](../execution-plans/2026-11-05/469-validation-claims.md)
- Claim type: construct-test worklist; no edge claim

## 1. Method

The 260-bar model fixture, with compression enabled, emits 89 of the 143
registered event codes. The existing family tests cover more events, but
the following 54 event IDs do not have a direct construct case. Each row is
a required case for the table-driven suite in #481; none is a validation
pass and none carries an edge claim.

| Code | Event | Role | Family |
|---:|---|---|---|
| 4 | `divergence.rsi_regular_bullish` | signal | divergence |
| 6 | `divergence.rsi_hidden_bullish` | signal | divergence |
| 10 | `signal.ema_stack_change` | signal | signal |
| 17 | `signal.compression_breakout` | signal | signal |
| 23 | `divergence.vwap_regular_bullish` | signal | divergence |
| 25 | `divergence.vwap_hidden_bullish` | signal | divergence |
| 40 | `signal.profile_concentration` | context | signal |
| 48 | `signal.climax_reversal` | signal | signal |
| 49 | `signal.climax_continuation` | signal | signal |
| 50 | `level.absorption_test` | context | level |
| 51 | `signal.absorption_break` | signal | signal |
| 52 | `signal.absorption_rejection` | signal | signal |
| 54 | `divergence.obv_regular_bullish` | signal | divergence |
| 57 | `divergence.obv_hidden_bearish` | signal | divergence |
| 62 | `divergence.cmf_hidden_bullish` | signal | divergence |
| 66 | `divergence.mfi_regular_bullish` | signal | divergence |
| 67 | `divergence.mfi_regular_bearish` | signal | divergence |
| 68 | `divergence.mfi_hidden_bullish` | signal | divergence |
| 72 | `divergence.ad_regular_bullish` | signal | divergence |
| 81 | `level.wick_test` | context | level |
| 85 | `divergence.atr_bearish` | signal | divergence |
| 86 | `divergence.rs_regular_bullish` | signal | divergence |
| 87 | `divergence.rs_regular_bearish` | signal | divergence |
| 88 | `divergence.lead_lag_bullish` | signal | divergence |
| 89 | `divergence.lead_lag_bearish` | signal | divergence |
| 90 | `divergence.breadth_regular_bullish` | signal | divergence |
| 91 | `divergence.breadth_regular_bearish` | signal | divergence |
| 92 | `signal.atr_compression` | context | signal |
| 93 | `signal.atr_expansion` | context | signal |
| 94 | `signal.atr_regime` | context | signal |
| 106 | `signal.candle_momentum` | context | signal |
| 107 | `signal.candle_rejection` | context | signal |
| 109 | `signal.candle_marubozu` | context | signal |
| 113 | `signal.pattern_hammer` | context | signal |
| 114 | `signal.pattern_star` | context | signal |
| 115 | `signal.pattern_three_soldiers` | context | signal |
| 116 | `signal.pattern_three_crows` | context | signal |
| 117 | `signal.gap_continuation` | context | signal |
| 118 | `signal.gap_fade` | context | signal |
| 126 | `signal.relative_strength_cross` | context | signal |
| 127 | `signal.lead_lag_cross` | context | signal |
| 128 | `signal.breadth_cross` | context | signal |
| 129 | `signal.co_movement_cross` | diagnostic | signal |
| 130 | `signal.dispersion_cross` | diagnostic | signal |
| 131 | `signal.session_open` | context | signal |
| 132 | `signal.session_close` | context | signal |
| 133 | `signal.opening_range_breakout` | context | signal |
| 134 | `signal.opening_range_fade` | context | signal |
| 135 | `signal.overnight_continuation` | signal | signal |
| 136 | `signal.overnight_fade` | signal | signal |
| 137 | `signal.session_range_expansion` | context | signal |
| 138 | `signal.session_range_contraction` | context | signal |
| 141 | `level.session_vwap_test` | context | level |
| 142 | `level.opening_range_test` | context | level |
| 143 | `level.opening_range_break` | context | level |

## 2. Coverage rule

A case counts only when it reaches the detector with inputs that satisfy
the event formula and asserts the published event code. A family-level
golden test that does not include the event is not a direct case.
