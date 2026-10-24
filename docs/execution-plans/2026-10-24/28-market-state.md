# Execution Plan: Market State and Consensus (M12)

- Issue: [#28](https://github.com/quant-rule/quant-rule/issues/28) — feat: build the consensus diagnostic and the market-state output contract
- Branch: `feat/28-market-state`
- Pull request: pending
- Status: Verified
- Date: 2026-10-24 (UTC)
- Program plan: [Completion plan (M2–M14)](14-model-engine-completion.md)
- Contract: [`docs/model/2026-10-21/four-axis-analytical-model.md`](../../model/2026-10-21/four-axis-analytical-model.md)
- Transparency log: [`docs/ai-transparency-log/2026-10-24/28-market-state.md`](../../ai-transparency-log/2026-10-24/28-market-state.md)

## 1. Intent

Assemble the published market state: each timeframe's latest axis, context, leg, level, and event state, the cross-timeframe aggregates, and the per-timeframe cross-axis consensus. The state is numeric only, keeps a missing axis out of the consensus, and never combines the four axes into one score.

## 2. Current state

- `main` is at `3c22247`; #23–#27 are merged.
- The axes, context, events, and cross-timeframe aggregates exist.
- No assembled per-timeframe state or consensus exists yet.

## 3. Scope

Included:
- `TimeframeInput`, `TimeframeMarketState`, `MarketStateConfig`, and `MarketState`.
- Per-timeframe consensus counts and the weakest directional axis code.
- Per-timeframe snapshot assembly from candles: readings, axes, context, events, ATR, references, levels, and leg origin times.
- A four-timeframe ladder test built through the data engine.
- A consensus golden fixture.
- Contract clarification for the per-timeframe consensus.

Excluded:
- Shell commands (#29).
- Hardening and the full contract audit (#30).
- Any change to an axis or event formula.

## 4. Assumptions and uncertainty

- The consensus is published per timeframe; each timeframe's counts cover Direction, Momentum, and Structure only.
- Missing axes are excluded from the counts and are not neutral.
- `weakest_directional_axis_code` uses the lowest valid percentile; ties go to the lower axis code.
- Empty timeframe inputs are omitted from the state.
- Structure and Divergence share the configured pivot scale, default major.
- Events are combined and sorted by `(available, occurred, type, subtype, source, reference)`.
- The state holds the latest snapshot per timeframe; the per-bar series remain inside the engine.
- The full per-timeframe field set is the already-verified axis/event/context bars; this change adds the assembly and consensus. The consensus CSV is the new checked-in fixture; the four-timeframe ladder is tested structurally.

## 5. Decisions

| ID | Decision | Alternatives | Choice | Rationale |
|---|---|---|---|---|
| D1 | Consensus scope | One record for the ladder; per timeframe | Per timeframe | Counts over the three axes need one timeframe's axis values; the ladder publishes each |
| D2 | Missing axis | Count neutral; exclude | Exclude | A missing axis is not a neutral observation |
| D3 | Weakest tie | Higher code; lower code | Lower code | Deterministic and consistent with the axis registry order |
| D4 | Empty timeframe | Include with nulls; omit | Omit | The state has no latest bar; the ladder test covers non-empty series |
| D5 | Event order | Detector order; availability order | Availability, then occurred/type/subtype/source/reference | A total deterministic order across the three detectors |
| D6 | Structure/Divergence scale | Separate scales; shared | Shared configured scale | Both are structural; one scale avoids blending |

## 6. Acceptance criteria

- Every assembled timeframe exposes the §9 fields through its axis bars, context, legs, levels, and events.
- The consensus counts sum to the number of valid directional axes and never include Progression.
- A missing axis is not neutral.
- The weakest code is the lowest valid percentile; a tie goes to the lower code.
- The four-timeframe ladder assembles without panic, including short higher timeframes.
- A consensus golden fixture and a determinism test pass.
- `fmt`, `clippy -D warnings`, `cargo test`, and `doc -D warnings` are clean.

## 7. Work breakdown

1. Read contract §9, §11, and the aggregate/event/axis APIs.
2. Add `state.rs` with the input, config, per-timeframe state, consensus, and assembled state.
3. Add six state tests: single timeframe, four-timeframe ladder, consensus counts, consensus fixture, empty state, determinism.
4. Generate the consensus golden fixture.
5. Clarify contract §11 for the per-timeframe consensus and tie rule.
6. Run the full workspace checks.
7. Commit, push, open the PR, move #28 to Done, merge, and clean up.

## 8. Verification

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --all-targets -- -D warnings`: clean.
- `cargo test --workspace`: 658 passed, 0 failed.
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`: clean.
- State tests: 6 passed.

The consensus fixture has three rows covering all three counts, missing axes, threshold boundaries, and the weakest-code tie. The four-timeframe ladder test builds M1/M5/M15/H1 through the data engine from the readings fixture.

## 9. Risks and recovery

| Risk | Mitigation |
|---|---|
| A missing axis is counted as neutral | The counts only iterate present values; a missing-axis case is tested |
| The consensus mixes timeframes | One consensus per timeframe; the ladder test asserts four records |
| The weakest tie is unstable | Lower code wins; a fixture row covers it |
| Short higher timeframes panic | The ladder test includes H1 with a short series |
| Event order is unstable | Total sort key; the assembly test checks non-decreasing availability |
| The full state CSV is absent | The field set is covered by the axis/event/context golden tests; #30 can add a full fixture |

## 10. Stop conditions

- A contract change beyond the recorded clarification.
- A new dependency.
- An axis/event formula change outside #28.
- A failing check that cannot be corrected within scope.
