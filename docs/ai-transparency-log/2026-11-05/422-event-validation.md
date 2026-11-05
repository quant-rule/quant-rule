# AI Transparency Log — Event Validation and Precedence (#422)

- Date: 2026-11-05 (UTC)
- Tool: OpenCode agent session (DeepSeek V4.1 Flash)
- Work item: event validation and precedence
- Epic: [#405](https://github.com/quant-rule/quant-rule/issues/405)
- Plan: [`docs/execution-plans/2026-11-05/405-indicator-platform.md`](../../execution-plans/2026-11-05/405-indicator-platform.md)
- Status: event validation suite implemented and passing

## Intent and boundary

Prove the invariants every event batch promises: availability never precedes
occurrence, a batch is in availability order, one underlying occurrence is
recorded once, the v4 level reference order is fixed, and appending bars never
changes a past record. Add regression fixtures for the extended and session
event batches.

The suite is validation only. It does not change the model, the registry, or the
event formulas.

## Implemented work

| Item | Detail |
|---|---|
| `tests/event_validation.rs` | Availability, order, uniqueness, level precedence, causality, determinism |
| `extended_events_golden.csv` | 1,744 extended event records over the 260-bar fixture |
| `session_events_golden.csv` | 4 session event records over the same fixture with the London calendar |
| Panel validation | A three-instrument panel checks availability and occurrence keys |
| Checklist | Table B `A` and `N` checked; the suite section added |

The extended and session fixtures are regression locks generated from the
implementation. They are not independent oracles, unlike the v4 level,
divergence, and signal fixtures. An ignored test regenerates them deliberately:

```bash
cargo test -p quantrule-model --test event_validation -- --ignored regenerate_event_fixtures
```

## Verification and evidence

Observed on 2026-11-05:

| Command | Result |
|---|---|
| `cargo test -p quantrule-model --test event_validation` | 7 tests pass, 1 ignored regeneration test |
| `cargo test -p quantrule-model panel_signals` | 1 panel signal test passes |
| `cargo test --workspace` | all result groups pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo fmt --check` | clean |
| `python3 tools/check_registry.py` | 46 indicators, 138 events, 110 parameters |

## Learning and rejected suggestions

- The v4 batch has no set-level dedup, so the suite checks full-record
  uniqueness and the v4/extended disjointness; the extended batch also checks
  its documented occurrence key.
- The panel fixture was lengthened and its slopes diverged after bar 30 so the
  co-movement and dispersion signal branches actually fire; a passing test that
  never reached those branches would not prove them.
- `P` (precedence) stays open for the extended families. The level-reference
  order is a v4 level property; the extended batch proves one occurrence per
  key, which is a different invariant.
- The golden fixtures are regression locks, not independent oracles; the log and
  the fixture headers say so rather than implying an independence that was not
  built.
