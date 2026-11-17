# 4. Strategy

- Status: Current
- Audience: operator testing a human-owned rule against a frozen model artifact

## Goal

Run the strategy alone, explain one published decision, and understand why a
standalone decision sequence is diagnostic rather than evidence.

## Before you start

| Need | Value |
|---|---|
| Model artifact | A succeeded model stage with an `artifact_id` |
| Config | A valid config that carries the strategy file |
| Profile | The profile that owns the model artifact |
| Understanding | The strategy reads events, states, and its own position; it does not read fills |

The strategy stage does not need execution, cash, P&L, or a portfolio config.

## Steps

| # | Shell | Browser | Expect |
|---:|---|---|---|
| 1 | `quantrule stage run strategy <model_artifact_id> <config_id>` | **Workbench → Strategy** → Model artifact id, Config id → Run strategy stage | `stage_run_id`, `state: queued`, `created: true` |
| 2 | `quantrule stage list --kind strategy --status succeeded --limit 20` | Strategy tab → stage table | One row per strategy stage |
| 3 | `quantrule stage show <stage_run_id>` | Strategy tab → open the stage | Provenance, `evidence: false`, artifact id, files |
| 4 | `quantrule stage explain <stage_run_id> --bar 12` | Strategy tab → Explain bar | Decision, side, reason, position before/after, published evidence |
| 5 | `quantrule stage fetch <stage_run_id> ./strategy-stage` | Strategy tab → Download | `decisions.ndjson`, `summary.txt`, `provenance.json`, each verified |

`stage explain` selects a record whose `bar_index` matches `--bar`. It does not
recompute a decision from a reading or a later bar.

## What you see

| Field | Meaning |
|---|---|
| `decision` | `hold`, `enter`, `enter_bracket`, or `exit` |
| `side` | `long`, `short`, or `none` |
| `reason` | The published rule reason |
| `position_before`, `position_after` | The strategy's own simulated position |
| `atr14` | Execution metadata in the decision record, not a strategy input |
| `evidence` | The published evidence fields for the decision |
| Artifact files | Exactly `decisions.ndjson`, `summary.txt`, and `provenance.json` |

The decisions bundle carries no fills, cash, equity, P&L, exposure, drawdown, or
canonical digest. The Strategy tab shows the decision table and the explain
panel; it never shows a P&L for a stage.

## Self-simulated versus coupled position

| Path | Position the strategy sees |
|---|---|
| Standalone strategy stage | The position its own decisions intended |
| Coupled backtest | The position execution actually holds |

A rejected coupled entry leaves the coupled strategy flat on the next bar while
the standalone stage may still hold the position it entered. The decision
sequences can therefore differ. The divergence is deliberate: stage decisions
are never fed into a backtest, and a stage is never accepted as a baseline.

## What it proves

**Diagnostic.** The strategy stage proves the rule ran over the frozen model
artifact and published explainable decisions. It does not prove execution,
accounting, or an edge. Those require the coupled run.

## Failure

| Code or symptom | Cause | Next action |
|---|---|---|
| `stage.not_found` | The model artifact or strategy stage is unknown | Re-run the model stage or re-read the stage list |
| `stage.invalid` | The artifact is not a model artifact, or provenance disagrees | Read the message; use a succeeded model stage from this profile |
| `stage.conflict` | The stage is terminal or has no artifact | Read `stage show`; wait or submit again |
| `config.invalid` | The config or strategy file failed validation | Fix the config; do not bypass the validator |
| `service.unavailable` | A required service is down | Read `health`; stop |
| Bar not found in explain | The requested bar has no published decision | Use a bar index present in `decisions.ndjson` |
| Table empty after submit | The stage has not completed | Read `stage show`; a failed stage shows its error |

## Related

- [Model](03-model.md)
- [Portfolio](05-portfolio.md)
- [Stage to backtest tutorial](../tutorials/2026-11-17/stage-to-backtest.md)
- [Strategy configuration](../config/2026-11-06/strategy-config.md)
