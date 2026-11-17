# AI transparency log — Session VWAP availability (P2.1, AUD-45)

- Date: 2026-11-17 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: [Audit Workflow](../../workflows/audit-workflow.md) stage 3
  (Execution) for `AUD-2026-11-17-45`.
- Claim type: Behaviour Claim.
- Intent: stop the session-anchored VWAP from treating a missing candle volume
  as zero. The session VWAP feeds `level.session_vwap_test`; a fabricated value
  could make that signal event fire on a measurement the market did not supply.
- Boundary: the work is on branch `fix/audit-session-vwap`, based on current
  `main` `7ae3a984`. No push, PR, merge, or board change. The Quant Validator
  pass is sequential by the same agent, not independent.

## What changed

- `sessions.rs` keeps a per-session `volume_complete` flag. A candle with
  `volume() == None` sets it false; the session VWAP is `None` from that bar to
  the end of the session. A new session starts with a fresh flag and can publish
  a VWAP again.
- A zero volume is still a valid value: it contributes zero to the weighted sum
  and the baseline, and the VWAP remains unavailable only when the baseline is
  zero. The distinction is missing versus measured zero.
- The normal-volume session golden fixture is unchanged. The regression test
  `a_missing_volume_makes_the_session_vwap_unavailable` sets one bar inside the
  London session to `None` and asserts the VWAP is available before it, null at
  and after it, and available again in the next session.

## Verification observed

| Command | Result |
|---|---|
| `cargo test -p quantrule-model --lib` | 469 passed, 0 failed |
| `cargo test -p quantrule-model --tests --no-fail-fast` | all suites pass; fixture/benchmark ignores unchanged |
| `cargo test -p quantrule-model --lib sessions::` | 7 passed, 0 failed |
| `cargo fmt --all -- --check` | pass |
| `tools/gate.sh` | pass: 2,322 passed, 0 failed, 16 ignored; 25 checkers |

## Quant validation (sequential, not independent)

- **Expected invariant:** a volume-weighted reading whose window contains a
  missing volume is `null`, never zero (`docs/model/2026-11-05/indicator-coverage.md:51-53`;
  `AGENTS.md` §3). The prior critical `AUD-2026-11-16-01` fixed the extended
  readings; the session path was missed.
- **Observed before:** `sessions.rs` used `candle.volume().unwrap_or(0.0)` and
  published `session_vwap` from the zero-filled sum.
- **Observed after:** the session has a `volume_complete` flag; the VWAP is
  `None` from the first missing bar. The event reader
  `events/session.rs::session_vwap_level` returns `None` when the VWAP is
  absent, so `level.session_vwap_test` cannot fire on the fabricated value.
- **Normal data:** the session golden fixture has complete volume; its values
  and the event output are unchanged. No accepted model or strategy baseline
  uses a missing-volume session; the accepted compression baselines are
  unaffected and no `INVALIDATIONS.md` row is added.
- **Residual risk:** a price-only source now loses the session VWAP for the rest
  of each session, which is the correct unavailable state; a strategy that
  relied on the fabricated value will stop seeing the event.
- **Independence:** the Quant Researcher/Developer/Validator roles were
  performed sequentially by the same agent. This is not an independent
  validation and is not claimed as one.

## Learning

- The session VWAP is a separate accumulator from the extended readings; the
  prior critical fix did not cover it. The audit's finding was a real recurrence
  of the same class, not a new rule.
- The `volume_complete` flag is per session, so the next session is not
  permanently poisoned by one missing bar. That matches the session-anchored
  definition.
- The event code already handled a `None` VWAP; the defect was only the
  fabricated Some value, so no event code change was needed.
