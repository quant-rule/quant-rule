# Experiment Matrix Runner

`run-matrix.sh` runs one `backtest` per manifest cell, in manifest order, with an
isolated output root. It is deterministic: it reads no clock, uses no randomness,
and does not reorder the cells.

## Manifest

```text
# cell       run config
eurusd-m15   /path/to/eurusd-m15/run.json
eurusd-h1    /path/to/eurusd-h1/run.json
```

Cell names use letters, digits, `.`, `-`, and `_`. The configuration must be a
valid `run v2` file. Each cell gets:

```text
<out-root>/<cell>/model/
<out-root>/<cell>/strategy/
<out-root>/<cell>/execution/
<out-root>/<cell>/portfolio/
<out-root>/<cell>/analytical/
<out-root>/<cell>/backtest.json
<out-root>/manifest.txt
```

The output root must be absent or empty. A failure stops the run; use a fresh
root for a new matrix. The script does not create an experiment record; the
Governing Swarm writes that from the results and the lab summaries.

## Example

```bash
QR=./target/release/quantrule tools/experiment/run-matrix.sh matrix.txt data/lab-matrix-2026-10-30
```
