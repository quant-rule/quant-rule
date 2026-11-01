# Full-System Audit Remediation Plan

- Date: 2026-10-31 (UTC)
- Status: Approved
- Work item: `full-system-audit`
- Audit: [`docs/audits/2026-10-31/full-system-audit.md`](../../audits/2026-10-31/full-system-audit.md)
- AI transparency log: [`docs/ai-transparency-log/2026-10-31/full-system-audit.md`](../../ai-transparency-log/2026-10-31/full-system-audit.md)
- Baseline: last audited head `c8b1dc1`; head `e34693f`
- Human approval: approved on 2026-10-31 (UTC) via "continue"; D1–D17 accepted as recommended
- Claim type: program remediation plan; no strategy edge claim

## 1. Intent

Turn the 2026-10-31 full-system audit into a reviewable remediation plan. The
audit found 29 findings at `e34693f`: 0 Critical, 7 Major, 18 Minor, and 4
Info. The Major findings are concentrated in output safety, reproducibility,
evaluation evidence, and one false architecture baseline. The Minor findings
cover execution and model edge cases, store robustness, CLI contracts,
documentation drift, and test gaps.

The audit is read-only evidence. This plan is the only implementation
authorization, and only after the human approves it. It does not authorize
commits, pushes, or pull requests (AGENTS.md section 9).

## 2. Current state

- `main` is at `e34693f`; the last audited head was `c8b1dc1`.
- The audited delta is 70 files, +3,873/−273, 28 commits. The ten findings from
  the previous audit remain fixed.
- Global gates pass at the head: fmt, clippy with warnings denied,
  `cargo test --workspace` (1,518 passed, 0 failed, 7 ignored; 47 suites),
  rustdoc with warnings denied, and `git diff --check`.
- The audit, this plan, and the AI transparency log are merged as PR #320.
- The issue tree is created: W1–W11 are #308–#318. W1 (#308) and W2 (#309)
  are merged as PRs #319 and #321. W3 (#310) is implemented and opened as
  PR #322; the issue moves to `Done` on the board. The other leaves are
  `Todo`.

### 2.1 Planner verification

Every finding was re-read against `e34693f` by the same agent. The audit
finding was corrected once during triage: the entry-bar stop case (AUD-48)
does not pre-empt a target, because a valid candle's low is at or below its
open; the defect is the stop reference price. The severity is therefore Minor,
not Major, and the detail block records the corrected effect. No finding is
stale or a duplicate of AUD-2026-10-31-01..40.

| Finding | Re-verified at | Result |
|---|---|---|
| 41 | `command/fetch.rs:99-105,261`; `src/export.rs:149-177,273-277`; `command/chart.rs:109-113` | Confirmed: one output's final path can equal another's `.tmp`; the source can be overwritten |
| 42 | `command/strategy.rs:198-207,220-224,326-328`; `tests/strategy_run.rs:331-332` | Confirmed: config bounds are applied but not recorded |
| 43 | `src/diagnosis.rs:546-571`; `src/lab_summary.rs:552-586` | Confirmed: diagnosis omits the config-hash check |
| 44 | `diagnosis/compute.rs:232-233,1080-1104`; `summary.rs:18-19` | Confirmed: only two of six pairs |
| 45 | `diagnosis/compute.rs:1390-1392,1423-1439` | Confirmed: `Unknown` gets failure text |
| 46 | `research/mod.rs:148,239-259`; `research/render.rs:83-88` | Confirmed: only metadata is validated |
| 47 | `AGENTS.md:48`; `docs/architecture/README.md:43`; `Cargo.toml:8` | Confirmed: docs omit the data edge; no graph guard |
| 48 | `execution/engine.rs:727-733,832-848`; probe | Confirmed: stop reference is the bar open, not the stop; error ≤ slippage |
| 49 | `runner/mod.rs:280-282`; `tests/strategy_run.rs:423-436`; `README.md:540` | Confirmed: empty success vs documented failure |
| 50 | `diagnosis/text.rs:457,551-553,729-730` | Confirmed: `-` in CSV nulls |
| 51 | `research/metadata.rs:525-531`; `layers/mod.rs:229-233` | Confirmed: present, not configured, ladder |
| 52 | `progression/mod.rs:506-511`; `target.rs:169-179` | Confirmed: zero ATR is dropped |
| 53 | `strategy/ladder.rs:151-153,277-285`; `context/mod.rs:166-168,739-742` | Confirmed: unavailable progression returns `Some(false)` |
| 54 | `execution/engine.rs:642-653`; `portfolio/sizing.rs:237-249` | Confirmed: cap floor has no tolerance |
| 55 | `timeframe_set.rs:164-171`; `aggregate/progression.rs:218`; `context/mod.rs:556` | Confirmed: unchecked `i64` arithmetic |
| 56 | `consensus.rs:57-69`; `aggregate/mod.rs:186-198` | Confirmed: NaN counts neutral/present |
| 57 | `aggregate/events.rs:186-190`; `report/order.rs:13-23` | Confirmed: shorter tie-break key |
| 58 | `store/mod.rs:57-63,234-241` | Confirmed: WAL is enabled before the version check |
| 59 | `timeframe.rs:110-111`; `store_query.rs:238-245`; `fetch.rs:53-57` | Confirmed: 4h accepted, docs say four |
| 60 | `command/lab.rs:648-651,364-410`; `lab_summary.rs:173-221` | Confirmed: halted defaults false; provenance subsets differ |
| 61 | `help.rs:255,267`; `command/lab.rs:580-583`; `fetch.rs:66-72` | Confirmed: wrong default and "required" PNG |
| 62 | `README.md:250,286`; `per-engine-reports.md:47-48,137-141`; release links | Confirmed: eight doc mismatches, four broken links |
| 63 | context, ledger, diagnosis, lab, report, architecture tests | Confirmed: the named gaps are absent |
| 64 | `ledger.rs:210,249,349,425,437-447,456-475` | Confirmed: metric and halt observe different points |
| 65 | `runner_metrics_golden.csv:20`; `runner/tests.rs:188-208` | Confirmed: stray line is ignored |
| 66 | `position.rs:159-160`; `context/mod.rs:385-389` | Confirmed: both public items are unreachable |
| 67 | `schema.rs:18-28`; `command/store.rs:88-91`; `store/mod.rs:242-243` | Confirmed: internal ids, `--` names, version-0 migration |
| 68 | `src/diagnosis.rs:209-231` | Confirmed: constants, not observed versions |
| 69 | `render.rs:38-41`; `timeframe.rs:348-350`; `market-report.md:86` | Confirmed: stale docs |

