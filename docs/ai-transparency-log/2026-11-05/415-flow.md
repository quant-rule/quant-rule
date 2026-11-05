# AI Transparency Log — Volume-Price Flow (#415)

- Date: 2026-11-05 (UTC)
- Tool: OpenCode agent session (DeepSeek V4.1 Flash)
- Work item: volume-price flow readings — OBV, CMF, MFI, A/D line
- Epic: [#405](https://github.com/quant-rule/quant-rule/issues/405)
- Plan: [`docs/execution-plans/2026-11-05/405-indicator-platform.md`](../../execution-plans/2026-11-05/405-indicator-platform.md)
- Checklist: [`docs/improvement/2026-11-05/INDICATOR-VERIFICATION.md`](../../improvement/2026-11-05/INDICATOR-VERIFICATION.md)
- Status: implemented and tested

## Intent and boundary

Continue the indicator programme with the volume-price flow area. The work adds
readings and events only; no strategy or baseline change.

## Implemented work

| Item | Status |
|---|---|
| OBV, OBV mean, OBV slope | Done |
| CMF and CMF slope | Done |
| MFI | Done |
| A/D line and A/D mean | Done |
| OBV/CMF/MFI/A-D events and divergences | Done |
| Registry entries and parameters | Done |
| CSV columns | Done |

## Verification and evidence

Observed on 2026-11-05:

| Command | Result |
|---|---|
| `cargo test -p quantrule-model extended` | 23 extended tests pass |
| `cargo test --workspace` | all 49 result groups pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `python3 tools/check_registry.py` | 19 indicators, 74 events, 85 parameters |

The new tests cover signed OBV accumulation, CMF at +1 when every close is the
high, MFI at 100 on a rising series, flow causality, an OBV/MA cross, and a CMF
zero cross.

## Learning and rejected suggestions

- Flow means and slopes need the flow window; MFI needs its own window. The
  defaults are 20 and 14.
- A flat bar contributes zero money-flow volume rather than a null; the ratio
  remains defined when total volume is positive.
- Clippy required let-chains for the nested slope and MFI-zone conditions; the
  logic is unchanged.
- Absolute money-flow thresholds were rejected; CMF and MFI are normalized.
- Publishing flow features without a volume window was rejected: OBV is defined
  from bar 1, but its mean and slope need the window.
