# AI Transparency Log — Research Program Platform

- Date: 2026-11-04 (UTC)
- Tool: OpenCode agent session (DeepSeek V4.1 Flash)
- Work item: concurrent research program platform
- Epic: [#385](https://github.com/quant-rule/quant-rule/issues/385)
- Plan: [`docs/execution-plans/2026-11-04/385-research-program-platform.md`](../../execution-plans/2026-11-04/385-research-program-platform.md)

## Intent and boundary

The human approved the full implementation of the research program platform.
The agent was asked to build a way to run many pre-registered research cells
concurrently without an automatic search.

The agent implemented:

- a program manifest and validator;
- a cell runner and result schema;
- result adapters for path validation, strategy evaluation, and summary JSON;
- a bounded parallel batch runner in config and worktree modes;
- an aggregator that reports every cell and applies the declared correction;
- a registry that records a program but never accepts a baseline;
- tests, a three-cell fixture dry run, the swarm runbook, and prompt templates.

The agent did not run a real market hypothesis, open validation, change a model
or strategy, or accept a baseline.

## Proposed work versus implemented work

| Item | Status |
|---|---|
| Program manifest and validator | Implemented |
| Cell runner and adapters | Implemented |
| Parallel batch runner | Implemented |
| Aggregator and registry | Implemented |
| Tests | Implemented and passing |
| Reference dry run | Implemented and passing |
| Runbook and prompts | Written |
| Verification | See below |
| PR and merge | Pending at the time of writing |

## Verification and evidence

Observed on 2026-11-04 in the platform worktree:

| Command | Result |
|---|---|
| `cd tools/research && python3 -m unittest test_program test_cell test_batch test_report test_registry` | 23 tests pass |
| `python3 tools/check_terms.py` | no banned terms |
| `python3 tools/research/program.py --program config/research/reference/program.json --check` | `ok PROGRAM-2026-11-04-99 cells=3 primary=3 phase=dev` |
| `python3 tools/research/batch.py ... --mode config --max-parallel 3` | 3 cells; all fail the fixture |
| `python3 tools/research/report.py ...` | decision `no_edge`; correction 3 primary cells |
| `python3 tools/research/registry.py ...` | status `no_edge` |

The reference dry run is a fixture, not a market claim. The three fixture cells
deliberately fail; the point is that the platform runs three cells concurrently,
reports every cell, and opens no validation.

## Learning and rejected suggestions

- An automatic optimizer was rejected: the platform reports candidates; the
  human promotes; validation is one-shot.
- A single shared output directory was rejected: each cell writes its own
  output, and code-bearing cells get their own worktree.
- A report that re-grades the gate was rejected: the adapter records the gate's
  metrics and verdict; the report applies the declared correction and reports
  every cell.
- The registry was made non-accepting: an accepted row requires explicit human
  fields, and the candidate still needs an independent Quant Validator.
- The reference dry run uses fixtures, not the model pack, so it consumes no
  validation data and makes no market claim.
