# AI Transparency Log — Missing Event Batch (#405)

- Date: 2026-11-05 (UTC)
- Tool: OpenCode agent session (DeepSeek V4.1 Flash)
- Work item: BOS/CHOCH, session-VWAP, and opening-range level events
- Epic: [#405](https://github.com/quant-rule/quant-rule/issues/405)
- Plan: [`docs/execution-plans/2026-11-05/405-indicator-platform.md`](../../execution-plans/2026-11-05/405-indicator-platform.md)
- Status: events implemented, registered, and covered by the event fixtures

## Intent and boundary

Close the event gaps recorded in the verification checklist without inventing a
second definition for an event that already exists. The batch adds:

- `level.bos` and `level.choch` (Level subtypes 4 and 5, source 36).
- `level.session_vwap_test` (Level subtype 1, source 50).
- `level.opening_range_test` and `level.opening_range_break` (Level subtypes 1
  and 3, source 51).

`level.strength_break` is the same test event as `level.strength_test`; the
panel's `rs_regime` and `lag_divergence` names are reconciled to the existing
one-cross-per-reading events. `signal.structure_change` is shared by
`swing_structure` and `bos_choch`.

## Implementation

| Item | Detail |
|---|---|
| BOS/CHOCH | Classified by the prior `structure_state`; a neutral prior state emits neither |
| Session levels | `SessionEventSeries` now takes `Readings` for ATR and publishes the VWAP test |
| Opening range | Test and break levels publish after the opening range completes |
| Registry | 143 events; level subtypes 4 and 5 added; version `2026-11-05.15` |
| Fixtures | Extended and session event fixtures regenerated deliberately |

## Quant role passes

The plan approved the event definitions (Quant Researcher pass). The
implementation and the fixture regeneration are the Quant Developer pass. The
Quant Validator pass is the independent reproduction in this same session:
`cargo test --workspace`, the event validation suite, causality, determinism,
and the regenerated fixtures. These were sequential passes by one agent, not
independent model judgments.

## Verification and evidence

| Command | Result |
|---|---|
| `cargo test -p quantrule-model events::session` | 6 session tests pass |
| `cargo test -p quantrule-model --test event_validation` | 7 tests pass; new level keys exercised |
| `cargo test --workspace` | all result groups pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo fmt --check` | clean |
| `python3 tools/check_registry.py` | 46 indicators, 143 events, 110 parameters |

The extended fixture exercises 3 BOS and 2 CHOCH events; the session fixture
exercises 15 session-VWAP tests and one opening-range test and break.

## Learning and rejected suggestions

- BOS and CHOCH need distinct subtypes. Using the same Level subtype and source
  for both would make the spec predicate and the occurrence key unable to tell
  them apart. Subtypes 4 and 5 are additive to the v4 level contract.
- The opening-range test uses the bar high/low against the range boundary, not
  the close. A monotonic fixture never touched the range, so the focused test
  moves one bar back to the boundary.
- The session close filter now restricts to Signal subtype 23; the new session
  VWAP level event shares source 50.
