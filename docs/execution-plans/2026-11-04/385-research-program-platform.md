# Research Program Platform

- Date: 2026-11-04 (UTC)
- Status: **Approved** (human, 2026-11-04, "proceed with the full implementation")
- Epic: [#385](https://github.com/quant-rule/quant-rule/issues/385)
- Sub-issues: [#388](https://github.com/quant-rule/quant-rule/issues/388),
  [#389](https://github.com/quant-rule/quant-rule/issues/389),
  [#390](https://github.com/quant-rule/quant-rule/issues/390),
  [#391](https://github.com/quant-rule/quant-rule/issues/391),
  [#392](https://github.com/quant-rule/quant-rule/issues/392),
  [#393](https://github.com/quant-rule/quant-rule/issues/393),
  [#394](https://github.com/quant-rule/quant-rule/issues/394)
- Branch: `feat/385-research-program-platform`
- Claim type: research tooling; no model, strategy, or edge claim

## 1. Intent

Let a coordinator run many pre-registered hypotheses concurrently without
turning the research program into an automatic search. The platform records a
program manifest, runs cells in parallel, writes one immutable result per cell,
applies the declared correction, and reports every cell. The human promotes to
one-shot validation; the platform never does.

## 2. Current state

- The five-family directional program was run manually as five independent
  branches. It failed at the model gate; the evidence is preserved by the
  governance closeout.
- There is no program manifest, cell runner, batch runner, aggregator, or
  registry.
- The existing engines and improvement tools are deterministic and reusable.
- The governance rules are added in the closeout PR.

## 3. Scope

### Included

- `tools/research/program.py`: program schema and validation.
- `tools/research/cell.py`: one cell, one result.
- `tools/research/adapters.py`: path-validation, strategy-evaluation, and
  summary-json result parsing.
- `tools/research/batch.py`: config mode and worktree mode, bounded parallelism.
- `tools/research/report.py`: every cell, declared correction, candidate only.
- `tools/research/registry.py`: program record; no auto-acceptance.
- Tests for each module.
- `config/research/reference/`: a three-cell fixture dry run.
- `config/research/README.md`.
- `docs/workflows/research-swarm.md` and the coordinator/cell prompt templates.

### Excluded

- No model or strategy code change.
- No engine, execution, portfolio, policy, or analytics change.
- No research campaign and no validation access.
- No automatic model/strategy search, optimization, or selection.
- No baseline acceptance.

## 4. Assumptions and uncertainty

- The existing `path_validation.py` and `strategy_v9_evaluate.py` outputs are
  stable enough to parse by their JSON fields.
- A cell command can be expressed as a string list with `{config}` and `{out}`
  substitutions.
- The worktree mode relies on the program manifest and cell configs being
  committed on the base revision; the reference dry run uses config mode.
- The platform cannot create an edge; it only makes the search honest.

## 5. Decisions

| Decision | Consequence |
|---|---|
| Manifest is data, not code | Cells are inspectable and reviewable |
| One result per cell | No shared mutable result state |
| Config mode and worktree mode | Config-only batches are cheap; code-bearing cells are isolated |
| Report never promotes | The human remains the gate |
| Registry never accepts | A candidate needs an independent quant validator and human acceptance |
| Reference dry run is a fixture | It tests orchestration without consuming validation data |

## 6. Acceptance criteria

- `python3 -m unittest` in `tools/research` passes all tests.
- `python3 tools/check_terms.py` passes.
- `git diff --check` passes.
- The reference dry run runs three cells concurrently and produces
  `program-results` with `no_edge`.
- The reference dry run opens no validation and creates no baseline.
- The registry writes a `no_edge` program record.
- The coordinator/cell prompts and the runbook are committed.
- The platform never starts a real campaign; that is a human-approved next step.

## 7. Work breakdown

| Step | Work | Paths |
|---|---|---|
| 1 | Program manifest and validator | `tools/research/program.py`, `test_program.py` |
| 2 | Result adapters | `tools/research/adapters.py` |
| 3 | Cell runner | `tools/research/cell.py`, `test_cell.py` |
| 4 | Parallel batch runner | `tools/research/batch.py`, `test_batch.py` |
| 5 | Aggregator | `tools/research/report.py`, `test_report.py` |
| 6 | Registry | `tools/research/registry.py`, `test_registry.py` |
| 7 | Reference program and fixture | `config/research/reference/`, `tools/research/tests/fixture_cell.py` |
| 8 | Runbook and prompts | `docs/workflows/research-swarm.md`, `docs/prompts/` |
| 9 | Verify, commit, push, open PR, merge | branch `feat/385-research-program-platform` |

## 8. Verification

| Check | Command | Expected |
|---|---|---|
| Research tests | `cd tools/research && python3 -m unittest test_program test_cell test_batch test_report test_registry` | all pass |
| Terms | `python3 tools/check_terms.py` | no banned terms |
| Diff | `git diff --check` | clean |
| Reference manifest | `python3 tools/research/program.py --program config/research/reference/program.json --check` | ok 3 cells |
| Reference batch | `python3 tools/research/batch.py ... --mode config --max-parallel 3` | 3 cell results |
| Reference report | `python3 tools/research/report.py ...` | decision `no_edge` |
| Reference registry | `python3 tools/research/registry.py ...` | status `no_edge` |

Limitations: the reference dry run uses fixture cells, not market data. The
platform is deterministic tooling; it does not test a market hypothesis. The
worktree mode is exercised by the runbook and the prepare-only path; the
reference dry run uses config mode.

## 9. Risks and recovery

- **A cell result is missing or malformed.** The report records `missing`; the
  coordinator stops.
- **A worktree path collides.** The batch runner refuses to overwrite; the
  coordinator cleans the path first.
- **A validation phase is opened early.** The program requires `one_shot` and a
  promoted cell list; the human is the gate.
- **The report is read as an automatic selection.** The report says candidate
  for human review; it never opens validation.
- **The platform is mistaken for an edge generator.** The reference dry run and
  the docs state that it only makes the search honest.

## 10. Stop conditions

- A test failure that cannot be fixed without weakening it.
- A missing shared input or an invalid manifest.
- A need to change the model, strategy, execution, portfolio, policy, or
  analytics contract.
- A request to run a real campaign, open validation, or accept a baseline.
- A request to commit, push, or merge outside the current authorization.
