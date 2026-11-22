# Research Programs

- Status: M2.x–M4.x historical evidence; MR5.x current research platform
  Historical programs carry `"retired": true`; `research batch` and `research cell` refuse them, while `research program --check`, `research report`, and `research registry` still work. Their Python command arrays are audit evidence, not current runners.

The M2.x configs predate the flat-model cutover. Their `runs/**/run.json` files
reference retired sibling files, so they are kept for reproducibility of the
M2.x evidence, not as current run configs. The M3.x configs are retained for the
underpowered preflight. The M4.x programme (`PROGRAM-2026-11-07-01`) is the last
shared-model programme. The MR5.x platform (`reference-mr5` and the planned
`PROGRAM-2026-11-08-01`) uses per-cell model configs, conditions, and the current
sibling schemas. The current example runs are under `config/examples/`.

A research program is a premise plus a finite set of pre-registered cells. The
manifest is data; the runner and engine do the work. A program is not a
parameter sweep and does not select a winner automatically. An MR5 cell also
carries `model.json` and `condition.json`; the model and condition variants are
pre-registered, not tuned after a result.

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
  runs/
    shared/
    <INSTRUMENT>/
      policy.json
      <YEAR>/
        data.json
        run.json
```

A model cell uses one-year run configs so each run stays inside the 600,000 M1
candle input cap. The runner merges the yearly bundles per instrument.

## Commands

```bash
# Validate the manifest and the declared phase.
qr-governance research program --program config/research/<PROGRAM-ID>/program.json

# Run one model cell (four instruments + one pooled evaluation).
qr-governance improvement model-cell \
  --cell-dir config/research/<PROGRAM-ID>/cells/<CELL-ID> \
  --phase dev \
  --out data/research/<PROGRAM-ID>/dev/<CELL-ID>

# The Python coverage, grid-builder, and model-cell tools were retired with the
# historical research stack. The committed configs remain the audit record.

# Run one cell.
qr-governance research cell \
  --program config/research/<PROGRAM-ID>/program.json \
  --cell CELL-01 \
  --phase dev \
  --out data/research/<PROGRAM-ID>/dev/CELL-01

# Run the cells concurrently in config mode.
qr-governance research batch \
  --program config/research/<PROGRAM-ID>/program.json \
  --phase dev \
  --mode config \
  --max-parallel 5

# Aggregate every cell and apply the declared correction.
qr-governance research report \
  --program config/research/<PROGRAM-ID>/program.json \
  --results data/research/<PROGRAM-ID>/dev \
  --out data/research/<PROGRAM-ID>/program-results

# Write the program registry record.
qr-governance research registry \
  --program config/research/<PROGRAM-ID>/program.json \
  --report data/research/<PROGRAM-ID>/program-results/program-results.json
```

## Reference dry run

`config/research/reference/` contains three fixture cells. It is a platform
test, not a market hypothesis. It uses `validation_access: none`; no validation
is opened. The expected report is `no_edge`.

```bash
qr-governance research batch \
  --program config/research/reference/program.json \
  --phase dev \
  --mode config \
  --max-parallel 3 \
  --out /tmp/opencode/research-reference

qr-governance research report \
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
