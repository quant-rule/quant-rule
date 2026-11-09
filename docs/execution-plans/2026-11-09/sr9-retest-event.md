# SR9 Retest Event — `signal.breakout_retest_hold`

- Date: 2026-11-09 (UTC)
- Status: **Implemented**
- Programme: `SR9-15M-STRUCTURE`
- Claim type: model measurement; no strategy result

## 1. Intent

Add the second confirmation the structure-transition strategy needs: after a
compression breakout, wait for a retest that holds instead of entering on the
break alone.

## 2. Current state

- `signal.compression_breakout` is a registered signal event. It requires a
  close beyond a coiling range by 0.2 ATR with RVOL at least 1.2.
- The event publishes the breakout direction and the broken boundary.
- No event waits for a retest. The strategy engine cannot express an ordered
  break → retest → hold sequence.

## 3. Scope

Included:

- A `BreakoutRetestHoldSeries` detector consuming the compression breakouts.
- The fixed retest window (4 bars) and tolerance (0.2 ATR).
- Event code 146, subtype 24, source 5.
- Registry, validation, verification, and emitter updates.
- Unit and integration tests.

Excluded:

- A new model configuration flag or threshold; the event runs under the
  existing compression-breakout flag.
- A strategy run.
- A change to the compression-breakout event.

## 4. Definition

A break at `b0` is followed by a retest when a bar in `b0+1..=b0+4` touches the
broken boundary within 0.2 ATR and closes back on the breakout side. A close
through the boundary against the breakout before the hold invalidates the break.
One event per break.

## 5. Acceptance criteria

- The event fires at the hold bar and is knowable there.
- A failed retest emits nothing.
- A no-touch bar emits nothing.
- A retest after the window emits nothing.
- The short side is symmetric.
- Causality and determinism hold.
- Existing event values are unchanged.
- Registry, validation, verification, and emitter checks pass.

## 6. Verification

- `cargo test`
- `cargo clippy --all-targets -- -D warnings`
- `cargo doc` with warnings denied
- `cargo fmt --check`
- `python3 tools/check_registry.py`
- `python3 tools/check_validation.py`
- `python3 tools/check_verification.py`
- `python3 tools/check_event_emitters.py`
- Repository checks and `git diff --check`

## 7. Risks and stop conditions

Risks: the retest reduces trade count; a false hold can still fail. The
break-only control and the stop/time stop bound both.

Stop if the event cannot be made causal, if it changes an existing event, or if
the registry cannot represent the new subtype.
