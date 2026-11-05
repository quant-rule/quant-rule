# AI Transparency Log — Indicator Platform (#405)

- Date: 2026-11-05 (UTC)
- Tool: OpenCode agent session (DeepSeek V4.1 Flash)
- Work item: indicator completeness and model-spec platform foundation
- Execution plan:
  [`docs/execution-plans/2026-11-05/405-indicator-platform.md`](../../execution-plans/2026-11-05/405-indicator-platform.md)
- Coverage:
  [`docs/model/2026-11-05/indicator-coverage.md`](../../model/2026-11-05/indicator-coverage.md)
- Checklist:
  [`docs/improvement/2026-11-05/INDICATOR-VERIFICATION.md`](../../improvement/2026-11-05/INDICATOR-VERIFICATION.md)
- Status: first batch implemented and tested; remaining phases tracked as Issues
  #414–#430

## Intent and boundary

The human approved the 10-area, 36-indicator programme and the v6–v10 swarm on
2026-11-05. This session established the registry contract and implemented the
first batch: rolling VWAP and rolling volume profile, with level, divergence, and
signal events.

The agent did not implement the remaining 34 indicators, the spec engine, or the
swarm. It did not run a strategy or accept a baseline. All inputs are OHLCV and
time; no statistics or fitted coefficients enter a reading or event.

## Implemented work

| Item | Status |
|---|---|
| Registry schema requires an event `formula` | Done |
| Source codes extended to 1–54; signal subtypes to 1–23 | Done |
| New taxonomies for volume, participation, flow, price action, path, cross-asset, session, calendar | Done |
| VWAP reading: rolling, distance, slope, MAD bands | Done |
| Volume-profile reading: POC, VAH/VAL, skew, concentration, migration | Done |
| VWAP events: test/cross/break, four divergences, reclaim/rejection/slope | Done |
| Profile events: POC/VA test/break/re-entry, POC divergences, VA breakout/POC cross/skew/concentration | Done |
| Extended batch separated from `TimeframeModel::events()` | Done |
| Event CSV publishes both v4 and extended batches | Done |
| Registry entries for 12 indicators and 40 events | Done |
| Remaining 34 indicators | Todo (#414–#418) |

## Verification and evidence

Observed on 2026-11-05:

| Command | Result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo test --workspace` | all test binaries pass |
| `python3 tools/check_registry.py` | registry ok: 12 indicators, 40 events |
| `cargo test -p quantrule-model extended` | 10 extended tests pass |
| `cargo test -p quantrule-analytics` | 87 unit tests pass |
| `cargo test -p quantrule-shell` | all binaries pass |

The extended tests cover the VWAP formula, zero-volume nulls, profile POC and
concentration, flat-range nulls, causality (append future bars), and repeated
computation. The event tests cover a VWAP level cross, empty/short series,
causality, and determinism, plus the occurrence-key dedup for reclaim/rejection.

## Learning and rejected suggestions

- **Extended events must be additive.** Publishing them in
  `TimeframeModel::events()` changed the legacy report, dashboard, analytics, and
  store fixtures. The fix is a separate `extended_events()` batch, with the CSV
  publishing both. The v4 event domain stays stable.
- **The flat report must keep the v4 event domain.** Filtering it with
  `is_legacy_axis_event` dropped DI, trend, and compression events; the correct
  filter is source < 19.
- **The 23-bin signal subtype array is for the event aggregate, not the flat
  report.** The flat report keeps its five legacy bins; the event section uses
  23.
- **Occurrence identity is one event per timeframe, type, subtype, source, bar,
  and reference.** A VWAP reclaim and rejection on the same bar collapse to the
  first (reclaim); the occurrence key dedup enforces it.
- **Source and subtype ranges are contract values.** The store, strategy
  boundary, and analytics dictionary all needed the extension to 1–54 and 1–23.
- **Volume uses ratio/profile features.** Absolute-volume rules are not used;
  the unit shifts are disclosed in the coverage doc.
- Adding the remaining indicators before the spec engine was rejected: the
  registry/coverage contract and the first vertical slice are the prerequisite.
- Claiming the full 36-indicator checklist complete was rejected; only the first
  two indicators are implemented, and the rest are tracked as open issues.
