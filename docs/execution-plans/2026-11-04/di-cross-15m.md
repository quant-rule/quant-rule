# Execution Plan: DI Cross 15m Strategy Brief

- Date: 2026-11-04 (UTC)
- Status: **Verified** — executed to the model dev gate; stopped as no edge
- Issue: none; the human's task brief is the authority for this work item
- AI transparency log:
  [`docs/ai-transparency-log/2026-11-04/di-cross-15m.md`](../../ai-transparency-log/2026-11-04/di-cross-15m.md)
- Claim type: model and strategy development gate; no edge claim, no validation
  claim, no profitability claim.

## 1. Intent

Test one pre-registered 15m event family — the +DI/−DI crossover — for a
cost-aware directional edge on top of the accepted v4 model. The brief is a
single fixed protocol: the event definition, timeframe, target, stop, horizon,
filters, and costs are frozen before any result is seen. A failed gate is
recorded as "no edge" and stops the brief.

## 2. Current state

- Phase 0 is merged to `main`: `time_stop_bars` exists in
  `crates/quantrule-backtest/src/strategy/event.rs`; `state_applicable` exists in
  `tools/improvement/path_validation.py`; the v4 development model pack is
  present at `data/model-v4/dev/btc-2018-adx25/model/readings.csv`.
- The v4 model gate tested trend continuation (H1), MACD line/signal cross (H2),
  RSI 50 cross (H3), and EMA level break (H4). It did not test the DI crossover.
- `config/model-v4/shared/model-adx25.json` enables `di_cross: true`; the model
  already publishes signal events with type 3, subtype 2 and source 17 (+DI),
  reference 18 (−DI).
- The strategy schema v8 (`event-composite`) supports a single zero-direction
  event entry with an ATR bracket and a time stop.
- The run store templates are `MODEL-2026-11-04-04` and
  `STRATEGY-2026-11-04-01`.

## 3. Scope

Included:

- Strategy, execution, and portfolio configs under
  `config/di-cross-15m/strategy/shared/`.
- Model gate families under `config/di-cross-15m/model/`.
- A strategy matrix builder `tools/improvement/build_di-cross-15m_matrix.py`,
  derived from `build_strategy_v9_matrix.py` with only the path/name constants
  changed.
- Model gate development run `MODEL-2026-11-04-15`, model gate validation run
  `MODEL-2026-11-04-20`, strategy development run `STRATEGY-2026-11-04-06`,
  strategy validation run `STRATEGY-2026-11-04-11`.
- The candidate document, this plan, and the AI transparency log.

Excluded:

- Any change to `crates/**`, `config/model-v4/**`, `config/model-gate/**`,
  `tools/improvement/path_validation.py`,
  `tools/improvement/strategy_v9_evaluate.py`, `docs/improvement/index.md`,
  `docs/improvement/baselines.md`, `docs/improvement/INVALIDATIONS.md`.
- Any change to the event, timeframe, target, stop, horizon, filters, or costs
  after a result is seen.
- Compression variants, a timeframe above 1h, or a second hypothesis.
- Validation, baseline acceptance, portfolio work, push, pull request, merge, or
  deletion.

## 4. Assumptions and uncertainty

- The model bundles were produced by `model-adx25`; the DI crossover event is
  enabled there. The `adx20` sensitivity bundle is compared for isolation, not
  for tuning.
- The development model gate uses `--primary-cells 5` in the Bonferroni
  correction, as the brief fixes it.
- A pass on both model partitions and both strategy partitions makes the
  candidate eligible for independent Quant Validator review; it does not make it
  an accepted baseline.
- The 15m execution fill can only use data the data engine emits for 15m; the
  1h ladder member is context only and the rule does not read it.

## 5. Decisions

| Decision | Choice | Approval |
|---|---|---|
| Event | Type 3, subtype 2, source 17, reference 18, 15m, zero direction | Human, in brief |
| Target / stop / horizon | 2 ATR / 1 ATR / 20 bars | Human, in brief |
| Costs | 4 bps commission per operation, 2 bps slippage, no funding | Human, in brief |
| Model dev pass | selection non-null, config `adx25`, grade A or B | Human, in brief |
| Model validation | one shot, `--config adx25`, `--skip-sensitivity` | Conditional pre-approval |
| Strategy dev pass | evaluator `passed` true and every criterion true | Human, in brief |
| Strategy validation | one shot 2024 with identical flags | Conditional pre-approval |
| Baseline acceptance | not granted; report for human acceptance | Reserved to human |

## 6. Acceptance criteria

- `MODEL-2026-11-04-15` records a non-null selection under `adx25` with grade A
  or B, or records no edge and stops.
