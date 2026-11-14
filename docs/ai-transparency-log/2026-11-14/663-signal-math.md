# AI transparency log — DIST-060 signal-path math guard

- Date: 2026-11-14 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Intent: enforce the fixed numeric contract at the source level, so a
  transcendental or fused-multiply-add call cannot enter the signal path.
- Boundary: test-only. The guard reads the production source of `quantrule-data`,
  `quantrule-model`, and the strategy, execution, and portfolio modules of
  `quantrule-backtest`. It excludes the benchmark (evaluation, not signal
  generation), test files, and inline test modules. No production code changed.
- Verification: three tests pass — the clean tree, a seeded `powf`/`mul_add`
  source, and the allowlisted arithmetic. A real `powf` appended to
  `quantrule-data/src/lib.rs` made the clean-tree test fail with the seeded line;
  the file was restored and the test passed again. `tools/gate.sh` passed.
- Learning: the model tests use `sin` to generate interesting fixtures. The
  guard truncates a file at its first `#[cfg(test)]` and skips `tests.rs`, which
  keeps fixture math out without granting production code an exception.
- Rejected: scanning the whole backtest crate. `benchmark/mod.rs` uses `powf`
  for annualization; it is an evaluation metric, not a signal-path value, and
  forbidding it there would not protect determinism.
