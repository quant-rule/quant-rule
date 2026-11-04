# AI Transparency Log — 5m Compression Bracket

- Date: 2026-11-04 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Plan: [`docs/execution-plans/2026-11-04/379-compression-bracket-5m.md`](../../execution-plans/2026-11-04/379-compression-bracket-5m.md)
- Issue: [#379](https://github.com/quant-rule/quant-rule/issues/379)

## Intent and boundary

Test the same v10 compression bracket on 5m after the 1m bracket recorded no
edge. The 1m result showed the 4 bps commission was about 5.5× the gross edge;
5m ATR is larger relative to the fixed cost. The range event, bracket rule,
costs, stop, target, and time stop are unchanged. The work is a 5m model gate
and a 5m strategy matrix, not a tuning of the 1m rule.

## Design decisions

- The 5m gate reuses the existing `signal.compression_range` event and model
  config; only the timeframe selector changes.
- The 5m model bundles are the same per-year model runs used by the 1m gate;
  the 5m rows and events are already published by the model.
- The 5m strategy uses a 5m execution timeframe and a 15m context timeframe.
- `model_validation` 2021–2022 is a third use; the protocol and run record
  disclose it.

## Pre-registration

The 5m gate protocol, the 5m strategy rule, the costs, the folds, and the green
criteria are recorded before the first run. No threshold is selected from a
backtest.

## Result

The 5m range gate failed on the pre-registered back-lag control. The cell had
84,627 events, a +8.13pp lift, a +7.75pp corrected lower bound, random p 0.005,
and 3/3 replication, but the one-bar-lagged rule already captured +3.33pp,
leaving +4.80pp, 0.20pp below the +5pp gate.

The 2024 validation partition was not opened; the 5m strategy was not run; no 5m
baseline was accepted. The gate was not relaxed after the result.

## Verification

- The 5m harness reuses the model bundles and selects the 5m rows; the run record
  stores the matrix, command, streams, and artifact digests.
- `cargo fmt`, clippy, workspace tests, docs, and the registry/validation/link/
  term checks are recorded in the final report.
- No strategy result is claimed.
