# 5. Portfolio

- Status: Current
- Audience: operator replaying frozen decisions through execution and portfolio accounting

## Goal

Turn a decisions artifact into orders, fills, costs, equity, trades, and
risk events, and read the published accounting without treating it as evidence.

## Before you start

| Need | Value |
|---|---|
| Decisions artifact | A succeeded strategy stage with an `artifact_id` |
| Config | A valid config carrying execution and portfolio settings |
| Profile | The profile that owns the decisions artifact |
| Understanding | This stage replays decisions; it never decides a signal |

The portfolio stage derives `data_version` and `model_version` from the strategy
stage provenance. A supplied value that disagrees is refused.

## Steps

| # | Shell | Browser | Expect |
|---:|---|---|---|
| 1 | `quantrule stage run portfolio <decisions_artifact_id> <config_id>` | **Workbench → Portfolio** → Decisions artifact id, Config id → Run portfolio stage | `stage_run_id`, `state: queued`, `created: true` |
| 2 | `quantrule stage list --kind portfolio --status succeeded --limit 20` | Portfolio tab → stage table | One row per portfolio stage |
| 3 | `quantrule stage show <stage_run_id>` | Portfolio tab → open the stage | Provenance, `evidence: false`, prefixed artifact files |
| 4 | `quantrule stage fetch <stage_run_id> ./portfolio-stage` | Portfolio tab → Download | Every file verified before writing |
| 5 | Read `execution/fills.csv`, `portfolio/metrics.json`, and `portfolio/equity.csv` | Portfolio tab → metrics, orders, fills, rejections, trades | Costs, drawdown, exposure, and the accounting invariant |

The execution and portfolio bundles are prefixed. A flat merge would collide on
`summary.txt` and `provenance.json`, so the prefix is a contract.

## Artifact set

| Prefix | Files |
|---|---|
| `execution/` | `orders.csv`, `fills.csv`, `rejections.csv`, `summary.txt`, `provenance.json` |
| `portfolio/` | `trades.csv`, `equity.csv`, `funding.csv`, `risk_events.csv`, `metrics.json`, `summary.txt`, `provenance.json` |

`orders.csv` records the final order state. `fills.csv` records reference price,
executed price, slippage, fee, bar, and reason. `rejections.csv` records the
stable rejection reason. `trades.csv` is one row per closed trade.
`equity.csv` is one row per UTC date with a mark: cash, position value,
equity, peak, and drawdown. `risk_events.csv` records drawdown-limit breaches.

## What you see

| Metric | Meaning |
|---|---|
| `initial_cash` | Starting account cash |
| `final_equity` | Equity at the last mark, fill, or initial cash |
| `net_pnl` | Realized plus unrealized P&L |
| `execution_fees` | Fees paid on fills |
| `commission` | Portfolio commission |
| `funding_paid`, `funding_received` | Funding charges and credits |
| `max_drawdown` | Largest observed drawdown |
| `exposure` | Marked exposure; `null` when no mark defines it |
| `halted` | Whether the drawdown rule halted entries |

The published ledger invariant is:

```text
final_equity = initial_cash + net_pnl
```

A missing measurement is `null`, never zero. A valid zero is a value.

## What it proves

**Diagnostic.** The portfolio stage proves that the frozen decisions replay
through execution and accounting and publish a consistent ledger. It does not
prove a strategy edge. The P&L is diagnostic accounting; the coupled backtest is
the evidence path, and it recomputes strategy, execution, and portfolio.

## Failure

| Code or symptom | Cause | Next action |
|---|---|---|
| `stage.not_found` | The decisions artifact or portfolio stage is unknown | Re-run the strategy stage or re-read the list |
| `stage.invalid` | The artifact is not a decisions artifact, or provenance disagrees | Use a succeeded strategy stage from this profile |
| `stage.conflict` | The stage is terminal or has no artifact | Read `stage show`; wait or submit again |
| `config.invalid` | Execution, portfolio, or cost settings failed validation | Fix the config; do not change the reported result |
| `service.unavailable` | A required service is down | Read `health`; stop |
| Invariant failure | Published metrics disagree with the ledger | Treat it as a defect; record the run and stop |
| Missing metric | The evidence does not support it | Keep `null`; do not substitute zero |

## Related

- [Strategy](04-strategy.md)
- [Backtest](06-backtest.md)
- [Stage to backtest tutorial](../tutorials/2026-11-17/stage-to-backtest.md)
- [Portfolio Research workflow](../workflows/portfolio-research.md)
