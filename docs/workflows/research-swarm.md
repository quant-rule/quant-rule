# Research Swarm Runbook

- Date: 2026-11-05 (UTC)
- Updated: 2026-11-07 to make the runbook tier-agnostic
- Status: Current; no active campaign
- Contract: [`docs/governance/2026-11-04/research-mandate.md`](../governance/2026-11-04/research-mandate.md)
- M2.x programme: [`docs/execution-plans/2026-11-05/410-m2-model-programme.md`](../execution-plans/2026-11-05/410-m2-model-programme.md)
- Scope: concurrent research programs executed through the Improvement Workflow

## 1. Purpose

Run several pre-registered hypotheses concurrently without turning the search
into an automatic optimizer. A swarm explores; the human promotes; validation is
one-shot.

This runbook is the execution mechanism for any tier. The model-specific
exploration and confirmation modes are in
[`model-research.md`](model-research.md); the strategy and portfolio runbooks are
[`strategy-research.md`](strategy-research.md) and
[`portfolio-research.md`](portfolio-research.md). The cell, worktree,
concurrency, and aggregation rules below apply to every tier.

A **program** is a premise plus a finite set of cells. A **cell** is one
hypothesis, one config, one worktree, and one result.

## 2. Roles

| Role | Responsibility |
|---|---|
| Human | Approves the charter, the cells, validation access, and the baseline |
| Coordinator | Creates the program, worktrees, and cell briefs; spawns cells; aggregates; stops |
| Cell agent | Runs one cell in its worktree, writes one result, commits, reports |
| Quant Validator | Independently reproduces a candidate before human acceptance |

The coordinator does not tune, vote, or promote. A cell agent does not read
another cell's result or change its config after seeing a result.

## 3. Model programme modes

The model programme modes are the worked example of the tier-specific rules in
[`model-research.md`](model-research.md). A model program has two modes. The mode
is declared in the cell and in the program report; it is not inferred from a
result.

| Mode | Cells | Partition | Evidence | Ranked | Validation |
|---|---|---|---|---|---|
| Exploratory model | Ten area cells, one per information area | `model_dev` only | Coverage and hypothesis generation | No | Never |
| Confirmatory model | Five primary M2.x cells | `model_dev`, then one-shot `model_validation` | Corrected primary evidence | Yes | Only after human promotion |

### 3.1 Exploratory model mode

The exploratory mode exists so that every registry event is tried at least once
without turning the search into an optimizer.

- There is one cell per information area: volume-weighted price, volume by
  price, participation, volume-price flow, volatility structure, structure,
  candle anatomy and gaps, path and duration, cross-asset, and calendar/session.
- Each cell activates the events of its area and publishes a directional vote.
  The cell is labelled `exploratory` in `cell-result.json` and in the program
  report.
- The ten area cells are **not ranked** and receive **no multiple-testing
  credit**. A positive area cell is a hypothesis for a primary cell, not a
  candidate.
- The exploratory mode never opens `model_validation`, never changes a primary
  spec, and never feeds a strategy.
- A coverage checker reads the ten specs and the registry and fails the program
  if any registry event ID is absent from every exploratory prediction. Shared
  event IDs are counted once. It also checks all 46 indicator IDs: an indicator
  with no event (for example `calendar_bucket`) is exercised as a reading or a
  condition in its area cell.
- Diagnostic events may appear in exploratory cells. A primary cell that will
  feed a strategy uses only `signal` and `context` events.
- The human freezes the ten area specs and the five primary specs before the
  first run. A cell agent may not add, remove, or retune an event after seeing a
  result. A failed exploratory cell is retired; it is not retried with a
  variant.
- Exploratory results are reported in full, including failures and zero-event
  counts. They are evidence about coverage and data quality, not a performance
  claim.

### 3.2 Confirmatory model mode

The confirmatory mode is the only mode that can produce an accepted M2.x
baseline.

- The five primary cells are the only confirmatory cells. The correction is
  Bonferroni over five.
- A spec is frozen by ID, configuration hash, and code commit before the first
  development run.
- The human reviews every cell and promotes at most the declared primary cells.
- The promoted cells run `model_validation` once. The validation access is
  recorded in the registry.
- A validation cell that fails is not retried with a post-hoc variant.

The event-to-area mapping and the exact cell specs are fixed in the M2.x charter
before the first run. The charter is the only place where the finite cell list
can change, and only before a run.

## 4. Program lifecycle

1. The human approves a charter using
   [`program-charter-template.md`](../governance/2026-11-04/program-charter-template.md).
2. The coordinator writes `config/research/<PROGRAM-ID>/program.json` and the
   cell configs.
3. The coordinator validates the manifest:
   ```bash
   python3 tools/research/program.py \
     --program config/research/<PROGRAM-ID>/program.json --check
   ```
4. The coordinator prepares one worktree per code-bearing cell, or uses config
   mode for config-only cells.
