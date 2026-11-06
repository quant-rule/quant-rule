# P8b — Build and Run the M3.x Programme

- Date: 2026-11-05 (UTC)
- Issue: [#476](https://github.com/quant-rule/quant-rule/issues/476)
- Status: **Verified** 2026-11-05; outcome `underpowered` at the count gate.
  The development run, validation, and strategy tier were not reached and
  remain closed. Approved by the human instruction "proceed" on 2026-11-06
  following the approved charter
- Claim type: model programme implementation and development preflight; no edge
  claim
- Charter: [`M3-CHARTER.md`](../../governance/2026-11-05/M3-CHARTER.md)
- Predecessor: [`476-m3-charter.md`](476-m3-charter.md)
- AI transparency log:
  [`docs/ai-transparency-log/2026-11-05/476-m3-programme.md`](../../ai-transparency-log/2026-11-05/476-m3-programme.md)

## 1. Intent

Build the approved M3.x programme and run its 15 cells on `model_dev`, without
opening `model_validation` or the strategy tier. The result is either a grade-A
primary cell promoted for a later one-shot validation or a recorded `no_edge` /
`underpowered` stop.

This is the first implementation of the state-conditioned evaluation. It does
not add a Rust indicator, event, state, or model formula.

## 2. Current state

- The M2 programme has 10 exploratory + 5 primary cells and a working
  `model_cell.py`/`model_prediction.py` path, but the evaluator has no state
  mode and the runner hard-codes `family M2`, four instruments, and
  `spec-<instrument>.json`.
- `coverage.py` is M2-specific: it expects M2 cell IDs, family M2, one timeframe,
  four instruments per cell, and one `spec-<instrument>.json` per instrument.
- The M2 run configs contain retired `strategy`/`analytics` fields and are not
  loadable by the current shell. The M3 programme needs fresh run configs with
  the current `quantrule.run/v2` fields.
- `readings.csv` publishes `context_code`, `adx_regime`, `adx_slope`,
  `ema_stack`, `di_state`, `rsi_zone`, and `macd_state`, so the approved gates
  are measurable without a model change.
- The M1 data pack is local: XAU/BTC/S&P 500/EURUSD M1 2018–2024. The shell cap
  is 600,000 M1 candles, so the programme uses yearly folds.
- The M2 programme took minutes per cell with five parallel workers; the M3 run
  has 15 cells and two timeframe count runs, so the batch is expected to run for
  tens of minutes.

## 3. Scope

Included:

- Add the state-gate schema and parser to `model_prediction.py`.
- Add state-filtered metrics, state-matched base/controls, the state increment,
  and the grade-A-only promotion rule.
- Extend `model_cell.py` with a per-cell instrument set, per-timeframe spec
  selection, `--state-file`, `--timeframe auto`, and a count-only mode.
- Add the M3 coverage path and tests.
- Add the M3 programme manifest, 15 cell directories, state files, specs, run
  configs, and the timeframe-selection record.
- Run coverage, the count-only switch, and all 15 development cells.
- Write the run record, review, decision, index/baseline updates, and closeout.
- Update the documentation and AI transparency log.

Excluded:

- `model_validation`, `strategy_dev`, `strategy_validation`, portfolio, and
  live work.
- Changing a Rust formula, event, state, reading, or the model contract.
- Changing the approved premise, cells, state gates, instruments, outcome,
  costs, correction, or green criteria after a result.
- Tuning a failed cell or adding a variant.
- Deleting the M2 historical programme or configs.

## 4. Decisions

| ID | Decision | Status |
|---|---|---|
| P8b-1 | Spec files are `spec-<instrument>-<timeframe>.json`; the runner selects the timeframe, and `selection.json` freezes the count-only choice | Approved for this plan |
| P8b-2 | The evaluator gains `--state-file`; the raw model prediction is unchanged and the state is applied at evaluation | Approved |
| P8b-3 | The primary cell result uses the state metrics; `unconditional_metrics`, `state_increment`, and `state_gate` are recorded separately | Approved |
| P8b-4 | A state-gated cell promotes only at grade A; grade B is `development_only` | Approved |
| P8b-5 | M3 run configs are fresh current-schema configs; the M2 run configs are not reused | Approved |
| P8b-6 | The M3 coverage path is reachable from `coverage.py` and validates per-cell instruments, two timeframes, and state gates | Approved |
| P8b-7 | The development run uses `model_dev` only; validation stays closed | Approved |

## 5. Work breakdown

1. **Evaluator.** Add the state-gate parser, state-filtered base/entries, state
   metrics, state increment, state controls, grade-A promotion, Markdown output,
   and tests.
2. **Runner.** Extend `model_cell.py` for per-cell instruments, timeframe
   selection, state file, count-only mode, and `family M3`.
3. **Coverage.** Add the M3 coverage path and tests; keep the M2 path unchanged.
4. **Programme configs.** Create `PROGRAM-2026-11-05-04`: `areas.json`,
   `program.json`, 10 exploratory cells, 5 primary cells, state files, specs for
   both timeframes, and current-schema run configs.
5. **Coverage gate.** Run the M3 coverage check; it must report 15 cells,
   46 indicators, 143 events.
6. **Count switch.** Run the five primary cells at 1h and 15m with no outcome;
   write `selection.json`; freeze the selected timeframe.
7. **Development run.** Run all 15 cells on `model_dev` with five parallel
   workers; record the artifacts and result hashes.
8. **Report and decision.** Apply the correction, review every cell, write the
   run record and closeout. If no primary reaches grade A, record `no_edge`; if
   the count gate fails, record `underpowered`.
9. **Documentation and verification.** Update the programme index, baselines if
   applicable, docs index, plan, and AI log; run the full gate.

## 6. Acceptance criteria

- The evaluator, runner, and coverage tests pass, including the M2 regression
  path and the new state tests.
- The M3 coverage check passes with 15 cells, 46 indicators, and 143 events.
- `selection.json` records the count-only 1h/15m decision and the counts.
- All 15 cells run on `model_dev`; no cell reads 2021 or later.
- The result table records n, clusters, lift, LCB, controls, back-lag,
  replication, state increment, grade, and verdict for every cell.
- The programme records `no_edge`, `underpowered`, or a grade-A candidate; no
  validation is opened.
- The full Rust/Python/document gate passes.
- The issue remains open if a validation or strategy gate remains; no edge claim
  is made.

## 7. Verification

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps
python3 -m unittest discover -s tools/improvement -p 'test_*.py'
python3 -m unittest discover -s tools/research -p 'test_*.py'
python3 tools/check_registry.py
python3 tools/check_validation.py
python3 tools/check_links.py
python3 tools/check_terms.py
python3 tools/check_legacy.py
python3 tools/research/coverage.py --program config/research/PROGRAM-2026-11-05-04/program.json --family M3
git diff --check
```

The programme evidence is the coverage output, `selection.json`, the cell
results, the run record, and the development report. The run is not a strategy
or profitability claim.

## 8. Risks and recovery

| Risk | Mitigation |
|---|---|
| State sample is too small | The count-only switch chooses 1h or 15m; otherwise stop as `underpowered` before any outcome |
| BTC/ETH volume contaminates a cell | Volume cells use XAU/S&P 500/EURUSD; the trend cell uses all four |
| Evaluator state mode changes M2 behavior | No state file means the existing path is unchanged; M2 regression tests must pass |
| Run configs are rejected by the current shell | Build current `quantrule.run/v2` configs without retired fields; test one run before the batch |
| The run exceeds the timeout | Use five parallel workers; each cell has a 3,600s timeout; record a timeout as a stop, not a retry |
| A cell is changed after a result | The manifest, specs, state files, and selection are frozen and hashed before the run |
| Validation is opened early | The runner has no validation phase in this plan; validation needs a separate human approval |
| A failed cell is tuned | The iteration cap is 1; a failed cell is recorded and the premise is retired |

Recovery: the code and configs are new; the M2 programme and its evidence remain
at `8e8373b`. The development run artifacts are local under `data/research/`;
the committed result is the run record.

## 9. Stop conditions

- A Rust model change is required.
- The state gate cannot be validated without changing the registry claim.
- The coverage check cannot prove 46/143.
- The count gate finds no timeframe with 200 state-matched predictions and 20
  clusters in every primary cell.
- A primary cell reaches grade A but the human has not approved validation.
- A run produces a schema or provenance error that cannot be fixed without
  changing the approved protocol.
- The human stops the programme.

## 10. Quant roles

- **Quant Researcher:** owns the approved charter and the cell/result
  interpretation; confirms no post-hoc change.
- **Quant Developer:** implements the state evaluator, runner, coverage, and
  configs; writes the tests.
- **Quant Validator:** reproduces the coverage, count switch, and cell results;
  checks look-ahead, split leakage, state filtering, correction, and costs.

The three passes are sequential by the same agent. They are not independent
agents or independent model judgments.

## 11. Execution record

Executed on `feat/476-m3-programme`, based on `8e8373b`.

### Built

- `tools/improvement/model_prediction.py`: state-gate parser, state-filtered
  base and entries, state metrics, state increment, state-matched controls,
  grade-A-only promotion, and Markdown state table.
- `tools/research/model_cell.py`: per-cell instruments, per-timeframe specs,
  `--timeframe auto`, `--state-file`, `--family`, and count-only mode.
- `tools/research/coverage.py`: M3 coverage path; the M2 path is unchanged.
- `tools/research/build_m3_program.py`: deterministic 15-cell programme builder.
- `tools/research/select_timeframe.py`: count-only 1h/15m selector.
- `config/research/PROGRAM-2026-11-05-04`: 10 exploratory cells, 5 primary
  cells, 112 specs, state gates, current-schema run configs, and
  `selection.json`.

### Coverage

```text
ok PROGRAM-2026-11-05-04 cells=15 indicators=46 events=143
```

### Count gate

| Cell | 1h n | 1h clusters | 15m n | 15m clusters | Passes |
|---|---:|---:|---:|---:|---|
| M3.1 | 1,226 | 911 | 5,059 | 3,782 | both |
| M3.2 | 1,924 | 775 | 10,127 | 3,452 | both |
| M3.3 | 3,025 | 1,007 | 11,626 | 3,208 | both |
| M3.4 | 0 | 0 | 0 | 0 | neither |
| M3.5 | 0 | 0 | 2 | 2 | neither |

No timeframe passes every cell. The approved stop rule applies: **underpowered**.
M3.1–M3.3 are not run alone, and M3.4/M3.5 are not tuned. No forward outcome was
read, no development cell ran, `model_validation` was not opened, and the
strategy tier remains closed.

The raw event predictions confirm the sample issue rather than a code error:
M3.4 has 1 raw prediction at 1h and 58 at 15m; M3.5 has 7 at 1h and 13 at 15m.
The approved state gates remove almost all of them.

### Verification

| Check | Observed result |
|---|---|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --all-targets -- -D warnings` | Pass |
| `cargo test` | 982 passed, 0 failed, 7 ignored across 28 suites |
| `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps` | Pass |
| `python3 -m unittest discover -s tools/improvement` | 79 passed |
| `python3 -m unittest discover -s tools/research` | 43 passed |
| M2 coverage | 15 cells, 46 indicators, 143 events, 60 specs |
| M3 coverage | 15 cells, 46 indicators, 143 events, 112 specs |
| `python3 tools/check_registry.py` | 46 indicators, 143 events, 110 parameters |
| `python3 tools/check_validation.py` | 188 claims |
| `python3 tools/check_links.py` | 545 Markdown files; no broken links |
| `python3 tools/check_terms.py` | 323 files; no banned terms |
| `python3 tools/check_legacy.py` | 353 current files in the scan set; no retired labels |
| `git diff --check` | Pass |
| Smoke run | One M3.2 XAU 2018 1h run produced the expected bundle |
| Count selector | Exit 1 with `status: "underpowered"`; no timeframe selected |

### Review

The Quant Researcher pass confirmed the count gate applied the approved cells
and did not select a post-hoc subset. The Quant Developer pass checked the state
filter, state-matched base, state increment, grade-A promotion, count-only
implementation, and the M2 regression path. The Quant Validator pass reproduced
the coverage and count results and checked that no outcome, validation, or
strategy data was read. The passes are sequential by the same agent, not
independent reviews.

The plan's "all 15 cells run" criterion was not reached because the approved
count gate stopped the programme first. That stop is the recorded outcome; no
cell result, baseline, or edge claim exists.

## 12. Residual risk

- The approved M3.4/M3.5 state/event pairs are structurally too rare to test on
  this data; the result is `underpowered`, not `no_edge`.
- M3.1–M3.3 clear the count gate but are not evidence of edge; they were not run
  because the programme-level gate failed.
- The state evaluator has unit tests but no development outcome evidence.
- `model_validation` 2021–2022 remains unused for M3; prior compression reuse is
  unchanged.
- A new attempt needs a new premise and a new approved charter; the current
  programme is stopped and is not re-registered with a different state/event
  pairing.
