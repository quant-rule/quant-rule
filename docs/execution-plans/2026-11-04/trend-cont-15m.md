# Trend Continuation 15m — Strategy Brief

- Date: 2026-11-04 (UTC)
- Status: Approved
- Branch: `feat/trend-cont-15m`
- Roles: Quant Developer (implementation), Test Engineer (verification),
  Result Analyst / Triage Officer (run records)
- Claim type: development and one-shot validation evidence for one
  pre-registered strategy hypothesis; no edge and no baseline acceptance

## Intent

Test one 15m event family — the first bar of the trend-continuation conjunction,
type 3 / subtype 6 / source 16 — for a cost-aware directional edge on top of the
accepted v4 model. The new payoff hypothesis is 2 ATR target against 1 ATR stop
within 20 bars; the v4 gate tested the same family at 1 ATR / 1 ATR and failed on
5m, 15m, and 1h.

The brief is a retest of a known family with a pre-registered payoff, not a new
family. A failed gate is "no edge" and stops the brief. No threshold, target,
stop, horizon, timeframe, filter, or cost changes after a result is seen.

## Current state

- Phase 0 is merged: `time_stop_bars` exists in the strategy/runner path, the
  path-validation harness carries `state_applicable`, and the v4 model bundle
  `data/model-v4/dev/btc-2018-adx25/model/readings.csv` exists.
- The v4 development gate returned grade C for H1 at 1 ATR / 1 ATR on all
  timeframes (`docs/improvement/2026-11-04/MODEL-2026-11-04-01-model-v4-dev.md`).
- `config/model-v4/dev/matrix.json` evaluates `adx20` and `adx25`, primary
  `adx25`; `config/model-v4/validation/matrix.json` mirrors it for 2021–2022.
- The strategy engine supports `quantrule.strategy/v8` `event-composite` with
  `time_stop_bars`; execution supports `market_next_open`, ATR brackets, 2 bps
  slippage, and 4 bps commission.

## Scope

Included:

- `config/trend-cont-15m/**`: model family lists, shared strategy/execution/
  portfolio configs, and the dev/validation run matrices.
- `tools/improvement/build_trend-cont-15m_matrix.py`: a copy of the v9 matrix
  builder with the pre-registered paths, names, and rule.
- Run records `MODEL-2026-11-04-17`, `MODEL-2026-11-04-22`,
  `STRATEGY-2026-11-04-08`, `STRATEGY-2026-11-04-13` under the local data
  workspace.
- `docs/improvement/2026-11-04/TREND-CONT-15M-CANDIDATE.md`, this plan, and the
  AI transparency log.

Excluded:

- Any Rust change, model/V4 config change, execution/portfolio/policy/analytics
  change, or timeframe above 1h.
- Compression variants, additional targets/stops/horizons, or extra filters.
- Push, pull request, merge, branch/worktree deletion, or a baseline acceptance
  claim.

## Assumptions and uncertainty

- The v4 `model-adx25` bundle is the accepted model measurement for this brief;
  the prompt pre-approves model validation and treats a pass as the brief's
  model baseline.
- The event is emitted once per conjunction onset and is available on its
  occurrence bar; the strategy window is one bar.
- Prior evidence is disclosed: this is the same H1 family that failed at
  1 ATR / 1 ATR. The 2 ATR target is the only changed payoff parameter.
- A 2 ATR target is harder to fill than 1 ATR; the base-rate comparison is the
  test, and a fail is a valid result.

## Decisions

- Model dev runs both `adx20` and `adx25` with no `--config`; the gate requires
  the selection to be non-null, on `adx25`, grade A or B.
- Model validation is opened once only if dev passes; it uses `--config adx25`,
  one primary cell, and `--skip-sensitivity`.
- Strategy dev is 2018–2022, one-year folds, three instruments; validation is
  2024 once. Both use the same evaluator flags and the same pre-registered green
  criteria.
- The strategy is side-symmetric (`direction: 0`) and reads only the published
  event; the ladder is 15m/1h for context, execution fills at 15m.
- Human approval of this plan authorizes commits on `feat/trend-cont-15m` only.

## Acceptance criteria

- The plan and AI log exist and record actual commands and results.
- The model dev gate passes: selection non-null, `adx25`, grade A/B, with
  event count ≥ 200 and clusters ≥ 20; otherwise the brief stops with no edge.
- If dev passes, the one-shot validation gate passes on the same criteria;
  otherwise the brief stops with development-only / no edge.
- If validation passes, strategy dev is green: average instrument return > 0,
  worst fold drawdown ≤ 0.10, ≥ 30 trades, pooled profit factor > 1.0, ≥ 2 of 3
  instruments positive, no fold halts; otherwise no edge.
- If dev is green, 2024 validation is green on the same criteria; otherwise no
  edge.
- The candidate document reports the hypothesis, gate table, strategy table,
  verdict, and limitations, and does not call a pass an edge or a baseline.

## Work breakdown

1. Quant Developer: write the plan, configs, family lists, and matrix builder.
2. Quant Developer: write the model dev manifest, run the harness, record
   results, review, and decision.
3. If dev passes: run the model validation gate once and record it.
4. If validation passes: build `cargo build --release -p quantrule-shell`, run
   the strategy dev matrix, evaluate, and record.
5. If dev is green: run the 2024 validation matrix once and record.
6. Test Engineer: verify manifests with `validate.py`, rerun determinism checks,
   confirm artifact digests, and check the final diff for shared-file changes.
7. Result Analyst / Triage Officer: write the candidate document and the run
   reviews/decisions; report to the human.

## Verification

- `python3 tools/improvement/validate.py <manifest>` prints `ok <run_id>` for
  each run.
- `path_validation.json` records selection, grade, n, clusters, and controls.
- `strategy_trend_cont_15m.json` records `passed` and every criterion.
- A byte-identical rerun of the final executed matrix is compared by artifact
  hash.
- `git diff --stat main...feat/trend-cont-15m` contains only owned paths.
- Limitation: same-session review; no independent Quant Validator record, no
  human acceptance, and no claim of an edge.

## Risks and recovery

- Data or bundle missing: stop, do not substitute another timeframe or payload.
- Harness or shell failure: record the exit code and streams, stop; a program
  defect is routed to the Audit Workflow, not patched here.
- A gate fails: record no edge and stop; the branch is not pushed.
- A run record is written before its run and never edited afterwards; a rerun
  would be a new run ID, which this brief does not pre-approve.

## Stop conditions

Phase 0 missing; data/model pack missing; event count < 200 or clusters < 20;
gate grade C or no selection; validation fail; strategy dev not green; 2024
fail; any test or determinism failure; any need to change the event, target,
stop, horizon, timeframe, filter, or cost; any need for a Rust change; any
conflict with another session. Stop and report rather than improvise.

## Outcome (2026-11-04)

Executed the plan through the model development gate. `MODEL-2026-11-04-17`
returned `selection: null`: the `adx25` 15m cell is grade C on
`incremental_lcb` (−0.0035) and `back_lag` (+0.2128), with 4,118 events and
2,895 clusters. The brief stopped under the pre-registered stop condition. Model
validation, strategy development, and 2024 validation were not opened; only the
development run record exists. Verdict: no edge. Evidence is in
`data/improvement/2026-11-04/MODEL-2026-11-04-17/` and
[`docs/improvement/2026-11-04/TREND-CONT-15M-CANDIDATE.md`](../../improvement/2026-11-04/TREND-CONT-15M-CANDIDATE.md).