The audit's claim that AUD-48 could convert a target into a loss was rejected:
a valid candle cannot have `open < low`, so the stop is always touched when
the pre-entry open is beyond it. The corrected finding is the stop reference
price, and the probe reproduces it.

## 3. Scope

Included:

- The 29 findings, grouped into eleven work items.
- A regression guard for every accepted fix that fails before and passes after,
  or a written reason why a fail-before test is impossible.
- Contract, module, command-reference, and architecture-baseline updates that
  follow the behavior.
- Quant Validator records for W4, W7, and W8.
- A crate-graph test, a relative-link check, and the named test gaps.

Explicit non-goals:

- New strategy hypotheses, thresholds, fitted weights, or signal-path changes.
- Statistical, machine-learning, or neural-network signal generation.
- New dependencies, `unsafe`, performance work, or live trading.
- Changing an evaluation threshold to turn a failing lab into a passing one.
- Rewriting dated historical records beyond the corrections named here.
- Creating fix issues or starting implementation before this plan is approved.
- Commits, pushes, or pull requests (AGENTS.md section 9).

## 4. Assumptions and uncertainty

- AUD-48's fix changes the stop fill price only when the stop distance is
  smaller than the entry slippage. Any golden with a tight stop changes; the
  diff must be reviewed for that exact case.
- AUD-52's fix changes a valid flat-prefix series from "no target/reason 4" to
  "degenerate target/reason 3". Existing goldens with a zero ATR at a pivot
  origin may change.
- AUD-53's fix changes an unavailable progression context from an exit to a
  hold. Existing strategy goldens may change only if they contain that state.
- AUD-49's disposition is a product decision: keep the validated empty-success
  behavior and fix the README, or restore the older typed error. The plan
  recommends keeping the behavior and making zero bars explicit.
- AUD-64's disposition is a policy decision: halt at fills/funding, or declare
  the halt mark-only and align the metric.
- AUD-62 contains two items where the doc may be the intended contract:
  strategy summary field names and `store show` ordering. The human decides
  whether the doc or the code changes.
- AUD-41's identity check must work when the target does not exist yet; parent
  canonicalization is sufficient, and file identity is an additional check for
  existing targets.
- A finding may become stale before its issue opens. Re-verify against `main`
  before starting each work item.

## 5. Decisions

Each decision is proposed; none is approved until the human accepts this plan.

