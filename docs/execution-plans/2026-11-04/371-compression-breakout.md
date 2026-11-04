# Execution Plan: Compression-Breakout Range Expansion

- Date: 2026-11-04 (UTC)
- Status: **Verified — inconclusive; validation not opened**
- Issue: [#371](https://github.com/quant-rule/quant-rule/issues/371)
- Protocol: [`docs/model/2026-11-04/compression-breakout.md`](../../model/2026-11-04/compression-breakout.md)
- AI transparency log:
  [`docs/ai-transparency-log/2026-11-04/371-compression-breakout.md`](../../ai-transparency-log/2026-11-04/371-compression-breakout.md)
- Claim type: model evidence for forward range expansion; no strategy, cost, or
  edge claim.

## 1. Intent

Test the model claim the prior program never tested: that a coiling volatility
regime followed by a confirmed compression breakout predicts a larger forward
range, not a direction. The result is a model baseline or no edge. No strategy
or portfolio work starts without an accepted model baseline.

## 2. Current state

- The v3 and v4 gates tested directional claims; all 12 v4 cells were grade C.
- The model contract already names a non-directional claim: ADX/context →
  forward range expansion, not direction.
- The context reading (`atr_ratio`, `rvol_ratio`, `context_code`) is published
  but is diagnostic; no event turns it into a timing signal.
- The model_validation partition 2021–2022 is unopened for this hypothesis.

## 3. Scope

Included:

- A new signal event `signal.compression_breakout` (type 3, subtype 7, source
  ATR 5) with a coiling window, a compression range, a breakout distance, and
  an RVOL confirmation.
- v4 config fields and defaults for the event.
- Registry event, parameters, subtype code, and validation claim.
- A range-expansion harness with matched coiling, placebo-transition, random,
  and lagged controls.
- A development matrix on BTCUSD, ETHUSD, XAUUSD, 2018–2020, and one-shot
  validation in 2021–2022 only if development passes.

Excluded:

- Directional claims and any threshold/timeframe rescue.
- Strategy, execution, portfolio, and cost claims.
- 4h and higher timeframes.
- New indicators outside the registered families.
- Post-hoc threshold changes.

## 4. Assumptions and uncertainty

- Volatility clusters; compression precedes expansion; a breakout with
  participation confirms the expansion.
- Expansion may be predictable while the breakout direction is not; the model
  claim is deliberately direction-neutral.
- The event may be too rare at 1h; the primary timeframe is chosen from event
  counts only, before any outcome is computed.
- The same instruments were used for directional tests; the new outcome is
  different, and the validation partition is unopened. Residual snooping risk
  remains.

## 5. Decisions

| Decision | Choice |
|---|---|
| Event | Compression breakout: K coiling bars, close beyond range by 0.2 ATR, RVOL ≥ 1.2 |
| K (coiling bars) | 5 |
| Break distance | 0.2 ATR |
| RVOL confirmation | 1.2 |
| Outcome | Forward range over 20 bars ≥ 2 ATR |
| Primary timeframe | 1h if dev event count ≥ 200, otherwise 15m; count-only switch |
| Secondary timeframe | The other of 1h/15m, descriptive |
| Baselines | All bars; coiling bars; placebo transition (coiling→expanding without breakout) |
| Controls | Random coiling bars, 20-bar back-lag |
| Pass | n ≥ 200, J ≥ 20, lift vs coiling ≥ +5pp, corrected LCB > 0, controls pass, 2/3 instruments |
| Validation | One-shot 2021–2022 only after freeze |
| Strategy | Only after human acceptance of the model baseline |

## 6. Work breakdown

| Phase | Work | Paths | Role |
|---|---|---|---|
| 0 | Plan, protocol, AI log | `docs/execution-plans/`, `docs/model/`, `docs/ai-transparency-log/` | Orchestrator |
| 1 | Event and settings | `crates/quantrule-model/src/events/compression.rs`, `signal.rs`, `input.rs`, `timeframe.rs` | Quant Developer |
| 2 | Config and result | `crates/quantrule-shell/src/config/model_v4.rs`, `render_report/json.rs`, defaults | Quant Developer |
| 3 | Registry and contract | `docs/registry/`, `docs/model/2026-11-04/` | Quant Developer |
| 4 | Range harness | `tools/improvement/compression_validation.py`, tests | Test Engineer |
| 5 | Development matrix | `config/compression/`, `data/model-v4/compression/` | Quant Developer |
| 6 | Gate and decision | `docs/improvement/2026-11-04/`, `data/improvement/` | Result Analyst / Triage |
| 7 | One-shot validation | Configs and run record, only on a dev pass | Quant Validator |

## 7. Verification

Observed:

- `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test --workspace`, and `cargo doc --workspace --no-deps` passed.
- Event causality, determinism, and edge-case tests passed; the range harness
  unit and end-to-end tests passed (94 improvement-tooling tests total).
- Config parser tests for the new event fields and defaults passed.
- Registry, validation, link, and term checks passed.
- A regenerated BTCUSD 2018 bundle is byte-identical to the matrix bundle for
  `events.csv`, `readings.csv`, and `result.json`.
- The development gate is recorded; validation was not opened.

## 8. Risks and recovery

- Expansion may be predictable but the event may be too rare; then the result
  is inconclusive, not a reason to tune thresholds.
- The event may not beat the coiling or placebo baselines; then the program
  records no edge and stops.
- Adding a signal subtype is a model contract change; the registry and docs are
  updated in the same change.
- Local matrix artifacts are not committed; the committed record names the data
  versions, configs, and run commands.

## 9. Stop conditions

- No dev pass: record no edge; do not open validation.
- Underpowered sample: record inconclusive; do not lower the threshold.
- Validation failure: record development-only; stop.
- Model pass but no human baseline acceptance: stop before strategy.
- A required decision outside this plan: stop and ask.

## 10. Result

The event, v4 config fields, registry claim, range harness, and development
matrix are implemented. The development gate ran on nine one-year bundles.

- 1h primary: 302 events, 290 clusters, hit 0.9967, coiling base 0.9888,
  lift +0.0079, corrected LCB +0.0004, random p 0.1443, back-lag lift −0.0119,
  replication 3.
- 15m secondary: 1,355 events, 1,274 clusters, hit 0.9786, coiling base
  0.9583, lift +0.0203, corrected LCB +0.0112, random p 0.0050, back-lag lift
  −0.0129, replication 3.
- The 2 ATR / 20-bar outcome saturates: the maximum possible lift is 1.12pp at
  1h and 4.17pp at 15m, below the pre-registered 5pp threshold.

Verdict: **inconclusive — outcome saturated**. The validation partition was not
opened, no baseline was accepted, and no strategy or portfolio work started.
The 15m secondary result is descriptive and is not promoted.

- Gate: [`COMPRESSION-GATE`](../../improvement/2026-11-04/COMPRESSION-GATE.md)
- Run record: [`MODEL-2026-11-04-02`](../../improvement/2026-11-04/MODEL-2026-11-04-02-compression-breakout.md)

## 11. Acceptance criteria

| Criterion | Result |
|---|---|
| Event and v4 config fields implemented | Met |
| Registry event, parameters, subtype, and claim | Met |
| Range harness with baselines and controls | Met |
| Development matrix recorded | Met: 9 one-year bundles, 1h/15m |
| Development pass or no-edge/inconclusive recorded | Met: inconclusive (saturation) |
| One-shot validation | Not opened: no development pass |
| No strategy or portfolio work | Met |
