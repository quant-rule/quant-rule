# The Human Improvement Loop

- Date: 2026-10-30
- Work item: `audit-improvement-workflow`
- Audience: the human owner of the policy and strategy

## 1. The three levels you own

| Level | File | You decide |
|---|---|---|
| Policy | `policy.json` | Account size, performance target, max drawdown, position size, leverage, tickets, timeframes |
| Strategy | `strategy.json` (with the human thesis) | Why the edge exists, entry, exit, risk/reward |
| Engine contracts | `data.json`, `model.json`, `execution.json`, `portfolio.json`, `analytics.json` | Technical mechanics; only change with a planned hypothesis |

You do not write an entry rule inside `model.json`, and you do not put an
indicator inside `strategy.json`.

## 2. Write the policy

Use only global, prop-firm-style values. No news filters, no weekend rules, no
session tricks.

```json
{
  "schema": "quantrule.policy/v1",
  "account": { "currency": "USD", "initial_cash": 10000.0 },
  "objective": {
    "performance_target": 0.10,
    "evaluation_days": 365,
    "max_drawdown": 0.20
  },
  "risk": {
    "max_position_fraction": 1.0,
    "max_leverage": 1.0,
    "max_open_positions": 1
  },
  "universe": {
    "tickets": ["EURUSD"],
    "timeframes": ["15m"]
  }
}
```

Validate it with:

```bash
quantrule policy check --config config/examples/solusd-2025/run.json
```

## 3. Write the strategy in trader words

The strategy must answer:

- Why should this edge exist?
- When do you enter long, and when short?
- When do you exit?
- Where is the stop, and what reward-to-risk do you require?
- What would prove the idea wrong?

The machine config is the contract; the thesis is the human explanation.
Render it with:

```bash
quantrule strategy explain --config config/examples/solusd-2025/run.json
```

A valid strategy is deterministic, causal, and free of statistics in its
decision path.

## 4. Pre-register an experiment

Before running, record in `docs/experiments/<date>/`:

- tickets and timeframes;
- data versions;
- in-sample and out-of-sample/walk-forward split;
- costs;
- green criteria: return, drawdown, minimum trades, stability;
- maximum number of variations;
- stop conditions.

## 5. Run the matrix

Each cell is one ticket/timeframe/config and one isolated output root. The
Governing Swarm may run independent cells in parallel. Do not mix a program fix
and a trading change in one experiment.

## 6. Read the result and triage

| Observation | Decision |
|---|---|
| Crash, wrong value, contradiction, silent corruption | Program → audit → plan → engineering |
| Model implementation wrong | Quant investigation |
| Model/strategy/execution/portfolio configuration | One pre-registered Quant change |
| Analytics/measurement | Fix measurement; never tune it to green |
| Data issue | Data audit/fix |
| No out-of-sample improvement | Stop or state a new hypothesis |

## 7. Decide, record, repeat

- If it is a program problem: write/route an audit finding, let the Planner write
  the plan, then execute and verify. The finding is `Fixed` after merge and
  `Verified` after the Improvement Verifier checks it.
- If it is a trading problem: one change, one hypothesis, one validation. The
  Quant Validator must be independent.
- If the edge is not there: stop. Do not tune until green.

## 8. Green is not “one good backtest”

Green requires:

- out-of-sample or walk-forward evidence;
- realistic costs;
- enough trades;
- acceptable drawdown;
- stability across the pre-registered cells;
- no post-hoc threshold changes;
- a Quant Validator record for model/strategy changes.

## 9. What not to do

- Do not change analytics to make a result look better.
- Do not drop losing tickets after seeing the results.
- Do not optimize thresholds on the same sample used to judge them.
- Do not put a statistical filter into the strategy.
- Do not let an agent merge or release a trading change without human approval.
- Do not call a single in-sample backtest an edge.
