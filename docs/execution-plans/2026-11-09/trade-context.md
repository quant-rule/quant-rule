# Trade Context and Exit-Quality Tooling

- Date: 2026-11-09 (UTC)
- Status: **Verified** — all acceptance criteria were evaluated against the observations below.
- Work item: `trade-context`
- Workflows: Requirements & Design, Strategy Research (exploratory evaluation)
- Claim type: evaluation and tooling; no strategy or edge claim.

## 1. Intent

Export the recorded model state around every entry and exit, including the last
250 bars and all readings, and add an annotation path for arbitrary chart marks.
The result supports an exit-quality study without changing the model or strategy.

## 2. Scope

Included:

- `tools/trades/trade_context.py`: per-trade 250-bar entry/exit context, post
  window, events, decision, orders, fills, summary.
- `tools/trades/annotations.py`: timestamp-to-bar context for human marks.
- `tools/trades/exit_quality.py`: one row per trade with MFE/MAE and post-exit
  continuation.
- Tests, README, artifact contract, tutorial, changelog, AI log, CI step.
- Run on Binance 2018–2023 and Dukascopy 2018–2022.

Excluded:

- Model or strategy changes.
- New exit rules or parameter tuning.
- 2024 access or a partition change.
- A new shell command.

## 3. Design

- The tools are read-only over the backtest bundle.
- `context.csv` copies all `model/readings.csv` columns and ends at the decision
  bar (`entry_bar - 1` or `exit_bar - 1`).
- `post.csv` is separate; it starts at the entry fill bar or after the exit fill.
- `summary.json` contains every reading at the decision bar, the state labels,
  the decision evidence, and events available at that bar.
- Annotations carry explicit `action`, `quality`, and `side`; quality is never
  inferred from the drawing.
- Exit quality reports MFE/MAE over 5/10/20/50 bars after exit and whether the
  recorded target or stop would have been touched.

## 4. Acceptance criteria

- Entry/exit context has 250 rows when history allows and all 99 readings
  columns.
- No context row is after the decision bar.
- `summary.json` matches the decision bar and contains no post-decision values.
- Annotation timestamps map to the expected bar or fail clearly.
- `exit_quality.csv` has one row per trade and covers the 5/10/20/50 windows.
- Binance context covers 73 trades; Dukascopy covers 30.
- All Python tests, repository checks, and link/term/changelog checks pass.

## 5. Verification

- `python3 -m unittest discover -s tools/trades -p 'test_*.py'`
- Run the three tools on a real bundle.
- Inspect a context file: 250 rows, last bar = decision bar.
- Inspect an annotation summary: mapped bar and state.
- Inspect exit-quality rows: one per trade.
- Repository checks and `git diff --check`.

## 6. Risks and stop conditions

- Leakage if the context window includes the fill bar close: prevented by ending
  at the decision bar.
- Annotation ambiguity: explicit timestamps and enums.
- Data snooping: exploratory only; no rule change.
- Stop if a strategy/model change would be required or 2024 would be accessed.

## 7. Outcome

| Criterion | Observed |
|---|---|
| Binance context | 73 trades across 2018–2023 |
| Dukascopy context | 30 trades across 2018–2022 |
| Entry/exit window | 250 rows per leg when history allows; all 99 readings columns |
| Leakage | context ends at `entry_bar - 1` / `exit_bar - 1`; post window is separate |
| Summary | all reading columns plus `state_labels`, decision evidence, and events |
| Annotation example | 2 marks mapped to 2018-01-27 20:15 and 20:45 UTC |
| Exit quality | 73 Binance rows and 30 Dukascopy rows; 5/10/20/50-bar windows |
| Tests | `tools/trades` 3 pass; chart 11 pass |
| Repository gate | file size, structure, changelog, links, terms, and diff check pass |
| Model/strategy | unchanged |
| 2024 | not accessed |

The tools are evaluation-only. The exit-quality table is exploratory and does not
define a new exit rule.