| ID | Findings | Decision | Options | Recommendation | Approval |
|---|---|---|---|---|---|
| D1 | 41 | Output-path identity | A resolve final and temp paths; B unique temp names; C both | C | Pending |
| D2 | 42, 49 | Strategy-run contract | A record `effective_period` and document empty success with `bars: 0`; B ignore config bounds; C restore the typed error | A | Pending |
| D3 | 43, 60 | Lab evidence and identity | A shared rerun state plus full provenance validation and null `halted`; B document the differences | A | Pending |
| D4 | 44 | Axis correlation | A all six pairs; B narrow the check to two pairs | A | Pending |
| D5 | 45 | Triage text | A status-aware text; B omit Unknown checks | A | Pending |
| D6 | 46 | Research export validation | A implement the contract table before render; B narrow the contract | A | Pending |
| D7 | 47 | Architecture baseline | A document analytics→data and add a crate-graph test; B remove the dependency | A | Pending |
| D8 | 48 | Entry-bar stop reference | A evaluate from the fill on the entry bar; B reject a stop tighter than slippage | A | Pending |
| D9 | 54 | Risk-cap step floor | A reuse the tolerant `floored_steps`; B add a local tolerance | A | Pending |
| D10 | 64 | Drawdown and halt | A check the halt after open/close/funding; B declare mark-only and align the metric | A unless the human chooses B | Pending |
| D11 | 66 | Lifecycle and context error | A implement `Closed`/`MissingContextTimeframe`; B remove them and fix the docs | B | Pending |
| D12 | 52 | Zero ATR | A keep `Some(0.0)`, emit reason 3, guard displacement; B amend the contract to reason 4 | A | Pending |
| D13 | 55 | Extreme timestamps | A validate the range at the boundary; B checked arithmetic; C both | C | Pending |
| D14 | 59 | `4h` in store/fetch | A reject at parse time; B support H4 and update the docs | A | Pending |
| D15 | 62, 69 | Documentation authority | A update docs to code, except where the human says the doc is the contract; B change the code | A | Pending |
| D16 | 68 | Report versions | A read the five bundle versions; B document the constants | A | Pending |
| D17 | 67 | Store identity | A omit ids, validate names, set `application_id`, document verify; B keep and document | A | Pending |

## 6. Work breakdown

| Work item | Findings | Severity | Deliverable | Roles | Depends on |
|---|---|---|---|---|---|
| W1 | 41 | Major | Output-path identity and safe temp commits | Implementation, Test, Code Reviewer | D1 |
| W2 | 42, 49 | Major/Minor | Strategy-run period provenance and empty-result contract | Implementation, Test, Code Reviewer | D2 |
| W3 | 43, 60 | Major/Minor | One lab evidence/identity definition | Implementation, Test, Code Reviewer | D3 |
| W4 | 44, 45, 50, 68 | Major/Major/Minor/Info | Diagnosis checks, triage, CSV nulls, observed versions | Quant Developer, Test, Quant Validator | D4, D5, D16 |
| W5 | 46, 51 | Major/Minor | Research export validation and manifest ladder | Implementation, Test, Code Reviewer | D6 |
| W6 | 47 | Major | Architecture baseline and crate-graph guard | Architect, Test, Code Reviewer | D7 |
| W7 | 48, 53, 54, 64, 66 | Minor/Info | Execution, context, risk-cap, drawdown, lifecycle | Quant Developer, Test, Quant Validator, Code Reviewer | D8, D9, D10, D11 |
| W8 | 52, 56, 57 | Minor | Model zero ATR, non-finite values, event tie-break | Quant Developer, Test, Quant Validator | D12 |
| W9 | 55, 58, 59, 67 | Minor/Info | Timestamp bounds, store open safety, 4h rejection, store identity | Implementation, Test, Code Reviewer | D13, D14, D17 |
| W10 | 61, 62, 69 | Minor/Info | Help, command/report docs, stale code docs | Docs, Implementation, Code Reviewer | D15 |
| W11 | 63, 65 | Minor | Test gaps, architecture guard, golden parser | Test Engineer, Code Reviewer | — |

Recommended order: W1, W3, W2, W4, W5, W6, W7, W8, W9, W10, W11.
W4, W7, and W8 change measurement or execution behavior and need the Quant
Validator. W1 and W3 touch shell output and lab evidence. W6 is documentation
and a guard. W11 can land with each work item and close the remaining gaps at
the end.


### Issue tree

