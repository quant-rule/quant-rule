# Research Programs

A research program is a premise plus a finite set of pre-registered cells. The
manifest is data; the runner and engine do the work. A program is not a
parameter sweep and does not select a winner automatically.

The contract is
[`docs/governance/2026-11-04/research-mandate.md`](../../docs/governance/2026-11-04/research-mandate.md)
and the runbook is
[`docs/workflows/research-swarm.md`](../../docs/workflows/research-swarm.md).

## Layout

```text
config/research/<PROGRAM-ID>/
  program.json
  cells/
    CELL-01/
    CELL-02/
```

## Commands

```bash
# Validate the manifest and the declared phase.
python3 tools/research/program.py --program config/research/<PROGRAM-ID>/program.json

# Check the event and indicator coverage of the matrix.
python3 tools/research/coverage.py \
  --program config/research/<PROGRAM-ID>/program.json

# Run one cell.
python3 tools/research/cell.py \
  --program config/research/<PROGRAM-ID>/program.json \
  --cell CELL-01 \
  --phase dev \
  --out data/research/<PROGRAM-ID>/dev/CELL-01

# Run the cells concurrently in config mode.
python3 tools/research/batch.py \
  --program config/research/<PROGRAM-ID>/program.json \
  --phase dev \
  --mode config \
  --max-parallel 5

# Aggregate every cell and apply the declared correction.
python3 tools/research/report.py \
  --program config/research/<PROGRAM-ID>/program.json \
  --results data/research/<PROGRAM-ID>/dev \
  --out data/research/<PROGRAM-ID>/program-results

# Write the program registry record.
python3 tools/research/registry.py \
  --program config/research/<PROGRAM-ID>/program.json \
  --report data/research/<PROGRAM-ID>/program-results/program-results.json
```

## Reference dry run

`config/research/reference/` contains three fixture cells. It is a platform
test, not a market hypothesis. It uses `validation_access: none`; no validation
is opened. The expected report is `no_edge`.

```bash
python3 tools/research/batch.py \
  --program config/research/reference/program.json \
  --phase dev \
  --mode config \
  --max-parallel 3 \
  --out /tmp/opencode/research-reference

python3 tools/research/report.py \
  --program config/research/reference/program.json \
  --results /tmp/opencode/research-reference \
  --out /tmp/opencode/research-reference/results
```

## Rules

- Every cell is pre-registered before the first run.
- Every cell writes one immutable `cell-result.json`.
- The report includes every cell, including failures.
- The report recommends a candidate; it never opens validation.
- Validation is one-shot per hypothesis and requires human promotion.
- A failed cell is retired, not retried with a different target, stop, horizon,
  filter, or timeframe.
