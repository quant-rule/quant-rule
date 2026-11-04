# Research Mandate — Durable Rule-Based Edge

- Date: 2026-11-04 (UTC)
- Status: Current; approved by the human as the governing research mandate
- Scope: model, strategy, execution, and portfolio research conducted through
  the Improvement Workflow
- Contract: [`docs/workflows/improvement-workflow.md`](../../workflows/improvement-workflow.md)
  and [`docs/workflows/run-contract.md`](../../workflows/run-contract.md)

## 1. Goal

Find a durable, cost-aware, rule-based edge on OHLCV candles that can be
adapted to prop-firm risk rules. Months of research are acceptable.

The goal is not a passing backtest, a lucky fold, or a strategy that only works
in-sample. The goal is evidence that survives out-of-sample data, costs,
slippage, multiple instruments or regimes, a drawdown limit, a minimum trade
count, a profit factor above one, and stability across folds.

## 2. Evidence standard

A claim of an edge requires all of the following:

| Requirement | Rule |
|---|---|
| Out-of-sample | The confirmation partition was not used to generate or select the hypothesis |
| Costs | Commission, slippage, and funding are part of the result |
| Breadth | At least two instruments or regimes, unless the hypothesis is explicitly single-market |
| Sample | At least 30 closed trades and a meaningful cluster count |
| Drawdown | The pre-registered maximum is respected in every fold |
| Profit factor | Greater than 1.0 after costs |
| Stability | No single fold carries the result |
| Reproduction | The same data, config, and seed reproduce the artifacts |
| Independence | An independent Quant Validator record or a disclosed same-session review |
| Live boundary | A backtest is a filter; paper trading and small live validation follow before prop capital |

A backtest is not live proof. A model-gate pass is not a strategy edge. A
strategy-dev pass is not validation. A 2024 validation pass is a candidate
baseline pending human acceptance, not a live strategy.

## 3. Research model

The unit of work is a **program**, not a single strategy.

```text
PROGRAM = premise + finite pre-registered cells + correction + budget + stop rule
CELL    = one hypothesis, one config, one worktree, one result
```

A program is not a search for a passing configuration. A cell is not retried
with a different target, stop, horizon, filter, or timeframe after a result.

The coordinator creates the program, worktrees, and cell briefs, and aggregates
results. It does not tune, vote, or promote. The human approves the charter,
the validation access, and the baseline.

## 4. Exploration and confirmation

- **Exploration** is read-only and uses the development partition. It may
  generate hypotheses. It never consumes validation or supports a claim.
- **Confirmation** is a frozen, pre-registered run. Validation is one-shot per
  hypothesis.
- A batch of configurations is not a search. Every variation tried is recorded,
  including failures, and the correction is declared before the first run.
- The number of confirmatory cells is declared in the program charter and
  covered by the correction. Exploratory cells are not counted as confirmatory
  evidence.

## 5. Data budget

The validation partitions are scarce and are not reused casually:

| Partition | Window | Use |
|---|---|---|
| `model_dev` | 2018-01-01..2020-12-31 | Model exploration and gate development |
| `model_validation` | 2021-01-01..2022-12-31 | One-shot model confirmation |
| `strategy_dev` | 2018-01-01..2022-12-31 | Strategy exploration and gate development |
| `strategy_validation` | 2024-01-01..2024-12-31 | One-shot strategy confirmation |
| `portfolio_dev` | 2024-01-01..2024-12-31 | Execution and portfolio exploration |
| `portfolio_holdout` | 2025-01-01..2025-12-31 | Final portfolio holdout |

2023 is excluded. A partition that has been used to choose among hypotheses is
no longer fresh for those hypotheses. The program registry records every use.

New data — 2025, 2026, and later — may refresh validation. The human approves a
new partition before it is used. Walk-forward or rolled holdout protocols
require a new approved charter.

## 6. Programs and hypotheses

Before the first run, the human approves a program charter with:

- the premise and economic rationale;
- the tier and frozen inputs;
- the finite hypothesis list and exact configs;
- the development and validation partitions;
- costs and green criteria;
- the multiple-testing correction and primary-cell count;
- the iteration cap and stop rule;
- the validation access.

A program stops when its premise is falsified, its budget is exhausted, or the
human stops it. A failed hypothesis is retired. It is not re-pre-registered as
a variant. A new program requires a new premise and a new approved charter; it
reuses the platform, partitions, costs, and accepted baselines.

The template is
[`program-charter-template.md`](program-charter-template.md).

## 7. Human role and residual risk

The human owns the goal, the policy, the program charter, the validation
access, and the final acceptance. The AI executes, records, and reports. It does
not declare an edge, accept a baseline, or approve its own result.

The workflow may conclude that the current model, timeframe, or envelope has no
edge. That is a valid result. It is not a failure of the process, and it is not
a reason to tune a failed hypothesis.

## 8. No guarantee

The process cannot promise an edge. It can promise that a positive result is
not a false positive promoted by repeated testing, and that a negative result
is recorded rather than hidden.

Markets are adaptive and noisy. Most systematic strategies fail after costs.
The purpose of this mandate is to make the search honest, not to make it
succeed on demand.

## 9. Current program status

The **v4 directional strategy foundation is stopped** after the v4 path gate,
the five 15m event families, divergence, cross-timeframe trend agreement, and
the progression pre-flight. No directional strategy baseline exists. The
accepted compression model baselines remain valid and direction-neutral.

The stop record is
[`docs/improvement/2026-11-04/V4-DIRECTIONAL-STRATEGY-STOP.md`](../../improvement/2026-11-04/V4-DIRECTIONAL-STRATEGY-STOP.md).
A new program requires a new premise and a new approved charter.

The session/liquidity spike was the first new premise tested after the v4 stop.
It was pre-registered on `model_dev`, found a real XAU session structure, and
still failed its primary directional cells. It is recorded as no edge in
[`docs/improvement/2026-11-04/SESSION-LIQUIDITY-SPIKE.md`](../../improvement/2026-11-04/SESSION-LIQUIDITY-SPIKE.md)
and stopped. The same rule applies: no tuning, no variant, no validation without
a new charter.

The EURUSD DST-aware session spike was the next new data premise. It generated
the EURUSD model pack, used explicit `Europe/London` and `America/New_York`
calendars, and found a real session structure. Its New York opening-range
breakout had a positive lower bound but failed the back-lag and direction-shuffle
controls. It is recorded as no edge in
[`docs/improvement/2026-11-04/EURUSD-SESSION-SPIKE.md`](../../improvement/2026-11-04/EURUSD-SESSION-SPIKE.md)
and stopped. The cost hurdle at EURUSD 15m is severe.

The v5 program then tested the remaining explanation for the v4 failures: that
the 20-bar horizon was too short for cost and trend persistence. The 1h swing
gate used the existing v4 events on a 4 ATR target / 2 ATR stop over 100 bars.
All three cells graded C. Trend continuation had a +3.23pp lift but its
back-lag placebo was +7.43pp and it did not add to the trend state; the pivot
break and EMA50 test were flat. The horizon explanation is falsified. It is
recorded in
[`docs/improvement/2026-11-04/V5-SWING-1H-GATE.md`](../../improvement/2026-11-04/V5-SWING-1H-GATE.md)
and stopped. Phase 2 needs a new charter and human approval.