- If dev passes, `MODEL-2026-11-04-20` records a non-null selection with grade A
  or B, or records no edge and stops.
- If model validation passes, `STRATEGY-2026-11-04-06` evaluates green on
  `strategy_dev`, or records no edge and stops.
- If dev is green, `STRATEGY-2026-11-04-11` evaluates green on 2024, or records
  no edge and stops.
- Every run folder has the contract layout, an immutable manifest written before
  the run, `results.json` with the exact command, exit code, streams, and
  artifact hashes, and `review.md` and `decision.md`.
- The candidate document reports the gate table, strategy table, verdict, and
  limitations without claiming an edge.

## 7. Work breakdown

| Step | Role | Path |
|---|---|---|
| 1. Write plan, configs, AI log | Quant Developer | `docs/execution-plans/2026-11-04/di-cross-15m.md`, `config/di-cross-15m/**` |
| 2. Pre-register model dev manifest, run gate | Quant Developer | `data/improvement/2026-11-04/MODEL-2026-11-04-15/` |
| 3. Pre-register model validation manifest, run once | Quant Developer | `data/improvement/2026-11-04/MODEL-2026-11-04-20/` |
| 4. Build strategy matrix, run dev folds, evaluate | Quant Developer | `tools/improvement/build_di-cross-15m_matrix.py`, `data/di-cross-15m/**`, `data/improvement/2026-11-04/STRATEGY-2026-11-04-06/` |
| 5. Run 2024 folds once, evaluate | Quant Developer | `data/improvement/2026-11-04/STRATEGY-2026-11-04-11/` |
| 6. Write candidate document, verify, commit | Quant Developer / Test Engineer | `docs/improvement/2026-11-04/DI-CROSS-15M-CANDIDATE.md` |

## 8. Verification

- Preflight commands from the brief, each returning the expected match.
- `python3 tools/improvement/validate.py <run>/manifest.json` prints
  `ok <run_id>` for each run.
- `cargo build --release -p quantrule-shell` completes.
- The backtest loop runs each generated `run.json`; a non-zero exit stops the
  brief.
- `strategy_v9_evaluate.py` writes `strategy_di_cross_15m.json` and `.md`; the
  `passed` field and every criterion are read, not assumed.
- Artifact SHA-256 digests are recorded in `results.json`.
- Limitations: one data pack, three instruments, no independent Quant Validator
  record in this session, and no claim of an edge.

## 9. Risks and recovery

- A failed gate is recorded as no edge; no threshold is retuned. Recovery is a
  new hypothesis with human approval, not a rerun of this brief.
- The run folders are local under `data/improvement/`; the committed evidence is
  the candidate document and this plan.
- A data or tool mismatch stops the brief instead of being worked around.

## 10. Stop conditions

Phase 0 missing; model pack missing; event count below 200 or clusters below 20;
gate grade C or no selection; validation fail; strategy dev not green; 2024
fail; any test or determinism failure; any need to change the event, target,
stop, horizon, timeframe, filter, or cost; any need for a Rust change; any
conflict with another session.

## 11. Execution evidence

Observed on 2026-11-04 (UTC) in this worktree.

- The preflight commands from section 2 all matched. The three source CSVs and
  `data/model-v4` are symlinks, not copies.
- `MODEL-2026-11-04-15` manifest was written before the run and validated with
  `python3 tools/improvement/validate.py` → `ok MODEL-2026-11-04-15`.
- The model dev command exited 0 and wrote `path_validation.json` and
  `path_validation.md` (SHA-256 recorded in `results.json`). A determinism rerun
  to `/tmp/di-model-dev-rerun` produced byte-identical artifacts.
- Result: `adx25` grade **C** with selection **null**. n = 17,851, clusters =
  3,563, hit = 0.3121, base = 0.3073, lift = +0.0048, lift LCB = −0.0038,
  random p = 0.0796, shuffle p = 0.7612, replication = 3. Failed checks:
  `lift`, `lift_lcb`, `random`, `shuffle`.
- `adx20` produced the identical event set and is recorded as the sensitivity
  cell.
- The stop condition "gate grade C or no selection" fired. `MODEL-2026-11-04-20`,
  `STRATEGY-2026-11-04-06`, and `STRATEGY-2026-11-04-11` were not opened. No
  strategy backtest, evaluator run, Rust build, or shared-file edit was made.
- `cargo build --release -p quantrule-shell` was not run because no strategy
  stage was opened; this is not a test claim.
- The candidate report is
  `docs/improvement/2026-11-04/DI-CROSS-15M-CANDIDATE.md`.

Outcome: **no edge**; the brief is stopped. No validation or baseline claim is
made.
