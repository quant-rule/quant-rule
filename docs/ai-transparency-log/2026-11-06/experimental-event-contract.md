# AI Transparency Log — Experimental Event Contract

- Date: 2026-11-06 (UTC)
- Plan: [`docs/execution-plans/2026-11-06/experimental-event-contract.md`](../../execution-plans/2026-11-06/experimental-event-contract.md)
- Tool: OpenCode agent (DeepSeek V4.1 Flash), single session
- Intent and boundary: correct the status and validation claim for `signal.rsi_zone_change`; no behavior change.

## 1. Preflight

`signal.rsi_zone_change` is emitted by the RSI detector and used by historical M2.2/M2.5 specs. It had `status: experimental` and no validation claim. The current total was 143 events, 142 active, 188 claims.

## 2. Implemented work

- Changed `signal.rsi_zone_change` from `experimental` to `active`.
- Added a pending `path_directional` claim with the same shape as the other signal events.
- Bumped registry and validation versions to `2026-11-06.2` and updated `REGISTRY_VERSION`.
- Regenerated `indicators.md` and updated the current counts in `README.md`, `docs/README.md`, and `indicator-coverage.md`.

## 3. Verification

Observed on 2026-11-06:

| Check | Result |
|---|---|
| `check_registry.py` | pass; 46 indicators, 143 events, 101 parameters |
| `check_validation.py` | pass; 189 claims for 46 indicators and 143 active events |
| `cargo fmt`, `cargo clippy`, `cargo test`, `cargo doc` | pass |
| `check_links.py`, `check_terms.py`, `check_legacy.py`, `check_file_size.py` | pass |
| `git diff --check` | pass |

No model output or fixture changed.

## 4. Learning and corrections

- The event was not a pending implementation: it was emitted by the RSI detector and used by historical M2.2/M2.5 specs. The correct correction was to give it a pending claim, not to gate it out.
- The claim is `pending`; no result, grade, or edge is invented.
- The active count moved from 142 to 143 and the claim count from 188 to 189.
