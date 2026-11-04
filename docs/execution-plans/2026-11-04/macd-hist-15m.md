# MACD Histogram Flip 15m — Execution Plan

- Date: 2026-11-04 (UTC)
- Status: Approved
- Branch: `feat/macd-hist-15m`
- Worktree: `../quant-rule--macd-hist-15m`
- Roles: Quant Developer and Test Engineer (sequential passes; no independent
  agent separation is claimed)

## Intent

Test one pre-registered 15m event family for a cost-aware directional edge on top
of the accepted v4 model. The hypothesis is that a MACD histogram slope flip
marks momentum re-acceleration, and that the next 20 15m bars reach +2 ATR in the
flip direction before −1 ATR more often than the unconditional base rate.

The brief is falsifiable and one-shot at each validation gate. A failed gate is
`no edge` and stops the brief; the rule is not tuned into passing.

## Current state

- Phase 0 is merged on `main` at `fec0a93`:
  `time_stop_bars` exists in `crates/quantrule-backtest/src/strategy/event.rs`,
  `state_applicable` exists in `tools/improvement/path_validation.py`, and the v4
  dev model pack is present.
- The accepted v4 model config is `config/model-v4/shared/model-adx25.json`; the
  model matrix evaluates `adx20` and `adx25`, with `adx25` primary.
- The v4 gate previously tested the MACD line/signal cross (H2), not the
  histogram slope flip. This brief tests type 3 / subtype 4 / source 15.
- The existing run records `MODEL-2026-11-04-04` and `STRATEGY-2026-11-04-01`
  supply the folder and document templates.

## Scope

Included:

- New configs under `config/macd-hist-15m/`.
- A copied-and-adapted strategy matrix builder at
  `tools/improvement/build_macd-hist-15m_matrix.py`.
- Model development and one-shot validation runs on 15m.
- Strategy development on 2018–2022 and one-shot validation on 2024.
- Run records, the candidate document, this plan, and the AI transparency log.

Excluded:

- Any change to `crates/**`, the model, execution, portfolio, policy, or
  analytics.
- Any timeframe above 1h, compression variant, filter, or post-result parameter
  change.
- A portfolio run, a release, or a claim of validation or edge.

## Assumptions and uncertainty

- The v4 model pack is the accepted model baseline for this brief; the brief
  pre-approves a model validation pass as the baseline.
- Event availability is the bar on which the histogram slope flip is published;
  the strategy enters at the next 15m open.
- Costs are 4 bps commission per operation, 2 bps slippage, and no funding.
- The main uncertainty is whether the unconditional 2:1 barrier base rate leaves
  room for an incremental edge after costs. A development failure is a valid
  outcome.

## Decisions

| Decision | Choice | Consequence |
|---|---|---|
| Event | type 3, subtype 4, source 15, 15m, direction ±1 | Tests histogram re-acceleration, not the line cross |
| Entry | single `direction: 0` event condition | Trades both sides of the same event |
| Risk | 1 ATR stop, 2 ATR target, 20-bar time stop | Fixed before any result |
| Costs | 4 bps commission, 2 bps slippage, 0 funding | Conservative relative to the bracket |
| Model gate | adx20 and adx25 on dev; adx25 only on validation | Matches the pre-registered protocol |
| Baseline | Strategy baseline is not pre-accepted | Human acceptance remains required |

Human approval for this plan is recorded by the brief that authorizes commits on
this branch. No further approval is assumed.

## Acceptance criteria

- Model dev selection is non-null, under `adx25`, grade A or B.
- Model validation selection is non-null, grade A or B.
- Strategy dev `passed` is true and every criterion is true.
- Strategy 2024 `passed` is true and every criterion is true.
- Every run record has a manifest written before the run, `results.json` with
  command, exit code, streams, and artifact hashes, plus `review.md` and
  `decision.md`.
- The candidate document reports the gate table, strategy table, verdict, and
  limitations without claiming validation or edge.

## Work breakdown

| Step | Work | Role | Paths |
|---|---|---|---|
| 1 | Write plan and AI log | Orchestrator | `docs/execution-plans/`, `docs/ai-transparency-log/` |
| 2 | Write strategy, execution, portfolio, and family configs | Quant Developer | `config/macd-hist-15m/` |
| 3 | Run model dev gate | Quant Developer / Test Engineer | `data/improvement/2026-11-04/MODEL-2026-11-04-16/` |
| 4 | Run model validation gate if dev passes | Quant Developer / Test Engineer | `data/improvement/2026-11-04/MODEL-2026-11-04-21/` |
| 5 | Build strategy matrix and run 2018–2022 | Quant Developer / Test Engineer | `tools/improvement/build_macd-hist-15m_matrix.py`, `data/improvement/2026-11-04/STRATEGY-2026-11-04-07/` |
| 6 | Run 2024 if dev is green | Quant Developer / Test Engineer | `data/improvement/2026-11-04/STRATEGY-2026-11-04-12/` |
| 7 | Write candidate document and commit | Orchestrator | `docs/improvement/2026-11-04/MACD-HIST-15M-CANDIDATE.md` |

## Verification

- `path_validation.py` output for each model gate, read from
  `artifacts/path_validation.json`.
- Determinism check: rerun the model dev gate with the same seed and inputs into
  a separate directory; both artifact SHA-256 digests matched.
- `python3 tools/improvement/validate.py <manifest>` for each run record.
- No Rust build, `cargo test`, shell backtest, or strategy evaluator was run:
  the model development gate failed before the strategy stage, and no Rust
  source changed. This is recorded as a limitation, not as a passing check.

## Risks and recovery

- A model or strategy failure stops the brief; it does not invalidate the
  accepted v4 model baseline.
- The 15m event count is large (tens of thousands across dev cells), so sample
  size is not the binding risk; cost drag and base-rate dominance are.
- The run folders are local (`data/**` is ignored); the committed evidence is
  this plan, the candidate document, and the AI log.
- Recovery from a failed gate is a new pre-registered hypothesis, not a parameter
  change.

## Stop conditions

- Phase 0 missing; data pack missing; event count < 200 or clusters < 20.
- Model grade C or no selection; model validation fail.
- Strategy dev not green; 2024 fail.
- Any test or determinism failure.
- Any need to change the event, target, stop, horizon, timeframe, filter, or
  cost, or any need for a Rust change.

## Outcome

Executed 2026-11-04 and stopped at the model development gate.

| Gate | Run | Result |
|---|---|---|
| Model development | `MODEL-2026-11-04-16` | fail: selection null, `adx25` grade C |
| Model validation | not opened | — |
| Strategy development | not opened | — |
| Strategy validation | not opened | — |

The `adx25` cell recorded 60,328 events and 74 clusters, hit rate 0.3151 against
a 0.3073 base (+0.78pp), lift LCB −1.26pp, shuffle p 0.3532, and replication 3.
It failed `lift`, `lift_lcb`, and `shuffle`. No validation or strategy partition
was opened, and the strategy matrix builder was not created: it belongs to the
strategy stage, which the failed model gate closed. The verdict is `no edge`; the
accepted v4 model baseline is not invalidated. Full evidence:
`docs/improvement/2026-11-04/MACD-HIST-15M-CANDIDATE.md`.
