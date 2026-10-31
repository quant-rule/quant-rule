# Remediation and Diagnosis-v1 Audit

- Date: 2026-10-31 (UTC)
- Audit ID: AUDIT-2026-10-31-remediation-diagnosis-v1
- Status: Published (PR #293); findings `Planned` (W1–W3 and W4a `Fixed`); remediation plan `Approved` and issue tree created
- Auditor: Audit Agent (AI), sequential read-only pass
- Baseline: `main` at `7c865de` (merge of PR #246; last audited state)
- Head: `main` at `c8b1dc1` (merge of PR #279)
- Toolchain: `rustc 1.99.1 (58cf8f9c3 2026-10-13)`; `cargo 1.99.1 (5f94df478 2026-08-27)`
- Scope: `git diff 7c865de..c8b1dc1`; 146 files, +11,664/−601, 50 commits
- Data: committed fixtures and existing `target/report-tests` output; no new probe
- Limitations: no independent verifier; findings are code inspection plus the commands named below
- Planner: re-verified at `c8b1dc1` on 2026-10-31; none stale or duplicate; see [`docs/execution-plans/2026-10-31/audit-remediation-diagnosis-v1.md`](../../execution-plans/2026-10-31/audit-remediation-diagnosis-v1.md)

## Summary

| Severity | Count |
|---|---:|
| Critical | 0 |
| Major | 1 |
| Minor | 9 |
| Info | 0 |

The audit is read-only. It did not modify code, tests, issues, or pull requests.
Every finding has a reproducible command and/or a `file:line` reference.

The diff contains two bodies of work: the remediation of the 30 findings in
`docs/audits/2026-10-31/unified-ladder-audit.md` (PRs #248–#277) and the
diagnosis-v1 feature (PR #279). The remediation is largely correct. The new
findings are concentrated in the diagnosis projection and checks, plus one
incomplete model guard and two shell robustness/documentation defects.

## Global gate evidence

| Check | Observed |
|---|---|
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo test --workspace` | 1504 passed, 0 failed, 7 ignored |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | exit 0 |
| `git diff --check` | clean |

The full gate log is `/tmp/opencode/audit-gates-2026-10-31.log` (scratch; not
committed). The commands above are the reproducible evidence.

## Findings

| ID | Severity | Area | Finding | Evidence | Impact | Options | Recommended | Verification | Status | Links |
|---|---|---|---|---|---|---|---|---|---|---|
| AUD-2026-10-31-31 | Major | shell/analytics | `evidence.out_of_sample` passes on file existence, not a different data version | `crates/quantrule-shell/src/diagnosis.rs:545-551`; `crates/quantrule-analytics/src/diagnosis/compute.rs:1248-1263`; `docs/reports/2026-10-31/diagnosis-v1.md:99` | A same-data `oos/` folder is reported as OOS evidence; the human may treat in-sample results as OOS-validated | A compare `data_version`; B require a valid OOS report and period; C make it `unknown` unless a distinct OOS version is declared | A | Same-version `oos/` -> `unknown`; different version -> `pass` | Fixed | [#280](https://github.com/quant-rule/quant-rule/issues/280) · [#294](https://github.com/quant-rule/quant-rule/pull/294) |
| AUD-2026-10-31-32 | Minor | analytics | `data.long_gap` passes with `0.0` when the 1m series is absent | `compute.rs:145`, `983-1011`; `docs/reports/2026-10-31/diagnosis-v1.md:81,88` | A missing measurement is published as a passing zero | A `None` -> `Unknown`; B require a 1m series at config load; C measure the fastest available timeframe and label it | A | A run without 1m -> `unknown`, no `0.0` value | Fixed | [#281](https://github.com/quant-rule/quant-rule/issues/281) · [#296](https://github.com/quant-rule/quant-rule/pull/296) |
| AUD-2026-10-31-33 | Minor | model/aggregate | Divergence direction guard still overflows for `i8::MAX` | `crates/quantrule-model/src/aggregate/events.rs:131`; `crates/quantrule-model/src/events/mod.rs:76` | Public `CrossTimeframeAggregates::compute` panics in debug on a malformed public `direction_code`; the prior audit's claimed test is absent | A `checked_add(1)`; B explicit `-1..=1` match; C validated newtype | A | Debug test with `direction_code = i8::MAX` does not panic | Fixed | [#282](https://github.com/quant-rule/quant-rule/issues/282) · [#297](https://github.com/quant-rule/quant-rule/pull/297) |
| AUD-2026-10-31-34 | Minor | analytics | `identity.report_versions` passes with fewer than five versions | `compute.rs:917-936`; `docs/reports/2026-10-31/diagnosis-v1.md:85` | A lab missing bundle versions passes a check that claims all five are recorded | A require `len() == 5`; B require the five expected bundle names; C fail on any missing version | B | Four versions -> `fail`; five -> `pass` | Fixed | [#287](https://github.com/quant-rule/quant-rule/issues/287) · [#299](https://github.com/quant-rule/quant-rule/pull/299) |
| AUD-2026-10-31-35 | Minor | analytics | `average_hold_bars` is `0.0` for an exit reason with no trades | `compute.rs:500-504`; `crates/quantrule-analytics/src/diagnosis/types.rs:288` | A missing measurement is published as a valid zero | A `Option<f64>` and null; B NaN; C omit the row | A | Zero-trade reason -> `None` in the struct, `null` in JSON, empty CSV | Planned | [#289](https://github.com/quant-rule/quant-rule/issues/289) |
| AUD-2026-10-31-36 | Minor | shell | `manifest.txt` omits itself from the artifact table | `crates/quantrule-shell/src/diagnosis.rs:878-897`; observed `target/report-tests/diagnosis-command/diagnosis/manifest.txt` | The manifest cannot be used to verify the complete artifact set | A add the `manifest.txt` row; B change the header to "16 listed + this manifest"; C drop the count | A | Manifest row set equals the 17 files in the directory | Planned | [#288](https://github.com/quant-rule/quant-rule/issues/288) |
| AUD-2026-10-31-37 | Minor | analytics | `trades_enriched.csv` MFE can be negative | `compute.rs:883-897`; `docs/reports/2026-10-31/diagnosis-v1.md:157-159` | A documented non-negative invariant is violated when the entry fill is outside the entry bar's range | A clamp MFE/MAE at 0; B amend the contract to allow fill-relative signs; C measure from the reference/open | A | Test with a fill outside the bar range -> `mfe >= 0` | Planned | [#290](https://github.com/quant-rule/quant-rule/issues/290) |
| AUD-2026-10-31-38 | Minor | analytics | Daily `entry_fills` omits an entry still open at run end | `compute.rs:583-613`, `384`; `types.rs:116-123` | The daily funnel undercounts fills and disagrees with `funnel.csv` | A build from `input.fills` filtered to `entry`; B add open positions; C document the closed-trade basis | A | One open entry on day D -> `entry_fills = 1` on D | Planned | [#291](https://github.com/quant-rule/quant-rule/issues/291) |
| AUD-2026-10-31-39 | Minor | docs | `lab.rs` module doc names only `lab summary` | `crates/quantrule-shell/src/command/lab.rs:1-5` | The module doc is stale for `lab diagnose` and `lab list` | A update the module doc; B split the module per command | A | `cargo doc`; review the module header | Planned | [#285](https://github.com/quant-rule/quant-rule/issues/285) |
| AUD-2026-10-31-40 | Minor | shell | `lab list` can panic on a non-ASCII `config_hash` | `crates/quantrule-shell/src/command/lab.rs:717`, `616-626` | A malformed lab folder panics the shell instead of returning an error | A `get(..12).unwrap_or(config_hash)`; B validate 16 hex characters; C typed error for a malformed lab | B | Multi-byte `config_hash` -> error, no panic | Planned | [#286](https://github.com/quant-rule/quant-rule/issues/286) |

## Detail

### AUD-2026-10-31-31 — Out-of-sample check is a file-existence test

- Expected invariant: `evidence.out_of_sample` passes only when an `oos/` lab
  with a different `data_version` exists
  (`docs/reports/2026-10-31/diagnosis-v1.md:99`).
- Observed: `has_out_of_sample` returns
  `lab_dir/oos/portfolio/report.json.is_file()`
  (`crates/quantrule-shell/src/diagnosis.rs:545-551`). The boolean is passed to
  `DiagnosisInput.out_of_sample` (`diagnosis.rs:175`), and the analytics check
  passes whenever it is true (`compute.rs:1248-1263`). The file contents and
  the data version are never read.
- Root cause: the shell reuses an existence check instead of the data-version
  comparison already implemented for `lab summary`
  (`crates/quantrule-shell/src/lab_summary.rs:542-549`).
- Impact: an `oos/` folder produced from the same data version is reported as
  out-of-sample evidence. A human or downstream report can treat an in-sample
  result as OOS-validated. This is materially misleading evaluation evidence.
- Affected contracts/docs: `docs/reports/2026-10-31/diagnosis-v1.md:99`;
  `checks.csv` and `diagnosis.json` consumers.
- Dependencies: none.
- Effort: small; reuse the `lab_summary` comparison and extend the evidence
  string to distinguish "no oos lab" from "oos lab uses the same data version".
- Options:
  - A. Compare `data_version` between `oos/portfolio/report.json` and
    `portfolio/report.json`; return `Unknown` when they are equal or the report
    is malformed, with evidence that says which.
  - B. Additionally require a valid OOS report and a recorded OOS period.
  - C. Make the check `unknown` unless a distinct OOS data version is declared
    in config.
- Recommendation: A.
- Verification: a shell test with an `oos/` report using the same
  `data_version` must produce `unknown`; a different `data_version` must
  produce `pass`. The existing `lab_summary` test is the model.
- Status: `Fixed`.
- Links: [#280](https://github.com/quant-rule/quant-rule/issues/280) · [#294](https://github.com/quant-rule/quant-rule/pull/294).

### AUD-2026-10-31-32 — `data.long_gap` false pass without a 1m series

- Expected invariant: a missing measurement is `unknown`, never `pass`
  (`docs/reports/2026-10-31/diagnosis-v1.md:81`); the check is the longest 1m
  gap (`:88`).
- Observed: `data_quality` builds `raw_gaps` with
  `run.timeframe(60).map(raw_gaps).unwrap_or_default()`
  (`compute.rs:145`). The `None` branch of `data.long_gap` then passes with
  value `0.0` and evidence "no 1m gap was found"
  (`compute.rs:983-1011`). An absent 1m series and a 1m series with no gaps are
  indistinguishable.
- Root cause: the absence of the series is collapsed to an empty gap vector
  before the check is built.
- Impact: a run without a 1m series reports a passing gap check and zero gap.
  The data-quality table and the triage action also say nothing about the
  missing series.
- Affected contracts/docs: `docs/reports/2026-10-31/diagnosis-v1.md:81,88`;
  `data-quality.csv`; the `data.long_gap` triage action
  (`compute.rs:1361`).
- Dependencies: none.
- Effort: small.
- Options:
  - A. `None` -> `Unknown` with no value and evidence "no 1m series".
  - B. Require a 1m series at config load and reject a run without it.
  - C. Measure the fastest available timeframe and label the check with that
    timeframe.
- Recommendation: A.
- Verification: a unit test with a `ModelRun` lacking 1m expects `unknown` and
  no `0.0` value; the triage action names the missing series.
- Status: `Fixed`.
- Links: [#281](https://github.com/quant-rule/quant-rule/issues/281) · [#296](https://github.com/quant-rule/quant-rule/pull/296).

### AUD-2026-10-31-33 — Incomplete divergence direction guard

- Expected invariant: library code must not panic on invalid input
  (AGENTS.md §3); a public snapshot may carry a direction outside `-1..=1`, and
  the aggregate must ignore it.
- Observed: the guard is
  `if let Ok(index) = usize::try_from(direction + 1) && index < len`
  (`crates/quantrule-model/src/aggregate/events.rs:131`). `direction` is `i8`
  (`crates/quantrule-model/src/events/mod.rs:76`), so `direction + 1` is
  evaluated as `i8` before `try_from`; `i8::MAX + 1` overflows and panics in a
  debug build. `TimeframeSnapshot` and `CrossTimeframeAggregates::compute` are
  public (`crates/quantrule-model/src/aggregate/mod.rs:36,83`).
- Root cause: the bounds check was added after the addition, so the addition
  itself is unguarded.
- Impact: a caller that constructs a public snapshot with
  `direction_code = i8::MAX` panics in debug. The prior audit's AUD-23
  verification column says "Out-of-range direction test does not panic", but no
  such test exists in the repository.
- Affected contracts/docs: `docs/audits/2026-10-31/unified-ladder-audit.md`
  AUD-23; `crates/quantrule-model/src/aggregate/tests.rs`.
- Dependencies: none.
- Effort: small.
- Options:
  - A. Use `direction.checked_add(1).and_then(|value| usize::try_from(value).ok())`.
  - B. Match `direction` explicitly: `-1 => 0, 0 => 1, 1 => 2, _ => skip`.
  - C. Replace the public `i8` field with a validated newtype or enum.
- Recommendation: A (or B); add regression cases for `i8::MAX`, `i8::MIN`, and
  `2`.
- Verification: a debug-build unit test constructs a snapshot with
  `direction_code = i8::MAX` and calls `CrossTimeframeAggregates::compute`; the
  call returns without panicking and does not increment the divergence
  direction counts.
- Status: `Fixed`.
- Links: [#282](https://github.com/quant-rule/quant-rule/issues/282) · [#297](https://github.com/quant-rule/quant-rule/pull/297).

### AUD-2026-10-31-34 — `identity.report_versions` passes with fewer than five

- Expected invariant: all five bundle versions are recorded
  (`docs/reports/2026-10-31/diagnosis-v1.md:85`).
- Observed: the check passes when the list is non-empty and every version
  string is non-empty (`compute.rs:917-936`). One to four versions pass, while
  the threshold text says "5 report versions recorded".
- Root cause: the pass condition tests presence, not the required count or
  bundle names.
- Impact: a lab missing one or more bundle versions passes an identity check
  that claims all five are recorded. The shell currently supplies five
  hard-coded versions (`crates/quantrule-shell/src/diagnosis.rs:208-231`), so
  the practical exposure is the public analytics input and any future caller;
  the check itself is vacuous.
- Affected contracts/docs: `docs/reports/2026-10-31/diagnosis-v1.md:85`;
  `identity.report_versions` in `checks.csv` and `diagnosis.json`.
- Dependencies: none.
- Effort: small.
- Options:
  - A. Require `len() == 5`.
  - B. Require the five expected bundle names (`model`, `strategy`,
    `execution`, `portfolio`, `analytical`) and non-empty versions.
  - C. Fail on any missing version.
- Recommendation: B.
- Verification: a unit test with four versions expects `fail`; with the five
  expected names expects `pass`.
- Status: `Fixed`.
- Links: [#287](https://github.com/quant-rule/quant-rule/issues/287) · [#299](https://github.com/quant-rule/quant-rule/pull/299).

### AUD-2026-10-31-35 — Zero-trade average hold is published as zero

- Expected invariant: a missing measurement is null, never replaced by zero
  (AGENTS.md §3; `docs/reports/2026-10-31/diagnosis-v1.md:187`).
- Observed: `exit_reason_stats` sets `average_hold_bars: 0.0` when the reason
  has no trades (`compute.rs:500-504`). The field is `f64`
  (`crates/quantrule-analytics/src/diagnosis/types.rs:288`).
- Root cause: the empty-trade case was given a numeric default instead of an
  absent value.
- Impact: a reason with no trades publishes `0.0`, which is indistinguishable
  from a real zero-hold trade. The `trades` column already says zero, but the
  average is a missing measurement and should be null.
- Affected contracts/docs: `strategy-exit-reasons.csv`;
  `docs/reports/2026-10-31/diagnosis-v1.md:43,187`.
- Dependencies: `text.rs` and the CSV writer both need the optional type.
- Effort: small.
- Options:
  - A. Change `average_hold_bars` to `Option<f64>` and emit `null`.
  - B. Keep `f64` and use NaN.
  - C. Omit the row when `trades == 0` (loses the fixed reason order).
- Recommendation: A.
- Verification: a unit test with a zero-trade reason expects `None` in the
  struct, `null` in JSON, and an empty CSV cell; a reason with trades still
  emits the mean.
- Status: `Planned`.
- Links: [#289](https://github.com/quant-rule/quant-rule/issues/289).

### AUD-2026-10-31-36 — Manifest omits itself from the artifact table

- Expected invariant: the manifest carries the artifact names and byte lengths
  (`docs/reports/2026-10-31/diagnosis-v1.md:52`).
- Observed: `manifest()` writes `artifacts: {artifacts.len() + 2} (including
  this manifest)` (`crates/quantrule-shell/src/diagnosis.rs:886-890`), then
  iterates only the pre-manifest artifacts and appends a `diagnosis.json` row
  (`:892-895`). `manifest.txt` has no row. The observed
  `target/report-tests/diagnosis-command/diagnosis/manifest.txt` says
  `artifacts: 17 (including this manifest)` and has 16 table rows; the
  directory has 17 files.
- Root cause: the manifest is built before its own bytes exist, so it cannot
  know its own length in the same pass.
- Impact: a consumer cannot verify the complete artifact set from the manifest
  alone; the count and the table disagree.
- Affected contracts/docs: `docs/reports/2026-10-31/diagnosis-v1.md:52`;
  `manifest.txt`.
- Dependencies: none.
- Effort: small; a two-pass build or a fixed-width placeholder can compute the
  final manifest length.
- Options:
  - A. Add the `manifest.txt` row with its final byte length.
  - B. Change the header to "16 listed + this manifest" and keep the table as
    is.
  - C. Drop the count from the header.
- Recommendation: A.
- Verification: a test parses the manifest rows and asserts the name set equals
  the 17 files in the directory, including `manifest.txt`.
- Status: `Planned`.
- Links: [#288](https://github.com/quant-rule/quant-rule/issues/288).

### AUD-2026-10-31-37 — Enriched-trade MFE can be negative

- Expected invariant: MFE is non-negative and MAE is non-positive, both
  measured from the entry price in the trade's direction
  (`docs/reports/2026-10-31/diagnosis-v1.md:157-159`).
- Observed: for a long, `mfe = highest - entry_price`; for a short,
  `mfe = entry_price - lowest` (`compute.rs:883-897`). `entry_price` is the
  fill price including slippage. If the fill is above the entry bar's high for
  a long (or below its low for a short), `highest`/`lowest` over the entry bar
  does not reach the fill, so MFE is negative (and MAE can be positive).
  `slippage_bps` has no upper bound
  (`crates/quantrule-backtest/src/execution/config.rs:44-60`), so this is
  reachable with a valid config.
- Root cause: the contract assumes the entry fill lies inside the entry bar's
  high-low range; the execution model permits a fill outside it.
- Impact: a documented invariant is violated. Consumers that assume
  non-negative MFE or non-positive MAE can misread the excursion; the
  contradiction is visible in `trades_enriched.csv` and `diagnosis.json`.
- Affected contracts/docs: `docs/reports/2026-10-31/diagnosis-v1.md:157-159`.
- Dependencies: the decision between clamping and changing the contract.
- Effort: small.
- Options:
  - A. Clamp MFE at 0 and MAE at 0 (conforms to the current contract; loses the
    information that the fill was outside the bar).
  - B. Amend the contract to state that excursions are fill-relative and can
    cross zero when the fill is outside the bar range.
  - C. Measure excursions from the entry bar's reference/open price instead of
    the fill price.
- Recommendation: A for contract conformance; B if fill-relative excursions are
  the intended semantics. The choice is a contract decision for the human.
- Verification: a unit test with an entry fill above the bar high (long)
  expects `mfe >= 0` under A; under B the contract and test assert the
  documented sign rule. Add a property test `mfe >= 0`, `mae <= 0` if A is
  chosen.
- Status: `Planned`.
- Links: [#290](https://github.com/quant-rule/quant-rule/issues/290).

### AUD-2026-10-31-38 — Daily entry fills exclude an open entry

- Expected invariant: the daily series reports entry attempts and entry fills
  (`docs/reports/2026-10-31/diagnosis-v1.md:42,133`).
- Observed: `funnel_series` builds `fills` from `input.trades`
  (`compute.rs:589-593`), which contains closed trades only. An entry fill
  still open at run end is not a trade, so its day shows `entry_fills = 0`.
  The aggregate `funnel()` counts entry fills from `input.fills`
  (`compute.rs:384`), so the daily series and `funnel.csv` can disagree.
- Root cause: the daily series reuses the closed-trade list instead of the fill
  list.
- Impact: the daily funnel undercounts fills; a reader comparing the daily
  series with the aggregate funnel sees an unexplained difference.
- Affected contracts/docs: `docs/reports/2026-10-31/diagnosis-v1.md:42,133`;
  `FunnelPoint` (`types.rs:116-123`).
- Dependencies: none.
- Effort: small.
- Options:
  - A. Build the daily fills from `input.fills` filtered to
    `reason == "entry"`.
  - B. Add the open position's entry fill to the closed-trade list.
  - C. Document that the daily series counts closed trades only.
- Recommendation: A.
- Verification: a unit test with one closed trade and one open entry on
  different days expects both fills counted on their fill days.
- Status: `Planned`.
- Links: [#291](https://github.com/quant-rule/quant-rule/issues/291).

### AUD-2026-10-31-39 — Stale `lab.rs` module documentation

- Expected invariant: module docs explain the module's role and its boundary
  (AGENTS.md §5).
- Observed: `crates/quantrule-shell/src/command/lab.rs:1` says
  "The `lab summary` command", but the module also implements `lab diagnose`
  and `lab list`.
- Root cause: the module doc was not updated when the two commands were added.
- Impact: documentation is stale; a reader of the module header does not see
  the full command set.
- Affected contracts/docs: `crates/quantrule-shell/src/command/lab.rs:1-5`.
- Dependencies: none.
- Effort: trivial.
- Options:
  - A. Update the module doc to name all three commands and their shared
    boundary.
  - B. Split the module per command.
- Recommendation: A.
- Verification: `cargo doc` builds; the module header names `lab summary`,
  `lab diagnose`, and `lab list`.
- Status: `Planned`.
- Links: [#285](https://github.com/quant-rule/quant-rule/issues/285).

### AUD-2026-10-31-40 — `lab list` panic on a non-ASCII `config_hash`

- Expected invariant: the shell returns a typed error for invalid input; it does
  not panic on a malformed lab folder.
- Observed: the text branch slices the hash by byte length:
  `&config_hash[..config_hash.len().min(12)]`
  (`crates/quantrule-shell/src/command/lab.rs:717`). `config_hash` is read from
  an arbitrary `provenance.json` (`lab.rs:616-626`) with no hex validation. A
  multi-byte string whose byte 12 is not a character boundary panics.
- Root cause: the display truncation assumes an ASCII hex hash.
- Impact: a malformed lab folder panics `lab list` instead of returning an
  error. The command is read-only and is exposed to arbitrary lab roots.
- Affected contracts/docs: `lab list` command contract (README line 521).
- Dependencies: none.
- Effort: trivial.
- Options:
  - A. Use `config_hash.get(..12).unwrap_or(config_hash)` for the display.
  - B. Validate the hash as 16 hex characters when reading the lab and return a
    typed error for a malformed lab.
  - C. Skip malformed labs with a warning (changes the output contract).
- Recommendation: B, with A as a display defense.
- Verification: a lab fixture with a multi-byte `config_hash` makes `lab list`
  return an error without panicking.
- Status: `Planned`.
- Links: [#286](https://github.com/quant-rule/quant-rule/issues/286).

## Coverage and limitations

- The full `7c865de..c8b1dc1` diff was reviewed by area: backtest execution,
  portfolio, runner, strategy/context, model aggregate/event order, analytics
  diagnosis and research, shell commands/help/config/charts, and the new
  diagnosis artifacts.
- The remediation fixes were reviewed at the diff level and spot-checked
  against their claimed regression tests. AUD-09, AUD-17, AUD-25, AUD-27, and
  AUD-08 have the claimed evidence. AUD-23 does not: finding 33 records the
  incomplete guard and the missing test.
- No new probe or test was added. Findings 31–40 are based on code inspection
  and existing fixture/generated output; the verification column names the
  regression test each closure needs.
- Data files under `data/` and generated `target/` artifacts were not audited
  as code. `target/report-tests` was used only as observed output for the
  manifest finding.
- The project board could not be inspected: `gh` has no ProjectV2 #1 in this
  repository. Board state is not part of this audit.
- No performance benchmarks were run; the diff does not add a
  performance-sensitive path that would require one.

## Positive observations

- The critical gap-fill fix (reject a pending order on a non-contiguous
  execution bar, keep the open position's bracket active) is implemented and
  tested.
- The ledger's exit-quantity reconciliation, equity-state refresh, monthly
  return carry, and step-aware risk cap are implemented and tested.
- The strategy context identity check, missing-context hold, and weight-overflow
  guard are implemented and tested.
- The event batch order is stable by `(available_bar_index,
  occurred_bar_index)`, preserving detector order within a bar; the bar-28/37
  shell regression test exists.
- The Layer 3 export uses the configured ladder and writes null blocks for
  empty higher series; the short-source test asserts the fixed 356-column
  width.
- The diagnosis is deterministic: the byte-identical artifact test passes, and
  the diagnosis code uses ordered collections and no clock.

## Next step

The plan is approved and the issue tree is created: W1–W7 are #280–#286,
and the W4 and W5 sub-issues are #287–#291. W1 (#280) is `Fixed` by PR #294,
W2 (#281) by PR #296, W3 (#282) by PR #297, and W4a (#287) by PR #299; the
audit, plan, and log are in PR #293. The remaining findings are `Planned`
(W4b #288 continues W4) and move to `Fixed` when their pull requests exist.
