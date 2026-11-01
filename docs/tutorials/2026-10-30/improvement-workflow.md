# The Improvement Workflow

- Date: 2026-10-30 (UTC)
- Updated: 2026-11-01 for the workflow reset
- Work item: `audit-improvement-workflow`
- Audience: the human owner of the policy and strategy
- Workflow: [`docs/workflows/improvement-workflow.md`](../../workflows/improvement-workflow.md)
- Contract: [`docs/workflows/run-contract.md`](../../workflows/run-contract.md)

This tutorial runs one improvement cycle. It changes one tier at a time, records
the run, and stops when the evidence says “no edge.”

The run-store tooling lives in `tools/improvement/` (Python). It validates the
manifest, creates the run folder, invokes the shell, extracts evidence, and
indexes the result. The shell still executes the backtest. See
[`tools/improvement/README.md`](../../../tools/improvement/README.md) for the
current scripts.

## 1. Know which tier you are changing

| Tier | You change | You must not change | Development | Validation |
|---|---|---|---|---|
| Model | Model implementation or config | Strategy, execution, portfolio, data, policy | `model_dev` | `model_validation` |
| Strategy | Strategy rule, config, thesis | Model, execution, portfolio, data, policy | `strategy_dev` | `strategy_validation` |
| Portfolio | Execution and portfolio config | Model, strategy, data, policy | `portfolio_dev` | `portfolio_holdout` |

If the change needs two tiers, split it into two runs. A combined run cannot be
attributed.

## 2. Write the policy and strategy

The human owns the envelope and the thesis. Use global, prop-firm-style values:
no news filters, no weekend rules, no session tricks.

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
    "tickets": ["BTCUSD"],
    "timeframes": ["15m"]
  }
}
```

Validate it and render the strategy in trader words:

```bash
quantrule policy check --config config/examples/btcusd-2024/run.json
quantrule strategy explain --config config/examples/btcusd-2024/run.json
```

A valid strategy is deterministic, causal, and free of statistics in its decision
path. If you cannot state why the edge exists, the run is not ready.

## 3. Pre-register the run

Pick the run ID before the first artifact:

```text
<MODEL|STRATEGY|PORTFOLIO>-<YYYY-MM-DD>-<NN>
```

Write `manifest.json` at a staging path outside the canonical run folder, for
example `data/improvement/staging/STRATEGY-2026-11-02-01/manifest.json` or a
temporary directory. The run store creates the canonical
`data/improvement/<date>/<run-id>/` folder and copies the manifest into it. Do
not pre-create the canonical run folder with the manifest: `run.py` refuses a
non-empty output root.

The manifest fixes the hypothesis, the tier, the partition, the changed/frozen
split, the run config, the code commit, the seed, and the green criteria. Record
the criteria **before** running:

- return target and maximum drawdown;
- minimum trades and exposure;
- cost assumptions: fees, slippage, funding, commission;
- the exact cells and tickets;
- the pass/fail rule and the iteration cap;
- the stop conditions.

A criterion changed after the result is seen is a new hypothesis, not a rerun.

## 4. Run the development partition

Validate the manifest:

```bash
python3 tools/improvement/validate.py \
  data/improvement/staging/STRATEGY-2026-11-02-01/manifest.json
```

Then let the run store create the canonical folder, copy the configuration, and
invoke the shell:

```bash
python3 tools/improvement/run.py \
  data/improvement/staging/STRATEGY-2026-11-02-01/manifest.json \
  --qr target/debug/quantrule
```

`run.py` refuses a non-empty run folder, copies the manifest unchanged, copies
the run config and its sibling JSON files into `config/`, writes
`config/sources.json`, and runs:

```text
<qr> backtest --config <run_config> --out <root>/artifacts --json
```

`results.json` records the command, exit code, captured streams, parsed stdout,
and a sorted artifact inventory with SHA-256 digests. A non-zero shell exit is a
failure, not a partial success. A rerun with the same inputs must be
byte-identical.

Write the evidence summary:

```bash
python3 tools/improvement/evidence.py \
  data/improvement/2026-11-02/STRATEGY-2026-11-02-01
