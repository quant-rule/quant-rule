# AI transparency log — DIST-101 simulation meta port and PORT-001

- Date: 2026-11-14 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Intent: add the missing meta-label and context-split stage to
  `quantrule-simulation`, capture a golden fixture from the retired Python
  simulation harness, register the first port-parity entry, and add
  determinism and edge-case tests.
- Boundary: `crates/quantrule-simulation/**`, `docs/port-parity/**`,
  `CHANGELOG.md`, and this log. The Python harness was read and executed for
  capture only; no Python file was edited or deleted. No model, strategy,
  execution, or portfolio behavior changed. No dependency was added.

## Implementation

- Added `crates/quantrule-simulation/src/meta.rs`: nearest-bar mapping, the
  forward path in ATR, the EMA/ATR context cells, the best-cell rule, the
  unmarked event-study check, and the pure `build` result. The check reuses
  `crate::check::check` for the threshold verdict.
- Added `crates/quantrule-simulation/tests/port_parity.rs`: reads
  `docs/port-parity/PORT-001.golden.json`, dispatches on the case stage, and
  compares every value, including every number to the fixture tolerance.
- Added `docs/port-parity/PORT-001.golden.json` (17 cases) and the `PORT-001`
  registry entry. Added the target and test markers.
- Made `check::mean` sum in value order rather than input order so a reordered
  set cannot change the result.
- Added unit tests for empty windows, a single bar, an unmarked check set,
  reordered marks, reordered check values, and empty review sets.

## Fixture capture

The capture tool lived outside the repository at
`/tmp/opencode/capture_port_001.py`; it was not committed because the product
carries no Python. It was run from the repository root:

```bash
PYTHONPATH=tools/simulation python3 /tmp/opencode/capture_port_001.py
```

It reads `tools/simulation/fixtures/mini/` and calls the Python reference
functions directly: `meta.meta_rows`, `meta.cell_table`, `meta.best_cell`,
`meta._check`, `check.event_rows`, `check.bar_rows`, `check._stats`,
`check._non_overlapping`, `check._passes_direction`, `check._passes_filter`,
`exit_lab.simulate`, `window_select.select_windows`, `marks.validate_marks`,
`oracle.oracle_for_window`, and `marks.count_by_quality`.

## Verification

| Command | Observed |
|---|---|
| `cargo test -p quantrule-simulation` | 75 unit and 5 integration tests passed |
| `cargo test -p quantrule-simulation --test port_parity` | 5 passed |
| `tools/test_tiers.sh parity` | `port parity ok: 1 registered port(s)` |
| `python3 tools/check_port_parity.py` | `port parity ok: 1 registered port(s)` |
| `cargo clippy --all-targets -p quantrule-simulation -- -D warnings` | clean |
| `RUSTDOCFLAGS="-D warnings" cargo doc -p quantrule-simulation --no-deps` | clean |
| `cargo fmt --check` | clean |
| `tools/gate.sh` | passed |

## Learning

- The existing Rust stages are a deliberate redesign, not a line-by-line port of
  the Python harness. The window selector is equal-span PCG, not the Python
  year-stratified event selector; the oracle ranks forward results rather than
  ATR target/stop paths; the review compares two mark sets rather than writing
  the Python tables; and the exit lab has no swing policy. Their outputs cannot
  equal the Python outputs in general.
- The fixture therefore compares the reductions that are genuinely shared on the
  checked-in mini bundle and names every omission in the case notes. The meta
  stage is a full port and is compared end to end. The historical SIM-01..SIM-10
  runs cannot be replayed because their external data root is not in the
  repository.
- The check reduction is order-sensitive only through its mean. Summing in value
  order removes that dependence without changing the threshold arithmetic.

## Rejected or corrected

- Rejected: inventing historical SIM outputs or claiming a full replay. The
  fixture uses the checked-in mini bundle and documents the case set.
- Rejected: editing `tools/simulation/**` to make the Python output fit the Rust
  API. The harness is the reference and is retired later by #683.
- Rejected: adding a crate or a Python file to the product. The capture tool is
  external and the fixture is checked-in JSON.
- Corrected: an initial meta `build` filtered rows by timestamp and bar identity;
  it now carries the parsed moment on the private row, so duplicate timestamps
  cannot confuse the label window.
- Not done: moving #679 to `Done` or merging. The issue's full SIM-01..SIM-10
  parity claim remains unmet for the reasons above.
