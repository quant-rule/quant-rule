# AI Transparency Log — Backtest Command

- Date: 2026-10-28 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash Free
- Work item: [#176](https://github.com/quant-rule/quant-rule/issues/176)
- Program: [#141](https://github.com/quant-rule/quant-rule/issues/141)
- Plan: [`docs/execution-plans/2026-10-28/141-mvp-v1-final-contract.md`](../../execution-plans/2026-10-28/141-mvp-v1-final-contract.md)
- Branch/worktree: `feat/176-backtest-command` in `quant-rule--176-backtest-command`
- PR: [#188](https://github.com/quant-rule/quant-rule/pull/188)
- Status: PR open; merge after checks pass.

## Intent and boundary

B3 adds the single config-driven full-run command. `backtest --config
<run.json> [--out <dir>] [--json]` loads and validates the run, loads the
candles once, calls `run_backtest` once, and writes the five bundles from that
result. The model and analytical bundles use the run's model and layers; they do
not recompute the model. The command adds no new statistic and changes no
standalone `report` output.

## Work performed

1. Added `build_layers_from_run` to the analytics crate and refactored
   `build_layers` to share the same assembly, with an equivalence test.
2. Split each report bundle into a `*_from` function that takes the already
   computed model run or `BacktestRun` and returns the written bundle.
3. Added `command/backtest.rs` with the parser, the single-run execution, the
   five-bundle writes, the summary envelope, and the failure cleanup.
4. Added the `backtest` command to the interactive and argv parsers, the help,
   and the README, including the full-year example.
5. Added binary-level tests for the five bundles, byte identity with every
   standalone report, the JSON summary, the non-empty root, and the failed-run
   cleanup.

## Decisions and corrections

- **One run, five bundles.** The command calls `run_pipeline` once. The model,
  strategy, and analytical bundles take `run.model_run()`; the execution and
  portfolio bundles take the same `&BacktestRun`. The test compares every bundle
  byte for byte with the standalone `report` bundle for the same config.
- **The analytical layers now build from the model run.** R3's bundle built the
  layers from candles, which recomputed the model inside `build_layers`. The
  plan's `build_layers_from_run` was not implemented in R3; B3 adds it, switches
  the analytical bundle to it, and keeps `build_layers` on the same assembly. The
  equivalence test covers both paths.
- **A failed run removes only what it wrote.** The run root is checked before any
  write. On failure the command removes the five engine directories and the root
  only when the root did not exist before the command started.
- **The summary envelope names paths, not bytes.** `--json` prints the data
  version, the requested and actual periods, the five bundle paths, and the
  portfolio's final equity, return, net P&L, drawdown, trade count, and halt
  flag. The bundles themselves are the artifacts.
- The first non-interactive `backtest` run failed with "unknown command" because
  the argv parser was not wired while the interactive parser was. The test added
  for the command covers both entry points through the binary.
- The `backtest` command's `--out` default is the run config's `output_root`; an
  explicit `--out` is the run root, not an engine directory.

## Verification and evidence

Commands were run in `quant-rule--176-backtest-command` on 2026-10-28.

| Command | Observed result |
|---|---|
| `cargo fmt --all -- --check` | Passed; no diff |
| `cargo clippy --all-targets -- -D warnings` | Passed; no warning |
| `cargo test` | 1,346 passed, 0 failed, 1 ignored (`store_100k`) |
| `cargo test -p quantrule-shell --test backtest_command` | 6 passed, 0 failed |
| `cargo doc --no-deps` | Passed; exit 0; no warning |

The full-suite count rose from 1,338 (R3 baseline) to 1,346: one analytics
equivalence test, one command unit test, and six binary-level command tests. No
existing test was changed or removed.

## Learning

- A bundle writer that takes data rather than fetching it is what makes "one run,
  five bundles" possible. The `*_from` split is the seam.
- Byte identity is the only convincing check that the standalone and full-run
  bundles are the same artifact; comparing field names would not catch a
  provenance or ordering drift.
- The run root is a boundary separate from the engine directories. Checking it
  before any write is what makes a failed run leave no partial root.
