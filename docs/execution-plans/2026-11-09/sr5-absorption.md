# SR5 Absorption Rejection — 15m Entry with 1h Stack Context

- Date: 2026-11-09 (UTC)
- Status: **Stopped — count gate underpowered; no backtest; 2024 not opened**
- Charter: [`SR5-CHARTER.md`](../../governance/2026-11-09/SR5-CHARTER.md)
- Summary: [`SR5-ABSORPTION.md`](../../improvement/2026-11-09/SR5-ABSORPTION.md)
- Predecessor: [`SR4-STACK-FADE.md`](../../improvement/2026-11-09/SR4-STACK-FADE.md)
- Claim type: strategy research; no edge or profitability claim

## 1. Intent

Test a different event family on BTCUSD: a 15m absorption rejection with a 1h
EMA-stack context. The count gate runs first so an underpowered event stops the
programme before any backtest compute.

## 2. Current state

- SR4 found the stack-aligned compression fade BTC-only; no multi-instrument
  edge.
- The BTC compression fade has 25 trades over five years and breaches 5%
  drawdown.
- 2024 remains closed; no strategy baseline exists.
- The accepted model baseline is `MODEL-2026-11-04-04..05`.
- `signal.absorption_rejection` is a registered signal event (type 3, subtype
  12, source 22), with direction equal to the rejection direction.

## 3. Scope

Included:

- A read-only count gate for A1 events on 2018–2022.
- Six pre-registered variants: A1/A2/A3 × long/short.
- 30 base cells, one calendar year per cell.
- 30 feasibility preflights at 6 bps.
- A1 9 bps sensitivity only if A1 passes the base.
- One development record and decision.

Excluded:

- 2024 validation; not opened.
- Any 1h-only or 1m model cell.
- Any post-hoc filter, threshold, target, stop, timeframe, or cost change.
- A portfolio or live/prop-firm claim.

## 4. Assumptions and uncertainty

- Absorption rejection is frequent enough on 15m BTC to pass the count gate.
- The 1h stack context is available at the 15m event's as-of bar; the count gate
  uses the same close-time rule as the strategy context.
- The 1h stack may not improve the event-only control; that is a valid result.
- The 5% halt is bar-close and can overshoot; any observed cell above 5% fails.

## 5. Decisions

| Decision | Choice | Consequence |
|---|---|---|
| Primary rule | A1 absorption rejection + 1h stack | New event family, larger sample |
| Instrument | BTCUSD only | SR4 showed the earlier rule is BTC-specific |
| Entry | 15m | The user's desired entry timeframe |
| Context | 1h EMA stack | Keeps the rejection a pullback, not a countertrend guess |
| Costs | 6 bps base, 9 bps A1 | Cost-aware gate |
| Count gate | ≥30 A1 events per side | Stops an underpowered programme early |
| Validation | Closed | No 2024 without a freeze and approval |

## 6. Acceptance criteria

- The count gate runs and records A1/A2/A3 counts by side.
- If A1 has fewer than 30 events on either side, the programme stops as
  underpowered with no backtest.
- If the count gate passes, 30/30 preflight reports pass and 30/30 base cells
  complete with qualification records.
- A1 is evaluated against the charter's gate; the verdict is recorded even when
  it is `no candidate`.
- 9 bps A1 cells run only if A1 passes the base.
- 2024 is not read.
- The run record, committed summary, AI log, and registry are updated.

## 7. Work breakdown

| Step | Work | Owner | Paths |
|---|---|---|---|
| 1 | Write charter, plan, count config, count tool | Quant Researcher / Developer | `docs/governance/2026-11-09/`, `config/sr5/count/`, `tools/improvement/absorption_count.py` |
| 2 | Run the count gate on the 15m/1h bundle | Quant Validator | `data/improvement/2026-11-09/SR5-ABSORPTION/count/` |
| 3 | If the count gate fails, stop and record underpowered | Result Analyst | run record and summary |
| 4 | If it passes, build six strategy configs and 30 runs | Quant Developer | `config/sr5/`, `config/strategy/sr5-*.json` |
| 5 | Run 30 preflights, 30 base cells, qualification | Quant Developer | `data/improvement/2026-11-09/SR5-ABSORPTION/` |
| 6 | If A1 passes, run 10 9 bps A1 cells | Quant Developer | `artifacts-9bps/` |
| 7 | Analyze and write `results.json`, `review.md`, `decision.md` | Result Analyst | local run record |
| 8 | Write the committed summary and update the registry | Quant Researcher / Orchestrator | `docs/improvement/2026-11-09/SR5-ABSORPTION.md` |
| 9 | Verify, commit, PR, merge, clean up | Engineering / Orchestrator | branch `feat/sr5-absorption` |

## 8. Verification

- Count gate: `absorption_count.py` over `events.csv` and `readings.csv`.
- `python3 tools/improvement/feasibility.py` on every cell at 6 bps.
- `target/release/quantrule backtest run` per cell.
- `target/release/quantrule qualify` per base cell.
- Pooled metrics from `portfolio/metrics.json` and `portfolio/trades.csv`.
- Byte-identical rerun for the passing A1 side.
- `cargo fmt --check`, `cargo test`, `cargo clippy --all-targets -- -D warnings`,
  `cargo doc` with warnings denied.
- Python improvement tests and the documentation/registry checkers.
- `git diff --check`.

## 9. Risks and recovery

- **Count gate fails:** stop as underpowered; do not lower the threshold.
- **Cost failure:** report `no edge`; do not lower costs.
- **Context failure:** report A1 vs A3; do not promote A3.
- **Drawdown overshoot:** fail the cell; do not relax the 5% ceiling.
- **Machine restart:** runs are resumable per cell; re-run only missing cells.
- **Data gap:** stop the affected cell; do not interpolate.

## 10. Stop conditions

Stop when the count gate fails, a preflight fails, A1 fails the development or
9 bps gate, any A1 cell exceeds 5% drawdown, the iteration cap is reached, or
the human stops the programme. A stopped programme does not open 2024 and is
not retried with a new variant under the same premise.

## 11. Outcome

| Check | Observed |
|---|---|
| A1 count | 14 long, 12 short |
| A2 count | 7 long, 8 short |
| A3 event-only count | 30 long, 32 short |
| Count gate | fail; threshold 30 per side |
| A1 by year | 2018 0/0; 2019 11/4; 2020 2/2; 2021 0/0; 2022 1/6 |
| Backtest | not run; count gate failed |
| 9 bps | not run |
| 2024 | not opened |

The programme stops as underpowered. The event-only control is not promoted.
