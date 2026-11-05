# AI Transparency Log — M2.x Development Swarm (#428)

- Date: 2026-11-05 (UTC)
- Tool: OpenCode agent session; model DeepSeek V4.1 Flash
- Work item: #428 P5 M2.x development swarm
- Parent plan: [`docs/execution-plans/2026-11-05/410-m2-model-programme.md`](../../execution-plans/2026-11-05/410-m2-model-programme.md)
- Charter: [`docs/governance/2026-11-05/M2-CHARTER.md`](../../governance/2026-11-05/M2-CHARTER.md)
- Result record: [`docs/improvement/2026-11-05/M2-DEV-SWARM.md`](../../improvement/2026-11-05/M2-DEV-SWARM.md)
- Status: P5 executed; decision `no_edge`; commit and pull request follow this
  record

## 1. Intent and boundary

Run the 15 pre-registered M2.x cells on `model_dev`, report every cell, and stop
without opening `model_validation`. The work does not change a model hypothesis,
indicator formula, event definition, strategy rule, or cost after seeing a
result. The only code changes are the plumbing required to execute the
pre-registered commands and the v2 validation fix described below.

## 2. Implemented work

| Item | Detail |
|---|---|
| `tools/research/model_cell.py` | Runs four instrument specs over the phase's yearly folds, merges yearly prediction/readings per instrument, writes one pooled evaluation |
| `config/research/PROGRAM-2026-11-05-03/runs/` | 20 one-year instrument run configs (2018–2022), four policies, five shared configs |
| `crates/quantrule-model/src/spec/v2.rs` | Removed the active-event duplicate-code rejection that contradicted the v2 contract; added a regression test |
| `docs/model/2026-11-05/model-spec-v2.md` | States that several registry IDs may share one numeric code and the event is counted once per bar by code |
| `tools/research/tests/fixture_model_shell.py` | Year-aware synthetic fixture so yearly bundles merge without duplicate timestamps |
| `tools/research/test_model_cell.py` | Dev/validation year mapping, deterministic rerun, merged bundle, duplicate-fold rejection |
| `docs/workflows/research-swarm.md`, `config/research/README.md` | Yearly fold and merged-bundle layout |
| `docs/improvement/2026-11-05/M2-DEV-SWARM.md` | Full 15-cell result, hashes, gate detail, finding P5-F1 |

Pre-run plumbing defects corrected: multi-year run configs exceeded the 600,000
M1-candle cap; `available_timeframes` omitted `1m`; `execution.fill_timeframe`
was `15m` against a `5m` ladder; and the v2 validator rejected event IDs that
share a runtime code. No hypothesis, cell, horizon, target, stop, filter, or
cost changed after a result.

## 3. Development run

```text
python3 tools/research/batch.py \
  --program config/research/PROGRAM-2026-11-05-03/program.json \
  --phase dev --mode config --max-parallel 5
ok batch PROGRAM-2026-11-05-03 dev cells=15
elapsed 2:18.02
```

```text
python3 tools/research/report.py \
  --program config/research/PROGRAM-2026-11-05-03/program.json \
  --results data/research/PROGRAM-2026-11-05-03/dev \
  --out data/research/PROGRAM-2026-11-05-03/program-results
ok report PROGRAM-2026-11-05-03 -> data/research/PROGRAM-2026-11-05-03/program-results
decision: no_edge — no primary cell passed the development gate
candidate: none
```

All 15 cells failed. M2.1 is the closest primary cell: pooled lift +0.0024,
LCB -0.0263, both controls fail. EXPL-02 is the closest exploratory cell: lift
+0.0142, controls pass, replication 4, but the 0.02 lift gate and the back-lag
gate fail. EXPL-08 produced no predictions because its three compression events
are disabled by the frozen model defaults.

## 4. Verification and evidence

Observed on 2026-11-05 in the `feat/428-m2-dev-swarm` worktree:

| Command | Observed result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo test --workspace --quiet` | exit 0; every reported suite passed |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `python3 -m unittest discover -s tools/research -p 'test_*.py'` | 42 passed |
| `python3 -m unittest discover -s tools/improvement -p 'test_*.py'` | 135 passed |
| `python3 tools/research/program.py --program ... --check` | `ok ... cells=15 primary=5 phase=dev` |
| `python3 tools/research/coverage.py --program ...` | `ok ... cells=15 indicators=46 events=143` |
| `python3 tools/check_links.py` | 541 files, no broken links |
| `python3 tools/check_terms.py` | 341 files, no banned terms |
| `python3 tools/check_registry.py` | 46 indicators, 143 events, 110 parameters |
| M2.1 second run | same prediction digest, metrics, and grade |

Runtime emission coverage was also measured from the produced `events.csv`
files: 101 of 106 unique codes, 138 of 143 event IDs at code level. The five
never-emitted IDs are recorded in the result document; they are the P5-F1
finding, not a claim of coverage.

## 5. Learning

- The 600,000 M1-candle cap makes a three-year partition impossible in one
  shell run. One-year folds plus a deterministic merge keep the declared
  partition and the input contract.
- The v2 contract already allowed several registry IDs to share one numeric
  event code. The validator rejected the active set anyway, so a documented
  contract and the code disagreed. The regression test pins the contract.
- Dense predictions collapse into few clusters under the horizon-gap rule.
  EXPL-04 and EXPL-06 fail the cluster gate because their predictions are
  contiguous, not because their sample is small.
- Static activation coverage is not runtime emission coverage. Two registry
  events have no emitter, and three compression events are disabled by the
  frozen defaults. A runtime check is needed before a future programme claims
  full event coverage.
- The yearly merge must reindex bars and reject duplicate timestamps; the
  synthetic fixture was made year-aware so the dry run exercises the same path.

## 6. Rejected or deferred

| Suggestion | Decision | Reason |
|---|---|---|
| Promote EXPL-02 after the run | Rejected | Exploratory cells are not ranked, it fails the 0.02 lift and back-lag gates, and post-hoc promotion is forbidden |
| Change the lift gate or horizon | Rejected | Post-hoc criteria change |
| Enable compression and rerun EXPL-08 / M2.5 | Deferred | Would be a new pre-registration; the iteration cap is 1 and the premise is retired |
| Open `model_validation` | Rejected | No primary cell passed; the human has not approved validation |
| Add a missing-event implementation in P5 | Deferred | Needs an audit/plan and a new programme; recorded as P5-F1 |
| Keep the multi-year run configs | Rejected | They exceed the shell input cap and cannot run |

## 7. Boundary

This log does not claim:

- that any M2.x model has an edge;
- that a candidate exists;
- that validation was opened or that a baseline was accepted;
- that the 143-event registry is fully implemented at runtime;
- that a strategy, execution, or portfolio result exists.

The next gate is the human's review of the 15 cells and acceptance of
`no_edge`. P6 stays closed. Any new hypothesis needs a new premise and a new
approved charter.
