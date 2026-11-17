# AI transparency log — VWAP slope prior index (P2.4, AUD-49)

- Date: 2026-11-17 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: [Audit Workflow](../../workflows/audit-workflow.md) stage 3
  (Execution) for `AUD-2026-11-17-49`.
- Claim type: Behaviour Claim.
- Intent: make `vwap_slope` subtract the VWAP one full window back, as the
  registered formula `S=VWAP-VWAP[n]` states, instead of one bar short.
- Boundary: branch `fix/audit-vwap-slope`, based on current `main` `7ae3a984`.
  No push, PR, merge, or board change. The Quant Validator pass is sequential by
  the same agent, not independent.

## What changed

- `extended.rs` now guards `bar >= config.vwap_window` and reads
  `vwap[bar - window]`. The prior code read `vwap[bar + 1 - window]`, so a
  24-bar window produced a 23-bar change and the first value appeared at bar
  46 instead of bar 47.
- A unit test uses a two-bar VWAP over four candles and asserts the slope at bar
  3 is `vwap[3] - vwap[1]` (`20.0`), not `vwap[3] - vwap[2]` (`10.0`), and that
  bar 2 has no slope because `vwap[0]` is unavailable.
- The fixture regeneration is deliberate: the extended readings golden changes
  only the `vwap_slope` column, and the extended events golden changes only the
  six `signal.vwap_slope` events. No other reading or event row changed.
- The coupled worker baseline digest is updated because the model artifact
  bytes changed. The fixture strategy does not read `signal.vwap_slope`, so the
  change is confined to the model artifacts and the digest derived from them.

## Fixture diff evidence

| Fixture | Change |
|---|---|
| `extended_readings_golden.csv` | 214 `vwap_slope` cells; every other column unchanged; the first value moves from bar 46 to bar 47 and equals `vwap[47] - vwap[23]` |
| `extended_events_golden.csv` | six `signal.vwap_slope` rows (code 29, subtype 14, source 19) move one bar and take the new slope value; no other event code changed |
| worker coupled baseline | the 250-minute `a_queued_run_matches_the_coupled_baseline` oracle changes from `sha256:c23925d4…` to `sha256:81e73e07…`; the test recomputes the artifact set from the fixture and passed twice with the same new digest |

## Verification observed

| Command | Result |
|---|---|
| `cargo test -p quantrule-model --lib readings::extended::tests::vwap_slope` | 1 passed, 0 failed |
| `cargo test -p quantrule-model --test extended_readings_golden --test event_validation` | 13 passed, 0 failed, 2 ignored |
| `cargo test -p quantrule-model --lib --tests` | all suites pass |
| `cargo test -p quantrule-backtest-service --test worker path::a_queued_run_matches_the_coupled_baseline -- --exact` | 1 passed twice; both runs produced `sha256:81e73e07…` |
| `cargo fmt --all -- --check` | pass |
| `tools/gate.sh` | pass: 2,322 passed, 0 failed, 16 ignored; 25 checkers |

## Quant validation (sequential, not independent)

- **Expected invariant:** `S = VWAP - VWAP[n]` uses the value `n` bars before
  the current bar (`docs/model/2026-11-05/indicator-coverage.md:59`;
  `docs/registry/indicators.json` `vwap` formula). Peer readings
  `poc_migration` and `obv_slope` use `bar - window`.
- **Observed before:** `prior = bar + 1 - window`; at bar 24 with window 24 the
  prior was bar 1, a 23-bar change, and the first value was at bar 46.
- **Observed after:** `prior = bar - window`; at bar 47 with window 24 the prior
  is bar 23, a full 24-bar change, and the first value is at bar 47. The new
  golden value at bar 47 is `98.84866546672885 - 99.28397188928562 =
  -0.43530642255677776`.
- **Causality:** the read is still a prior bar; appending later candles cannot
  change an earlier slope. The unit test pins the exact prior index, and the
  existing prefix/causality tests in the extended suite still pass.
- **Event impact:** `signal.vwap_slope` fires on `sign(S[t]) != sign(S[t-1])`.
  Six events move one bar later and change value; the event count is unchanged
  on the fixture. No accepted model or strategy baseline names
  `signal.vwap_slope`; the accepted compression baselines compare compression
  events, so no `INVALIDATIONS.md` row is added.
- **Residual risk:** the first slope is now available one bar later and every
  slope is a full-window change. A consumer that treated the old value as the
  contract output will see different numbers; that is the intended correction.
- **Independence:** the Quant Researcher/Developer/Validator roles were
  performed sequentially by the same agent. This is not an independent
  validation and is not claimed as one.

## Learning

- The fixture locked a contract violation in place, which is why the audit
  required a deliberate regeneration rather than a silent update. The diff was
  first reduced to the affected column and event code, then regenerated.
- The first slope necessarily appears at `2*window - 1` because the prior VWAP
  must itself be available at `window - 1`. A test that only checks a later bar
  would not catch a one-bar prior error; the two-bar fixture with known VWAPs
  does.
- The 2026-11-05 `INDICATOR-AUDIT.md` recorded that the VWAP formula and
  implementation agree. That statement was wrong; this dated fix and its
  fixture diff are the correction, and the historical document is left as
  history rather than rewritten.
