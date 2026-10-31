# Unified Ladder Audit

- Date: 2026-10-31 (UTC)
- Audit ID: AUDIT-2026-10-31-unified-ladder
- Status: Published
- Auditor: Audit Agent (AI), sequential read-only pass
- Baseline: `main` at `7c865de` (merge of PR #246)
- Toolchain: see `rustc --version` / `cargo --version`; global gates were run
- Data: committed fixtures and a temporary `/tmp` probe; the full-year SOLUSD source was not committed
- Limitations: no independent verifier; findings are code inspection plus the commands named below

## Summary

| Severity | Count |
|---|---:|
| Critical | 1 |
| Major | 7 |
| Minor | 16 |
| Info | 6 |

The audit is read-only. It did not modify code, tests, issues, or pull requests.
Every finding has a reproducible command and/or a file:line reference.

## Global gate evidence

| Check | Observed |
|---|---|
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --all-targets -- -D warnings` | Passed |
| `cargo test --workspace` | Passed; 45 suites, 0 failures |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | Passed |
| `git diff --check` | Clean |

A temporary probe outside the repository (`/tmp/opencode/audit-probe`) reproduced
the two highest-severity execution/accounting findings. No repository file was
changed.

## Findings

| ID | Severity | Area | Finding | Evidence | Impact | Options | Recommended | Verification | Status | Links |
|---|---|---|---|---|---|---|---|---|---|---|
| AUD-2026-10-31-01 | Critical | backtest | Pending entry fills on the first bar after a data gap | `runner/mod.rs:165,176`; probe `EX-01` | Silent wrong entry with pre-gap ATR | A guard fill-time contiguity; B document next-available fill and re-anchor ATR; C cancel pending on gap | A | Missing-bar test rejects the entry | Fixed | [#247](https://github.com/quant-rule/quant-rule/issues/247) · [#257](https://github.com/quant-rule/quant-rule/pull/257) |
| AUD-2026-10-31-02 | Major | config | Real v2 strategy configs never reach the migration message | `config/strategy.rs:58,66`; probe output | Migration path broken; user sees unknown-field error | A check schema before field validation; B detect legacy fields; C change docs to unknown-field error | A | Real v2 fixture prints the documented migration sentence | Fixed | [#248](https://github.com/quant-rule/quant-rule/issues/248) · [#258](https://github.com/quant-rule/quant-rule/pull/258) |
| AUD-2026-10-31-03 | Major | shell/data | `model-series`/`market-chart` advertise 4h but cannot access H4 | `pipeline.rs:56,80,144`; `command/model.rs:311`; probe | 4h silently prints nothing or errors | A build H4 for path commands; B remove 4h from their help/parsers | A | 4h CLI tests emit H4 rows/write PNG | Fixed | [#249](https://github.com/quant-rule/quant-rule/issues/249) · [#259](https://github.com/quant-rule/quant-rule/pull/259) |
| AUD-2026-10-31-04 | Major | strategy/config | Structure context weights and context `kind` literals are accepted but inert | `strategy/ladder.rs:267`; `config/strategy.rs:319-386`; experiment C | Config/experiment imply a decision dimension that does nothing | A document diagnostics; B use score in rule (new hypothesis); C remove fields (new schema) | A now | Two structure weights produce identical decisions, documented | Planned | — |
| AUD-2026-10-31-05 | Major | portfolio | Exit fill quantity is not reconciled to the open position | `portfolio/ledger.rs:357,537`; probe `PF-01` | Ledger cash/equity can be corrupted by a partial exit | A reject quantity mismatch; B support partial exits; C clamp | A | Mismatched exit returns typed error and state unchanged | Planned | — |
| AUD-2026-10-31-06 | Major | docs/README | README and current-status docs describe the retired rule and stale outputs | `README.md:456-459`; `README.md:306`; `docs/model/2026-10-30/per-timeframe-model.md:4,128`; `docs/README.md:106` | Current example cannot reproduce; users are misled | A regenerate current docs/output; B mark examples historical | A | README envelope matches current 5m example; statuses say merged | Planned | — |
| AUD-2026-10-31-07 | Major | config/docs | Execution v3 contract says v2 is retired while the loader requires v2 | `docs/config/2026-10-30/execution-v3.md:95-98`; `config/execution.rs:48` | Users cannot tell which schema is current | A correct the doc; B implement v3 | A now | v2 config passes; v3 is documented as target | Fixed | [#248](https://github.com/quant-rule/quant-rule/issues/248) · [#258](https://github.com/quant-rule/quant-rule/pull/258) |
| AUD-2026-10-31-08 | Major | analytics | Layer 3 block width depends on whether higher series are empty | `analytics/layers/mod.rs:229,382`; `research/render.rs:127`; probe 155 columns | Fixed-schema contract violated; readers can misalign | A pass configured ladder and emit null blocks; B document variable width | A | Short source still exports configured blocks | Planned | — |
| AUD-2026-10-31-09 | Minor | model/events | Event batch order differs from the contract | `model/timeframe.rs:155`; `events/level.rs:197`; probe bar 28/37 | Published sequence order is contractually wrong | A preserve detector order; B explicit contract rank; C narrow contract | A | Integration test pins batch order | Planned | — |
| AUD-2026-10-31-10 | Minor | config | `model.json` `timeframes` is documented optional but required | `docs/config/2026-10-30/model-v2.md:50`; `config/model.rs:84`; probe | Valid documented config rejected | A default absent to `{}`; B document required | A | Config without `timeframes` parses | Fixed | [#248](https://github.com/quant-rule/quant-rule/issues/248) · [#258](https://github.com/quant-rule/quant-rule/pull/258) |
| AUD-2026-10-31-11 | Minor | model/dashboard | Forming-leg average net displacement contradicts dashboard contract | `docs/model/2026-10-25/dashboard-state.md:257`; `dashboard/assemble/progression.rs:169`; golden rows | Contract and golden fixture disagree | A amend contract; B gate average; C document exception | A or B | Forming-leg test matches chosen contract | Planned | — |
| AUD-2026-10-31-12 | Minor | strategy | `context_lost` fires on missing/unavailable context | `strategy/ladder.rs:151,249,280`; no open-position golden row | Exit reason is not a measured flip | A hold on missing; B document forced exit; C add policy switch | A or C | Open-position missing-context golden row | Planned | — |
| AUD-2026-10-31-13 | Minor | context/config | Unbounded weights can overflow and publish NaN | `context/mod.rs:281,710`; `strategy/config.rs` validation | Valid-looking config silently blocks entries | A reject non-finite weight sum; B normalize; C cap | A | `1e308` weights rejected | Planned | — |
| AUD-2026-10-31-14 | Minor | execution/portfolio | Risk-envelope cap bypasses `quantity_step` | `runner/mod.rs:203`; `execution/engine.rs:592`; `portfolio/sizing.rs:180` | Capped fill can violate step contract | A cap with step; B conservative pre-size; C document override | A | Capped quantity is a step multiple | Planned | — |
| AUD-2026-10-31-15 | Minor | execution | Second `submit` on one bar silently overwrites pending order | `execution/engine.rs:305,353` | Orphaned order remains pending | A reject second submit; B supersede record; C debug assert | A | Two submits yield one order and no pending | Planned | — |
| AUD-2026-10-31-16 | Minor | portfolio | `drawdown`/`peak_equity` stale after fills/funding | `portfolio/ledger.rs:202,348,418`; `state.rs:178` | Public state contradicts field contract | A centralize equity update; B document mark-only | A | Post-fill drawdown matches formula | Planned | — |
| AUD-2026-10-31-17 | Minor | portfolio | Exit commission uses prior mark, not fill-time equity | `portfolio/ledger.rs:369,428`; `docs/config/2026-10-28/per-engine-config.md:336` | Gap exits mis-charge commission | A use fill-time equity; B document prior-mark rule; C define one rule | A or B | Gap-exit commission test | Planned | — |
| AUD-2026-10-31-18 | Minor | shell | `strategy-run --config` ignores run data bounds | `command/strategy.rs:198-214`; `config/run.rs:104` | Same config can analyze different period | A default to run bounds; B document CLI-only bounds | A | Config period is honored unless overridden | Planned | — |
| AUD-2026-10-31-19 | Minor | config/docs | `missing_context` documented default but parser requires key | `docs/config/2026-10-30/strategy-v3.md:81`; `config/strategy.rs:524` | Doc-following config rejected | A default to reject; B document required | B | Missing-key test matches chosen rule | Fixed | [#248](https://github.com/quant-rule/quant-rule/issues/248) · [#258](https://github.com/quant-rule/quant-rule/pull/258) |
| AUD-2026-10-31-20 | Minor | context | Context identity, neutral band, and audit trail are not pinned | `context/mod.rs:139,712`; `strategy/ladder.rs:187`; no context artifact | Caller can mispair contexts; band drifts | A validate identity/share band; B private fields; C persist context | A/C | Mispair test; context artifact row count | Planned | — |
| AUD-2026-10-31-21 | Minor | backtest | Portfolio imports `Side` through the strategy module | `portfolio/config.rs:583`; `portfolio/records.rs:85` | Boundary edge contrary to stack | A use `crate::side::Side`; B neutral module | A | `rg` finds no portfolio→strategy import | Planned | — |
| AUD-2026-10-31-22 | Minor | intent/config | `valid_until`/`max_slippage_bps` are not wired to the shipped strategy | `intent.rs:277`; `strategy/ladder.rs:199`; `config/strategy.rs:475` | Engine checks reachable only from tests/custom callers | A add config fields; B remove claims/example | A or B | Config-driven expiry/slippage test | Planned | — |
| AUD-2026-10-31-23 | Minor | model/aggregate | Public aggregate can panic on malformed divergence direction | `model/aggregate/events.rs:129,210` | Library panic on invalid input | A validate/clamp; B return Result; C private fields | A | Out-of-range direction test does not panic | Planned | — |
| AUD-2026-10-31-24 | Minor | backtest/context | Zero-bar context series aborts backtest instead of missing-context rejection | `context/mod.rs:492`; `runner/mod.rs:142`; `strategy.rs:222` | Short source fails instead of rejecting entries | A represent as missing; B preflight error; C require source | A or B | Short 4h-context run behaves consistently | Planned | — |
| AUD-2026-10-31-25 | Info | execution/report | Capped fill order quantity differs from fill quantity | `execution/engine.rs:592,634`; `mod.rs:32` | Order row overstates the filled quantity | A add filled quantity; B reject over-cap; C document | A | Cap test reconciles order/fill/position | Planned | — |
| AUD-2026-10-31-26 | Info | reporting | All sizing rejections collapse to `missing_quantity` | `runner/mod.rs:187`; `portfolio/sizing.rs:266` | Causes indistinguishable in report | A carry reason; B separate record; C document | A | Short-disabled run shows distinct reason | Planned | — |
| AUD-2026-10-31-27 | Info | reporting | `slippage.total` is per-unit sum, not quote-currency cost | `report_run/bundles.rs:328`; `execution/fill.rs:101` | Total is not comparable with fees | A multiply by quantity; B rename | A | Quantity-100 fill reports currency total | Planned | — |
| AUD-2026-10-31-28 | Info | portfolio | Month after an empty month loses its return | `portfolio/ledger.rs:516`; `records.rs:227` | Contradicts prior-equity rule | A carry last equity; B document | A | Jan+Mar equity test | Planned | — |
| AUD-2026-10-31-29 | Info | docs | Current portfolio schema points to a superseded doc; M15 statement stale | `portfolio/config.rs:8`; `docs/config/2026-10-28/per-engine-config.md:6`; `execution/mod.rs:11` | Readers cannot find current schema | A update current contract; B annotate historical | A | Current docs contain portfolio schema | Planned | — |
| AUD-2026-10-31-30 | Info | tests | H4-specific and multi-context coverage is thin | `context/tests.rs` single context; no 4h CLI tests | Several findings above lack regression guards | A add multi-context/4h tests; B document gap | A | New tests fail first, pass after fixes | Fixed | [#249](https://github.com/quant-rule/quant-rule/issues/249) · [#259](https://github.com/quant-rule/quant-rule/pull/259) |

## Detail

### AUD-2026-10-31-01 — Pending entry fills across a data gap

- Expected: a decision on bar N either fills on the next contiguous bar or is
  rejected; the contract says execution validates a contiguous bar
  (`docs/config/2026-10-30/trade-intent.md:118`).
- Observed: the runner advances execution before it computes contiguity
  (`crates/quantrule-backtest/src/runner/mod.rs:165,176`). A probe submitted a
  long entry on bar 0 and advanced a bar timestamped 1800s without the 900s bar;
  the engine filled at bar 1, time 1800, using the bar-0 ATR.
- Root cause: `ExecutionEngine::advance` treats the next supplied bar as the
  next bar and never receives the expected fill time or a gap flag.
- Impact: silent wrong entries and stale risk distances on sparse data.
- Options:
  - A. Pass the expected fill time or a `fill_contiguous` flag into
    `advance`/`submit`; reject a pending entry on a non-contiguous bar with a
    typed rejection. Define the exit policy separately.
  - B. Document “next bar” as “next available bar” and re-anchor ATR at the
    fill bar when available.
  - C. Cancel all pending orders on a detected gap.
- Recommendation: A for entries; decide the exit policy explicitly.
- Verification: a missing-bar test rejects the entry and leaves the position
  flat; a pending-exit test covers the chosen exit policy.
- Status: `Fixed` by #247 and PR #257. The validator record is
  `docs/reports/2026-10-31/w1-gap-fill-validation.md`; the Improvement Verifier
  has not marked it `Verified`.
- Links: [#247](https://github.com/quant-rule/quant-rule/issues/247) ·
  [#257](https://github.com/quant-rule/quant-rule/pull/257)

### AUD-2026-10-31-02 — Real v2 strategy configs miss the migration message

- Expected: a real `quantrule.strategy/v2` file produces
  `quantrule.strategy/v2 is retired; use quantrule.strategy/v3 with rule "ladder-context"`.
- Observed: the parser calls `check_fields` before the schema check
  (`crates/quantrule-shell/src/config/strategy.rs:58,66`). A real v2 config fails
  first at `$.long_threshold` with `expected a known field`.
- Root cause: unknown-field validation precedes `require_v3_schema`.
- Impact: the documented migration path is unreachable; the acceptance criterion
  is false for real old configs.
- Options:
  - A. Read `schema` before `check_fields` and branch to the migration message.
  - B. Add legacy v2 fields to a detection path, then emit the migration error.
  - C. Change the docs/tests to the actual unknown-field error.
- Recommendation: A, with a regression test built from the historical v2 shape.
- Verification: `strategy explain` on a real v2 file prints the migration
  sentence as its first error.
- Status: `Fixed` by #248 and PR #258. The regression test uses the historical
  v2 shape.
- Links: [#248](https://github.com/quant-rule/quant-rule/issues/248) ·
  [#258](https://github.com/quant-rule/quant-rule/pull/258)

### AUD-2026-10-31-03 — Path commands advertise 4h but cannot access H4

- Expected: `model-series --timeframe 4h` and `market-chart --timeframe 4h`
  either work or reject the option clearly.
- Observed: `model-series ... --timeframe 4h` exits 0 with no output;
  `market-chart` builds layers over the default four-timeframe ladder and then
  fails with `cannot render an empty chart`. Help and README advertise 4h.
- Root cause: `pipeline::model_run_from_set` and `ModelInputs::from_set` use
  `Timeframe::DEFAULT_LADDER`, which excludes H4; the parsers accept H4.
- Impact: a documented option silently produces no data or a misleading error.
- Options:
  - A. Build the path commands over `Timeframe::ALL` (or the full ladder) and
    filter the requested timeframe.
  - B. Remove 4h from these commands’ parsers/help and keep H4 for configured
    runs only.
- Recommendation: A; the data engine already builds H4.
- Verification: 4h `model-series` emits rows; 4h `market-chart` writes a PNG.
- Status: `Fixed` by #249 and PR #259; the 4h CLI tests are the regression
  guards.
- Links: [#249](https://github.com/quant-rule/quant-rule/issues/249) ·
  [#259](https://github.com/quant-rule/quant-rule/pull/259)

### AUD-2026-10-31-04 — Strategy context config fields are inert

- Expected: a field accepted by `strategy.json` either changes the rule or is
  documented as diagnostic-only.
- Observed: the rule reads only `context.structure.conflict_count`; it never
  reads `context.structure.score` or the weighted Structure score. The
  `entry.context.{direction.kind,momentum.kind,progression.direction}` literals
  are validated and then dropped. `stage_histogram` and the axis agreement codes
  have no consumer. Experiment cell C varies `weights.structure` but cannot
  change a decision.
- Root cause: the v3 config surface describes four weighted axes and context
  kinds, while the rule implements only the conflict count plus the
  direction/momentum/progression conditions.
- Impact: a human tuning the Structure weight or `kind` sees no effect; the
  experiment matrix implies a tested dimension that is not tested.
- Options:
  - A. Document the inert fields as diagnostic-only and keep the rule; update
    `strategy explain`, the experiment record, and the contract.
  - B. Use the Structure score/kind in the rule; this is a behavior change and
    needs a pre-registered hypothesis, Quant Developer, and Quant Validator.
  - C. Remove the fields in a new schema.
- Recommendation: A now; B only through the improvement loop.
- Verification: two different Structure weights produce identical decisions and
  fills, and the docs say why; or a new golden case distinguishes them after B.
- Links: —

### AUD-2026-10-31-05 — Exit fill quantity is not reconciled

- Expected: a close consumes exactly the open quantity, or a partial exit is
  explicitly supported.
- Observed: a probe opened 100 units, applied a 50-unit exit, and the ledger
  accepted it. Equity became `5460.03` while `initial_cash + net_pnl` was
  `10960.03`; the trade record claimed quantity 100 and gross 1000.
- Root cause: `PortfolioLedger::close` computes cash from `fill.quantity` and
  gross/trade quantity from `position.quantity`; `validate_fill` checks only
  positivity, not equality.
- Impact: silent accounting corruption for any partial-fill path or direct
  ledger caller. The shipped runner currently always closes the full quantity,
  so this is latent for the runner but public in the library.
- Options:
  - A. Reject an exit fill whose quantity differs from the open position’s
    quantity, leaving state unchanged.
  - B. Support partial exits by reducing the position and publishing the
    remainder/trade correctly.
  - C. Clamp the fill to the position and record a rejection for the difference.
- Recommendation: A for the current single-position model.
- Verification: a mismatch test returns a typed error and leaves cash/equity/
  position/trade unchanged; a full-close invariant test asserts
  `final_equity == initial_cash + net_pnl`.
- Links: —

### AUD-2026-10-31-06 — Current README/status docs describe the retired rule

- Expected: current docs match the live `ladder-context`/5m example.
- Observed:
  - `README.md:456-459` shows first/last open times 14:15/14:30, the 15m
    `axis-agreement` run; the current example is 5m with 14:05/14:50.
  - `README.md:306` says `timeframes` builds four series and H4 is unsupported;
    the code builds five.
  - `docs/model/2026-10-30/per-timeframe-model.md:4,128` says the context builder
    is pending.
  - `docs/README.md:106` says PR #246 is open and Phase 1 is pending.
  - `EXP-2026-10-30-01` records axis-agreement output from a fixture that is now
    v3.
- Root cause: the nine-phase program updated code and some contracts but not the
  top-level example/status surfaces.
- Impact: a user following the primary README cannot reproduce the envelope and
  is told H4 does not exist.
- Options:
  - A. Regenerate the current envelope/docs and update statuses; mark EXP-01
    historical.
  - B. Keep the old examples only as dated historical records and add current
    examples.
- Recommendation: A.
- Verification: README envelope matches a current `backtest --json`; stale greps
  return only historical records.
- Links: —

### AUD-2026-10-31-07 — Execution v3 contract contradicts the v2 loader

- Expected: one current execution schema.
- Observed: `docs/config/2026-10-30/execution-v3.md:95-98` says v2 is retired and
  rejected; the loader requires `quantrule.execution/v2`
  (`crates/quantrule-shell/src/config/execution.rs:48`). The status line of the
  same doc acknowledges the gap, but §7 contradicts it.
- Root cause: the target contract and the live loader are out of step.
- Impact: users cannot tell which file is valid; a v3 file is rejected.
- Options:
  - A. Make v2 the sole current contract and mark v3 as planned; remove the
    “retired” claim.
  - B. Implement the v3 loader and migrate configs.
- Recommendation: A now; B as a planned follow-up.
- Verification: v2 example passes; v3 is documented as a target with a clear
  migration status.
- Status: `Fixed` by #248 and PR #258; the document now says v2 is current and
  v3 is the target.
- Links: [#248](https://github.com/quant-rule/quant-rule/issues/248) ·
  [#258](https://github.com/quant-rule/quant-rule/pull/258)

### AUD-2026-10-31-08 — Layer 3 block width depends on data length

- Expected: one block per configured higher timeframe, with null cells when no
  bar closed (`docs/analytics/2026-10-24/research-output.md:273-277`).
- Observed: on `single.csv` the export has 155 Layer 3 columns (zero higher
  blocks) because empty higher series are filtered out before block construction
  (`crates/quantrule-analytics/src/layers/mod.rs:229,382`; render at
  `research/render.rs:127`). The contract’s `155 + 67*n` formula assumes the
  configured ladder.
- Root cause: the configured ladder is not passed to the block builder/renderer.
- Impact: downstream readers expecting a fixed width can misalign; the
  block-absence rule is not expressible for an empty series.
- Options:
  - A. Pass the configured ladder and emit null blocks for empty higher series.
  - B. Document variable width and update the formula/readers.
- Recommendation: A.
- Verification: a source shorter than 5m still exports the configured blocks
  with null cells.
- Links: —

### AUD-2026-10-31-09 — Event batch order differs from the contract

- Expected: the contract’s fixed Level reference order and Momentum-before-
  Structure signal order.
- Observed: on `state_m1.csv`, bar 28 publishes a minor-pivot test before the
  EMA20 break; bar 37 publishes a Structure signal before the Momentum signal.
  `TimeframeModel::compute` sorts by type/subtype/source/reference
  (`crates/quantrule-model/src/timeframe.rs:155`), reversing the detector order.
- Impact: event sequence is contractually wrong; order-sensitive consumers and
  goldens can disagree. Values/counts are unaffected.
- Options:
  - A. Preserve detector append order within the same availability/occurrence
    bar.
  - B. Sort by an explicit contract rank.
  - C. Narrow the contract to availability order.
- Recommendation: A or B.
- Verification: integration test pins the bar-28/bar-37 order.
- Links: —

### AUD-2026-10-31-10 — `model.json` `timeframes` is required despite “optional”

- Expected: the contract says the map is optional
  (`docs/config/2026-10-30/model-v2.md:50`).
- Observed: removing `timeframes` gives `model.json: $.timeframes: expected a
  required field` (`crates/quantrule-shell/src/config/model.rs:84`).
- Impact: a documented valid config is rejected.
- Options: A. default an absent map to `{}`; B. document it as required.
- Recommendation: A.
- Verification: parse without `timeframes`.
- Status: `Fixed` by #248 and PR #258; the absent map defaults to the
  per-timeframe readings.
- Links: [#248](https://github.com/quant-rule/quant-rule/issues/248) ·
  [#258](https://github.com/quant-rule/quant-rule/pull/258)

### AUD-2026-10-31-11 — Dashboard forming-leg average contract conflict

- Expected: the dashboard contract says a forming leg’s same-leg measurement is
  `Undefined` and a confirmed-leg measurement is not published before
  `origin_available_at` (`docs/model/2026-10-25/dashboard-state.md:257-258`).
- Observed: `average_net_displacement_atr_per_bar` is computed for forming legs
  and the golden fixture asserts non-null values (7 rows with
  `origin_confirmed_code=0`).
- Impact: the contract and golden fixture disagree; consumers cannot tell
  whether the average is availability-gated.
- Options: A. amend the contract; B. gate the average and regenerate the
  golden; C. document the average as the exception.
- Recommendation: A or B after a human contract decision.
- Verification: a forming-leg test matches the chosen contract.
- Links: —

### AUD-2026-10-31-12 — `context_lost` fires on unavailable context

- Expected: `context_lost` means the context condition that justified the entry
  no longer holds (`docs/config/2026-10-30/strategy-v3.md:133-134`).
- Observed: `context_side` returns false when context is incomplete or a share
  is null (`crates/quantrule-backtest/src/strategy/ladder.rs:249,280`), so an
  open position exits with `context_lost` on missing/unavailable context.
- Impact: the exit reason is not a measured flip; behavior is undocumented and
  has no open-position missing-context golden row.
- Options: A. hold on missing/unavailable; B. document a forced exit; C. add a
  policy switch.
- Recommendation: A or C.
- Verification: open-position missing-context golden row.
- Links: —

### AUD-2026-10-31-13 — Unbounded weights can publish NaN

- Expected: weights are finite and positive, and the weighted output is finite.
- Observed: two `1e308` weights on one axis pass validation; the sum overflows
  to infinity, and `inf/inf` publishes NaN shares/value. `axis_side` then sees
  `None` and blocks entries; an open position can be forced out.
- Root cause: validation checks each weight but not the per-axis sum
  (`crates/quantrule-backtest/src/context/mod.rs:281,710`).
- Impact: a valid-looking config silently disables the rule.
- Options: A. reject a non-finite weight sum; B. normalize to a finite sum;
  C. cap weights.
- Recommendation: A.
- Verification: `1e308` weights are rejected at load.
- Links: —

### AUD-2026-10-31-14 — Risk cap bypasses `quantity_step`

- Expected: every fill quantity is a multiple of `quantity_step`
  (`crates/quantrule-backtest/src/portfolio/sizing.rs:1-9`).
- Observed: the runner passes a notional cap; execution caps with
  `max_notional / fill.price` and never applies the step
  (`crates/quantrule-backtest/src/execution/engine.rs:592`).
- Impact: capped fills/positions can violate the sizing contract.
- Options: A. cap with the step; B. conservative pre-size; C. document the
  override.
- Recommendation: A.
- Verification: capped quantity is a step multiple.
- Links: —

### AUD-2026-10-31-15 — Second submit overwrites the pending order

- Expected: one accepted order per bar or an explicit rejection/supersede
  record.
- Observed: both entry and exit paths assign `self.pending = Some(...)` without
  checking an existing order (`crates/quantrule-backtest/src/execution/engine.rs:305,353`).
  The first order id never resolves, so its record stays `Pending`.
- Impact: orphaned order in reports; lifecycle contract violation.
- Options: A. reject a second submit; B. supersede with a record; C. debug
  assertion.
- Recommendation: A.
- Verification: two submits yield one order and no pending order after finish.
- Links: —

### AUD-2026-10-31-16 — Portfolio drawdown/peak stale after fills/funding

- Expected: `drawdown = 1 - equity/peak_equity` and `peak_equity` is the highest
  equity seen (`crates/quantrule-backtest/src/portfolio/state.rs:178`).
- Observed: only `mark` updates peak/drawdown; `open`, `close`, and
  `apply_funding` change equity without updating them.
- Impact: public ledger state contradicts its fields between marks.
- Options: A. centralize equity updates; B. document mark-time-only.
- Recommendation: A.
- Verification: post-fill drawdown matches the formula.
- Links: —

### AUD-2026-10-31-17 — Exit commission uses the prior mark

- Expected: commission is `rate × equity before the operation`
  (`docs/config/2026-10-28/per-engine-config.md:336`).
- Observed: `close` uses the last marked equity, not `cash + position_value(fill.price)`.
- Impact: gap exits mis-charge commission; the final-equity invariant still
  holds because cash/trade use the same number.
- Options: A. use fill-time equity; B. document the prior-mark rule; C. define
  one rule for both sides.
- Recommendation: A or B after contract clarification.
- Verification: gap-exit commission test.
- Links: —

### AUD-2026-10-31-18 — `strategy-run --config` ignores run data bounds

- Expected: a run config’s `data.from/to` defines its period.
- Observed: `strategy-run` loads the CSV with CLI `args.bounds` and only takes
  strategy/model config from the run (`crates/quantrule-shell/src/command/strategy.rs:198-214`).
- Impact: the same config can analyze a different period than `backtest`.
- Options: A. default to run bounds when no CLI bounds; B. document CLI-only.
- Recommendation: A.
- Verification: config period is honored, CLI overrides.
- Links: —

### AUD-2026-10-31-19 — `missing_context` doc vs parser

- Expected: doc says missing context defaults to reject
  (`docs/config/2026-10-30/strategy-v3.md:81`).
- Observed: the parser requires the key (`crates/quantrule-shell/src/config/strategy.rs:524`).
- Impact: doc-following config rejected.
- Options: A. default absent to reject; B. document required.
- Recommendation: B (strict).
- Verification: missing-key test matches chosen rule.
- Status: `Fixed` by #248 and PR #258; the contract now says the key is
  required.
- Links: [#248](https://github.com/quant-rule/quant-rule/issues/248) ·
  [#258](https://github.com/quant-rule/quant-rule/pull/258)

### AUD-2026-10-31-20 — Context identity, band, and audit trail are not pinned

- Expected: a context used by `decide` belongs to the input bar/timeframe; the
  neutral band matches the model; the selected context is auditable.
- Observed: `LadderContext` has public identity fields and `decide` checks only
  the input timeframe; `0.1` is duplicated instead of using
  `quantrule_model::kernel::COMPACT_STATE_THRESHOLD`; the strategy bundle writes
  no context artifact.
- Impact: a library caller can mispair contexts; a model threshold change
  silently drifts the context; the ADR’s audit claim is not persisted.
- Options: A. validate identity and use the shared band; B. private fields;
  C. add a context CSV to the strategy bundle.
- Recommendation: A/C.
- Verification: mispair test returns Hold; context CSV row count matches bars.
- Links: —

### AUD-2026-10-31-21 — Portfolio imports `Side` through strategy

- Expected: one-way stack; portfolio should use the neutral `crate::side::Side`.
- Observed: `crates/quantrule-backtest/src/portfolio/config.rs:583` and
  `records.rs:85` use `crate::strategy::Side`.
- Impact: portfolio compiles against strategy, contrary to the decision stack.
- Options: A. use `crate::side::Side`; B. neutral module.
- Recommendation: A.
- Verification: `rg "crate::strategy" portfolio` is empty.
- Links: —

### AUD-2026-10-31-22 — Intent expiry/slippage are not wired

- Expected: `valid_until` and `max_slippage_bps` are configurable if the
  contract advertises them.
- Observed: `RiskPolicy::intent` always sets both to `None`; the strategy config
  has no fields; the engine checks are reachable only from tests/custom callers.
- Impact: the documented risk controls are not available in the shell.
- Options: A. add config fields and attach them; B. remove the fields/example.
- Recommendation: A or B.
- Verification: config-driven expiry/slippage rejection test.
- Links: —

### AUD-2026-10-31-23 — Aggregate panic on malformed direction

- Expected: library code does not panic on invalid input.
- Observed: `CrossTimeframeAggregates` uses
  `divergence_directions[(direction + 1) as usize]` without a range check
  (`crates/quantrule-model/src/aggregate/events.rs:129`); a public snapshot can
  carry `direction_code = 2`.
- Impact: library panic on an out-of-contract event.
- Options: A. validate/clamp; B. return `Result`/`Option`; C. private fields.
- Recommendation: A.
- Verification: out-of-range direction test does not panic.
- Links: —

### AUD-2026-10-31-24 — Zero-bar context series aborts the backtest

- Expected: missing context rejects entries by default.
- Observed: `LadderContextBuilder::new` returns `MissingContextTimeframe` when a
  configured context series is absent; `run_backtest` propagates it as an error.
  `strategy-run` pre-checks and returns a clearer `NoTimeframeBars`.
- Impact: a short source fails instead of rejecting entries; behavior differs
  between commands.
- Options: A. represent absent series as missing; B. preflight error in the
  runner; C. require a source that supplies the ladder.
- Recommendation: A or B after deciding whether absence is data error or
  missing context.
- Verification: short 4h-context run behaves consistently.
- Links: —

### AUD-2026-10-31-25 — Capped fill order quantity differs from fill

- Expected: order, fill, and position quantities agree.
- Observed: the engine caps `fill.quantity` but the filled order record uses
  `pending.order.quantity` (`crates/quantrule-backtest/src/execution/engine.rs:592,634`).
- Impact: `orders.csv` overstates the filled quantity.
- Options: A. record filled quantity; B. reject over-cap entries; C. document
  the partial.
- Recommendation: A.
- Verification: cap test reconciles all three quantities.
- Links: —

### AUD-2026-10-31-26 — Sizing rejections collapse to `missing_quantity`

- Expected: a report distinguishes halt, short-disabled, below-step, and
  negative-equity sizing failures.
- Observed: the runner maps every `size_entry` error to `None`, which becomes
  `MissingQuantity`.
- Impact: diagnostics/reporting cannot distinguish causes.
- Options: A. carry the `SizingRejection`; B. separate report channel; C.
  document the collapse.
- Recommendation: A or B.
- Verification: short-disabled run reports a distinct reason.
- Links: —

### AUD-2026-10-31-27 — Slippage total is per-unit

- Expected: `slippage.total` is quote-currency cost if compared with fees.
- Observed: it sums `fill.slippage` (per unit), not `slippage × quantity`.
- Impact: high-quantity fills understate slippage cost.
- Options: A. multiply by quantity; B. rename to per-unit.
- Recommendation: A.
- Verification: quantity-100 fill reports the currency total.
- Links: —

### AUD-2026-10-31-28 — Monthly return after an empty month

- Expected: prior available equity is carried across a month with no points.
- Observed: `month_last`/`previous` become `None`, so the next month’s return is
  lost.
- Impact: month-long gaps null the following month.
- Options: A. carry last equity; B. document.
- Recommendation: A.
- Verification: Jan+Mar equity points test.
- Links: —

### AUD-2026-10-31-29 — Portfolio schema link and M15 statement

- Expected: the current portfolio schema is in a current contract.
- Observed: code points to a superseded `2026-10-28` doc; the current
  policy/strategy doc has no portfolio section; `execution/mod.rs:11` still says
  “M15 ladder.”
- Impact: readers cannot find the current schema.
- Options: A. add portfolio schema to current contract; B. annotate historical.
- Recommendation: A.
- Verification: current doc contains the schema; stale M15 grep is empty.
- Links: —

### AUD-2026-10-31-30 — H4-specific coverage is thin

- Expected: H4 boundary/gap/empty/CLI coverage.
- Observed: one complete and one partial H4 bucket test; no 4h CLI, no H4 gap,
  no multi-context weight test, no 4h context model test.
- Impact: the defects above can regress silently.
- Options: A. add focused tests; B. document the gap.
- Recommendation: A.
- Verification: new tests fail before fixes and pass after.
- Status: `Fixed` by #249 and PR #259 for H4 CLI, chart, and gap coverage; the
  multi-context weight test remains in W4.
- Links: [#249](https://github.com/quant-rule/quant-rule/issues/249) ·
  [#259](https://github.com/quant-rule/quant-rule/pull/259)

## Limitations

- No independent verifier or independent model judgment was available. This is a
  sequential read-only pass.
- The full-year SOLUSD source was not present in this worktree; the experiment
  and validator records were inspected, not reproduced.
- The temporary probe covers the gap-fill and exit-quantity findings; other
  findings are code inspection.
- Execution/portfolio internals, store, analytics, chart rendering, and the full
  CLI were not exhaustively exercised.
- The audit did not modify code, tests, issues, or PRs.

## Unexamined areas

- Full model formulas and event semantics beyond the findings above.
- Store schema/migrations and query behavior.
- Analytics aggregation beyond Layer 3 block construction.
- Live Dukascopy fetch behavior, retries, and pagination.
- Release tooling, dependency vulnerabilities, and deployment.
- Performance of the full matrix and 600k-bar model/report path.
