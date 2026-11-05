# M2.x Model Programme — Execution Plan

- Date: 2026-11-05 (UTC)
- Status: **Approved** (human, 2026-11-05); P0–P3 merged; P4 implemented; P5 next
- Owner: human
- Epic: [#410](https://github.com/quant-rule/quant-rule/issues/410)
- Related: [#405](https://github.com/quant-rule/quant-rule/issues/405) indicator platform
- Naming registry: [`docs/model/2026-11-05/model-families.md`](../../model/2026-11-05/model-families.md)
- Phase 1 audit: [`docs/improvement/2026-11-05/INDICATOR-AUDIT.md`](../../improvement/2026-11-05/INDICATOR-AUDIT.md)
- Legacy inventory: [`docs/execution-plans/2026-11-05/410-legacy-inventory.md`](410-legacy-inventory.md)
- Claim type: model programme, naming, and evaluation contract; no edge claim

## 1. Context

The indicator platform is implemented: 46 indicators, 143 unique events, 110
parameters, and a tested `quantrule.model-spec/v1` condition evaluator. The 37
programme rows are `tested`; they are not `verified` because the `Sign` column is
open. Phase 1 of the indicator plan (`INDICATOR-AUDIT.md`) was not delivered
before implementation; this programme delivers it retrospectively and labels
that limitation. Phases 6–8 of the indicator plan (the model swarm, validation,
and closeout) are open.

The existing v6–v10 names are **strategies**, not models. The new model family is
`M2.x`; the old model is `M1.0`; the old strategies are historical `S1.1–S1.5`.
The new strategy family is `S2.x`, opened only after an accepted M2.x baseline.

The spec v1 is condition-only. It cannot configure reading windows, activate or
deactivate events, define a prediction, or read panel/session events. The M2.x
programme requires a model spec v2, a prediction evaluator, a finite coverage
matrix, and a model-spec swarm mode.

## 2. Scope

Included:

- Naming registry and legacy inventory.
- Retrospective Phase 1 indicator audit.
- Model spec v2 with configurable readings and active events.
- Causal directional prediction (`+1` long, `-1` short, `0` flat) and an
  optional exploratory score.
- Prediction evaluator with forward outcomes, costs, time-series splits,
  controls, and a model grade.
- Panel context for XAU, BTC, S&P 500, and EURUSD at one timeframe.
- Session context for the evaluated instrument.
- Finite coverage matrix: every registry event used in at least one cell.
- Ten exploratory area cells and five primary M2.x cells.
- Model-spec swarm batch, leaderboard, coverage checker, and dry run.
- One-shot `model_validation` and a baseline or `no_edge`.
- Closeout, checklist `Sign`, audit closure, naming registry, and board cleanup.

Excluded:

- Cross-timeframe models (later, starting with two timeframes).
- Strategy, execution, portfolio, and live work.
- New indicators or events unless the audit finds a defect.
- Order book, tick data, funding, open interest, news, sentiment, fundamentals.
- Any statistical or fitted model in the signal path.
- Reusing the v6–v10 numbers for models.

## 3. Decisions

| Decision | Choice | Status |
|---|---|---|
| Naming | `M1.0`, `S1.x`, `M2.x`, `S2.x`; dot notation with a type prefix | Approved |
| Historical artifacts | Keep history records; remove legacy operational code, configs, and docs; mapping registry | Approved |
| Timeframe | 1h single timeframe first | Approved |
| Panel | XAU, BTC, S&P 500, EURUSD at 1h; no ETH | Approved |
| Prediction | Primary directional vote `+1/-1/0`; score exploratory only | Approved |
| Model spec | `quantrule.model-spec/v2` | Approved |
| Coverage | 10 exploratory area cells + 5 primary M2.x cells | Approved |
| Correction | Bonferroni over 5 primary cells | Approved |
| Validation | One-shot `model_validation` (2021–2022) | Approved |
| Strategy | `S2.x` only after an accepted M2.x model baseline | Approved |
| Audit | Retrospective `INDICATOR-AUDIT.md` | Approved |

The exact prediction horizon, costs, and green criteria are fixed in the M2.x
charter (P3) before the first development run. They are not chosen after a
result.

## 4. Data science protocol

The programme is a time-series prediction study, not a random split and not a
fitted model.

| Partition | Window | Use |
|---|---|---|
| `model_dev` | 2018-01-01..2020-12-31 | Exploration; all cells, event coverage, configuration choice |
| `model_validation` | 2021-01-01..2022-12-31 | One-shot model confirmation; no tuning |
| `strategy_dev` | 2018-01-01..2022-12-31 | Later, after an accepted model baseline |
| `strategy_validation` | 2024-01-01..2024-12-31 | Later, one-shot |
| `portfolio_dev` | 2024-01-01..2024-12-31 | Later |
| `portfolio_holdout` | 2025-01-01..2025-12-31 | Later; untouched |

Rules:

- **No random row split.** Every split is by timestamp. Shuffling candles or rows
  would leak the future.
- **Training means exploration.** In a rule-based model there are no fitted
  coefficients, no regression, and no learned weights. The cells are chosen on
  `model_dev`.
- **Validation is the model test.** A spec is frozen by ID and configuration hash
  before `model_validation`; one access per hypothesis.
- **Purge/embargo.** A prediction at bar `t` is scored over `t+1..t+H`. Drop the
  last `H` bars of `model_dev` (or the first `H` bars of validation) so a dev
  outcome cannot cross the split. Drop overlapping outcomes or cluster them by
  entry bar; the charter declares which.
- **2023 is excluded.** The model programme does not read 2024 or 2025.
- **Panel alignment.** The four instruments are aligned on common 1h close
  timestamps; a timestamp absent from one instrument is absent from the panel.
  No forward fill.
- **Per-instrument evidence.** Report every instrument and the pooled result. A
  candidate must show the direction on at least two instruments; one instrument
  does not carry the claim.
- **Costs.** The evaluator applies a declared commission and slippage hurdle to
  the forward outcome. The model gate is a filter; the strategy tier later uses
  execution and portfolio accounting.
- **Class imbalance.** Report the base rate, hit rate, lift, and lift lower bound.
  Accuracy alone is not evidence.
- **Multiple testing.** The five primary cells are Bonferroni-corrected. The ten
  area cells are exploratory and are not confirmatory evidence. Every variation
  is recorded, including failures.
- **Controls.** Back-lag (shift the prediction by `H`), direction shuffle, and a
  random-entry/base-rate control. A lift that dies under the controls is not a
  candidate.
- **No peeking.** The validation partition is not opened until the five primary
  cells are frozen and the human approves the one-shot access.

The data sources are `XAUUSD_M1_2018-2024`, `BTCUSD_M1_2018-2024`,
`ESANDP500_M1_2018-2024`, and `EURUSD_M1_2018-2024` in the local workspace.
S&P is the E-mini (`ESANDP500`). ETH is excluded because it is too correlated
with BTC to add an independent panel vote.

## 5. Naming and legacy

| New ID | Old name | Kind |
|---|---|---|
| `M1.0` | current model v4 | Model baseline |
| `S1.1`–`S1.5` | strategy v6–v10 | Historical strategies |
| `M2.1`–`M2.5` | new model versions | New models |
| `S2.1+` | future strategies | New strategies |

- The naming registry maps every old name and alias and fixes the rules in
  [`model-families.md`](../../model/2026-11-05/model-families.md).
- Historical records (AI transparency logs, improvement run records, audit
  reports, execution plans, reports) are kept. Legacy operational configs, old
  strategy contract docs, and unused strategy-specific code are removed in a
  dedicated `chore/legacy-removal` change after the inventory and reference
  check.
- The current model engine (`M1.0`) and its tests are not legacy. They remain the
  baseline and the v4 event source.
- Issue `#426` (the old v6–v10 charter) is closed as superseded and replaced by
  the M2.x charter issue. It is not deleted, so the record and links remain.
- Issue `#410` is renamed “M2.x model swarm and validation”; `#427–#430` are
  re-scoped to M2.x.
- The board’s container statuses are fixed by native sub-issue links or by the
  repository workflow; they are not hand-edited.

## 6. Swarm exploratory model mode

The M2.x swarm has two modes. They are not interchangeable.

| Mode | Cells | Partition | Evidence | Eligible for validation |
|---|---|---|---|---|
| Exploratory model | 10 area cells, one per information area | `model_dev` only | Coverage and hypothesis generation | No |
| Confirmatory model | 5 primary cells | `model_dev`, then one-shot `model_validation` | Corrected primary evidence | Yes, if promoted |

The **exploratory model mode** exists so that every one of the 143 events is
tried at least once without turning the search into an optimizer:

- Each area cell activates the events of one information area and publishes a
  directional vote. The cell is labelled `exploratory` in `cell-result.json` and
  in the program report.
- The ten area cells are **not ranked** and receive **no multiple-testing
  credit**. A positive area cell is a hypothesis for a primary cell, not a
  candidate.
- The exploratory mode is read-only with respect to the validation partition. It
  never opens `model_validation`, never changes a primary spec, and never feeds
  a strategy.
- A coverage checker reads the ten specs and the registry and fails the program
  if any registry event ID is absent from every exploratory prediction. Shared
  event IDs are counted once. It also checks all 46 indicator IDs: an indicator
  with no event (for example `calendar_bucket`) is exercised as a reading or a
  condition in its area cell.
- Diagnostic events may appear in exploratory cells. A primary cell that will
  feed a strategy uses only `signal` and `context` events.
- The human freezes the ten area specs and the five primary specs before the
  first run. A cell agent may not add, remove, or retune an event after seeing a
  result. A failed exploratory cell is retired; it is not retried with a variant.
- Exploratory results are reported in full, including failures. They are
  evidence about coverage and data quality, not a performance claim.

The **confirmatory model mode** is the only mode that can produce an accepted
M2.x baseline. It uses the five primary cells, the declared correction, the
frozen specs, and the one-shot validation access.

## 7. Model spec v2

```json
{
  "schema": "quantrule.model-spec/v2",
  "id": "M2.1",
  "family": "M2",
  "instrument": "XAU",
  "timeframes": [3600],
  "panel": {
    "leader": "BTC",
    "sources": {
      "BTC": "../../data/sources/BTCUSD_M1_2018-01-01_2024-12-31.csv",
      "SP500": "../../data/sources/ESANDP500_M1_2018-01-01_2024-12-31.csv",
      "EURUSD": "../../data/sources/EURUSD_M1_2018-01-01_2024-12-31.csv"
    }
  },
  "session": {
    "zone": "london",
    "start_hour": 8,
    "end_hour": 16,
    "opening_range_bars": 2
  },
  "readings": {},
  "active_events": [],
  "prediction": {
    "kind": "vote",
    "long": [],
    "short": [],
    "min_votes": 2
  },
  "conditions": []
}
```

- `timeframes` has exactly one entry in v2. Cross-timeframe is a later schema.
- `panel` is optional. It names a leader and the other instruments' M1 sources.
  The evaluated instrument comes from the run config and is not repeated in the
  panel sources. The panel aligns on common timestamps; no forward fill.
- `session` is optional. It is the calendar used by the session events.
- `readings` exposes the v4 and extended reading windows and periods. Event
  thresholds remain contract-fixed; only activation and reading windows vary.
  A field that would turn every event threshold into a free parameter is
  rejected.
- `active_events` is the set of registry IDs the model version uses. The
  prediction references only active events. Only `signal` events may vote; a
  `context` or `diagnostic` event is a condition, not a vote.
- `prediction.kind = vote` counts each event once: a long event adds `+1`, a
  short event adds `-1`, and the sign of the total above `min_votes` is the
  prediction. `kind = score` is exploratory only and is not a primary cell.
- `conditions` is optional and reads the combined base, extended, session, and
  panel event set. It is diagnostic and does not change the vote.
- No stop, target, position, cash, or P&L field. The prediction is a model
  output, not a strategy.
- The evaluator, not the spec, owns the horizon, costs, and partition. The spec
  cannot read a forward outcome.
- The run writes `readings.csv`, `events.csv` (active-event projection),
  `levels.csv`, `conditions.csv`, `prediction.csv`, `result.json`, and
  `provenance.json`. The provenance names the spec ID/family, file hash,
  registry version, sources, panel order, session, reading overrides, active
  events, and prediction definition.

The implemented contract and its limits are in
[`model-spec-v2.md`](../../model/2026-11-05/model-spec-v2.md). P1 is implemented;
the checks are recorded in the PR and the P1 AI log.

## 8. Prediction evaluator

- Reads `prediction.csv`, the model bundle readings, and the source candles.
- Joins each prediction at its bar `t` with the forward path `t+1..t+H`.
- Supports a forward ATR-normalized return and the existing barrier target/stop.
- Applies commission and slippage as a hurdle.
- Reports n, clusters, hit rate, base rate, lift, lift LCB, incremental lift,
  back-lag, direction shuffle, per-instrument, per-year fold, and grade A/B/C/D.
- Writes `model_prediction.json` with schema, spec ID/family, instrument,
  timeframe, horizon, costs, partition, metrics, verdict, and limitations.
- Reuses `tools/improvement/path_validation.py` and `prediction_pilot.py` for
  the barrier/forward-return and control machinery.
- Is deterministic: fixed seed, sorted inputs, no wall clock.

## 9. Coverage matrix and charter

- **Ten exploratory area cells**, one per information area. Each cell activates
  the events of that area and predicts from them. These cells guarantee every one
  of the 143 unique events is used at least once.
- **Five primary M2.x cells**:
  - `M2.1` volume and participation
  - `M2.2` structure and volatility
  - `M2.3` cross-asset
  - `M2.4` session and time
  - `M2.5` all-area vote
- The five primary cells are the only confirmatory cells; the correction covers
  them. The ten area cells are exploratory and labelled as such.
- A coverage checker reads the specs and the registry and fails the program if
  any event ID is absent from every prediction.
- The coverage checker also checks all 46 indicator IDs. An indicator with no
  event (`calendar_bucket`) is exercised as a reading or condition in its area
  cell.
- The prediction for a strategy-facing candidate uses only `signal`/`context`
  events. Diagnostic events are allowed in exploratory cells and are not used in
  a candidate that will feed a strategy.
- The charter declares the premise, cells, correction, costs, green criteria,
  stop rule, iteration cap, and validation access. The human approves it before
  the first run.

The event-to-area mapping is the registry `indicator` field grouped by the ten
coverage areas in
[`indicator-coverage.md`](../../model/2026-11-05/indicator-coverage.md). The
coverage checker validates that the ten area specs together cover the registry
event set exactly once or more.

## 10. Swarm tooling

- `tools/research/program.py` — add model-spec cells: a spec path, a model
  command, and a prediction evaluator command.
- `tools/research/batch.py` — config-only model-spec batch; no code worktrees
  unless an agent changes code.
- `tools/research/adapters.py` — add the `model_prediction` parser.
- `tools/research/report.py` — model leaderboard columns and correction.
- `tools/research/coverage.py` — every event covered, every primary spec frozen.
- `tools/research/cell.py` — record the spec ID, prediction digest, metrics, and
  verdict.
- The coordinator creates the program and cells; cell agents run one cell and
  cannot read another cell’s result. No post-hoc tuning.

## 11. Phase 1 audit

- Retrospective, read-only audit of the top ten indicators/areas: VWAP, volume
  profile, RVOL, volume climax, absorption, OBV, CMF, MFI, ATR term, and session.
- Records the formula, inputs, outputs, events, role, claim, tests, and any
  residual gap.
- Clearly labelled retrospective: it is not the pre-registration the original
  plan intended, but it closes the missing artifact.
- Path: `docs/improvement/2026-11-05/INDICATOR-AUDIT.md`.

## 12. Work breakdown

| Phase | Work | Files | Role | Depends |
|---|---|---|---|---|
| P0 | Naming registry, audit, legacy inventory | `docs/model/2026-11-05/model-families.md`, `docs/improvement/2026-11-05/INDICATOR-AUDIT.md`, `docs/execution-plans/2026-11-05/410-legacy-inventory.md` | Architect, Audit | — |
| P1 | Model spec v2 | `crates/quantrule-model/src/spec*`, `crates/quantrule-shell/src/spec.rs`, `report_run.rs`, `pipeline.rs` | Architect, Quant Developer | P0 |
| P2 | Prediction evaluator | `tools/improvement/model_prediction.py`, `tools/research/adapters.py` | Quant Developer, Test Engineer | P1 |
| P3 | Coverage matrix and M2.x charter | `docs/governance/2026-11-05/M2-CHARTER.md`, `config/research/M2-*/program.json`, `tools/research/coverage.py` | Quant Researcher, Human | P2 |
| P4 | Swarm model mode | `tools/research/program.py`, `batch.py`, `report.py`, `coverage.py` | Implementation, Test Engineer | P2, P3 |
| P5 | Development swarm | `data/research/M2-*/dev`, `docs/improvement/2026-11-05/M2-DEV-SWARM.md` | Coordinator, Cell agents | P4, charter approval |
| P6 | Validation and baseline | `data/improvement/.../MODEL-...`, `docs/improvement/.../M2-VALIDATION.md` | Quant Validator, Human | P5 |
| P7 | Closeout | `docs/registry`, `docs/improvement/index.md`, `baselines.md`, checklist, board | Coordinator, Human | P6 |
| P8 | Strategy (later) | `S2.x` | Quant Researcher, Quant Developer, Validator | P6 |

### P0 details

1. Write the naming registry and aliases.
2. Write the retrospective audit.
3. Inventory legacy strategy code, configs, docs, and data. Mark each as keep
   (history record) or remove (legacy). Check references, tests, and configs.
4. Close #426 as superseded; rename #410; create the M2.x charter issue.
5. Fix the board’s native sub-issue links.

P0 documentation (items 1–3) is written in this worktree. Items 4–5 are issue
and board actions and remain pending until the human authorizes them.

### P1 details

1. Write `docs/model/2026-11-05/model-spec-v2.md`.
2. Add v2 spec types and validation; keep v1 valid.
3. Add reading overrides (v4 and extended) and active-event sets.
4. Build the `MarketStateConfig`/`ExtendedConfig` from the spec.
5. Add panel/session inputs to the spec run.
6. Add the vote predictor and `prediction.csv`.
7. Update provenance and `report model --spec`.
8. Tests: parse, validate, override, prediction, panel/session, causality,
   determinism, byte-identical rerun, edge cases.

P1 implementation (2026-11-05): the v2 contract is
[`model-spec-v2.md`](../../model/2026-11-05/model-spec-v2.md). The model crate
owns `spec/v2.rs`; the shell owns the v2 parser and the report branch; the
`report model --spec` bundle publishes `prediction.csv`, the active-event
projection of `events.csv`, optional `conditions.csv`, and the v2 provenance.
`MarketStateConfig` carries the extended reading config so the override reaches
`TimeframeModel`. The verification is in the P1 PR and the #454 AI log.

### P2 details

1. Write `model_prediction.py` with forward-return and barrier modes.
2. Add purge/embargo at the split.
3. Add costs, base rate, lift, LCB, controls, per-instrument/fold.
4. Write `model_prediction.json`.
5. Add the `model_prediction` adapter.
6. Tests: look-ahead, causality, known lift, determinism, empty/short series.

P2 implementation (2026-11-05): the contract is
[`docs/analytics/2026-11-05/model-prediction.md`](../../analytics/2026-11-05/model-prediction.md).
The evaluator supports forward and barrier outcomes, cost hurdles, split bounds
and outcome-crossing purge, direction-matched base rates, cluster-robust lift,
Wilson fallback, random/shuffle/back-lag controls, per-instrument and per-year
results, and grade A–D. The research adapter registers `model_prediction`.
Verification is in the P2 PR and the #455 AI log.

### P3 details

1. Write the premise and economic rationale for M2.x.
2. Define the 10 area cells and 5 primary cells with exact specs.
3. Declare the correction, costs, green criteria, stop rule, iteration cap, and
   validation access.
4. Build the coverage checker and prove every indicator and event is covered.
5. Human approves the charter.

P3 implementation (2026-11-05): the charter is
[`docs/governance/2026-11-05/M2-CHARTER.md`](../../governance/2026-11-05/M2-CHARTER.md).
The program is `config/research/PROGRAM-2026-11-05-03/program.json`, with ten
exploratory cells, five primary cells, and four instrument specs per cell
(60 specs). The coverage checker is `tools/research/coverage.py`; it proves
46/46 indicators, 143/143 events, and the required panel/session context.
The charter was approved by the human on 2026-11-05. No development run starts
before the P4 model-cell runner lands.

### P4 details

1. Add model-spec cell fields and commands.
2. Run a dry run on a tiny fixture; aggregate deterministically.
3. Verify the leaderboard and coverage output.
4. No validation access.

P4 implementation (2026-11-05): `tools/research/model_cell.py` runs the four
instrument specs, one pooled `model_prediction`, and writes the root result the
`model_prediction` adapter reads. The 32 run configs live under
`config/research/PROGRAM-2026-11-05-03/runs/`; the program cell command passes
`--phase {phase}`. `cell.py` records `spec_id` and `prediction_digest`, and
`report.py` renders the model leaderboard columns. The dry run is
`tools/research/test_model_cell.py`; it uses a fake shell and the real evaluator.
Verification is in the P4 PR and the #427 AI log.

### P5 details

1. Freeze the five primary specs by hash.
2. Run the 15 cells on `model_dev`.
3. Report every cell, including failures.
4. No post-hoc variant; no validation.

### P6 details

1. Human reviews every cell and promotes at most the declared primary cells.
2. Freeze the candidates.
3. Run `model_validation` once per promoted cell.
4. Quant Validator reproduces the candidate.
5. Human accepts a baseline or records `no_edge`.

### P7 details

1. Update the registry, index, and baselines.
2. Sign the checklist rows; close the audit.
3. Remove legacy artifacts per the inventory.
4. Clean worktrees and branches; update the board.

## 13. Acceptance criteria

- The naming registry maps every old name and every new ID.
- The retrospective audit exists and records the evidence and gaps.
- A v2 spec parses, validates, runs, and writes a causal `prediction.csv`.
- Reading overrides and active-event sets change the published output.
- Panel and session events are usable for the evaluated instrument.
- The prediction evaluator reproduces a known synthetic lift and rejects a
  lagged or shuffled prediction.
- Every one of the 46 indicators and 143 unique events is exercised in at least
  one cell. Shared event IDs are counted once; `calendar_bucket` is exercised as
  a reading or condition because it emits no event.
- The 10 exploratory and 5 primary cells are pre-registered and frozen.
- The development swarm reports every cell.
- The validation run is one-shot, reproducible, and Quant-Validator signed.
- A baseline is accepted or `no_edge` is recorded.
- The checklist rows are `verified` by the human/Validator.
- Legacy paths are removed with no broken references; `cargo test` passes.
- The board is clean and the old container statuses are correct.

## 14. Verification

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
python3 tools/check_registry.py
python3 tools/check_links.py
python3 tools/check_terms.py
python3 -m unittest discover -s tools/research -p 'test_*.py'
python3 -m unittest discover -s tools/improvement -p 'test_*.py'
python3 tools/research/coverage.py --program config/research/M2-*/program.json
```

A model spec run must be byte-identical on rerun. The evaluator must be
deterministic and must fail a back-lagged prediction. The validation run must
record the data version, spec hash, config hash, code commit, and seed.

## 15. Risks and stop conditions

| Risk | Mitigation |
|---|---|
| Legacy deletion breaks references | Inventory, reference check, separate PR, Git history |
| Spec v2 grows into a strategy language | Bounded prediction kinds; no stops/positions/P&L |
| Panel/session complexity | Single timeframe; optional panel/session; no forward fill |
| Data gaps in S&P/EURUSD | Common timestamps; report coverage; skip missing bars |
| Multiple testing / search | 10 exploratory + 5 primary; Bonferroni; no post-hoc variants |
| Validation leakage | Purge/embargo; frozen specs; one-shot access |
| Naming collision | Type prefix; mapping registry |
| Issue deletion loses history | Close as superseded; delete only on explicit destructive approval |
| Retroactive audit is not pre-registration | Label it retrospective; do not claim it screened the backlog |
| Strategy drift | Strategy tier closed until a model baseline is accepted |

Stop when: the premise is falsified; no primary cell passes; a validation cell
fails; the iteration cap is reached; a shared-file conflict blocks a cell; the
human stops the program.

## 16. Human decisions

The human approved the following on 2026-11-05:

1. 1h as the single timeframe.
2. Panel XAU/BTC/SP500/EURUSD and common-timestamp coverage.
3. The 10+5 cell budget and Bonferroni over 5.
4. The retrospective audit.
5. The legacy removal boundary (old strategy configs/docs/code; keep the engine
   and history records). The exact list is proposed in
   [`410-legacy-inventory.md`](410-legacy-inventory.md) and awaits review.
6. Closing #426 as superseded rather than deleting it.
7. `S2.x` remains closed until an M2.x baseline is accepted.

Still fixed in the P3 charter before the first run:

1. Prediction horizon, costs, and model green criteria.
2. Confirmation that `model_validation` is fresh and approval of the one-shot
   access.

## 17. Issue changes

| Action | Issue | Why |
|---|---|---|
| Close as superseded | #426 old v6–v10 charters | Replaced by M2.x |
| Rename | #410 “M2.x model swarm and validation” | Remove the strategy-name collision |
| Rename/re-scope | #427 “Model-spec batch and leaderboard” | Model specs, not strategy configs |
| Re-scope | #428 dev swarm, #429 validation, #430 closeout | M2.x |
| New | Model spec v2 | V1 cannot configure or predict |
| New | Prediction evaluator | No forward-outcome model gate |
| New | Naming registry and legacy inventory | Order and history |
| New | Retrospective indicator audit | Phase 1 |
| New | Coverage matrix / M2.x charter | Every event, finite cells |
| Close | #409 | #423–#425 are done |
| Update | #405 phase table | Phases 6–8 become M2.x P3–P7 |

Executed 2026-11-05: #410 was renamed “M2.x model swarm and validation”, set
In Progress, and linked to its eight sub-issues; #405 was renamed and linked to
#410; #426 was closed as superseded; #409 was closed as delivered; #427–#430
were re-scoped; #454 (P1), #455 (P2), #456 (P3), and #457 (P0) were created.
PR [#453](https://github.com/quant-rule/quant-rule/pull/453) carries the P0
documentation and closes #457.

## 18. Approval

- Approved by: human, 2026-11-05
- Scope: the full M2.x programme through P7; P8 is a later decision
- Not authorized by this plan: strategy work, live capital, a new dependency, or
  deleting the historical records
- Commits, pushes, pull requests, and merges remain separate approval gates
