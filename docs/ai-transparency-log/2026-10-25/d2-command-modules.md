# AI Transparency Log: D2 Command Modules

- Work item: `d2-command-modules`
- Plan: [`docs/execution-plans/2026-10-25/d2-command-modules.md`](../../execution-plans/2026-10-25/d2-command-modules.md)
- Pull request: [#113](https://github.com/quant-rule/quant-rule/pull/113)
- Date: 2026-10-25 (UTC)

## Entry: command-family module split

- **Date and tool:** 2026-10-25 (UTC). opencode (DeepSeek V4.1 Flash 1017).
- **Intent and boundary:** Close D2 by moving the parsing and execution of each
  command family into its own module. No command name, argument, output, error
  text, exit code, dependency, or schema changes.
- **Implementation:**
  - Reduced `command.rs` to 40 lines of module declarations and re-exports.
  - Added `types`, `parse`, `model`, `chart`, `data`, `fetch`, `research`,
    `store`, `layers`, and `help` modules under `command/`.
  - Moved `command.rs` tests to `command/tests.rs` without changing assertions.
  - Moved the store query and verify argument structs into `types.rs` so the
    vocabulary no longer imports the implementations.
  - Kept `parse` and `parse_argv` converging on the same `Command` and
    `execute`.
- **Verification and evidence:**
  - `cargo test`: 883 passed, 4 ignored; the CLI suite is 67 of those tests.
  - `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
    and `cargo doc --no-deps` with `-D warnings`: clean.
  - Largest command module is `store_query.rs` at 619 lines; every module is
    below the 1,000-line guidance.
  - 100k release model benchmark: 2.447 s. 100k layer/export: build 4.578 s,
    render 4.416 s. 100k store round-trip: ingest 9.004 s, verify 21.968 s,
    export 8.055 s.
- **Learning:**
  - A first scripted cut left duplicated doc comments and grouped a function
    with the wrong module. The compiler and the CLI suite caught the structural
    symptoms; the files were rebuilt from the original line ranges and reviewed
    before verification.
  - The test module had retained its own `mod tests` wrapper after the cut.
    Removing the wrapper kept the test names and count unchanged.
  - The split needed no behavior change: all 883 tests passed after the move.
- **Remaining:** the exact 1,500/100,001/1,200/H1-readiness/>1,024 fixtures
  listed in the predecessor plan.
