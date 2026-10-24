# Execution Plan: The Indicator Readings Layer

- Issue: [#18](https://github.com/quant-rule/quant-rule/issues/18) — feat: build the indicator readings layer
- Branch: `feat/18-indicator-readings`
- Pull request: [#59](https://github.com/quant-rule/quant-rule/pull/59)
- Status: Verified
- Date: 2026-10-24 (UTC)
- Program plan: [Completion plan (M2–M14)](14-model-engine-completion.md)
- Contract: [`docs/model/2026-10-21/four-axis-analytical-model.md`](../../model/2026-10-21/four-axis-analytical-model.md)
- Umbrella plan: [`docs/execution-plans/2026-10-21/14-model-engine.md`](../2026-10-21/14-model-engine.md)
- Transparency log: [`docs/ai-transparency-log/2026-10-24/18-indicator-readings.md`](../../ai-transparency-log/2026-10-24/18-indicator-readings.md)

## 1. Intent

Build the model engine's calculation inputs as pure per-bar series: EMA 20/50/200, RSI 14, ATR 14, relative volume 20, and confirmed swing pivots at a minor and a major scale. The readings are the only inputs the four axes, the event detectors, and the context diagnostic consume, so their null rules, seeds, availability bars, and ordering are fixed here rather than re-derived later.

The outcome is a `quantrule-model::readings` module that publishes series aligned to the input length, never substitutes zero for a missing measurement, and never exposes a confirmed pivot before the bar on which it became knowable. A role registry records each use of a reading, so no reading is owned by an axis.

Affected users: the agents implementing M3–M14, and the human reviewing the model's numeric foundation.

## 2. Current state

- `main` at `2a845b9` contains the M1 kernel in `crates/quantrule-model/src/kernel.rs`; the model crate exposes only `kernel` (`crates/quantrule-model/src/lib.rs`).
- The data engine is complete and supplies `quantrule_data::Candle`, whose OHLC fields are finite and validated and whose volume is optional.
- The contract fixes the formulas in §5, the operational uses in §6, the pivot and candidate rules in §5, and the source, scale, and axis codes in §14.
- No readings, axes, events, aggregates, market state, or shell surface exist.
- Baseline checks pass on `main`: 265 tests, with `fmt`, `clippy -D warnings`, and `doc` clean.

## 3. Scope

Included:

- `quantrule-model::readings` with EMA 20/50/200, RSI 14, ATR 14, RVOL 20, and swing pivots and candidates at the minor (2 bars each side) and major (5 bars each side) scales.
- A role registry using the contract's source, axis, event-type, and scale codes, with one definition per use and a candidate/confirmed discriminator for the pivot families.
- A `Readings` bundle that computes every reading from `&[Candle]`.
- Unit, causality, determinism, edge-case, and golden-fixture tests.
- The M2 plan and transparency log.

Excluded:

- The four axes, events, context, aggregation, market state, and shell commands (M3–M13).
- Changes to the contract's formulas, seeds, periods, scale lookbacks, or code values.
- New dependencies, I/O, clock access, randomness, or shell changes.
- The role registry as a published numeric field; it is engine metadata and is never rendered.

## 4. Assumptions and uncertainty

- The contract is authoritative. The interpretations below are recorded so a reviewer can reject them without reading the code.
- **RVOL baseline window.** `RVOL20 = Volume / SMA20(Volume)` is read literally: the SMA includes the current bar, so the first value is available on the 20th closed bar. The contract says "strictly before the current bar" for the percentile and does not say it here.
- **Missing volume.** A missing current volume, or any missing volume inside the 20-bar baseline window, makes the baseline unavailable and RVOL null. A measured zero volume against a positive baseline is a valid zero.
- **ATR seed.** The first true range is at bar 1 because bar 0 has no previous close. The seed is the mean of the true ranges at bars 1..=14, so ATR is first available at bar 14 (the 15th closed bar).
- **RSI seed.** The first average gain and loss are the means of the 14 changes at bars 1..=14, so RSI is first available at bar 14. A flat first window yields null; a zero average loss with a positive gain yields 100.
- **Pivot availability.** A candidate is knowable on its own bar. A confirmed pivot at bar `i` with lookback `k` is first knowable at bar `i + k`, after the `k` right-hand bars have closed.
- **Series alignment.** The scalar readings and the candidate series have exactly one entry per input candle. The confirmed-pivot vectors also have one entry per input bar, indexed by the availability bar, so reading at or before a bar cannot expose a later confirmation.
- **Pivot artifacts and source codes.** Contract §14 gives source codes 7 and 8 to confirmed pivots and gives no code to a candidate, while §8.4 makes the candidate a reading in its own right. The registry keys both artifacts by the pivot family's code and carries `PivotArtifact::Candidate` or `PivotArtifact::Confirmed` on each pivot role. The event source code still means the confirmed pivot.
- **No dependencies.** The module uses the standard library and `quantrule-data` only.

## 5. Decisions

| ID | Decision | Options | Choice | Status |
|---|---|---|---|---|
| D1 | Scalar series type | One `ReadingSeries`; a newtype per family; a generic series | One `ReadingSeries` with `Option<f64>` values; the `Readings` field names carry the family meaning | Approved with this plan |
| D2 | RVOL baseline | Include the current bar; use the preceding 20 bars | Include the current bar, per the literal `SMA20(Volume)`; flagged in the PR | Approved with this plan |
| D3 | Confirmed-pivot storage | Occurrence-indexed list; availability-indexed vector; both | Availability-indexed vector per scale, with `occurred_at` and `available_at` on each pivot, plus `confirmed_as_of` | Approved with this plan |
| D4 | Role registry | Strings as model output; numeric codes plus a static definition table | A static metadata table keyed by the contract's source codes; never part of the numeric output | Approved with this plan |
| D5 | Invalid period | Panic; return all nulls; reject at the type level | Return an all-null series for period zero or a series shorter than the period; the model only calls the fixed periods | Approved with this plan |
| D6 | Golden fixture | Values embedded in Rust; a checked-in CSV | A checked-in CSV with the input candles and every expected reading, parsed by an integration test | Approved with this plan |
| D7 | Pivot artifact in the registry | Separate candidate source codes; one family code with an artifact discriminator; exclude candidates | One family code 7/8 with `PivotArtifact` on each pivot role. Inventing a candidate code would change the §14 registry; excluding candidates would omit a reading §8.4 names. | Implemented; flagged in the PR |

## 6. Acceptance criteria

- Every reading has a golden fixture, a causality test, and a determinism test.
- The output length equals the input length for every scalar reading and for the candidate series.
- EMA 20/50/200 seed with the SMA of the first `n` closes and are null before bar `n - 1`.
- RSI 14 is null before bar 14, null when both Wilder averages are zero, 100 when the average loss is zero and the gain is positive, and 0 when the gain is zero and the loss is positive.
- ATR 14 is null before bar 14, seeds with the mean of the first 14 true ranges, and keeps a valid zero on a flat series.
- RVOL 20 is null when volume is absent, when any volume in its baseline window is absent, or when the baseline is zero; a zero volume against a positive baseline is a valid zero.
- Candidates exist at both scales, are knowable on their own bar, and are discarded when a bar is both a candidate high and a candidate low at the same scale.
- Confirmed pivots are knowable only at `occurred_at + lookback`; a test shows `confirmed_as_of` excludes a pivot before that bar and includes it at that bar.
- Every confirmed pivot agrees with the candidate at its occurrence bar in kind and price.
- Strict comparisons mean equal highs and equal lows produce no candidate and no pivot.
- The role registry covers every reading, names no axis ownership, records each use with its own definition, and distinguishes candidate from confirmed pivot uses.
- Edge cases are covered: empty series, single candle, series shorter than the required history, a gap, a zero-volume candle, and equal highs or lows.

## 7. Work breakdown

| Step | Action | Paths | Depends on |
|---|---|---|---|
| 1 | Record the approved program plan and link it from the umbrella plan | `docs/execution-plans/2026-10-24/14-model-engine-completion.md`, `docs/execution-plans/2026-10-21/14-model-engine.md` | Approval |
| 2 | Add the series helper, EMA, RSI, ATR, and RVOL modules with unit tests | `crates/quantrule-model/src/readings/{series,ema,rsi,atr,rvol}.rs` | 1 |
| 3 | Add the pivot and candidate module with availability and consistency tests | `crates/quantrule-model/src/readings/pivots.rs` | 2 |
| 4 | Add the role registry, its exact-use fixture, and its tests | `crates/quantrule-model/src/readings/roles.rs` | 2 |
| 5 | Add the `Readings` bundle and expose the module | `crates/quantrule-model/src/readings/mod.rs`, `crates/quantrule-model/src/lib.rs` | 2–4 |
| 6 | Add the golden CSV and the integration test | `crates/quantrule-model/tests/fixtures/readings_golden.csv`, `crates/quantrule-model/tests/readings_golden.rs` | 2–5 |
| 7 | Run the full checks, review the diff against the contract, and record the evidence | all changed files | 6 |
| 8 | Write the transparency log, commit, push, and open the PR | `docs/ai-transparency-log/2026-10-24/18-indicator-readings.md` | 7 and the commit/PR authorization |

## 8. Verification

Per reading:

- **EMA:** seed equals the SMA of the first `period` closes; the recurrence matches a hand-computed series; null before the seed; empty, single, and short series; causality by prefix; determinism by repeated computation.
- **RSI:** hand-computed two-change and Wilder-smoothed values; all-gain 100; all-loss 0; flat-window null; null before bar 14; causality; determinism; empty, single, short.
- **ATR:** hand-computed true ranges including a gap; seed and Wilder recurrence; flat series zero; null before bar 14; causality; determinism; empty, single, short.
- **RVOL:** hand-computed 3-bar ratios; current-inclusive baseline; missing current volume; missing volume in the window; zero baseline; zero volume; causality; determinism; empty, single, short.
- **Pivots:** known minor and major pivots; strict equal-high and equal-low cases; both-kind discard; candidate at its own bar; pivot at `occurred_at + lookback`; candidate/pivot agreement; causality by prefix; determinism; empty, single, short, gap, zero-volume candle.
- **Roles:** the exact set of `(reading, artifact, target)` uses matches a checked-in expectation; every reading has at least one use; every pivot family records both artifacts; at least one reading serves more than one axis; every definition is non-empty; the codes match the contract; the table is stable across iterations.
- **Golden:** every reading at every bar of a checked-in 260-bar series, compared with a `1e-9` tolerance; the fixture is data and is not generated at test time.

Commands:

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`

The evidence is the observed command output, recorded in the PR and the transparency log. No claim is made about an axis, event, aggregate, or shell behavior this item does not implement.

## 9. Risks and recovery

| Risk | Mitigation |
|---|---|
| An off-by-one in a seed shifts every later value | Hand-computed seed tests and the golden fixture; the first available bar is asserted explicitly |
| A pivot is exposed before its confirmation bar | Availability-indexed storage and a `confirmed_as_of` test that fails at `available_at - 1` |
| RVOL includes the current bar when the reviewer expected the preceding 20 | The choice is stated in this plan and flagged in the PR; changing it is a one-line window shift and a fixture regeneration |
| A missing volume in the window is silently treated as zero | The baseline is computed only from complete windows; a test covers a missing middle bar |
| Floating-point drift between the reference generator and Rust | A `1e-9` comparison tolerance and hand-computed unit fixtures; the operations and accumulation order follow the contract |
| The role registry becomes a second source of truth for the formulas | The registry stores definitions only; the formulas live in the reading modules and the registry test checks coverage, not arithmetic |
| The module grows past the file-size limit | One file per calculation; `pivots.rs` and `roles.rs` are split only if they approach the limit |
| Recovery from a bad merge | Revert with a new commit; no migrations or persistent state are involved |

## 10. Stop conditions

Stop and request human direction if:

- The contract requires a semantic change to a formula, seed, period, scale, or code.
- A new dependency is needed.
- `quantrule_data::Candle` lacks a field or invariant the readings need.
- The role registry cannot be expressed without making a reading the owner of an axis.
- A test cannot distinguish a valid zero from an unavailable value.
- The golden fixture cannot be reproduced from the checked-in candles.
- The toolchain cannot run `fmt`, `clippy`, `test`, or `doc`.
- A merge authorization is withheld or the PR is rejected.

## 11. Evidence

Observed on 2026-10-24 in the task worktree, after the implementation and before the pull request.

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | Clean, exit 0. |
| `cargo clippy --all-targets -- -D warnings` | Clean, exit 0. |
| `cargo test` | 360 passed, 0 failed: data 90, model unit 100, model golden 11, shell unit 93, shell CLI 47, doc 19 (data 2, model 17). |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | Clean, exit 0. |

The golden CSV holds 260 bars. Its expected values were produced by an independent Python implementation of the contract formulas, run outside the repository and not part of the build. It contains 159 minor candidates, 44 confirmed minor pivots, 127 major candidates, and 24 confirmed major pivots. Every scalar and every candidate and pivot matched within `1e-9`. The first available bars are 19 for EMA20 and RVOL20, 49 for EMA50, 199 for EMA200, and 14 for RSI14 and ATR14.

The read-only review recomputed the fixture independently and found the numeric core sound. It found four registry omissions and two documentation problems; all were fixed before the PR: pivot candidates are now recorded with `PivotArtifact`, EMA records its Signal crossover use, ATR records its Divergence and Context uses, the module boundary text no longer claims ignorance of axis and event codes, and the role definitions now name the contract section instead of restating axis formulas. The review's test-coverage gaps were closed with an exact registry fixture, a changed-later-bar causality test, a property test over input lengths, a mixed-equality pivot test, and a direct pivot-price-to-candle-extreme assertion.
