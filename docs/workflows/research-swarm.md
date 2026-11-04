# Research Swarm Runbook

- Date: 2026-11-04 (UTC)
- Status: Current
- Contract: [`docs/governance/2026-11-04/research-mandate.md`](../governance/2026-11-04/research-mandate.md)
- Scope: concurrent research programs executed through the Improvement Workflow

## 1. Purpose

Run several pre-registered hypotheses concurrently without turning the search
into an automatic optimizer. A swarm explores; the human promotes; validation is
one-shot.

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

## 3. Program lifecycle

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

## 4. Concurrency

- `max_parallel` is declared in the program manifest, default 5, maximum 64.
- Config-only cells run through the batch runner with isolated output
  directories.
- Code-bearing cells get one worktree and one branch each.
- Shared inputs are read-only: model pack, source CSVs, engine configs, and
  measurement tools.
- Each cell writes only its own `data/research/<PROGRAM-ID>/<CELL-ID>/`.

## 5. Commands

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

The worktree mode copies each cell result back to the coordinator's
`data/research/<PROGRAM-ID>/<phase>/<CELL-ID>/` for aggregation. The worktree
remains for code review and is removed after the program closes.

## 6. Cell boundaries

- One hypothesis, one config, one worktree, one result.
- No cell edits a shared file.
- No cell reads another cell's result before recording its own.
- No cell opens validation; only the human promotes.
- No cell changes the event, timeframe, target, stop, horizon, filter, or cost
  after seeing a result.
- A failed cell is retired. It is not retried with a variant in the same
  program.

## 7. Human gates

| Gate | Human action |
|---|---|
| Charter | Approve the premise, cells, correction, budget, and stop rule |
| Development results | Review every cell; promote at most the declared number |
| Validation access | Approve the one-shot access and the promoted cells |
| Candidate | Require an independent Quant Validator record; accept or reject |
| Closeout | Record `no_edge`, `development_only`, `candidate`, or `accepted` |

A charter approval does not authorize commits, pushes, PRs, merges, or
deployments. Those have their own gates.

## 8. Stop conditions

- No primary cell passes the development gate.
- A validation cell fails.
- The iteration cap is reached.
- A shared-file conflict, missing prerequisite, or invalid manifest blocks a
  cell.
- The human stops the program.

A stopped program does not open the next tier. A new program requires a new
premise and a new approved charter.

## 9. Closeout

- Preserve every `cell-result.json` and the program report.
- Write the registry record.
- Report `no_edge`, `development_only`, `candidate`, or `accepted`.
- Clean worktrees and branches only after the evidence is preserved.
- Do not re-register a failed hypothesis as a variant.
