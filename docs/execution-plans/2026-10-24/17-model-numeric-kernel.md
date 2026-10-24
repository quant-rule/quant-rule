# Execution Plan: The Model Numeric Kernel

- Issue: [#17](https://github.com/quant-rule/quant-rule/issues/17) — feat: build the shared numeric kernel of the model
- M-item: M1
- Branch: `feat/17-model-numeric-kernel`
- Status: Verified
- Date: 2026-10-24 (UTC)
- Umbrella plan: [`docs/execution-plans/2026-10-21/14-model-engine.md`](../../execution-plans/2026-10-21/14-model-engine.md)
- Contract: [`docs/model/2026-10-21/four-axis-analytical-model.md`](../../model/2026-10-21/four-axis-analytical-model.md)
- Transparency log: [`docs/ai-transparency-log/2026-10-24/17-model-numeric-kernel.md`](../../ai-transparency-log/2026-10-24/17-model-numeric-kernel.md)

## Intent

Every axis, event, and aggregate needs the same numeric vocabulary: a way to say
"missing" without using zero, the unavailable-reason and readiness registries,
the contract's fixed clamp, the sign operation, the compact-state bucketing, the
empirical percentile, and the per-axis state counters. Without one kernel, each
axis would define its own version and the definitions would drift.

The intended outcome is a single `kernel` module in `quantrule-model` that owns
those definitions, contains no axis knowledge, and is covered by tests. The
affected user is the human developer: the definitions become reviewable in one
place before any axis consumes them.

## Current state

- `crates/quantrule-model` is the nine-line stub created by issue #46. It has no
  modules other than `lib.rs`.
- The data engine is complete: issue #9 and its sub-issues #46–#50 are closed,
  and PRs #51–#57 are merged. M1 does not consume the data engine.
- The contract fixes the percentile and counters in §9, and the reason codes in
  §14. The compact-state threshold is §9 and A5.
- The project board has #17 in `In Progress` with `Start date` 2026-10-24. No
  other leaf issue is in flight. #14 is the model container and does not occupy
  the work-item slot.
- The workspace declares `unsafe_code = "forbid"` and `missing_docs = "warn"`.
  Rustdoc examples are compiled by `cargo test`.

## Scope

Included:

- `crates/quantrule-model/src/kernel.rs`: the registries, clamp, sign,
  compact-state, percentile, and state counters.
- `crates/quantrule-model/src/lib.rs`: expose the module and its public items.
- One clarifying sentence in contract §9 for the counter age and null-freeze
  semantics, so the fixed numeric contract states what the code does.
- Unit tests beside the code, a hand-computed percentile fixture, a determinism
  test over reordered samples, and edge cases for missing data, short history,
  NaN, infinity, and the 50-bar change window.
- This plan and the matching transparency log.

Explicit non-goals:

- No readings, axes, events, aggregation, market state, or shell command. Those
  are M2–M13.
- No change to `quantrule-data`, the shell, the README, or the command set.
- No new dependency. The kernel uses the standard library only.
- No percentile cache or rolling buffer abstraction. The first version takes a
  slice; M2 owns the series storage.
- No change to the four axes, the output field names, or the code values.

## Assumptions and uncertainty

- **The contract is the source of the numeric values.** The kernel implements
  §9 and §14 and does not re-derive them.
- **Samples arrive in bar order.** `empirical_percentile` is documented to take
  valid magnitudes strictly before the current bar, oldest first. It takes the
  last 252 valid values in the order given and ignores non-finite values. The
  order is a documented precondition: with more than 252 samples, a different
  order selects a different window. Within the selected window, the count does
  not depend on order once the values are sorted.
- **The state counter is agnostic to the code family.** It accepts any `i8`
  code, so Direction and Momentum can pass -1..+1, Structure -2..+2, and
  Progression 0..5. The registry that defines those families belongs to the
  axis that publishes them.
- **The counter age convention is the contract's wording.** "Closed bars the
  state has held" is implemented as: the first closed bar with a code has age 1,
  each later bar with the same code increments it, a change resets it to 1, and
  a null bar freezes it. The contract sentence added in this change states this
  so it is not re-derived by each axis. This is the only local interpretation in
  M1 and is flagged for review.

## Decisions

| # | Decision | Alternatives | Choice | Approval |
|---|---|---|---|---|
| D1 | Where the kernel lives | A crate of its own; a module per helper; one `kernel` module | One `quantrule-model::kernel` module, cohesive and under the file-size limit | Approved with the work item |
| D2 | How missing values are represented | A sentinel zero; `Option<f64>`; a nullable wrapper type | `Option` for every fallible calculation; no helper returns zero for a missing measurement | Approved with the work item |
| D3 | State-counter age on the change bar | 0 on the change bar; 1 on the change bar | 1, because the contract counts closed bars the state has held; null bars freeze both counters | Local interpretation, flagged in the plan and the log |
| D4 | Percentile sample handling | Trust the caller; filter non-finite values; reject the whole call | Take the last 252 finite samples in bar order; return `None` when fewer than 50 remain or the value is not finite | Approved with the work item |
| D5 | Registry representation | Free functions returning constants; enums with explicit codes and `ALL` arrays | Enums with `#[repr(u8)]`, `code()`, `from_code()`, and `ALL`; tests assert the contract's values | Approved with the work item |
| D6 | Clamp bounds | A generic `clamp(value, lower, upper)`; a `clamp_unit` helper | `clamp_unit`, because §12 fixes the bounds and call sites must not re-derive them | Approved with the work item |

## Acceptance criteria

From issue #17, observable and checked in this change:

- Every helper has unit tests beside it.
- The percentile matches a hand-computed fixture.
- No helper can return 0 for a missing measurement, and a valid 0 is preserved.
- The reason-code and readiness registries are checked-in tables that match the
  contract's §14 unavailable reasons and §9 readiness codes.
- A determinism test proves the percentile does not depend on the order in which
  the samples were collected.
- `cargo fmt --check`, `cargo clippy --all-targets`, `cargo test`, and
  `cargo doc --no-deps` are clean.

## Work breakdown

| Step | Action | Role | Paths | Depends on |
|---|---|---|---|---|
| 1 | Write this plan and move #17 to In Progress with its start date | Orchestrator | this file, board | — |
| 2 | State the counter age and null-freeze rules in contract §9 | Implementation | `docs/model/2026-10-21/four-axis-analytical-model.md` | 1 |
| 3 | Implement the registries, clamp, sign, compact-state, percentile, and counters | Implementation | `crates/quantrule-model/src/kernel.rs`, `lib.rs` | 2 |
| 4 | Add unit, fixture, determinism, and edge-case tests | Test Engineer | `kernel.rs` tests | 3 |
| 5 | Run `fmt`, `clippy`, `test`, and `doc`; correct within scope | Test Engineer | workspace | 4 |
| 6 | Review the full diff against the contract and this plan | Code Reviewer | all changed files | 5 |
| 7 | Record the evidence, mark this plan `Verified`, and write the transparency log | Orchestrator | this file, log | 6 |
| 8 | Stop for commit authorization | Orchestrator | git state | 7 |

## Verification

Commands, with the observed evidence in this worktree (`rustc 1.99.1`,
`cargo 1.99.1`; `~/.cargo/bin/cargo` because `cargo` is not on the default
`PATH`):

| Command | Observed |
|---|---|
| `cargo fmt --check` | Exit 0, no output |
| `cargo clippy --all-targets -- -D warnings` | Exit 0, no warnings |
| `cargo test` | Exit 0: 90 data unit + 23 model unit + 93 shell unit + 47 shell integration + 2 data doctests + 10 model doctests = 265 passed, 0 failed |
| `cargo doc --no-deps` | Exit 0, no warnings |

Specific evidence recorded:

- **Percentile fixture.** The 50-sample series 1..=50 gives 0% at 0, 2% at 1,
  50% at 25, 100% at 50, and 100% at 51. A 49-sample series returns `None`; a
  300-sample series uses the most recent 252 values, so 48 is 0% and 49 is
  `100/252`. All-equal samples give 100% at the sample value and 0% below it.
- **Determinism.** The percentile returns the same value for the bar-order,
  reversed-order, and interleaved-order copies of the same 50 samples. The
  counter run over a fixed code sequence is identical across repeated runs.
- **State counters.** The tests show age 1 on the first code, increments while
  the code holds, a reset to 1 on a change, a freeze on `None`, no window slot
  consumed by `None`, and the 50-code change count falling back to 0 after 50
  unchanged codes.
- **Registries.** The tests assert unavailable reasons 1..6 and readiness 0..2,
  with `from_code` round trips and `None` for undefined codes.
- **Null versus zero.** `compact_state(NaN)` is `None`, short or non-finite
  percentile inputs are `None`, and a measured zero stays `Some(0)` or `0.0`.
- **Review.** A read-only review pass found no blockers. It raised one major
  performance issue (the percentile scanned the full history although it kept
  only 252 values) and minor documentation/test gaps. The percentile now scans
  backward and stops at 252 finite values; the docs state the NaN pass-through
  is a defect signal, and tests were added for an empty/one-sample series,
  non-finite samples not consuming a window slot, an all-null counter series,
  and `Default` matching `new`.
- **Scope.** The diff touches `quantrule-model` and the model contract only. No
  data-engine, shell, README, command, or dependency change is present.

Limitations:

- M1 is a pure kernel; it does not demonstrate an axis, a reading, or a shell
  command. Those claims belong to later M-items.
- The 252/50 windows are unit-tested on slices. M2 owns the per-bar series that
  feeds them.
- No benchmark is added; the kernel has no history-sized allocation beyond the
  bounded percentile copy.
- The review was an AI review pass in the same session, not an independent human
  approval. The human still owns acceptance.

## Risks and recovery

- **The age convention could be read differently.** It is stated in the contract
  and in rustdoc, and the test fixes it. If the human prefers 0 on the change
  bar, one constant and one assertion change.
- **A permissive counter accepts out-of-range codes.** The counter is
  deliberately generic; the axis tests own the code ranges. This avoids axis
  knowledge in the kernel.
- **Non-finite filtering could hide a caller defect.** The percentile returns
  `None` when fewer than 50 finite samples remain, and inputs are expected to be
  finite. The filtering is defensive, not a substitute for the caller's own
  validity checks.
- **Recovery.** The change is one branch and one module. Reverting the commit
  restores the stub; no persistent state or migration is involved.

## Stop conditions

Stop and request human direction if:

- The toolchain cannot run `fmt`, `clippy`, `test`, or `doc`.
- The contract's §9 or §14 values contradict the issue or each other.
- A new dependency is needed.
- The age convention, null-freeze rule, or percentile semantics require a
  decision that changes a published meaning rather than clarifying it.
- The implementation needs an axis, reading, or shell change.
