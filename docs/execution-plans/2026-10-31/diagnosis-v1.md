# Execution Plan: Cross-Engine Diagnosis v1 (Phase 0 + Phase 1)

- Date: 2026-10-31 (UTC)
- Work item: `diagnosis-v1` (no GitHub issue number yet; descriptive slug per section 7)
- Status: Implemented; awaiting human acceptance
- Approval: The human instruction "now implement it as planned" on 2026-10-31
- Branch: `feat/diagnosis-v1`
- Worktree: `../quant-rule--diagnosis-v1`
- Plan / AI log base name: `diagnosis-v1`

## 1. Intent

A run currently writes five per-engine bundles and six PNGs. They prove that a run
happened and that it lost money, but they do not let a human or an agent decide
which engine caused the result or what to test next. The images are also hard to
read: markers merge into candles, events overplot, progression is clipped, and
the equity curve has no benchmark or halt annotation.

This work adds a read-only cross-engine diagnosis layer. It answers, in order:

```text
run identity -> data integrity -> model health -> strategy funnel
-> execution friction -> portfolio attribution -> evidence quality
```

It produces a separate `diagnosis/` artifact with a scorecard, a funnel, a money
flow, per-engine tables, and machine-readable checks. It never feeds the signal
path.

## 2. Current state

- `backtest` writes `model/`, `strategy/`, `execution/`, `portfolio/`,
  `analytical/`; the five bundles are pure per-engine outputs.
- `lab summary` copies published values, but `out-of-sample evidence` and
  `independent rerun` are hardcoded `[ ]`, and the supplied config is not
  compared with the bundle `config_hash`.
- `chart/mod.rs` draws entry/exit markers as identical 7x7 squares at the close;
  `chart/report.rs` draws every event at its bucket close and clips progression
  to `-1..+2` without showing the stage code.
- `lab summary` cannot distinguish a stale 15m lab from the current 5m run.
- No command exposes the strategy funnel, cost attribution, or enriched trades.
- The current example run (`config/examples/solusd-2025/run.json`) has 298 entry
  attempts, 100 fills, 198 post-halt `missing_quantity` rejections, 100 trades,
  net `-1989.59`, commission `1822.74`, and a halt at `20.05%`.

## 3. Scope

Included:

- Phase 0: readable PNGs, honest `lab summary` checks, stale-lab guard, golden
  and readability tests.
- Phase 1: `diagnosis` contract and artifact, `lab diagnose`, `lab list`,
  `backtest --diagnose`, five core pages, `trades_enriched.csv`.
- Docs: diagnosis contract, command reference, README, ADR, plan, AI log.

Explicit non-goals:

- No model, strategy, execution, or portfolio behavior change.
- No change to the five bundle boundaries; no P&L in model or analytical.
- No composite health score; checks are per-check and counted by status.
- No `lab compare`, no parameter sensitivity, no out-of-sample split generator
  (Phase 2/3).
- No new external dependency; reuse the pure canvas and PNG encoder.
- No diagnosis value may be read by the strategy; a boundary test proves it.

## 4. Assumptions and uncertainty

- The diagnosis can reconstruct the run from the supplied config, so `lab
  diagnose` re-runs the pipeline and compares key results with the lab. If the
  rerun differs, the lab is not interpretable.
- The five bundles already carry `config_hash`, `model_version`, and
  `data_version`; the diagnosis binds to all three.
- The existing canvas primitives are sufficient for the five pages. If a page
  needs a primitive that cannot be expressed, the page is simplified rather than
  adding a dependency.
- Thresholds in the checks are reporting defaults, recorded in the contract;
  they are not tuned to make the current run look good.

## 5. Decisions

