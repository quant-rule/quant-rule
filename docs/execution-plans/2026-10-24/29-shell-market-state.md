# Execution Plan: Expose the Market State Through the Shell (M13)

- Issue: [#29](https://github.com/quant-rule/quant-rule/issues/29) — feat: expose the market state through the shell
- Branch: `feat/29-shell-market-state`
- Pull request: [#89](https://github.com/quant-rule/quant-rule/pull/89)
- Status: Verified
- Date: 2026-10-24 (UTC)
- Program plan: [Completion plan (M2–M14)](14-model-engine-completion.md)
- Contract: [`docs/model/2026-10-21/four-axis-analytical-model.md`](../../model/2026-10-21/four-axis-analytical-model.md)
- Transparency log: [`docs/ai-transparency-log/2026-10-24/29-shell-market-state.md`](../../ai-transparency-log/2026-10-24/29-shell-market-state.md)

## 1. Intent

Make the market state reachable by typing a command. The shell parses the command, builds the M1/M5/M15/H1 ladder, calls the model, and renders the numeric output with fixed labels. The model stays numeric; the shell owns presentation.

## 2. Current state

- `main` is at `1ba7b12`; #23–#28 are merged.
- `MarketState::compute` publishes the per-timeframe state, aggregates, and consensus.
- The shell has data commands only; the `quantrule-model` dependency is declared but unused.

## 3. Scope

Included:
- `market-state`, `axis`, `events`, and `legs` commands.
- Parsing for interactive and non-interactive invocation.
- Fixed numeric-to-label mappings for axis states, structure, stage, context, readiness, reason, event type/subtype/source, lifecycle, scale, leg direction, and target basis.
- A JSON market-state object and NDJSON axis, event, and leg records.
- Help and README command reference.
- A CLI fixture and eight command tests plus parser tests.

Excluded:
- Backtests, strategies, and portfolio execution.
- Any model change.

## 4. Assumptions and uncertainty

- The command names `market-state`, `axis`, `events`, and `legs` are the implemented contract. Changing them is a command-contract decision reserved to the human.
- `market-state` prints one JSON object; `axis`, `events`, and `legs` print one JSON object per line.
- The shell labels are fixed mappings; the model remains numeric.
- The fixture is generated from the 260-bar readings fixture with one-minute timestamps, so the derived H1 series is short and its axes are unavailable. The command still assembles all four timeframes.
- The shell does not recompute an axis, aggregate, or event.

## 5. Decisions

| ID | Decision | Alternatives | Choice | Rationale |
|---|---|---|---|---|
| D1 | Command names | Other names | `market-state`, `axis`, `events`, `legs` | The proposed set from M13; implemented as the command contract |
| D2 | Market-state shape | NDJSON per timeframe; one object | One JSON object | The state is one nested document with aggregates and consensus |
| D3 | Axis/event/leg shape | One object; NDJSON | NDJSON | Each line is a complete record and the stream is pipe-friendly |
| D4 | Labels | Numeric only; numeric plus labels | Numeric plus labels | The model is numeric; the shell maps every code to a fixed word |
| D5 | Fixture timestamps | Raw readings timestamps; aligned M1 | Aligned one-minute stamps | The data engine requires whole-minute M1 timestamps |
| D6 | Empty timeframe | Error; skip | Skip | An empty series has no latest bar; existing data commands already handle it |

## 6. Acceptance criteria

- Each command's output is asserted in a test.
- Interactive and non-interactive output are byte-identical.
- Every command is reachable from `help`.
- No word is added to the model layer; labels live in the shell.
- The README command reference matches the shipped commands.
- `fmt`, `clippy -D warnings`, `cargo test`, and `doc -D warnings` are clean.

## 7. Work breakdown

1. Add `labels.rs` with the fixed code-to-word mappings.
2. Add `render_state.rs` for the JSON and NDJSON output.
3. Extend `command.rs` with the four commands, `Axis`, parsing, and runners.
4. Extend `main.rs` with interactive and non-interactive dispatch.
5. Generate `state_m1.csv` from the readings fixture.
6. Add eight CLI tests and parser tests.
7. Update `help` and the README.
8. Run the full checks, commit, push, open the PR, move #29 to Done, merge, and clean up.

## 8. Verification

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --all-targets -- -D warnings`: clean.
- `cargo test --workspace`: 669 passed, 0 failed.
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`: clean.
- Shell unit tests: 96 passed.
- Shell CLI tests: 55 passed, including the eight new model-command tests.

## 9. Risks and recovery

| Risk | Mitigation |
|---|---|
| A command output changes silently | CLI tests parse and assert the output shape and labels |
| Interactive and non-interactive diverge | A test compares the two stdout streams |
| A label mapping drifts | Labels are fixed functions; tests exercise the label fields |
| The shell recomputes model values | The runner only builds the ladder and calls `MarketState::compute` |
| The help fixture goes stale | The help fixture is regenerated from the shipped help |
| The fixture's short H1 confuses assertions | Tests assert structure, not every higher-timeframe value |

## 10. Stop conditions

- A command-contract change beyond the approved names.
- A model change outside #29.
- A new dependency.
- A failing check that cannot be corrected within scope.

## 11. Delivery

| Item | Value |
|---|---|
| Commit | `842d41a` — `feat: expose the market state through the shell` |
| Branch | `feat/29-shell-market-state` |
| Pull request | [#89](https://github.com/quant-rule/quant-rule/pull/89), open against `main` |
| Board | Issue #29 moved to `Done`; the workflow closes the issue |

Merge, release, tag, and deployment remain with the human.
