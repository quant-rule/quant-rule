# Experimental Event Contract — RSI Zone Change

- Date: 2026-11-06 (UTC)
- Status: Verified
- Issue: none (human-approved task slug)
- Branch: `fix/experimental-event-contract`
- Worktree: `../quant-rule--experimental-event-contract`
- Claim type: registry and validation-claim correction. No model behavior change.

## 1. Intent

Remove the last experimental event from the current registry. `signal.rsi_zone_change` is implemented and emitted, is used by historical M2.2/M2.5 specs, and has a construct test. It is not a pending implementation; it is an active event that was missing its validation claim. Promote it to `active` and add a pending predictive claim.

## 2. Current state

- `docs/registry/indicators.json` marks `signal.rsi_zone_change` as `experimental`.
- `docs/registry/validation.json` has no claim for it.
- The event is emitted by `crates/quantrule-model/src/events/signal.rs`.
- The construct test is `signal::tests::ema_stack_and_rsi_zone_changes_are_published`.
- Current counts: 143 events, 142 active, 188 claims.

## 3. Scope

Included:

- Change the registry status to `active`.
- Add a `path_directional`, `pending` claim with the same shape as the other signal events.
- Bump registry and validation versions to `2026-11-06.2`; update `REGISTRY_VERSION`.
- Regenerate `indicators.md`; update current counts in `README.md`, `docs/README.md`, and `docs/model/2026-11-05/indicator-coverage.md`.

Excluded:

- Any change to the RSI event formula, thresholds, timing, or output.
- Any claim that the event has an edge; the claim remains `pending`.
- Any change to historical M2 records.

## 4. Decision

Option A (recommended): promote to active with a pending claim. Option B (reject experimental events from `active_events`) would invalidate historical M2.2/M2.5 reruns and add a model-spec rule that has no current need. Option A is the honest correction.

## 5. Acceptance criteria

- `signal.rsi_zone_change` has `status: active`.
- `validation.json` has exactly one claim for it, with `primary_outcome: path_directional`, `predictive_status: pending`, `horizon_bars: 20`, `target_atr: 1.0`, `stop_atr: 1.0`, `baseline: matched_barrier`.
- `check_registry.py` reports 46 indicators, 143 events, 101 parameters.
- `check_validation.py` reports 189 claims for 46 indicators and 143 active events.
- No model output or fixture changes.
- `cargo test --workspace`, `cargo clippy`, `cargo doc`, and the Python gates pass.

## 6. Verification

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --workspace
RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps
python3 tools/check_registry.py
python3 tools/check_validation.py
python3 tools/check_links.py
python3 tools/check_terms.py
python3 tools/check_legacy.py
python3 tools/check_file_size.py
git diff --check
```

## 7. Roles and risks

Quant Researcher approves the claim wording; Quant Developer confirms no behavior change; Quant Validator records that the claim is pending, not a pass. The only risk is a count/documentation mismatch, caught by the two registry gates.


## 8. Verification record

Observed on 2026-11-06 in the task worktree:

| Check | Result |
|---|---|
| `python3 tools/check_registry.py` | pass; 46 indicators, 143 events, 101 parameters |
| `python3 tools/check_validation.py` | pass; 189 claims for 46 indicators and 143 active events |
| `cargo fmt --all -- --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| `cargo test --workspace --no-fail-fast` | pass; all test binaries and doctests, 0 failed |
| `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps` | pass |
| `check_links.py`, `check_terms.py`, `check_legacy.py`, `check_file_size.py` | pass |
| `git diff --check` | pass |

No model output or fixture changed. The event remains emitted exactly as before; only its registry status and validation metadata changed.
