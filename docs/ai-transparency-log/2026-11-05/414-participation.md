# AI Transparency Log — Participation Readings (#414)

- Date: 2026-11-05 (UTC)
- Tool: OpenCode agent session (DeepSeek V4.1 Flash)
- Work item: participation readings — RVOL extension, volume climax, absorption, volume oscillator
- Epic: [#405](https://github.com/quant-rule/quant-rule/issues/405)
- Plan: [`docs/execution-plans/2026-11-05/405-indicator-platform.md`](../../execution-plans/2026-11-05/405-indicator-platform.md)
- Checklist: [`docs/improvement/2026-11-05/INDICATOR-VERIFICATION.md`](../../improvement/2026-11-05/INDICATOR-VERIFICATION.md)
- Status: implemented and tested

## Intent and boundary

The human asked to continue the indicator programme. This work item implements
the participation area: fast/slow RVOL, volume oscillator, volume climax, and
absorption at VWAP or POC. It adds readings and events only; no strategy or
baseline change.

## Implemented work

| Item | Status |
|---|---|
| `rvol_fast`, `rvol_slow`, `rvol_ratio`, `volume_oscillator` | Done |
| `climax_flag`, `climax_direction`, `climax_strength` | Done |
| `absorption_flag`, `absorption_direction`, `absorption_strength` | Done |
| RVOL expansion/contraction/ratio signals | Done |
| Climax reversal/continuation signals | Done |
| Absorption test/break/rejection events | Done |
| RVOL and climax divergences | Done |
| Registry entries and parameters | Done |
| CSV columns | Done |

## Verification and evidence

Observed on 2026-11-05:

| Command | Result |
|---|---|
| `cargo test -p quantrule-model extended` | 17 extended tests pass |
| `cargo test --workspace` | all test binaries pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `python3 tools/check_registry.py` | 15 indicators, 53 events, 82 parameters |

The new tests cover the fast RVOL ratio, a climax bar, absorption near VWAP,
participation causality, a volume-expansion event, a climax reversal, and an
absorption break.

## Learning and rejected suggestions

- Fast RVOL, climax, and absorption must publish when the **fast** window is
  available; they must not be blocked by the slow 50-bar window. The loop was
  split so slow/ratio/oscillator are independent.
- The absorption test needs at least the ATR history; a 10-bar fixture with the
  default ATR 14 had no ATR and could not flag. The test uses 20 bars.
- The occurrence key dedup already handles the VWAP reclaim/rejection collision;
  the same generic rule covers the participation family.
- Absolute-volume thresholds were rejected; all participation features are
  ratios or ratios of means.
- Blocking the batch on the 50-bar slow RVOL was rejected after the test showed
  the fast features disappearing on short histories.
