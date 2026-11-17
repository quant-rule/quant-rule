# AI transparency log — Divergence availability window (P2.2, AUD-46)

- Date: 2026-11-17 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: [Audit Workflow](../../workflows/audit-workflow.md) stage 3
  (Execution) for `AUD-2026-11-17-46`.
- Claim type: Behaviour Claim.
- Intent: make the Divergence event causal when a configuration sets an RSI
  window wider than the pivot lookback. The event records
  `available_bar_index = confirmed pivot bar`; the old read could include RSI
  bars after that bar.
- Boundary: branch `fix/audit-divergence-window`, based on current `main`
  `7ae3a984`. No push, PR, merge, or board change. The Quant Validator pass is
  sequential by the same agent, not independent.

## What changed

- `rsi_window_value` takes the pivot's `available_at` and clamps the read to
  `min(occurred_at + window, last, available_at)`. Even a caller that bypasses
  the config parser cannot read a future RSI bar.
- The model v4 parser rejects `events.divergence.window` greater than the
  selected pivot scale's lookback. The default major/2 and the existing golden
  fixtures are unchanged.
- Tests: the model exercises a minor window of 6 (above the lookback) and
  asserts every event is available at or after bar 10 and every prefix is
  unchanged. The app parser test refuses major/6 and minor/3, accepts major/5.

## Verification observed

| Command | Result |
|---|---|
| `cargo test -p quantrule-model --lib divergence::` | 16 passed, 0 failed |
| `cargo test -p quantrule-app --lib config::tests` | 27 passed, 0 failed |
| `cargo fmt --all -- --check` | pass |
| `tools/gate.sh` | pass: 2,323 passed, 0 failed, 16 ignored; 25 checkers |

## Quant validation (sequential, not independent)

- **Expected invariant:** an event knowable at bar `available_at` never reads a
  later bar (`AGENTS.md` §3, §4). The default major/2 was safe; a configured
  window above the lookback was not.
- **Observed before:** `end = occurred_at + window` with
  `available_bar_index = current.available_at`; minor/3 and major/6 could read
  after the confirmation bar.
- **Observed after:** the read is clamped to `available_at`, and the config
  parser refuses the out-of-contract window. The prefix test with minor/6 is
  stable; the event cannot move earlier.
- **Normal data:** the default and the golden divergence fixture are unchanged.
  No accepted model or strategy baseline uses a wide divergence window; no
  `INVALIDATIONS.md` row is added.
- **Residual risk:** a config with a window above the lookback is now refused
  rather than silently clamped; that is the intended contract.
- **Independence:** the Quant Researcher/Developer/Validator roles were
  performed sequentially by the same agent. This is not an independent
  validation and is not claimed as one.

## Learning

- A defensive clamp in the detector plus a strict parser is the right pair: the
  parser keeps the config honest, and the clamp makes the public
  `with_window` constructor safe for a direct caller.
- The existing available-bar test covered the default window; the new
  wide-window prefix test is what would fail if the clamp were removed.
