# AI Transparency Log — M2.x Charter and Coverage (#456)

- Date: 2026-11-05 (UTC)
- Tool: OpenCode agent session; model DeepSeek V4.1 Flash
- Work item: #456 P3 M2.x charter and event coverage
- Parent plan: [`docs/execution-plans/2026-11-05/410-m2-model-programme.md`](../../execution-plans/2026-11-05/410-m2-model-programme.md)
- Charter: [`docs/governance/2026-11-05/M2-CHARTER.md`](../../governance/2026-11-05/M2-CHARTER.md)
- Program: `config/research/PROGRAM-2026-11-05-03/`
- Status: P3 approved by the human on 2026-11-05; commit and pull request follow
  this record

## 1. Intent and boundary

Pre-register the M2.x model programme: a premise, ten exploratory area cells,
five primary cells, a finite event/indicator coverage matrix, costs, a
correction, green criteria, a stop rule, and a one-shot validation gate. The
work does not run a model, open validation, or change a strategy. The charter is
marked **Proposed** until the human approves it.

The P1 vote was list-side only. The M2 cells need to use the direction coded on
each event, so the P1 contract was extended with
`prediction.kind = "direction"`: a listed signal event contributes its own
`direction_code`. A context or diagnostic event still cannot vote.

## 2. Implemented work

| Item | Detail |
|---|---|
| Direction prediction | `quantrule-model` v2 `PredictionKind::Direction`, validation, predictor; shell parser and tests |
| Area map | `config/research/PROGRAM-2026-11-05-03/areas.json`; all 46 indicators assigned to one of the ten areas |
| Program | `program.json` with 15 cells, 5 primary, 10 exploratory, 4 instruments per cell |
| Specs | 60 v2 specs: `spec-XAU/BTC/SP500/EURUSD.json` per cell |
| Coverage checker | `tools/research/coverage.py`; registry, areas, cell metadata, specs, prediction roles, panel/session |
| Tests | `tools/research/test_coverage.py`, model/shell direction tests |
| Charter | `docs/governance/2026-11-05/M2-CHARTER.md` |
| References | `config/research/README.md`, `docs/workflows/research-swarm.md`, `docs/model/2026-11-05/model-spec-v2.md` |

The primary cells use a curated one-family-per-area vote rather than every
signal event. The exploratory cells activate every signal event in their area because
they are coverage cells and are not ranked. Area 7 has no signal-role event; its
cell uses `signal.rsi_threshold` as a pre-registered benchmark and keeps every
candle/gap event active. `calendar_bucket` is reading-only and has no event.

## 3. Verification and evidence

Observed on 2026-11-05 in the `feat/456-m2-charter-coverage` worktree:

| Command | Observed result |
|---|---|
| `python3 tools/research/coverage.py --program config/research/PROGRAM-2026-11-05-03/program.json` | `ok ... cells=15 indicators=46 events=143` |
| `python3 tools/research/program.py --program ... --check` | `ok ... cells=15 primary=5 phase=dev` |
| `cargo test --workspace --quiet` | exit 0; every reported suite passed |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo fmt --check` | clean |
| `python3 -m unittest discover -s tools/research -p 'test_*.py'` | 37 passed |
| `python3 -m unittest discover -s tools/improvement -p 'test_*.py'` | 135 passed |
| `python3 tools/check_links.py` | 538 files, no broken links |
| `python3 tools/check_terms.py` | 340 files, no banned terms |
| `python3 tools/check_registry.py` | 46 indicators, 143 events, 110 parameters |

The coverage checker proves 46/46 indicators, 143/143 events, 60 specs, and the
required panel/session context. The negative tests inject an unused event and a
context-role vote and fail.

## 4. Learning

- A list-side vote cannot express the direction carried by events such as
  `signal.rsi_threshold`, `signal.overnight_continuation`, or
  `signal.compression_breakout`. The direction kind uses the published event
  direction and keeps the role rule.
- Counting every signal event in a primary cell would count correlated events
  several times. The primary cells use one directional family per area; the exploratory
  cells keep the full area surface.
- Area 7 has no signal-role event. A benchmark vote is declared explicitly and
  listed in `coverage.json`, rather than silently borrowing an event.
- The retained indicators needed an explicit area assignment: `atr` in area 5;
  `ema`, `rsi`, `macd`, `adx`, `dmi`, `pivots` in area 6; `context` and
  `progression` in area 8; `rvol` in area 3.
- The coverage checker must use a canonical `areas.json`; reading the cell's
  own `coverage.json` alone would let a cell declare a false area membership.

## 5. Rejected or deferred

| Suggestion | Decision | Reason |
|---|---|---|
| Run the development swarm in P3 | Rejected | P4 adds the model-cell runner; P5 runs after charter approval |
| Open validation now | Rejected | One-shot validation is a human gate after the primary cells are frozen |
| Primary vote over every signal event | Rejected | Correlated events would be counted more than once |
| A gate predictor over context conditions | Deferred | The current direction vote is the pre-registered primary form; a gate is a later hypothesis |
| A score prediction | Deferred | The approved primary prediction is a directional vote |
| Treat `calendar_bucket` as an event | Rejected | It is a reading with no event; area 10 exercises it as a session reading |

## 6. Boundary

This log does not claim:

- that any M2.x cell passes;
- that the development swarm has run;
- that validation was opened;
- that an edge exists.

The charter was approved by the human on 2026-11-05. The next gate is the P3
commit/PR, then P4's model-cell runner.
