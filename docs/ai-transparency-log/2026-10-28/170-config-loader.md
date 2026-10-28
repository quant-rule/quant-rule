# AI Transparency Log — Config Loader

- Date: 2026-10-28 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash Free
- Work item: [#170](https://github.com/quant-rule/quant-rule/issues/170)
- Program: [#141](https://github.com/quant-rule/quant-rule/issues/141)
- Plan: [`docs/execution-plans/2026-10-28/141-mvp-v1-final-contract.md`](../../execution-plans/2026-10-28/141-mvp-v1-final-contract.md)
- Branch/worktree: `feat/170-config-loader` in `quant-rule--170-config-loader`
- PR: [#182](https://github.com/quant-rule/quant-rule/pull/182)
- Status: PR open; merge after checks pass.

## Intent and boundary

C1 makes the seven config files the specification of a run. The shell parses
and validates them before any candle is read or any output byte is written, and
hands the engines validated Rust values. The engines gain no serde dependency
and perform no file I/O.

The boundary is the shell's config module and the bounded CSV loader. C1 adds no
command, no report writer, no output, and no backtest command. It adds no
dependency: `serde_json` was already a shell dependency. Absolute paths are
accepted; error messages print only the file's own name, never a resolved path.

## Work performed

1. Added one module per file: `run`, `data`, `model`, `strategy`, `execution`,
   `portfolio`, and `analytics`, plus `error` and the shared strict `read`
   helpers.
2. Added `RunConfig` with the run name, the resolved output root, the seven
   resolved paths, and the six engine configs.
3. Added cross-config validation: strategy timeframe in the ladder, fill
   timeframe equal to the strategy timeframe, analytics scale equal to the model
   scale, analytics timeframes a subset of the ladder, `from <= to`, leverage
   `1.0`, and position limit `1`.
4. Added `load_timeframes_with_limit(path, bounds, limit)` and rewired
   `load_extended_timeframes` through it, so the configured `max_candles` reaches
   the data engine through `TimeframeSet::with_limit`.
5. Added checked-in fixtures for a missing reference and invalid JSON, and a
   table-driven test per file that asserts both the error category and the JSON
   path.

## Decisions and corrections

- **The strict reader walks `serde_json::Value` rather than deriving.** The
  contract requires the file name and JSON path on every rejection, unknown
  fields at every object level, and exact JSON types. A derive would hide the
  path and would need custom deserializers for each rule. The walk is explicit
  and each helper owns one rule.
- **Error text never prints a path.** The contract accepts absolute paths but
  forbids printing one. The error carries the file's final component
  (`data.json`) and the JSON path, and the missing-file test asserts the
  absolute prefix is absent.
- **A missing and an unreadable file share `MissingFile`.** The contract names
  that category; the observed value distinguishes `missing` from `unreadable`
  without leaking the operating system's message, which is locale-dependent.
- **The ATR multiples are carried, not applied here.** `execution.json` exposes
  `stop_atr_multiple` and `target_atr_multiple`, and the loader validates and
  carries them in `ExecutionRunConfig`. The execution engine's `ExecutionConfig`
  currently holds the costs only and its bracket uses the 2/3 constants. The
  example fixes the multiples at 2 and 3, so the MVP run is consistent; applying
  a non-default value is left to the backtest runner (#172), whose paths include
  the execution config. This is the one known gap between the config contract
  and the engine, and it is recorded here rather than hidden by rejecting a
  value the contract allows.
- **The module is allowed to be unused until #172.** `RunConfig` has no caller in
  C1 by design. The module carries `#![allow(dead_code, unused_imports)]` with a
  comment naming #172-#176; the allow is module-scoped and is removed when the
  first command consumes the loader.
- **Test fixtures, not temporary files.** Validation tables call the pure
  parsers with in-memory JSON, so they touch no filesystem. The two file-level
  failures use checked-in fixtures under `tests/fixtures/config/`. The example
  config is read-only from the repository.
- The first strategy table asserted that `long_threshold: 0.0` was rejected.
  Zero is a valid long threshold; the case was corrected to `long == short == 0`
  and a separate case now covers a positive short threshold.
- The first `RunFile` and `validate_cross` were private; the tests could not
  name them. Both are now visible inside the config module only.

## Verification and evidence

Commands were run in `quant-rule--170-config-loader` on 2026-10-28.

| Command | Observed result |
|---|---|
| `cargo fmt --all -- --check` | Passed; no diff |
| `cargo clippy --all-targets -- -D warnings` | Passed; no warning |
| `cargo test` | 1,302 passed, 0 failed, 1 ignored (`store_100k`) |
| `cargo test -p quantrule-shell config` | 12 passed, 0 failed |
| `cargo doc --no-deps` | Passed; exit 0; no warning |

The full-suite count rose from 1,290 (E3 baseline) to 1,302. The increase is the
twelve config test functions, each of which runs a table of cases; no existing
test was changed or removed.

## Learning

- A strict reader is easiest to keep honest when each helper owns exactly one
  clause of the contract. The table then asserts the category and path, which is
  what a caller needs, rather than the prose of a message.
- Resolving a path and printing a path are different decisions. The loader
  resolves absolute paths for I/O but never puts one in an error, which keeps a
  config error safe to paste into an issue.
- A config field that the engine cannot yet apply is a gap to state, not a value
  to reject. Rejecting it would contradict the approved table; ignoring it
  silently would make the run irreproducible. Carrying it and naming the
  follow-up keeps both the contract and the limitation visible.