| Work item | Issue |
|---|---|
| W1 | [#308](https://github.com/quant-rule/quant-rule/issues/308) |
| W2 | [#309](https://github.com/quant-rule/quant-rule/issues/309) |
| W3 | [#310](https://github.com/quant-rule/quant-rule/issues/310) |
| W4 | [#311](https://github.com/quant-rule/quant-rule/issues/311) |
| W5 | [#312](https://github.com/quant-rule/quant-rule/issues/312) |
| W6 | [#313](https://github.com/quant-rule/quant-rule/issues/313) |
| W7 | [#314](https://github.com/quant-rule/quant-rule/issues/314) |
| W8 | [#315](https://github.com/quant-rule/quant-rule/issues/315) |
| W9 | [#316](https://github.com/quant-rule/quant-rule/issues/316) |
| W10 | [#317](https://github.com/quant-rule/quant-rule/issues/317) |
| W11 | [#318](https://github.com/quant-rule/quant-rule/issues/318) |

Only one work item is `In Progress` at a time. W4 and W5 both touch analytics
diagnosis/research but different modules; they still run sequentially to keep
the review bounded. W7 and W8 touch backtest and model; W7 runs first.

### W1 — Output-path identity (Major)

- Resolve the final path: canonicalize the parent (or absolutize when it does
  not exist) and compare parent + file name; when the target exists, compare
  device/inode where the platform supports it.
- Reject any final path equal to another output's final or temporary path
  before the first write. Keep unique temporary names as defense in depth.
- Apply the rule to `fetch` (CSV/PNG), `market-chart` (input/PNG), and any other
  command with two outputs or an input/output pair.
- Closure tests: a CSV equal to the PNG's declared `<target>.tmp.<pid>` path,
  and `market-chart data.csv --png ./data.csv`, fail before writing; a normal
  pair still writes both.
- Paths: `crates/quantrule-shell/src/command/fetch.rs`,
  `src/fetch.rs`, `src/export.rs`, `src/command/chart.rs`, and the command
  tests.
- Implementation evidence (#308): `export::outputs_conflict` and process-scoped
  temp names are implemented on `fix/308-output-path-collision`; full gates pass
  (1,525 tests). PR [#319](https://github.com/quant-rule/quant-rule/pull/319) is
  open.
- Issue: [#308](https://github.com/quant-rule/quant-rule/issues/308).

### W2 — Strategy-run period and empty result (Major/Minor)

- Add an `effective_period` (resolved from CLI dates or `config.data.bounds`)
  to the JSON envelope; keep `requested_period` as the CLI request and document
  the difference.
- Decide D2: keep the validated empty-success behavior and document it, or
  restore the typed error. Under the recommended A, add `bars` and
  `actual_period` so zero bars are explicit, and update the README.
- Closure tests: one-day config over a multi-day source records the one-day
  effective period; a source with no execution bar reports `bars: 0`. The
  five-row short series remains the context-missing case: `bars: 1`, no records.
- Paths: `crates/quantrule-shell/src/command/strategy.rs`,
  `crates/quantrule-shell/tests/strategy_run.rs`, `README.md`,
  `crates/quantrule-shell/src/command/help.rs`.
- Implementation evidence (#309): `effective_period`, `actual_period`, and
  `bars` are implemented on `fix/309-strategy-run-period`; full gates pass
  (1,528 tests). PR [#321](https://github.com/quant-rule/quant-rule/pull/321) is open.
- Issue: [#309](https://github.com/quant-rule/quant-rule/issues/309).

### W3 — Lab evidence and identity (Major/Minor)

- Move the rerun comparison beside `lab_evidence` with four states (absent,
  unreadable, differs, matches); require matching `config_hash` before
  comparing the five summary fields. Use it from `lab summary` and
  `lab diagnose`.
- Validate every provenance identity field (`data_version`, `model_version`,
  `config_hash`) against its report and the supplied config in both commands.
- Make `lab list`'s `halted` an `Option<bool>`; render `—` in text and `null`
  in JSON for a missing value.
- Closure tests: copied report and different-config rerun are `unknown` in
  both commands; matching config is `pass`; a provenance model mismatch fails
  both; a lab without `halted` renders unavailable.
- Paths: `crates/quantrule-shell/src/lab_evidence.rs`, `src/lab_summary.rs`,
  `src/diagnosis.rs`, `src/command/lab.rs`, the shell diagnosis/lab tests.
- Implementation evidence (#310): the shared rerun status, provenance
  identity checks, and nullable halt are implemented on
  `fix/310-lab-evidence-identity`; full gates pass (1,539 tests). PR [#322](https://github.com/quant-rule/quant-rule/pull/322)
  is open.
- Issue: [#310](https://github.com/quant-rule/quant-rule/issues/310).

### W4 — Diagnosis checks, triage, CSV nulls, observed versions (Major/Major/Minor/Info)

- Compute all six pairwise axis correlations and make the check evidence name
  the worst pair. Keep the two existing fields or add a pair table; do not
  break the report contract without a decision.
- Make triage status-aware: `Unknown` says record/measure; failure text only
  on `Fail`. Keep the existing failure text for a real failure.
- Use empty cells for missing numeric values in `checks.csv` and
  `model-health.csv`; keep `-` in `summary.txt`.
- Read the five bundle versions for `backtest --diagnose` instead of
  synthesizing them from constants.
- Closure tests: all six pairs at the 0.70 boundary; Unknown/fail text;
  empty CSV cells; a changed bundle version fails the identity check.
- Validator record: `docs/reports/2026-10-31/w4-diagnosis-checks-validation.md`.
- Paths: `crates/quantrule-analytics/src/diagnosis/compute.rs`,
  `src/diagnosis/text.rs`, `crates/quantrule-shell/src/diagnosis.rs`,
  `crates/quantrule-analytics/tests/diagnosis.rs`, shell diagnosis tests.
- Issue: [#311](https://github.com/quant-rule/quant-rule/issues/311).

### W5 — Research export contract (Major/Minor)

- Implement the contract's pre-render validation table (row counts, bar
  identity, resolution range/consistency, as-of causality, event/leg
  availability, dictionary/code completeness, 100k cap) and fail before
  returning files.
- Pass the configured ladder into `manifest_json` so `timeframes_secs` records
  the configured ladder, not only present rows.
- Closure tests: one invalid fixture per table row fails with no files; the
  valid export stays byte-identical; a short source records the configured
  higher timeframe.
- Paths: `crates/quantrule-analytics/src/research/mod.rs`,
  `src/research/render.rs`, `src/research/metadata.rs`,
  `src/layers/mod.rs`, `tests/research_verification.rs`.
- Issue: [#312](https://github.com/quant-rule/quant-rule/issues/312).

### W6 — Architecture baseline and crate-graph guard (Major)

- Update `AGENTS.md`, `docs/architecture/README.md`,
  `docs/architecture/characteristics.md`, and the diagnosis ADR to record the
  analytics→data edge, or execute D7=B to remove the dependency.
- Add a test that parses each workspace `Cargo.toml` and asserts the allowed
  edges; a forbidden edge fails with the crate pair named.
- Closure test: the current graph passes; a synthetic forbidden edge fails.
- Paths: `AGENTS.md`, `docs/architecture/`, `docs/adr/2026-10-31/`,
  `crates/quantrule-shell/tests/architecture.rs` or a workspace test.
- Issue: [#313](https://github.com/quant-rule/quant-rule/issues/313).

### W7 — Execution, context, risk cap, drawdown, lifecycle (Minor)

- AUD-48: on the entry bar, use the fill as the open reference and evaluate the
  stop/target from the fill and the bar's high/low; use the open gap rule only
  on later bars. Keep the existing entry-bar stop test.
- AUD-53: return `None` for an unavailable progression direction; a measured
  Mixed direction still returns `Some(false)`.
- AUD-54: reuse the tolerant step floor in the cap path and assert the largest
  step multiple.
- AUD-64: implement D10.
- AUD-66: implement D11 (recommended B: remove the dead variants and correct
  the contract).
- Closure tests: the probe case yields reference 100.01; an unavailable
  progression holds; exact cap boundaries fit; a drawdown breach at a fill
  either halts or the contract says mark-only; no dead public variant remains.
- Validator record: `docs/reports/2026-10-31/w7-execution-portfolio-validation.md`.
- Paths: `crates/quantrule-backtest/src/execution/engine.rs`,
  `src/execution/position.rs`, `src/strategy/ladder.rs`, `src/context/mod.rs`,
  `src/portfolio/ledger.rs`, `src/portfolio/records.rs`, the backtest tests.
- Issue: [#314](https://github.com/quant-rule/quant-rule/issues/314).

### W8 — Model robustness (Minor)

- AUD-52: keep `Some(0.0)` as a valid ATR at the origin; guard
  `displacement_atr` so it is null at zero; let a degenerate target produce
  reason 3; add a flat-prefix test.
- AUD-56: filter non-finite values before counting, means, shares, and
  weakest-percentile selection; add non-finite tests.
- AUD-57: use the shared `event_cmp` for the aggregate latest event and
  divergence direction; add a same-bar tie test.
- Closure tests: flat prefix → reason 3; NaN is never neutral; aggregate and
  report agree on the latest event.
- Validator record: `docs/reports/2026-10-31/w8-model-robustness-validation.md`.
- Paths: `crates/quantrule-model/src/axes/progression/`,
  `src/consensus.rs`, `src/aggregate/`, `src/report/order.rs`, the model tests.
- Issue: [#315](https://github.com/quant-rule/quant-rule/issues/315).

### W9 — Data, store, and timestamp bounds (Minor/Info)

- AUD-55: validate the supported timestamp range at `Candle::new`,
  `TimeframeSet::push_values`, `SourceCadence::observe`, and the fetch sink;
  use checked arithmetic in bucket/gap/context/runner additions; make the CSV
  formatter validate the year.
- AUD-58: read `user_version` before enabling WAL; reject a newer schema before
  any persistent PRAGMA.
- AUD-59: whitelist `1m`, `5m`, `15m`, `1h` in the store and fetch parsers.
- AUD-67: implement D17.
- Closure tests: extreme timestamps return typed errors; a newer-version file
  is byte-unchanged; `4h` is a usage error; the store identity tests pass.
- Paths: `crates/quantrule-data/src/`, `crates/quantrule-backtest/src/context/`,
  `src/runner/`, `crates/quantrule-shell/src/store/`,
  `src/command/store*.rs`, `src/command/fetch.rs`, `src/source/`.
- Issue: [#316](https://github.com/quant-rule/quant-rule/issues/316).

### W10 — Documentation sweep (Minor/Info)

- AUD-61: correct the `lab diagnose --out` default and the optional `fetch
  --png` description; update `help-all.txt`.
- AUD-62: update the eight items listed in the audit, or change the code where
  the human decides the doc is the contract (strategy summary fields, store
  order). Add a relative-link check to CI.
- AUD-69: correct the four-series, full-snapshot, `MarketReport::compute`,
  `ReportLeg`, reason 5, JSONP field, and default-path docs; document reason 5
  as dashboard-only or emit it consistently.
- Closure tests: `help all` matches the parser; the link check is zero; each
  doc row matches code; `cargo doc` passes.
- Paths: `README.md`, `docs/README.md`, `docs/reports/2026-10-28/`,
  `docs/analytics/`, `docs/model/`, `docs/adr/2026-10-24/`,
  `crates/quantrule-shell/src/command/help.rs`,
  `src/source/jsonp.rs`, `src/render.rs`, `crates/quantrule-model/src/`
  docs, and `.github/workflows/ci.yml` for the link check.
- Issue: [#317](https://github.com/quant-rule/quant-rule/issues/317).

### W11 — Test integrity and guards (Minor)

- AUD-63: add the multi-context weight test, the nonzero fee/funding ledger and
  runner tests, the diagnosis pass/fail/unknown table, `lab_evidence` unreadable
  state, and the non-vacuous report/detector tests; expand the architecture
  guard to every renderer and the diagnosis writer; standardize duplicate-option
  handling with tests.
- AUD-65: remove the stray fixture line; make the parser reject unknown rows
  and assert the expected row count.
- Closure tests: each named test fails before the corresponding fix (or records
  why a fail-before test is impossible).
- Paths: the named test modules and fixtures; `crates/quantrule-shell/tests/
  architecture.rs`; the command parsers.
- Issue: [#318](https://github.com/quant-rule/quant-rule/issues/318).

## 7. Acceptance criteria

- Every accepted finding has a regression guard that fails before the fix and
  passes after it, or a written reason why a fail-before test is impossible.
- W1: colliding output paths are rejected before writing; a normal pair writes.
- W2: the envelope records the effective analyzed period; the empty-result
  contract matches the command and reports zero bars (or the typed error is
  restored).
- W3: both commands agree on OOS and rerun evidence; provenance identity is
  fully validated; a missing halt is null/unavailable.
- W4: all six correlation pairs are measured; triage text matches status; CSV
  nulls are empty cells; `backtest --diagnose` reads observed versions.
- W5: the contract validation table runs before writing; the manifest records
  the configured ladder.
- W6: the architecture docs match the manifests; a crate-graph test guards
  the allowed edges.
- W7: the entry-bar stop reference is the stop price; unavailable context
  holds; the cap uses the tolerant floor; the drawdown/halt decision is
  implemented; no dead lifecycle variant remains.
- W8: zero ATR is a value or the contract says otherwise; non-finite values are
  excluded; the latest-event key is shared.
- W9: extreme timestamps fail typed; a newer-schema DB is untouched; `4h` is
  rejected; store identity is documented and tested.
- W10: the command/report/contract docs match the code; the link check is
  clean.
- W11: the named tests and guards exist and exercise the claimed behavior.
- The global gates in section 9 pass and no test is weakened, removed, or
  ignored.
- The audit findings move to `Fixed` with PR links, then to `Verified` only
  after the Improvement Verifier reproduces them.
- No open Critical finding exists; none was found.

## 8. Traceability

| Finding | Work item | Issue | Closure verification | Status |
|---|---|---|---|---|
| AUD-2026-10-31-41 | W1 | [#308](https://github.com/quant-rule/quant-rule/issues/308) | Colliding output paths rejected before any write; PR [#319](https://github.com/quant-rule/quant-rule/pull/319) | Fixed |
| AUD-2026-10-31-42 | W2 | [#309](https://github.com/quant-rule/quant-rule/issues/309) | Effective period equals the analyzed range; PR [#321](https://github.com/quant-rule/quant-rule/pull/321) | Fixed |
| AUD-2026-10-31-43 | W3 | [#310](https://github.com/quant-rule/quant-rule/issues/310) | One rerun state from both commands; PR [#322](https://github.com/quant-rule/quant-rule/pull/322) | Fixed |
| AUD-2026-10-31-44 | W4 | [#311](https://github.com/quant-rule/quant-rule/issues/311) | All six pairs; worst pair named | Planned |
| AUD-2026-10-31-45 | W4 | [#311](https://github.com/quant-rule/quant-rule/issues/311) | Unknown text does not assert failure | Planned |
| AUD-2026-10-31-46 | W5 | [#312](https://github.com/quant-rule/quant-rule/issues/312) | Invalid dataset fails before writing | Planned |
| AUD-2026-10-31-47 | W6 | [#313](https://github.com/quant-rule/quant-rule/issues/313) | Docs match manifests; graph test passes | Planned |
| AUD-2026-10-31-48 | W7 | [#314](https://github.com/quant-rule/quant-rule/issues/314) | Probe stop reference is 100.01 | Planned |
| AUD-2026-10-31-49 | W2 | [#309](https://github.com/quant-rule/quant-rule/issues/309) | Documented empty result with zero bars, or typed error; PR [#321](https://github.com/quant-rule/quant-rule/pull/321) | Fixed |
| AUD-2026-10-31-50 | W4 | [#311](https://github.com/quant-rule/quant-rule/issues/311) | Empty CSV cells for missing numbers | Planned |
| AUD-2026-10-31-51 | W5 | [#312](https://github.com/quant-rule/quant-rule/issues/312) | Manifest lists the configured ladder | Planned |
| AUD-2026-10-31-52 | W8 | [#315](https://github.com/quant-rule/quant-rule/issues/315) | Zero ATR → reason 3 | Planned |
| AUD-2026-10-31-53 | W7 | [#314](https://github.com/quant-rule/quant-rule/issues/314) | Unavailable progression → Hold | Planned |
| AUD-2026-10-31-54 | W7 | [#314](https://github.com/quant-rule/quant-rule/issues/314) | Exact cap boundary fits the largest step | Planned |
| AUD-2026-10-31-55 | W9 | [#316](https://github.com/quant-rule/quant-rule/issues/316) | Extreme timestamps return typed errors | Planned |
| AUD-2026-10-31-56 | W8 | [#315](https://github.com/quant-rule/quant-rule/issues/315) | NaN is excluded, never neutral | Planned |
| AUD-2026-10-31-57 | W8 | [#315](https://github.com/quant-rule/quant-rule/issues/315) | Aggregate and report agree on latest | Planned |
| AUD-2026-10-31-58 | W9 | [#316](https://github.com/quant-rule/quant-rule/issues/316) | Newer DB is byte-unchanged | Planned |
| AUD-2026-10-31-59 | W9 | [#316](https://github.com/quant-rule/quant-rule/issues/316) | `4h` is a usage error | Planned |
| AUD-2026-10-31-60 | W3 | [#310](https://github.com/quant-rule/quant-rule/issues/310) | Missing halt null; full provenance check; PR [#322](https://github.com/quant-rule/quant-rule/pull/322) | Fixed |
| AUD-2026-10-31-61 | W10 | [#317](https://github.com/quant-rule/quant-rule/issues/317) | Help matches parser | Planned |
| AUD-2026-10-31-62 | W10 | [#317](https://github.com/quant-rule/quant-rule/issues/317) | Docs match code; link check clean | Planned |
| AUD-2026-10-31-63 | W11 | [#318](https://github.com/quant-rule/quant-rule/issues/318) | Named tests and guards exist | Planned |
| AUD-2026-10-31-64 | W7 | [#314](https://github.com/quant-rule/quant-rule/issues/314) | Halt and metric observe the same points | Planned |
| AUD-2026-10-31-65 | W11 | [#318](https://github.com/quant-rule/quant-rule/issues/318) | Fixture has only metric rows; unknown row fails | Planned |
| AUD-2026-10-31-66 | W7 | [#314](https://github.com/quant-rule/quant-rule/issues/314) | No dead public variant | Planned |
| AUD-2026-10-31-67 | W9 | [#316](https://github.com/quant-rule/quant-rule/issues/316) | Stable dump, names validated, application id | Planned |
| AUD-2026-10-31-68 | W4 | [#311](https://github.com/quant-rule/quant-rule/issues/311) | Observed versions used | Planned |
| AUD-2026-10-31-69 | W10 | [#317](https://github.com/quant-rule/quant-rule/issues/317) | Doc greps match code | Planned |

No finding is deferred. No finding is marked `Won't fix`; only the human can
decide that.

## 9. Verification

Global gates, run at the end of each work item and before its pull request:

| Check | Command | Expected |
|---|---|---|
| Format | `cargo fmt --all -- --check` | exit 0 |
| Lints | `cargo clippy --all-targets -- -D warnings` | exit 0 |
| Tests | `cargo test --workspace` | all pass, no ignored addition |
| Docs | `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | exit 0 |
| Whitespace | `git diff --check` | clean |
| Links | the new relative-link check | zero broken links |

Targeted evidence:

- W1: shell tests with `.tmp` and `./` aliases; `git status` shows no file
  changes on rejection.
- W2: one-day config/multi-day source; short-series envelope.
- W3: copied/different-config rerun; missing `halted`; provenance mismatch.
- W4: six-pair synthetic correlation; unknown/fail triage; CSV empty cells;
  changed bundle version.
- W5: one invalid fixture per validation row; byte-identical valid export;
  short-source manifest.
- W6: crate-graph test; doc grep.
- W7: the execution probe, context unavailable, cap boundary, drawdown/halt,
  lifecycle test.
- W8: flat-prefix zero ATR; NaN consensus/aggregate; same-bar event tie.
- W9: extreme timestamps; newer-schema DB hash/mtime; 4h usage error; store
  identity.
- W10: help fixture; link check; doc greps.
- W11: each new test's fail-before/pass-after behavior.

The Quant Validator reproduces W4, W7, and W8 from committed fixtures and
records the commands and observed results. W1, W2, W3, W5, W6, W9, W10, and
W11 are covered by tests and code review only. No performance benchmark is
required: the plan adds no performance-sensitive path.

Limitations: the closure tests use unit and integration fixtures, not live
market data; the Quant Validator's reproduction is a sequential pass, not an
independent model judgment, and it is recorded as such.

## 10. Risks and recovery

- W1 changes only the rejection path and temp naming; a normal write must stay
  byte-identical.
- W2 changes the envelope shape (new fields) and possibly the empty-result
  contract; version the envelope and document the change.
- W3 changes `lab list` JSON `halted` from `false` to `null` for a malformed
  lab; update the text/JSON tests and the README.
- W4 changes check evidence and triage text, not thresholds. A failing check
  remains evidence, not a rule. Do not tune a threshold to make a lab pass.
- W5 changes write-time failure behavior; the valid export must stay
  byte-identical.
- W6 is documentation plus a new test; no runtime behavior.
- W7 changes fill prices for a tight-stop entry and a context exit; review any
  golden diff for exactly those cases. W7 also changes the drawdown halt
  behavior under D10=A; a lab may halt earlier.
- W8 changes a flat-prefix progression result and non-finite public input
  handling; review any model golden diff for exactly those cases.
- W9 changes public timestamp validation and store opening; existing valid
  inputs must keep their behavior.
- W10 is docs; the only risk is changing a contract the code should follow.
  The human decides the two items where the doc may be authoritative.
- W11 is tests and guards; it does not change product behavior.
- Recovery: one revertable pull request per work item; diagnosis and research
  artifacts are regenerated; there is no stored-schema migration that cannot
  be reverted.

## 11. Stop conditions

Stop and ask the human when:

- A decision in section 5 is not accepted as recommended, especially D2,
  D8, D10, D11, D12, and D15.
- A fix requires a new dependency, an `unsafe` block, a signal-path change, or
  a command-contract change beyond the items named here.
- A golden fixture changes outside the exact cases named in section 4.
- A finding no longer reproduces, or the fix reveals a wider defect.
- The audit/plan commit, push, or pull request has not been authorized.

## 12. Approval

Approved on 2026-10-31 (UTC) via the human's "continue"; D1–D17 accepted as
recommended. The approval authorizes the eleven work items and the issue tree.
It does not authorize commits, pushes, merges, or pull requests (AGENTS.md
section 9). The Planner created the issue tree (#308–#318) and marked the findings
`Planned`.
