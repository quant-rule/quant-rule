# AI transparency log — Event edge evidence and retest golden (P2.7, AUD-17/24)

- Date: 2026-11-17 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: [Audit Workflow](../../workflows/audit-workflow.md) stage 3
  (Execution) for `AUD-2026-11-17-17` and `AUD-2026-11-17-24`.
- Claim type: Behaviour Claim.
- Intent: make the verification matrix name tests that actually construct
  events for edge evidence, give `signal.breakout_retest_hold` a real golden,
  and stop the checker from accepting a readings-only test as an event-edge
  proof.
- Boundary: branch `fix/audit-event-edge`, based on current `main` `7ae3a984`.
  No push, PR, merge, or board change. This is test and traceability work; it
  does not change a model value, event definition, or strategy result. The
  Quant Validator pass is sequential by the same agent, not independent.

## What changed

- New `tests/event_edge_cases.rs` constructs the model, session, and panel
  event families on empty, single, short, flat, zero-volume, missing-volume,
  equal-tie, and gap series. It asserts every occurrence and availability stays
  inside the series, availability is never earlier than occurrence, the
  timeframe is non-zero, and every published value is finite.
- New `tests/retest_events_golden.rs` and
  `tests/fixtures/retest_events_golden.csv` cover a held long retest, a held
  short retest, and an invalidated break that emits nothing. The test also
  checks that appending bars does not change an earlier retest.
- `docs/registry/verification.json` now points every event's `edge` evidence at
  `event_edge_cases::event_batches_are_safe_on_edge_inputs`, and the retest
  event's `golden` evidence at
  `retest_events_golden::the_retest_events_match_the_golden_fixture`. The 30
  extended indicators also name the family edge tests that exercise their
  readings.
- `tools/check_verification.py` now scans test bodies and their directly called
  helpers and rejects an event `edge` entry that names no test constructing an
  `EventRecord` or reading an `.events()` batch. `tools/test_check_verification.py`
  adds the seeded rejection case.

## Verification observed

| Command | Result |
|---|---|
| `cargo test -p quantrule-model --test event_edge_cases` | 1 passed, 0 failed |
| `cargo test -p quantrule-model --test retest_events_golden` | 2 passed, 0 failed, 1 ignored |
| `python3 tools/check_verification.py` | 49 indicators, 150 events; all required classes covered |
| `python3 tools/check_validation.py` | 199 claims for 49 indicators and 150 active events |
| `python3 tools/check_registry.py` | 49 indicators, 150 events, 100 parameters |
| `python3 tools/test_check_verification.py` | 6 passed, including the readings-only rejection |
| `python3 tools/check_event_emitters.py` | 150 registered codes referenced or family-computed |
| `cargo fmt --all -- --check` | pass |
| `tools/gate.sh` | pass: 2,324 passed, 0 failed, 17 ignored; 25 checkers |

## Quant validation (sequential, not independent)

- **Expected invariant:** every active event has edge-case coverage, and the
  evidence names a test that constructs events (`AGENTS.md` §4; finding
  `AUD-2026-11-17-24`). A row-scoped edge entry names the proof, not only a
  readings helper (`AUD-2026-11-17-17`).
- **Observed before:** all 150 events pointed at
  `edge_cases::an_empty_series_publishes_no_readings`, which constructs no event
  stream; the retest event pointed at a signal fixture with subtypes 1–3 only.
- **Observed after:** the event edge suite runs the model, session, and panel
  detectors on the degenerate series; the retest fixture emits subtype 24 for
  both directions; the checker rejects the old readings-only name.
- **Model behavior:** unchanged. This work adds tests and evidence pointers; it
  edits no detector, reading, event definition, or strategy input.
- **Residual risk:** the checker's event-construction test is a source scan, not
  a semantic proof. It inspects the test body and directly called helpers for
  `EventRecord` or `.events()`; a future test could call an event helper through
  a deeper call chain and be missed, or name `EventRecord` without asserting on
  it. The named tests in this tree construct and assert real event batches.
- **Independence:** the Quant Researcher/Developer/Validator roles were
  performed sequentially by the same agent. This is not an independent
  validation and is not claimed as one.

## Learning

- The verification matrix was structural: it checked that a test with the right
  name existed, not that the test exercised the event path. Scanning the test
  body and its direct helpers closes the specific gap without pretending to
  prove semantics.
- The first retest fixture placed the long and short breaks on the same bar. The
  short scan correctly invalidated that setup on the next bar, which closed
  above the short boundary; moving the short break to its own bar produced the
  intended retest. The detector's invalidation rule was not changed.
- The retest event is in the merged model event batch, not in
  `SignalEventSeries`; the new fixture targets `BreakoutRetestHoldSeries`
  directly, so it proves the event without pretending the signal fixture covers
  it.
