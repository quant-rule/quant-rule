# AI Transparency Log — SR10 EMA/Fibonacci Exit Study

- Date: 2026-11-10 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash
- Work item: [`SR10-CHARTER`](../../governance/2026-11-10/SR10-CHARTER.md)
- Claim type: strategy research proposal; no edge claim.

## Intent and boundary

Turn the 2018–2019 annotation zone findings into a bounded strategy research
proposal. The proposal tests one entry rule (EMA-fast tap plus Fibonacci test)
and three exits (giveback, structure flip, MACD force loss). This entry records
planning only. No strategy config was run and no backtest was opened.

## Drafted

- `docs/governance/2026-11-10/SR10-CHARTER.md`, status `Proposed`.
- One entry rule: `level.fib_test` plus `level.test` source 1 (fast EMA), with a
  3-bar event window.
- Six planned cells: long/short × giveback/structure/force-loss exits.
- 1 ATR stop, 3 ATR target, 50-bar time stop; 6 bps base, 9 bps sensitivity.
- Dukascopy BTCUSD primary for the formal `strategy_dev` partition; Binance
  BTCUSDT is a cross-venue robustness check outside the partitions.
- Development 2018–2022; validation 2024 closed; 2023 excluded.
- Model-baseline exception requested because `M1.2` is not accepted and the new
  signal events are `predictive_status: pending`.

## Evidence

- Zone report: `docs/reports/2026-11-10/annotation-zone-analysis.md`.
- EMA tap: 91.2% long entries, 87.9% short entries; fast EMA 86.5%/83.2%.
- Fibonacci test: 85.7% of both entry groups.
- Zone evidence is descriptive and hindsight-biased; it is not a strategy result.

## Learning

- The strongest entry-side separators are the fast EMA tap and Fibonacci
  proximity; the exit side is where the evidence is weaker.
- The strategy tier is gated by an accepted model baseline. The proposal must
  not run without a human-approved development-only exception.
- The charter keeps the break-only control descriptive and does not promote it.

## Not claimed

- No strategy result, no profitability claim, no edge.
- No policy, model, execution, portfolio, or validation change.
- No 2024 or portfolio holdout access.