| Decision | Choice | Consequence |
|---|---|---|
| Artifact location | `<lab>/diagnosis/` | Keeps the five bundles pure |
| Computation home | `quantrule-analytics` over a plain `DiagnosisInput` projection | Preserves analytics -> model direction; shell only projects and renders |
| Run reconstruction | `lab diagnose` re-runs the config; `backtest --diagnose` reuses the run | Reproducibility check without a second pipeline |
| Staleness | Identity mismatch is an error; `--strict` also fails integrity checks | A stale lab is never interpreted silently |
| Checks | `pass` / `fail` / `unknown`, no score | No false precision |
| Charts | 1280-wide deterministic PNGs, embedded font | Same bytes on every run |
| Ordinal axes | Progression stage ribbon; codes counted | Follows the ordinal-code rule |
| Events | Per-bucket stacked counts | No last-marker-wins overplot |

## 6. Acceptance criteria

- `cargo fmt --check`, `cargo clippy --all-targets`, and `cargo test` pass.
- `backtest --config config/examples/solusd-2025/run.json --out <dir> --diagnose`
  writes the five bundles plus `diagnosis/` with `summary.txt`,
  `diagnosis.json`, five PNGs, and the tables.
- `lab diagnose <lab> --config <run.json>` re-runs, verifies identity, and writes
  the same diagnosis; a stale config hash is refused.
- `lab list --root data` prints a deterministic table; `--json` prints an array.
- The scorecard shows attempts -> fills -> trades and the cost waterfall.
- The data page shows per-timeframe coverage and the longest gaps.
- The funnel page shows entry attempts vs fills over time and exit-reason P&L.
- The execution page shows order states, rejection reasons, and slippage.
- The attribution page shows equity vs buy-and-hold, drawdown vs limit, halt,
  cost waterfall, and monthly returns.
- The six existing PNGs have legends, distinct marker shapes, a benchmark, a
  drawdown limit, a halt annotation, an event density strip, and a progression
  ribbon.
- `lab summary` computes OOS/rerun checks and refuses a config-hash or
  model-version mismatch.
- A boundary test proves the strategy input does not expose diagnosis fields.
- The diagnosis is byte-identical across two runs over the same input.

## 7. Work breakdown

| Step | Work | Role | Paths |
|---|---|---|---|
| 1 | Write this plan and the diagnosis contract | Orchestrator / Architect | `docs/execution-plans/2026-10-31/`, `docs/reports/2026-10-31/`, `docs/adr/2026-10-31/` |
| 2 | Add diagnosis data model, checks, tables, JSON/text | Implementation / Quant Developer (reporting only) | `crates/quantrule-analytics/src/diagnosis/` |
| 3 | Project `BacktestRun` into `DiagnosisInput`; add command | Implementation | `crates/quantrule-shell/src/command/lab_diagnose.rs`, `backtest.rs` |
| 4 | Render the five diagnosis pages | Implementation | `crates/quantrule-shell/src/chart/diagnosis/` |
| 5 | Fix the six existing PNGs | Implementation | `crates/quantrule-shell/src/chart/{mod,report}.rs` |
| 6 | Fix `lab summary`; add `lab list` | Implementation | `crates/quantrule-shell/src/{lab_summary.rs,command/lab.rs}` |
| 7 | Tests: diagnosis, rendering, stale lab, boundary, determinism | Test Engineer | crate `tests/` and module tests |
| 8 | Run full verification, record evidence, update docs | Orchestrator / Validator | plan, AI log, README |

No quant hypothesis or signal changes are in scope; the Quant Validator record
is not required. The diagnosis itself is evaluation evidence, and the existing
boundary test is extended.

## 8. Verification

Commands:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo run -q -p quantrule-shell -- backtest \
  --config config/examples/solusd-2025/run.json \
  --out /tmp/opencode/diagnosis-v1 --diagnose
cargo run -q -p quantrule-shell -- lab list --root /tmp/opencode/diagnosis-v1
cargo run -q -p quantrule-shell -- lab diagnose /tmp/opencode/diagnosis-v1 \
  --config config/examples/solusd-2025/run.json --out /tmp/opencode/diagnosis-v1-rerun
