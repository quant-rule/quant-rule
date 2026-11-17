# AI transparency log — One-bar Donchian window (P2.3, AUD-47)

- Date: 2026-11-17 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: [Audit Workflow](../../workflows/audit-workflow.md) stage 3
  (Execution) for `AUD-2026-11-17-47`.
- Claim type: Behaviour Claim.
- Intent: make a valid one-bar Donchian configuration safe at bar zero. The
  position event read `donchian_position[bar - 1]`; a one-bar window makes
  `donchian_position[0]` available, so the old code underflowed in debug and
  wrapped in release. The Keltner position used the same unguarded pattern.
- Boundary: branch `fix/audit-donchian-window`, based on current `main`
  `7ae3a984`. No push, PR, merge, or board change. The Quant Validator pass is
  sequential by the same agent, not independent.

## What changed

- The Keltner and Donchian position event blocks require `bar > 0` before
  reading the previous bar.
- A regression test sets `donchian_window = 1` on a flat-range series and
  asserts every emitted event is at bar 1 or later. The old code panics at bar
  zero in a debug build.

## Verification observed

| Command | Result |
|---|---|
| `cargo test -p quantrule-model --lib events::extended::tests` | 25 passed, 0 failed |
| `cargo fmt --all -- --check` | pass |
| `tools/gate.sh` | pass: 2,322 passed, 0 failed, 16 ignored; 25 checkers |

## Quant validation (sequential, not independent)

- **Expected invariant:** a valid configuration never panics or silently skips
  an event; a one-bar window is a valid boundary.
- **Observed before:** `donchian_position[0]` is `Some` for a non-zero range,
  and the event read `bar - 1` at `bar == 0`.
- **Observed after:** the read requires `bar > 0`; the event is absent at bar
  zero and works from bar one. The normal-window fixtures are unchanged.
- **Residual risk:** a one-bar window has no previous position at bar zero, so
  no position event can be emitted there; that is the correct boundary.
- **Independence:** the Quant Researcher/Developer/Validator roles were
  performed sequentially by the same agent. This is not an independent
  validation and is not claimed as one.

## Learning

- The underflow was a boundary bug, not a missing event; the guard is the
  smallest correct fix and keeps the config valid.
- The Keltner path is normally unreachable at bar zero because ATR is absent,
  but the same guard is applied so a future availability change cannot expose
  it.
