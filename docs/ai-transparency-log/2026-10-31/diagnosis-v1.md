# AI Transparency Log: Diagnosis v1

- Date: 2026-10-31 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash)
- Work item: `diagnosis-v1`
- Plan: `docs/execution-plans/2026-10-31/diagnosis-v1.md`
- Contract: `docs/reports/2026-10-31/diagnosis-v1.md`
- Decision: `docs/adr/2026-10-31/diagnosis-artifact.md`

## Intent and boundary

The human asked for a Phase 0 and Phase 1 implementation: make the existing report
images readable, make `lab summary` honest about out-of-sample and rerun evidence and
refuse a stale lab, and add a cross-engine diagnosis artifact with a scorecard, funnel,
data quality, execution friction, and portfolio attribution. The human approved the
approach with "your understanding and solution is perfect now implement it as planned".

The AI did not change model, strategy, execution, or portfolio behavior. It did not
add a dependency, change an existing command's meaning, or change the five per-engine
bundle contracts. New behavior arrived as new commands, new artifact files, and
additional checks. No diagnosis value is read by the signal path.

## Implemented

- `quantrule-analytics::diagnosis`: plain input facts, check/funnel/data/model/strategy/
  execution/portfolio sections, enriched trades with MFE/MAE, deterministic text and
  CSV renderers.
- Shell projection from the completed `BacktestRun` and `RunConfig`, `diagnosis.json`,
  manifest, and five deterministic pages.
- `backtest --diagnose`, `lab diagnose`, and `lab list` parsing, dispatch, help, and
  README references.
- `lab summary`: supplied-config hash and model-version guards; OOS checks from `oos/`;
  rerun checks from `rerun/`; diagnosis presence in the artifact list.
- `lab diagnose`: config-hash, data-version, model-version, and report-version checks;
  the rerun must reproduce the portfolio's key result.
- Phase 0 charts: distinct entry/exit triangles with a legend; equity benchmark,
  drawdown limit, and halt annotations; progression stage ribbon; per-bucket stacked
  event-density strip replacing the overplotted event markers.

## Verification and evidence

Commands and observed results:

| Command | Observed result |
|---|---|
| `cargo clippy -q --all-targets -- -D warnings` | No output (no warnings) |
| `cargo test -q` | All test binaries passed; shell integration, market-report goldens, and lab-summary goldens included |
| `cargo run -q -p quantrule-shell -- backtest --config config/examples/solusd-2025/run.json --out /tmp/opencode/diagnosis-v3 --diagnose` | Five bundle paths plus `/tmp/opencode/diagnosis-v3/diagnosis` |
| `diff -rq /tmp/opencode/diagnosis-final/diagnosis /tmp/opencode/diagnosis-final-rerun` | No differences (exit 0) after the final code changes |
| `diff -rq /tmp/opencode/diagnosis-v3/diagnosis /tmp/opencode/diagnosis-v2-rerun` | No differences (exit 0) after regenerating both from the same code |
| `cargo run -q -p quantrule-shell -- lab diagnose /tmp/opencode/diagnosis-v2 --config config/examples/solusd-2025/run.json --out /tmp/opencode/diagnosis-v2-rerun` | `wrote /tmp/opencode/diagnosis-v2-rerun` |
| `cargo run -q -p quantrule-shell -- lab list --root /tmp/opencode` | Deterministic table; `diagnosis-v3` shows `diagnosis=yes` |
| `lab summary` on `/tmp/opencode/diagnosis-v3` | Wrote `summary.txt`; artifact list shows `diagnosis/diagnosis.json present` |

The current example run's diagnosis reports 298 entry attempts, 100 entry fills, 198
post-halt `missing_quantity` rejections, 100 trades, gross P&L `-168.65`, commission
`1822.74`, net P&L `-1989.59`, maximum drawdown `20.0542%` against a `20.00%` limit,
and a halt at `2025-05-03T18:00:00Z`. It reports 5 passes, 8 failures, and 4 unknown
checks. These are observed values from the recorded run, not a profitability claim.

The market-report golden PNGs and the lab-summary golden text were regenerated because
the chart layout and the summary checks changed. The market-report text and JSON did
not change.

## Learning, rejected or corrected suggestions

- The first diagnosis pages clipped right-column text and overlaid bars on labels. The
  AI inspected the rendered PNGs, then moved labels into fixed columns, shortened lines,
  and rechecked the images. This was a rendering correction, not a data change.
- The first `lab summary` guard compared `--data-version` with the config's data version.
  The synthetic lab fixture uses `test:v1` with the example config, so that comparison
  rejected a valid fixture. The guard now compares the bundle data version with the
  supplied `--data-version` and compares the config hash and model version separately.
- The first `lab summary` regression test inserted a duplicate `config_hash` key. After
  the fixture gained a real hash, the duplicate was ignored by the JSON parser and the
  mismatch test passed for the wrong reason. The helper now replaces the value.
- `lab list` initially used an 18-character data-version column and overflowed. The
  column is now 54 characters.
- A `--strict` diagnosis writes its evidence before returning the integrity failure.
  This is deliberate: the evidence explains why the gate failed. The command still exits
  non-zero.

## Limitations

- One instrument and one year, in-sample only. The diagnosis does not call the result
  promising and cannot establish an edge.
- The OOS and rerun checks depend on the `oos/` and `rerun/` directory conventions; a
  lab without them reports `unknown`, not `pass`.
- The `model.progression_null` check reports the worst timeframe. On the current run the
  worst is 1m (88.2%), while 5m is 82.5%.
- The plan and implementation were not committed or opened as a pull request during
  this entry. The human has not authorized those Git operations.
