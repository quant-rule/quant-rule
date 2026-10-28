# AI Transparency Log — Strategy Types

- Date: 2026-10-27 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash Free
- Work item: [#157](https://github.com/quant-rule/quant-rule/issues/157)
- Container: [#11](https://github.com/quant-rule/quant-rule/issues/11)
- Program: [#141](https://github.com/quant-rule/quant-rule/issues/141)
- Plan: [`docs/execution-plans/2026-10-27/141-market-analysis-and-backtest.md`](../../execution-plans/2026-10-27/141-market-analysis-and-backtest.md)
- Boundary: [`docs/adr/2026-10-27/analysis-and-backtest-boundaries.md`](../../adr/2026-10-27/analysis-and-backtest-boundaries.md)
- Branch/worktree: `feat/157-strategy-types` in `quant-rule--157-strategy-types`
- PR: pending
- Status: Implementation and tests complete; PR pending. The type boundary is approved when the PR merges.

## Intent and boundary

S1 defines the pure type boundary for the strategy engine: the validated
configuration, the per-bar snapshot, the agreement classification, the entry
and exit records, and the `Strategy` trait. It does not implement the M15
strategy state machine, an order, a fill, a position, or P&L; those belong to S2
and later execution and portfolio work items.

The change adds one workspace crate, `crates/quantrule-backtest`, with only the
`strategy` module. It adds no shell command, changes no `market-*` command, and
changes no model output. The crate depends only on the workspace crates
`quantrule-data` and `quantrule-model`; it adds no external dependency.

## Work performed

1. Added `crates/quantrule-backtest` to the workspace members and created its
   manifest with the workspace version, edition, toolchain, and lints.
2. Added the `strategy` module, split into `config`, `input`, `decision`, and
   `tests` files.
3. Defined `StrategyConfig` with private validated fields, the three defaults
   (`900`, `+0.1`, `-0.1`), accessors, and `StrategyConfigError`.
4. Defined `StrategyInput` as a snapshot of the timeframe, bar index, open time,
   Direction value, Momentum force, and Structure score. Added
   `StrategyInput::from_model_bar` to project only those three published values
   from a `ModelBar`.
5. Defined `Agreement` and its strict classification: all three readings
   strictly above the long threshold is `Long`, all three strictly below the
   short threshold is `Short`, and every other available combination is
   `Neutral`. Equality at a threshold is neutral; a missing or non-finite
   reading is `None`, not `Neutral`.
6. Defined `Side`, `Entry`, `Exit`, `ExitReason`, `Decision`, and the causal
   `Strategy` trait. The records carry the decision bar and its three readings;
   they carry no price, quantity, fee, position, or P&L field.
7. Added unit and doc tests for valid and invalid configuration, the strict
   threshold comparisons, equality, null and non-finite readings, determinism,
   the side mapping, the records, the trait, and the `ModelBar` projection.
8. Updated the program plan, the docs index, and this log.

## Verification and evidence

Commands were run in `quant-rule--157-strategy-types` on 2026-10-27. `cargo` is
not on `PATH` in this environment, so each command was invoked through
`~/.cargo/bin/cargo`.

| Command | Observed result |
|---|---|
| `cargo fmt --all -- --check` | passed |
| `cargo clippy --all-targets -- -D warnings` | passed |
| `cargo test` | 1,116 passed, 0 failed, 6 ignored |
| `cargo doc --no-deps` | passed |

The new crate contributes 21 unit tests and 6 doc tests. The 1,116 total is the
previous 1,089 plus those 27 tests. No test was removed, skipped, or weakened.

The unit tests cover:

- the default configuration and an asymmetric valid configuration;
- zero timeframe, non-finite thresholds, a negative long threshold, a positive
  short threshold, and a long threshold not above the short threshold;
- all three readings strictly above and strictly below;
- a mixed bar, equality at each threshold, and a measured zero;
- each missing reading and each non-finite reading;
- repeated classification of the same input;
- the `Agreement`-to-`Side` mapping, the side codes, and the opposites;
- entry and exit records preserving the bar snapshot;
- a test strategy that enters on a long agreement and holds otherwise;
- the `ModelBar` projection over a three-bar model run.

The implementation has no I/O, clock, randomness, global mutable state, or
unordered iteration. `config.rs` imports only `std::fmt`; `input.rs` imports
`ModelBar`; the tests import the workspace data and model crates. The crate
files are 12–305 lines, below the 1,000-line guideline.

## Decisions and learning

- **The thresholds must straddle zero.** A valid configuration requires
  `long_threshold >= 0` and `short_threshold <= 0`, with the long threshold
  strictly greater. This keeps the names honest: a `Long` agreement always
  contains a strictly positive reading, and a `Short` agreement a strictly
  negative one. The rule is documented on the constructor. It is an invariant
  proposed for review; relaxing it later would be a small, local change.
- **Validation order was corrected.** The first implementation checked the
  positive-short rule before the threshold order, so `(0.1, 0.1)` returned
  `PositiveShortThreshold` instead of `LongNotAboveShort`. The order check now
  runs first, and the test names the relationship error. This was found by the
  test run, not by inspection.
- **Equality is neutral, not a third threshold.** The comparisons are strict
  (`>` and `<`). A reading exactly at either threshold is inside the neutral
  band. The test suite pins both boundaries.
- **Missing is not neutral.** A `None` reading, or a non-finite value, returns
  `None` from `Agreement::classify`. This follows the project rule that a
  missing measurement is not replaced by zero or by a neutral label.
- **The strategy input is a projection, not the whole model bar.** `from_model_bar`
  keeps only the three values the first rule uses. It deliberately drops the
  candle and every other reading so that a strategy cannot reach for a price or
  an unrelated axis.
- **An entry or exit is a decision, not a fill.** The records have no price,
  quantity, fee, or P&L field. `ExitReason::AgreementLost` is the first rule's
  only exit reason; a later rule adds its own variant rather than overloading
  this one.
- **No end-of-data exit was added.** If the data ends while the agreement still
  holds, S1 has no forced close. Final-position handling belongs to S2 and the
  portfolio work item, where unrealized P&L is defined.
- **Rejected:** implementing the M15 state machine in S1. That is S2's scope and
  would make the type boundary review depend on strategy behavior.
- **Rejected:** deriving or serializing JSON in the backtest crate. The shell
  renders records, and S1 adds no presentation layer.
- **Rejected:** adding `EndOfData` to `ExitReason` without a plan decision. The
  first rule exits on agreement loss only.