5. The coordinator spawns one cell agent per worktree.
6. Each cell agent runs the dev phase, writes `cell-result.json`, commits on its
   branch, and reports.
7. The coordinator aggregates with `tools/research/report.py` and reports every
   cell.
8. The human reviews all cells and promotes at most the declared number of
   primary cells to validation.
9. Promoted cells run the validation phase once. The validation access is
   recorded in the registry.
10. The human accepts or rejects; the registry is updated; the worktrees and
    branches are cleaned.

## 5. Concurrency

- `max_parallel` is declared in the program manifest, default 5, maximum 64.
- Config-only cells run through the batch runner with isolated output
  directories.
- Code-bearing cells get one worktree and one branch each.
- Shared inputs are read-only: model pack, source CSVs, engine configs, and
  measurement tools.
- Each cell writes only its own `data/research/<PROGRAM-ID>/<CELL-ID>/`.

## 6. Commands

```bash
# Config-only dev batch.
python3 tools/research/batch.py \
  --program config/research/<PROGRAM-ID>/program.json \
  --phase dev \
  --mode config \
  --max-parallel 5

# Prepare worktrees for code-bearing cells.
python3 tools/research/batch.py \
  --program config/research/<PROGRAM-ID>/program.json \
  --phase dev \
  --mode worktree \
  --worktree-root /home/zantgo/Proyectos/quant-rule--<PROGRAM-ID> \
  --prepare-only

# Coverage check: every indicator and event is in a frozen cell.
python3 tools/research/coverage.py \
  --program config/research/<PROGRAM-ID>/program.json

# Aggregate.
python3 tools/research/report.py \
  --program config/research/<PROGRAM-ID>/program.json \
  --results data/research/<PROGRAM-ID>/dev \
  --out data/research/<PROGRAM-ID>/program-results

# Registry record.
python3 tools/research/registry.py \
  --program config/research/<PROGRAM-ID>/program.json \
  --report data/research/<PROGRAM-ID>/program-results/program-results.json
```

A pre-registered M2.x cell runs each instrument spec over the phase's yearly
folds. The folds keep every run inside the 600,000 M1-candle input cap. The
runner merges the yearly prediction and readings bundles into one continuous
per-instrument bundle, then runs one pooled prediction evaluation. A cell with
`"result_parser": "model_prediction"` reads those metrics into
`cell-result.json`; the evaluator never opens validation.

```bash
python3 tools/research/model_cell.py \
  --cell-dir config/research/<PROGRAM-ID>/cells/<CELL-ID> \
  --phase dev \
  --out data/research/<PROGRAM-ID>/dev/<CELL-ID>
```

The runner uses `target/release/quantrule` by default; pass `--qr` to select the
binary. A model program uses `batch.py --mode config`; it never needs a code
worktree unless a cell changes code.

The worktree mode copies each cell result back to the coordinator's
`data/research/<PROGRAM-ID>/<phase>/<CELL-ID>/` for aggregation. The worktree
remains for code review and is removed after the program closes.

## 7. Cell boundaries

- One hypothesis, one config, one worktree, one result.
- No cell edits a shared file.
- No cell reads another cell's result before recording its own.
- No cell opens validation; only the human promotes.
- No cell changes the event, timeframe, target, stop, horizon, filter, or cost
  after seeing a result.
- A failed cell is retired. It is not retried with a variant in the same
  program.

## 8. Human gates

| Gate | Human action |
|---|---|
| Charter | Approve the premise, cells, correction, budget, and stop rule |
| Development results | Review every cell; promote at most the declared number |
| Validation access | Approve the one-shot access and the promoted cells |
| Candidate | Require an independent Quant Validator record; accept or reject |
| Closeout | Record `no_edge`, `development_only`, `candidate`, or `accepted` |

A charter approval does not authorize commits, pushes, PRs, merges, or
deployments. Those have their own gates.

## 9. Stop conditions

- No primary cell passes the development gate.
- A validation cell fails.
- The iteration cap is reached.
- A shared-file conflict, missing prerequisite, or invalid manifest blocks a
  cell.
- The human stops the program.

A stopped program does not open the next tier. A new program requires a new
premise and a new approved charter.

## 10. Closeout

- Preserve every `cell-result.json` and the program report.
- Write the registry record.
- Report `no_edge`, `development_only`, `candidate`, or `accepted`.
- Clean worktrees and branches only after the evidence is preserved.
- Do not re-register a failed hypothesis as a variant.

## 11. Related documents

- [`model-research.md`](model-research.md)
- [`strategy-research.md`](strategy-research.md)
- [`portfolio-research.md`](portfolio-research.md)
- [`improvement-workflow.md`](improvement-workflow.md)
- [`run-contract.md`](run-contract.md)
- [`../governance/2026-11-04/research-mandate.md`](../governance/2026-11-04/research-mandate.md)
- [`../governance/2026-11-04/program-charter-template.md`](../governance/2026-11-04/program-charter-template.md)
- [`../../AGENTS.md`](../../AGENTS.md)
