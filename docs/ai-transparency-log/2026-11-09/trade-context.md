# AI Transparency Log — Trade Context and Exit Quality

- Date: 2026-11-09 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash
- Work item: [`trade-context`](../../execution-plans/2026-11-09/trade-context.md)
- Claim type: evaluation and tooling; no edge claim.

## Intent and boundary

Export the recorded state at every trade entry and exit, including the last 250
bars and all readings, and add a table for judging exit quality. The work does
not change a model, strategy, order, or portfolio result.

## Implemented

- `tools/trades/bundle.py` reads the backtest bundle, events, decisions, orders,
  fills, trades, and manifest.
- `tools/trades/trade_context.py` writes entry/exit context, post windows,
  events, decisions, orders, fills, and all-column summaries.
- `tools/trades/annotations.py` maps explicit UTC marks to bars and writes the
  same context.
- `tools/trades/exit_quality.py` writes one row per trade with captured return,
  trade MFE/MAE, exit efficiency, the decision-bar state, and post-exit
  MFE/MAE over 5/10/20/50 bars.
- Generated contexts for Binance 2018–2023 (73 trades) and Dukascopy 2018–2022
  (30 trades), plus exit-quality tables.
- The chart fast EMA was corrected separately to model EMA20; the chart uses
  `ema_fast`/`ema_mid`/`ema_slow` (20/50/200).

## Verification

- `python3 -m unittest discover -s tools/trades -p 'test_*.py'` — 3 pass.
- Real 2018 Binance context: 11 trades; entry and exit context 250 rows each;
  99 columns.
- Annotation example for the 2018 trade 1 entry/exit mapped to the expected bars.
- Exit-quality aggregate: 73 Binance rows and 30 Dukascopy rows.
- Repository checks and `git diff --check` recorded in the plan.

## Learning

- The model already publishes every per-bar reading; only the per-trade packaging
  was missing. Duplicating indicator calculations into a trade record would
  create a second source of truth.
- The decision bar and the fill bar must be kept separate. The context ends at the
  decision bar; the fill bar is recorded separately.
- Most selected exits are bracket targets/stops, not state exits. The exit-quality
  table must be read as descriptive evidence, not a new rule.
- The hand-drawn circle style is not a reliable data format; the annotation CSV
  carries explicit action, quality, and side.

## Not claimed

- No model or strategy behavior change.
- No new exit rule.
- No 2024 access and no partition change.
- No edge or candidate claim.