```

Expected evidence:

- The current run's diagnosis reports 298 attempts, 100 fills, 198 post-halt
  rejections, commission `1822.74`, gross `-168.65`, net `-1989.59`, halt
  `20.05%`, and `context_lost` as the largest exit-reason loss.
- Two runs produce identical `summary.txt`, `diagnosis.json`, and PNG bytes.
- A lab with a different `config_hash` is refused.

Limitations: one instrument, one year, in-sample only. The diagnosis reports
`unknown` for OOS evidence and does not claim an edge.

## 9. Risks and recovery

- Re-running a full year is expensive; `backtest --diagnose` avoids the second
  run. `lab diagnose` is explicit about the cost.
- A new PNG page can grow the file; split `chart/diagnosis/` by page.
- A check threshold can be mistaken for a trading rule; the contract marks all
  thresholds as reporting defaults and the command prints `unknown` when the
  evidence is insufficient.
- The diagnosis directory is derived output; refusing a non-empty target with a
  clear message avoids deleting user data.

## 10. Stop conditions

Stop and request approval if:

- A change would alter model, strategy, execution, or portfolio behavior.
- A new external dependency is required.
- The five bundle contracts would need P&L or strategy labels.
- A check threshold would need post-hoc tuning to pass.
- The diagnosis would need to feed the signal path.

## 11. Implementation evidence (2026-10-31)

| Acceptance criterion | Evidence |
|---|---|
| Formatting, lints, tests | `cargo fmt --all -- --check` exited 0; `cargo clippy --all-targets -- -D warnings` produced no output; `cargo test` passed every test binary |
| `backtest --diagnose` writes the five bundles plus `diagnosis/` | Ran on `config/examples/solusd-2025/run.json` into `/tmp/opencode/diagnosis-v3`; printed the five bundle paths and the diagnosis path |
| `lab diagnose` re-runs and verifies identity | Ran into `/tmp/opencode/diagnosis-v2-rerun`; wrote the diagnosis; final equity, net P&L, trades, drawdown, and halt state reproduced |
| Same diagnosis from both paths | `diff -rq` over `/tmp/opencode/diagnosis-v3/diagnosis` and `/tmp/opencode/diagnosis-v2-rerun` reported no differences |
| `lab list` deterministic | Printed the fixed table; `diagnosis-v3` shows `diagnosis=yes`; `--json` prints an array sorted by folder name |
| Scorecard shows attempts -> fills -> trades and the cost waterfall | `00-run-scorecard.png` and `summary.txt` report 298 attempts, 100 fills, 100 trades, and the gross/commission/fees/funding/net waterfall |
| Data page shows coverage and longest gaps | `10-data-quality.png` reports 1m 99.70%, 5m 99.54%, 15m 99.19%, 1h 97.71%, and the 561/551/156-minute gaps |
| Funnel page shows attempts vs fills and exit-reason P&L | `30-strategy-funnel.png` and `strategy-exit-reasons.csv`; strategy exits net `-2322.62`, stops `-891.22`, targets `+1224.25` |
| Execution page shows states, rejections, and slippage | `40-execution-quality.png`; 398 orders, 200 filled, 198 `missing_quantity`, 2.0 bps mean slippage |
| Attribution page shows equity vs benchmark, drawdown vs limit, halt, waterfall, monthly returns | `50-portfolio-attribution.png`; final equity 8010.41, max drawdown 20.05%, halt on 2025-05-03 |
| Six existing PNGs are readable | Distinct entry/exit triangles with legend; benchmark, limit, and halt overlays; progression ribbon; stacked event-density strip |
| `lab summary` computes OOS/rerun checks and refuses stale labs | Golden tests pass; supplied-config hash and model-version guards; `oos/` and `rerun/` checks |
| Boundary test | The existing `StrategyInput` source-level guard remains; the diagnosis is a separate analytics module and is not imported by the strategy |
| Byte-identical output | `diff -rq` found no differences between the two diagnosis directories |

Known limitations: one instrument, one year, in-sample only. The diagnosis reports
`unknown` for OOS and rerun evidence unless the lab carries the `oos/` and `rerun/`
directories. The five per-engine bundles were not changed by the diagnosis.