```

`artifacts/evidence.json` now holds the published numbers: return, drawdown,
trades, profit factor, win rate, halt state, and final equity for a strategy or
portfolio run; version and coverage for a model run.

## 5. Review the results

The Result Analyst writes `review.md` against the accepted baseline. The review
answers:

- Is the comparison attributable to the one changed tier?
- Are costs, sample size, and exposure adequate?
- Is the result stable across the pre-registered cells?
- Was the validation partition touched by tuning?
- What would falsify the conclusion?

Use the template in [`docs/improvement/README.md`](../../improvement/README.md).
Do not write the decision yet.

## 6. Validate once, with a frozen candidate

If the development result is promising, freeze the candidate by configuration and
run the tier's validation partition:

- model: `model_validation`;
- strategy: `strategy_validation`, one-shot;
- portfolio: `portfolio_holdout`, one-shot and authorized by the human.

The validation run is a new run ID with the validation `partition`. Its
`baseline` names the frozen development run, and its `config/` copy must match
that run. No threshold, cost, split, or criterion may change after the validation
result is seen. If the validation fails, the result is not green.

A development run may never read the validation partition or the holdout. The
manifest declares `holdout_access: none` for every partition except
`portfolio_holdout`; the validator refuses a development run that declares
`final`.

## 7. Decide and record

The Triage Officer writes `decision.md` and replaces the `Verdict: pending` line:

- **Accept baseline** — the run is green, the human accepts it, and the next tier
  may start.
- **Reject** — the run is not green; record why and whether a new hypothesis
  follows.
- **Invalidate** — the run is not valid evidence; add a tombstone to
  [`docs/improvement/INVALIDATIONS.md`](../../improvement/INVALIDATIONS.md).
- **No edge** — the hypothesis is falsified; stop; do not tune until green.
- **New hypothesis** — one change, one pre-registration, one tier.

Only the human accepts a baseline. The agent may recommend; it may not accept.

## 8. Update the index and baselines

After the decision:

```bash
python3 tools/improvement/index.py --root data/improvement
```

The command regenerates `data/improvement/index.json` and
`data/improvement/index.md`. Copy the generated `index.md` to
`docs/improvement/index.md` when the run is committed. If a baseline was
accepted, update [`docs/improvement/baselines.md`](../../improvement/baselines.md)
and the accepted run's `decision.md`. If a run was invalidated, append the
tombstone row; never delete a row. The run store does not write the baseline or
invalidation registries automatically.

## 9. Stop or start the next hypothesis

The cycle end is one of:

- the next tier, when a baseline was accepted;
- a new hypothesis in the same tier, when the result was informative but not
  green;
- an audit finding, when the result exposed a program defect;
- `no edge`, when the hypothesis is falsified;
- stop, when the budget, iteration cap, or a stop condition is reached.

A stop is a valid result. The workflow is not required to turn red into green.

## Green is not “one good backtest”

Green requires:

- a pre-registered hypothesis and criteria;
- realistic costs;
- out-of-sample or one-shot validation evidence;
- enough trades and exposure;
- acceptable drawdown within the policy;
- stability across the pre-registered cells;
- deterministic, byte-identical artifacts;
- no post-hoc threshold changes;
- a Quant Validator record for model or strategy changes;
- human acceptance of the baseline.

## What not to do

- Do not change analytics to make a result look better.
- Do not drop losing tickets after seeing the results.
- Do not optimize thresholds on the sample used to judge them.
- Do not put a statistical filter into the strategy.
- Do not reuse the 2025 portfolio holdout for a second candidate.
- Do not mix a program fix and a trading change in one run.
- Do not let an agent accept a baseline or merge a trading change without human
  approval.
- Do not call a single in-sample backtest an edge.

## Related documents

- [Audit Workflow](../../workflows/audit-workflow.md)
- [Improvement Workflow](../../workflows/improvement-workflow.md)
- [Run contract](../../workflows/run-contract.md)
- [Improvement registry](../../improvement/README.md)
- [Improvement charter](../../governance/2026-10-30/improvement-charter.md)
