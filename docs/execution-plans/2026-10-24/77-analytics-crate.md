# Execution Plan: The Analytics Crate and Axis Resolution (AN2)

- Issue: [#77](https://github.com/quant-rule/quant-rule/issues/77) — feat: add the analytics crate and axis resolution
- Branch: `feat/77-analytics-crate`
- Pull request: pending
- Status: Verified
- Date: 2026-10-24 (UTC)
- Contract: [`docs/analytics/2026-10-24/research-output.md`](../../analytics/2026-10-24/research-output.md) §6
- Transparency log: [`docs/ai-transparency-log/2026-10-24/77-analytics-crate.md`](../../ai-transparency-log/2026-10-24/77-analytics-crate.md)

## 1. Intent

Create the pure `quantrule-analytics` crate and implement the per-axis resolution that Layers 2 and 3 consume. The mapping is the one the research contract fixes: a continuous value in `[-1, +1]`, an equal-width ordinal in `[0, 9]`, and a null plus reason when the axis is unavailable.

## 2. Current state

- `main` is at `417a977`; the research output contract is merged (#76).
- No analytics crate exists.
- The model publishes the four axis bars with their values, readiness, and unavailable reasons.
- The shell has no analytics dependency yet.

## 3. Scope

Included:

- `crates/quantrule-analytics` with the crate manifest, rustdoc, and workspace membership.
- `resolution`: `AxisResolution`, `AxisResolutions`, `ordinal_bucket`, and `clamp_unit`.
- Unit tests for the contract boundaries, every reachable bucket, clamping, null-and-reason behavior, and the four-axis wiring.
- The crate list in `AGENTS.md`, which the new crate makes incomplete.

Excluded:

- As-of alignment (#78).
- Layers 1–3 (#79).
- Export and metadata (#80), analysis (#81), verification (#82).
- Any store or shell code.

## 4. Assumptions and uncertainty

- The model guarantees a finite axis value in range, and a non-null `unavailable_reason` whenever the value is null. The direction and progression implementations set the reason on every missing value, so `from_bars` is infallible.
- The crate depends only on `quantrule-model` for now; `quantrule-data` is added when the layer builder needs candles (#79).
- The public API is internal to the workspace, so it may change before #79 without a migration.
- The contract's non-finite rule is an export-level rejection; the pure mapping remains total and documents its behavior.

## 5. Decisions

| ID | Decision | Alternatives | Choice | Rationale |
|---|---|---|---|---|
| D1 | Bucket computation | `(v + 1) / 0.2`; count the nine boundaries | Count the boundaries | A probe showed the division maps `-0.8` to bucket 0; the contract requires 1 |
| D2 | Who clamps | Clamp every axis in the constructor; clamp only Progression | Clamp only Progression's raw `progress` in `from_bars` | A published axis value is already in range; clamping it would hide a model defect |
| D3 | Missing reason | Make `from_bars` return `Result`; trust the model | Trust the model and stay infallible | The model sets a reason on every missing value; the export validates the pair |
| D4 | Non-finite input | Panic; return null; total clamp | Total clamp: `NaN` → 0, infinities → ±1 | The crate stays total; the contract rejects non-finite values at the export |
| D5 | Crate dependencies | Add `quantrule-data` now; add it with #79 | Add it with #79 | No unused dependency |
| D6 | `AGENTS.md` | Leave the crate list; update it | Update it in this PR | Documentation follows the code; the crate list is otherwise stale |

## 6. Acceptance criteria

- `-1.0 → 0`, `-0.8 → 1`, `0.0 → 5`, `+0.8 → 9`, `+1.0 → 9`.
- Every bucket `0..9` is reachable.
- A null axis stays null with its own reason; a measured value carries no reason.
- Progression's raw `progress` is clamped for the resolution, and the raw value is not modified.
- The crate has rustdoc for every public item and no warnings.
- `fmt`, `clippy -D warnings`, `cargo test`, and `doc -D warnings` are clean.

## 7. Work breakdown

1. Add the crate manifest and the workspace member.
2. Implement `resolution`.
3. Add the unit tests.
4. Update the crate list in `AGENTS.md`.
5. Run the checks, write the plan and the log, commit, push, open the PR, move #77 to Done, merge, and clean up.
6. Continue with #78 on its own branch.

## 8. Verification

| Check | Result |
|---|---|
| `cargo test -p quantrule-analytics` | 10 passed, 0 failed |
| `cargo test --workspace` | 687 passed, 0 failed, 1 ignored |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | clean |
| Boundary assertions | `-1→0`, `-0.8→1`, `0→5`, `0.8→9`, `1→9` |

## 9. Risks and recovery

| Risk | Mitigation |
|---|---|
| The bucket changes when #79 consumes it | The boundary table and the tests pin it; the contract is the reference |
| The crate API changes before #79 | The public API is internal; #79 extends it and updates the tests |
| `AGENTS.md` is a rule file | The edit is factual and limited to the crate list and the contract pointer |
| A non-finite value reaches the mapping | The export rejects it; the mapping's behavior is documented and tested |

## 10. Stop conditions

- The model does not guarantee a reason for a missing axis.
- The bucket mapping cannot match the contract's boundary examples.
- A check fails and the fix is outside this scope.

## 11. Delivery

| Item | Value |
|---|---|
| Commit | pending |
| Branch | `feat/77-analytics-crate` |
| Pull request | pending |
| Board | #77 moved to `Done` when the PR is open |

Merge, release, tag, and deployment remain with the human.
