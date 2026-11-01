# Full-System Audit

- Date: 2026-10-31 (UTC)
- Audit ID: AUDIT-2026-10-31-full-system
- Status: Drafted (uncommitted); finding 41 `Fixed`; findings 42–69 `Planned`; issue tree #308–#318 created; W1 PR #319 open
- Auditor: Audit Agent (AI), sequential read-only pass
- Baseline: `main` at `c8b1dc1` (last audited head, merge of PR #279)
- Head: `main` at `e34693f` (merge of PR #307)
- Delta: 70 files, +3,873/−273, 28 commits (`c8b1dc1..e34693f`)
- Toolchain: `rustc 1.99.1 (58cf8f9c3 2026-10-13)`; `cargo 1.99.1 (5f94df478 2026-08-27)`
- Data: committed fixtures and existing `target/report-tests` output; no live source
- Probe: `/tmp/opencode/execution-probe` (outside the repository; not committed)
- Limitations: no independent verifier or independent model judgment; findings are code inspection, the named commands, and one probe
- Planner: sequential pass by the same agent; plan [`docs/execution-plans/2026-10-31/full-system-audit.md`](../../execution-plans/2026-10-31/full-system-audit.md) marked `Approved`; issue tree #308–#318

## Summary

| Severity | Count |
|---|---:|
| Critical | 0 |
| Major | 7 |
| Minor | 18 |
| Info | 4 |

The audit pass was read-only: it did not modify code, tests, issues, or pull
requests. After the human approved the plan, the Planner pass created the issue
tree #308–#318. Every finding has a reproducible command and/or a `file:line`
reference.

The delta since the last audit is the remediation of AUD-2026-10-31-31..40,
the architecture baseline, the BTCUSD example move, and documentation cleanup.
Those ten fixes remain fixed. The full-system pass found no Critical defect. The
seven Major findings are concentrated in output safety, reproducibility,
evaluation evidence, and architecture documentation: output-path collisions that
can destroy a file, a strategy-run envelope that records the wrong period, two
different definitions of an independent rerun, two diagnosis checks that
overstate what they measured or require, a research export that does not run the
validation its contract promises, and an architecture baseline whose crate graph
does not match the workspace.

## Global gate evidence

| Check | Observed |
|---|---|
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo test --workspace` | 1,518 passed, 0 failed, 7 ignored; 47 suites |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | exit 0 |
| `git diff --check` | clean |

The full gate log is `/tmp/opencode/audit-gates-full-2026-10-31.log` (scratch;
not committed). The commands above are the reproducible evidence.

## Findings

| ID | Severity | Area | Finding | Evidence | Impact | Options | Recommended | Verification | Status | Links |
|---|---|---|---|---|---|---|---|---|---|---|
| AUD-2026-10-31-41 | Major | shell | Output-path collision can overwrite the CSV or the input source | `crates/quantrule-shell/src/command/fetch.rs:99-105,174-176,261`; `src/export.rs:149-177,273-277`; `command/chart.rs:109-113,155-160` | Silent loss of a fetched CSV or a source CSV; the command reports both outputs as written | A compare resolved final and temp paths; B unique temp names plus file identity | A+B | A CSV equal to the PNG's declared temp path and a `market-chart ./data.csv` test reject before writing | Fixed | [#308](https://github.com/quant-rule/quant-rule/issues/308) · [#319](https://github.com/quant-rule/quant-rule/pull/319) |
| AUD-2026-10-31-42 | Major | shell/backtest | `strategy-run --config` analyzes the run's date bounds but records only CLI dates | `command/strategy.rs:198-207,220-224,326-328`; `tests/strategy_run.rs:331-332` | The reproducibility envelope misstates the analyzed period; a replay differs | A record an `effective_period`; B ignore config bounds | A | One-day config over a multi-day source records the one-day effective period | Planned | [#309](https://github.com/quant-rule/quant-rule/issues/309) |
| AUD-2026-10-31-43 | Major | shell/analytics | `evidence.rerun` accepts a copied rerun and disagrees with `lab summary` | `src/diagnosis.rs:546-571`; `src/lab_summary.rs:552-586`; `quantrule-analytics/src/diagnosis/compute.rs:1307-1320` | A false independent-rerun pass; two commands contradict each other | A one shared rerun-state comparison; B rename and document metrics-only | A | Copied report and different-config rerun both `unknown`; matching config `pass` | Planned | [#310](https://github.com/quant-rule/quant-rule/issues/310) |
| AUD-2026-10-31-44 | Major | analytics | `model.axis_correlation` tests two of the six axis pairs | `diagnosis/compute.rs:232-233,282-283,1080-1104`; `docs/reports/2026-10-31/diagnosis-v1.md:93`; `src/summary.rs:18-19` | A Momentum/Structure correlation above 0.70 can pass a check that claims the largest pair | A compute all six pairs; B narrow the check and its evidence | A | Synthetic data with a 0.9 Momentum/Structure correlation fails | Planned | [#311](https://github.com/quant-rule/quant-rule/issues/311) |
| AUD-2026-10-31-45 | Major | analytics | Unknown checks receive failure-specific triage text | `diagnosis/compute.rs:1390-1392,1423-1439`; `1377-1383,1218-1225,1245-1252,1271-1278` | A no-trade run says net expectancy is negative; a no-fill run says slippage exceeds; a no-limit run says the limit was breached | A status-aware triage text; B omit Unknown checks | A | Unknown checks say record/measure; failure text only on `Fail` | Planned | [#311](https://github.com/quant-rule/quant-rule/issues/311) |
| AUD-2026-10-31-46 | Major | analytics | Research export does not run the contract's validation table | `quantrule-analytics/src/research/mod.rs:148,239-259`; `research/render.rs:83-88`; `docs/analytics/2026-10-24/research-output.md:591-604` | Invalid or non-finite public `LayerSet` values are written as a valid-looking dataset | A implement pre-render validation; B narrow the contract | A | Non-finite, out-of-range, duplicate-identity and row-count fixtures fail before writing | Planned | [#312](https://github.com/quant-rule/quant-rule/issues/312) |
| AUD-2026-10-31-47 | Major | architecture | Baseline and `AGENTS.md` omit the analytics→data dependency; no crate-graph guard | `AGENTS.md:48`; `docs/architecture/README.md:43`; `docs/architecture/characteristics.md:127`; `crates/quantrule-analytics/Cargo.toml:8`; `src/research/mod.rs:21`; `src/layers/mod.rs:36` | The living architecture baseline is false; an unrecorded edge can grow unnoticed | A document the dependency and add a crate-graph test; B remove the dependency | A | Test parses every `Cargo.toml` and asserts the allowed edges | Planned | [#313](https://github.com/quant-rule/quant-rule/issues/313) |
| AUD-2026-10-31-48 | Minor | backtest | Entry-bar stop uses the pre-entry open as its reference when the stop is tighter than slippage | `execution/engine.rs:242-243,727-733,832-848`; probe `/tmp/opencode/execution-probe` | The stop fill is worse than the stop price by up to the slippage; small silent P&L error | A evaluate the entry bar from the fill; B reject a stop distance at or below slippage | A | Probe: entry 100.02, stop 100.01, open 100.0, low 99.99 → reference 100.01, price 99.99 | Planned | [#314](https://github.com/quant-rule/quant-rule/issues/314) |
| AUD-2026-10-31-49 | Minor | shell/backtest | `strategy-run` returns empty success when an active-ladder timeframe is missing, while README says it fails | `quantrule-backtest/src/runner/mod.rs:280-282`; `command/strategy.rs:226-230`; `tests/strategy_run.rs:423-436`; `README.md:540` | Insufficient data can be mistaken for a valid no-trade result | A update the contract and expose `bars: 0`; B restore a typed error | A | Short-series test matches the documented empty-result contract and reports zero bars | Planned | [#309](https://github.com/quant-rule/quant-rule/issues/309) |
| AUD-2026-10-31-50 | Minor | analytics | `checks.csv` and `model-health.csv` render missing numbers as `-`, not an empty cell | `diagnosis/text.rs:457,551-553,729-730`; `docs/reports/2026-10-31/diagnosis-v1.md:54-55` | Machine consumers cannot distinguish missing from a literal `-`; CSV convention is inconsistent | A empty cell in CSV, `-` only in text; B amend the contract | A | Unknown check value and null model-health value produce empty cells | Planned | [#311](https://github.com/quant-rule/quant-rule/issues/311) |
| AUD-2026-10-31-51 | Minor | analytics | `manifest.json` records present timeframes, not the configured ladder | `research/metadata.rs:525-531`; `layers/mod.rs:229-233`; `command/layers.rs:46-50`; `docs/analytics/2026-10-24/research-output.md:514` | A configured higher timeframe with no bars disappears from the reproducibility record | A pass the configured ladder into the manifest; B add `configured_timeframes_secs` | A | Short M1 source with no H1 still lists H1 | Planned | [#312](https://github.com/quant-rule/quant-rule/issues/312) |
| AUD-2026-10-31-52 | Minor | model | A measured zero ATR at a pivot origin is treated as missing | `axes/progression/mod.rs:506-511`; `target.rs:169-179`; `leg.rs:193-203`; `tests/progression_golden.rs:551-563` | A flat-prefix series can publish the wrong target, reason, and lifecycle | A keep `Some(0.0)`, guard displacement, emit reason 3; B amend the contract | A | Flat prefix then pivot → target equals origin, reason 3, invalidation works | Planned | [#315](https://github.com/quant-rule/quant-rule/issues/315) |
| AUD-2026-10-31-53 | Minor | backtest/strategy | `context_lost` exits on an unavailable Progression context | `strategy/ladder.rs:151-153,277-285`; `context/mod.rs:166-168,739-742`; `docs/config/2026-10-30/strategy-v3.md:149-151` | A missing context closes a position with an incorrect exit reason | A return `None` for unavailable; B make unavailable propagate | A | Complete context with unavailable progression → Hold | Planned | [#314](https://github.com/quant-rule/quant-rule/issues/314) |
| AUD-2026-10-31-54 | Minor | backtest/execution | Risk-cap step flooring has no boundary tolerance | `execution/engine.rs:642-653`; `portfolio/sizing.rs:237-249`; `engine_tests.rs:603-623` | A cap at an exact decimal boundary can reject a valid entry or underfill one step | A reuse `floored_steps`; B add the same relative tolerance | A | Cap 0.7, step 0.1, price 1.0 → 0.7; price 7.0 → 0.1 | Planned | [#314](https://github.com/quant-rule/quant-rule/issues/314) |
| AUD-2026-10-31-55 | Minor | data/model/backtest | Extreme `i64` timestamps can overflow public arithmetic | `data/timeframe_set.rs:164-171`; `data/source_cadence.rs:42`; `model/aggregate/progression.rs:218`; `backtest/context/mod.rs:556,569,592`; `backtest/runner/mod.rs:176`; `data/candle.rs:119-169` | Debug panic or release wrap for public callers; provider path can write malformed dates | A validate the timestamp range at the boundary; B checked arithmetic | A+B | `i64::MIN`/`i64::MAX` tests return typed errors and do not panic | Planned | [#316](https://github.com/quant-rule/quant-rule/issues/316) |
| AUD-2026-10-31-56 | Minor | model | Non-finite public axis values are counted as neutral or present | `consensus.rs:57-69`; `aggregate/mod.rs:186-198`; `dashboard/assemble/cross_tf.rs:176-182`; `docs/model/2026-10-25/dashboard-state.md:363-364` | Public snapshots can produce NaN means and inflated share denominators | A filter `is_finite`; B validate at construction | A | NaN direction is excluded, never neutral; shares and means are finite | Planned | [#315](https://github.com/quant-rule/quant-rule/issues/315) |
| AUD-2026-10-31-57 | Minor | model | `EventAggregates` latest-event tie-break disagrees with the report's fixed key | `aggregate/events.rs:186-190`; `report/order.rs:13-23`; `report/assemble.rs:504` | One report can publish two different latest events for the same bar | A use the shared `event_cmp`; B define latest by availability/occurrence only | A | Two same-bar events produce the same latest subtype in aggregate and report | Planned | [#315](https://github.com/quant-rule/quant-rule/issues/315) |
| AUD-2026-10-31-58 | Minor | store | A newer-schema database is switched to WAL before the version check | `store/mod.rs:57-63,219-221,234-241`; `docs/adr/2026-10-24/sqlite-store.md:155-161`; `store/tests.rs:107-124` | `store init`/`ingest` mutates a newer database before rejecting it | A read the version before WAL; B check through a read-only connection | A | Newer-version file is byte-unchanged and has no `-wal` after rejection | Planned | [#316](https://github.com/quant-rule/quant-rule/issues/316) |
| AUD-2026-10-31-59 | Minor | shell/store | Store and fetch accept `4h` although the documented contract lists four timeframes | `data/timeframe.rs:110-111`; `command/store_query.rs:238-245`; `command/fetch.rs:53-57`; `source/dukascopy.rs:83-95`; `README.md:280`; `command/help.rs:151-154` | `store show` prints an empty 4h table; `fetch` fails with a source error instead of a usage error | A whitelist the four documented codes; B support H4 and update docs | A | Both commands reject `4h` as a usage error | Planned | [#316](https://github.com/quant-rule/quant-rule/issues/316) |
| AUD-2026-10-31-60 | Minor | shell/lab | Lab identity handling defaults a missing halt to `false` and checks different provenance fields | `command/lab.rs:648-651,364-410`; `lab_summary.rs:173-221`; `command/help.rs:519-520` | A malformed lab is listed as not halted; a stale provenance passes one command | A `Option<bool>` and one full provenance check; B narrow the help | A | Missing halted → `—`/null; provenance model mismatch fails both commands | Planned | [#310](https://github.com/quant-rule/quant-rule/issues/310) |
| AUD-2026-10-31-61 | Minor | shell/help | Help metadata is wrong for `lab diagnose --out` and `fetch --png` | `command/help.rs:255,139-142`; `command/lab.rs:580-583`; `command/help.rs:267,211-214`; `command/fetch.rs:66-72,150-154`; `tests/fixtures/help-all.txt:321,529` | Users expect the wrong output root or believe a PNG is required | A dedicated option descriptions and fixture update; B change behavior | A | `help all` names the default `<lab>/diagnosis` and calls PNG optional | Planned | [#317](https://github.com/quant-rule/quant-rule/issues/317) |
| AUD-2026-10-31-62 | Minor | docs | User-facing docs drift from code in eight places | See the detail block | Users and consumers are misled; four release links are broken | A update docs to code and add a link check; B change code where the doc is authoritative | A | Link check finds zero broken links; each item matches code | Planned | [#317](https://github.com/quant-rule/quant-rule/issues/317) |
| AUD-2026-10-31-63 | Minor | tests | Test and guard coverage gaps hide the defects above | See the detail block | Regressions in multi-context weights, costs, diagnosis checks, lab evidence, and architecture guards pass CI | A table-driven tests and guard expansion; B document the gaps | A | Each named test fails before the fix and passes after it | Planned | [#318](https://github.com/quant-rule/quant-rule/issues/318) |
| AUD-2026-10-31-64 | Minor | portfolio | `max_drawdown` includes fill/funding equity, but halt checks only marks | `portfolio/ledger.rs:210,249,349,425,437-447,456-475`; `portfolio/records.rs:272` | A report can show a drawdown above the limit with `halted=false` and no risk event | A check the halt after open/close/funding; B document mark-only and stop updating the metric at fills | A or B | A fill/funding breach latches the halt and records a risk event, or the contract says mark-only | Planned | [#314](https://github.com/quant-rule/quant-rule/issues/314) |
| AUD-2026-10-31-65 | Minor | tests | `runner_metrics_golden.csv` contains a stray test-runner line and the parser ignores it | `tests/fixtures/runner_metrics_golden.csv:20`; `src/runner/tests.rs:188-208` | A corrupted or extended fixture can pass; the golden claim is weakened | A remove the line and parse strictly; B parse to a map and assert names/count | A | Fixture contains exactly the expected metric rows; an unknown row fails | Planned | [#318](https://github.com/quant-rule/quant-rule/issues/318) |
| AUD-2026-10-31-66 | Info | backtest | `PositionState::Closed` and `MissingContextTimeframe` are unreachable public API | `execution/position.rs:159-160,176-180`; `execution/engine.rs:594,819`; `context/mod.rs:385-389,488-489,512-516`; `docs/config/2026-10-30/trade-intent.md:51-53` | Dead variants; the contract says a just-closed state is tracked | A implement the transition; B remove the variants and correct the docs | B | No `Closed`/`MissingContextTimeframe` remains, or a test reaches them | Planned | [#314](https://github.com/quant-rule/quant-rule/issues/314) |
| AUD-2026-10-31-67 | Info | store | Store identity and addressing edge cases; `store verify` is not an independent oracle | `store/schema.rs:18-28,134-147`; `store/lifecycle.rs:41-43`; `store/verify.rs:49-54`; `command/store.rs:122-137,88-91`; `command/store_query.rs:87-90`; `store/mod.rs:242-243`; `store/ingest/validate.rs:1-6`; `docs/adr/2026-10-24/sqlite-store.md:199` | Internal ids vary across histories; `--`-prefixed datasets are unaddressable; any version-0 SQLite file is migrated; verify catches drift, not an independent bug | A omit internal ids, validate names, set `application_id`, document verify; B contract changes | A | Same logical dump is stable; `--` name rejected; unrelated DB refused; verify scope documented | Planned | [#316](https://github.com/quant-rule/quant-rule/issues/316) |
| AUD-2026-10-31-68 | Info | analytics | `backtest --diagnose` synthesizes `report_versions` from constants | `src/diagnosis.rs:209-231`; `crates/quantrule-analytics/src/diagnosis/compute.rs:928-961`; `docs/reports/2026-10-31/diagnosis-v1.md:69` | The check records the binary's constants, not the five bundles, on the `backtest --diagnose` path | A read the five bundle versions; B document the constants and rely on `lab diagnose` | A | Removing a bundle version changes `backtest --diagnose` to `fail` | Planned | [#311](https://github.com/quant-rule/quant-rule/issues/311) |
| AUD-2026-10-31-69 | Info | docs | Stale code and module documentation | `shell/render.rs:38-41`; `model/timeframe.rs:348-350`; `docs/model/2026-10-27/market-report.md:86,255`; `model/report/assemble.rs:50,417`; `model/dashboard/assemble/progression.rs:290`; `shell/source/jsonp.rs:96-99`; `docs/adr/2026-10-24/sqlite-store.md:187` | Readers see four series, a full snapshot, a two-argument constructor, a report reason 5, six JSONP fields, and the wrong default path where the code differs | A correct the docs; B make the code match | A | Doc greps match the current code; `cargo doc` passes | Planned | [#317](https://github.com/quant-rule/quant-rule/issues/317) |

## Detail

### AUD-2026-10-31-41 — Output-path collision can destroy a file

- Expected invariant: two outputs that resolve to the same file, or one
  output equal to another output's temporary path, are rejected before any
  write.
- Observed: `fetch` compares only the raw `PathBuf` values
  (`crates/quantrule-shell/src/command/fetch.rs:99-105`, repeated at
  `src/fetch.rs:174-176`). Every atomic write uses `<target>.tmp`
  (`src/export.rs:273-277`). `fetch` commits the CSV before it renders the PNG
  (`src/fetch.rs:223` then `:261`). `market-chart` compares only `png == path`
  (`command/chart.rs:109-113`) and then writes over the target
  (`command/chart.rs:155-160`).
- Root cause: path identity is textual and the temporary naming scheme makes
  one output's final path equal another output's temp path.
- Reproduction: `fetch ... --csv out.png.tmp --png out.png` commits the CSV to
  `out.png.tmp`; the PNG temp is the same path, `File::create` truncates the
  CSV, and the PNG is renamed to `out.png`. The reported CSV path no longer
  exists. `market-chart data.csv --png ./data.csv` passes the textual check and
  overwrites the source after reading it. Tests cover only identical strings
  (`src/fetch.rs:592-604`; `command/tests.rs:490-496`).
- Impact: silent data loss; the command reports both paths as outputs.
- Affected contracts/docs: `fetch`, `market-chart`, and `export` command
  contracts in `README.md` and `help all`.
- Dependencies: none.
- Effort: small.
- Options:
  - A. Resolve each final path (parent canonicalization plus file name, and
    file identity when it exists) and reject any final/temp intersection.
  - B. Use unique per-write temporary names in a dedicated temp directory,
    then check the final/temp intersections before committing.
- Recommendation: A, with B's unique temp names as defense in depth.
- Verification: a shell test with a CSV equal to the PNG's declared temporary
  path (`<target>.tmp.<pid>`) and a `market-chart ./data.csv` test both reject
  before writing; no file changes.
- Closure evidence: `export::outputs_conflict`, process-scoped temporary names,
  and the regression tests are in PR #319.
- Status: `Fixed`.
- Links: [#308](https://github.com/quant-rule/quant-rule/issues/308) ·
  [#319](https://github.com/quant-rule/quant-rule/pull/319).

### AUD-2026-10-31-42 — `strategy-run --config` records the wrong period

- Expected invariant: the envelope records the period that was analyzed, so a
  later run can reproduce the result.
- Observed: `run_strategy_run` reads `run.data.bounds` from `--config`
  (`crates/quantrule-shell/src/command/strategy.rs:198-207`) and uses it when
  both CLI dates are absent (`:220-224`). The envelope writes only
  `args.from`/`args.to` as `requested_period` (`:326-328`). The test asserts
  both are null for the `--config` case (`tests/strategy_run.rs:331-332`). The
  fixture config has a one-day `data` range
  (`tests/fixtures/config-run/data.json`).
- Root cause: the effective bounds are computed after the envelope's inputs are
  fixed; there is no `effective_period` field.
- Impact: a recorded experiment can claim to cover the full file while the
  command analyzed one day. A replay from the envelope differs.
- Affected contracts/docs: `README.md` strategy-run envelope, `help all`.
- Dependencies: none.
- Effort: small.
- Options:
  - A. Add `effective_period` (and the resolved bounds) to the envelope, and
    document that `--config` supplies a date range.
  - B. Apply only CLI dates and document `--config` as strategy/model-only.
- Recommendation: A.
- Verification: one-day config over a multi-day source records the one-day
  effective period and its records match that range.
- Status: `Planned`.
- Links: [#309](https://github.com/quant-rule/quant-rule/issues/309).

### AUD-2026-10-31-43 — Two definitions of an independent rerun

- Expected invariant: `lab summary` and `lab diagnose` agree on whether the
  same `rerun/` folder is independent evidence.
- Observed: `diagnosis::rerun_matches` reads only the rerun and main portfolio
  reports and compares five summary fields
  (`crates/quantrule-shell/src/diagnosis.rs:546-571`). `lab_summary::rerun`
  additionally requires the two provenance `config_hash` values to match
  (`crates/quantrule-shell/src/lab_summary.rs:552-586`). The analytics check
  maps the boolean to `pass` or `unknown`
  (`crates/quantrule-analytics/src/diagnosis/compute.rs:1307-1320`).
- Root cause: the rerun comparison was not moved into the shared
  `lab_evidence` module when OOS evidence was centralized.
- Impact: a copied report with no provenance passes the diagnosis as an
  independent rerun and fails the summary; a different-config rerun with
  coincidentally equal metrics can pass the diagnosis.
- Affected contracts/docs: `docs/reports/2026-10-31/diagnosis-v1.md:103`;
  `lab summary` / `lab diagnose` help.
- Dependencies: none.
- Effort: small.
- Options:
  - A. Add a shared `rerun` state machine (absent, unreadable, differs,
    matches) requiring matching config hashes, and use it from both commands.
  - B. Rename the diagnosis check to metrics-only and document the difference.
- Recommendation: A.
- Verification: copied report and different-config rerun both `unknown`;
  matching config and metrics `pass`; both commands print the same state.
- Status: `Planned`.
- Links: [#310](https://github.com/quant-rule/quant-rule/issues/310).

### AUD-2026-10-31-44 — Axis correlation checks two of six pairs

- Expected invariant: the `model.axis_correlation` check measures the largest
  absolute correlation among the four axes, as its name and contract say.
- Observed: `timeframe_health` creates only `dm` (Direction/Momentum) and `ds`
  (Direction/Structure) accumulators
  (`crates/quantrule-analytics/src/diagnosis/compute.rs:232-233`, populated at
  `:282-283`). The check takes the maximum of those two
  (`:1080-1104`). The contract says "the largest absolute inter-axis
  correlation" (`docs/reports/2026-10-31/diagnosis-v1.md:93`), and
  `src/summary.rs:18-19` already defines all six pairs.
- Root cause: the diagnosis reused only the two correlations the report
  already published; the other four pairs were never accumulated.
- Impact: a Momentum/Structure correlation above 0.70 passes a check that
  claims to have tested the largest pair, hiding correlated axes.
- Affected contracts/docs: `diagnosis-v1.md:93,126-129`; `checks.csv`.
- Dependencies: none.
- Effort: small.
- Options:
  - A. Compute all six pairwise correlations and take the maximum; keep the
    existing two fields for compatibility or replace them with a pair table.
  - B. Narrow the check's name, evidence, and contract to the two pairs it
    actually measures.
- Recommendation: A.
- Verification: a synthetic model with a 0.9 Momentum/Structure correlation
  fails with that pair named in the evidence.
- Status: `Planned`.
- Links: [#311](https://github.com/quant-rule/quant-rule/issues/311).

### AUD-2026-10-31-45 — Unknown checks are triaged as failures

- Expected invariant: triage text matches the check status; `Unknown` asks for
  a measurement, and failure wording appears only on `Fail`.
- Observed: `triage` skips only `Pass` (`crates/quantrule-analytics/src/
  diagnosis/compute.rs:1390-1392`). It then emits fixed failure text:
  `net expectancy is negative` (`:1424`), `measured slippage exceeds`
  (`:1433`), `costs dominate` (`:1437`), and `the limit was breached`
  (`:1439`). The corresponding checks return `Unknown` for no trades, no
  fills, or no configured limit (`:1377-1383`, `:1218-1225`, `:1245-1252`,
  `:1271-1278`).
- Root cause: triage switches on the check id, not on the check status.
- Impact: a no-trade run prints "net expectancy is negative" beside an
  `UNKNOWN` status; a run with no limit prints "the limit was breached". The
  human sees false evidence.
- Affected contracts/docs: `diagnosis-v1.md:196-198`; `triage.json`.
- Dependencies: none.
- Effort: small.
- Options:
  - A. Select text by `CheckStatus` and value: `Unknown` says measure/record,
    `Fail` uses the failure wording.
  - B. Omit `Unknown` checks from triage, or prefix every unknown action.
- Recommendation: A.
- Verification: a no-trade input has no failure claim; a failing input keeps
  the failure text; a passing input has no action.
- Status: `Planned`.
- Links: [#311](https://github.com/quant-rule/quant-rule/issues/311).

### AUD-2026-10-31-46 — Research export skips its promised validation

- Expected invariant: the export validates the dataset before publishing it,
  as the contract's table requires (`docs/analytics/2026-10-24/
  research-output.md:591-604`), and a failure writes no file.
- Observed: `render_dataset_with_ladder` calls only `validate(meta)`
  (`crates/quantrule-analytics/src/research/mod.rs:148`), which checks the
  dataset name, data version, and root (`:239-259`). The renderer writes
  `value.to_string()` without a finite or range check
  (`research/render.rs:83-88`). `layers/mod.rs:16-18` says the export asserts
  the widths against the header, but no runtime assertion exists. The
  verification test checks ranges; a test is not a runtime guard.
- Root cause: the validation table describes the test suite, not the render
  function.
- Impact: a public `LayerSet` with an out-of-range or non-finite value is
  written as a valid-looking dataset. The CLI path relies on upstream
  constructors, so this is a contract-enforcement defect rather than an
  observed production corruption.
- Affected contracts/docs: `research-output.md:586,591-604`;
  `dataset-report.md:10`.
- Dependencies: none.
- Effort: medium.
- Options:
  - A. Add a pre-render validation pass over `LayerSet` and raw candles:
    finite/range/reason consistency, row identity/counts, configured-ladder
    blocks, and the 100k cap; fail before returning files.
  - B. Narrow the contract to say the properties are guaranteed by
    construction and covered by tests.
- Recommendation: A.
- Verification: invalid fixtures for each table row fail with no files;
  the existing valid export remains byte-identical.
- Status: `Planned`.
- Links: [#312](https://github.com/quant-rule/quant-rule/issues/312).

### AUD-2026-10-31-47 — Architecture baseline omits the analytics→data edge

- Expected invariant: the living architecture baseline and `AGENTS.md` match
  the workspace's crate graph, and a test guards the allowed edges.
- Observed: `AGENTS.md:48` says `quantrule-analytics` depends on
  `quantrule-model`; `docs/architecture/README.md:43` and
  `docs/architecture/characteristics.md:127` list only `quantrule-model`.
  `crates/quantrule-analytics/Cargo.toml:8` declares `quantrule-data`, and the
  code uses it in research, layers, chart, and diagnosis
  (`src/research/mod.rs:21`, `src/layers/mod.rs:36`,
  `src/diagnosis/text.rs:10`). `characteristics.md:72-73` claims source-level
  guards; no test reads `Cargo.toml`.
- Root cause: the analytics crate gained a direct `quantrule-data` dependency
  for candle and timestamp types, and the baseline was written from the
  intended graph rather than the manifest.
- Impact: audits and design decisions use a false graph; the unrecorded edge
  can grow without a guard. The dependency is acyclic and upstream, so this
  is a documentation/guard defect, not a cycle.
- Affected contracts/docs: `AGENTS.md` section 2; architecture README,
  characteristics, ADR `2026-10-31/diagnosis-artifact.md`.
- Dependencies: none.
- Effort: small.
- Options:
  - A. Document `quantrule-data` as an allowed analytics dependency and add a
    test that parses each `Cargo.toml` and asserts the allowed edges.
  - B. Remove the direct dependency by re-exporting the needed types through
    `quantrule-model`.
- Recommendation: A.
- Verification: the crate-graph test passes; a forbidden edge fails; the
  docs name the current graph.
- Status: `Planned`.
- Links: [#313](https://github.com/quant-rule/quant-rule/issues/313).

### AUD-2026-10-31-48 — Entry-bar stop uses a pre-entry price

- Expected invariant: a fill cannot be decided by a price before the entry;
  the open is a stop reference only when the open gaps through the stop after
  the entry exists.
- Observed: `advance` fills the pending entry (`execution/engine.rs:242-243`)
  and immediately applies the bracket (`:708-733`). The long stop branch uses
  `bar.open() <= position.stop_price` and fills at the open. The entry fill is
  `open + slippage` (`:832-848`). When the stop distance is smaller than the
  slippage, the resolved stop is above the bar open, so the pre-entry open is
  treated as a gap through the stop.
- Reproduction: probe `/tmp/opencode/execution-probe` with fee 0, slippage
  2 bps, ATR 0.01, stop multiple 1.0, target multiple 3.0. Entry fill 100.02,
  stop 100.01, bar open 100.0, low 99.99. The engine prints a stop fill with
  reference 100.0 and price 99.98; the stop price is 100.01, so the correct
  reference is 100.01 and the price 99.99.
- Root cause: the entry bar is evaluated with the bar open as if the position
  existed at the open.
- Impact: the stop fill is worse than the stop price by up to the slippage.
  The stop is always touched when the open is beyond it (the low of a valid
  bar is at or below the open), so the exit reason is correct; the price is
  not. The error is small but silent and appears in P&L and trade records.
- Affected contracts/docs: `execution/mod.rs:20-27,72-75`;
  `docs/config/2026-10-30/execution-v3.md`.
- Dependencies: none.
- Effort: small.
- Options:
  - A. On the entry bar, use the fill as the open reference and evaluate the
    stop/target from the fill and the bar's high/low; use the open gap rule
    only on later bars.
  - B. Reject an entry whose stop distance is at or below the entry slippage,
    with a typed `InvalidBracket`/`RiskLimit`, and document the restriction.
- Recommendation: A.
- Verification: the probe case yields reference 100.01 and price 99.99; the
  existing entry-bar stop test still passes.
- Status: `Planned`.
- Links: [#314](https://github.com/quant-rule/quant-rule/issues/314).

### AUD-2026-10-31-49 — `strategy-run` empty success contradicts README

- Expected invariant: the command's documented failure behavior matches what
  it prints.
- Observed: `walk_strategy` returns `StrategyWalk::default()` when the
  execution timeframe is absent
  (`crates/quantrule-backtest/src/runner/mod.rs:280-282`). The shell checks
  only that M1 is non-empty (`command/strategy.rs:226-230`) and prints nothing
  for an empty walk. The test asserts exit 0 with empty stdout/stderr
  (`tests/strategy_run.rs:423-436`). `README.md:540` says the command "fails
  with a message rather than printing an empty result when ... an
  active-ladder timeframe cannot be built." The W6b2 validation record
  intentionally changed the old error to empty success.
- Root cause: the behavior changed and the README was not updated; the JSON
  envelope has no bar count to make the empty result explicit.
- Impact: a user can mistake insufficient data for a valid no-trade run.
- Affected contracts/docs: `README.md:540`; `help all`;
  `docs/reports/2026-10-31/w6b2-zero-bar-context-validation.md`.
- Dependencies: none.
- Effort: small.
- Options:
  - A. Keep the validated empty-success behavior, update the README, and add
    `bars` (and `actual_period`) to the envelope so zero bars are explicit.
  - B. Restore a typed error when an active-ladder timeframe is absent.
- Recommendation: A, because the behavior was intentionally validated; the
  contract must say so.
- Verification: the short-series test asserts the documented empty result and
  the envelope reports `bars: 0`.
- Status: `Planned`.
- Links: [#309](https://github.com/quant-rule/quant-rule/issues/309).

### AUD-2026-10-31-50 — Diagnosis CSV nulls are `-`

- Expected invariant: a missing numeric value in a CSV table is an empty cell;
  `diagnosis.json` uses `null`; `-` is for text summaries
  (`docs/reports/2026-10-31/diagnosis-v1.md:54-55`).
- Observed: `checks_csv` writes `opt_number(check.value)`
  (`crates/quantrule-analytics/src/diagnosis/text.rs:457`);
  `model_health_csv` writes `opt_number` for `min`, `max`, and `mean`
  (`:551-553`); `opt_number` returns `"-"` for `None` (`:729-730`).
- Root cause: the CSV writers reuse the text formatter.
- Impact: machine consumers cannot distinguish a missing value from a literal
  `-`; the artifact set disagrees with the contract and with
  `strategy-exit-reasons.csv`, which was fixed in the prior audit.
- Affected contracts/docs: `diagnosis-v1.md:54-55`; `checks.csv`,
  `model-health.csv`.
- Dependencies: none.
- Effort: small.
- Options:
  - A. Use an empty string for `None` in CSV writers; keep `-` in
    `summary.txt`.
  - B. Amend the contract to make `-` the universal missing marker.
- Recommendation: A.
- Verification: an unknown check and a null model-health value produce empty
  cells; `summary.txt` still prints `-`.
- Status: `Planned`.
- Links: [#311](https://github.com/quant-rule/quant-rule/issues/311).

### AUD-2026-10-31-51 — Manifest records present, not configured, timeframes

- Expected invariant: `manifest.json` `timeframes_secs` is the configured
  ladder, fastest first (`docs/analytics/2026-10-24/research-output.md:514`).
- Observed: `manifest_json` writes `layers.timeframes`
  (`crates/quantrule-analytics/src/research/metadata.rs:525-531`), and
  `build_layers` filters out inputs with no candles (`layers/mod.rs:229-233`).
  The shell computes the configured `higher` ladder separately and passes it
  to the renderer (`command/layers.rs:46-50`, `command/research.rs:181`).
- Root cause: the manifest uses the present-row set; the configured ladder is
  not passed to `manifest_json`.
- Impact: a short M1 source with no H1 bar exports a null H1 block and lists
  H1 in the README, but the manifest says the ladder is `[60,300,900]`.
- Affected contracts/docs: `research-output.md:514`; `manifest.json`.
- Dependencies: none.
- Effort: small.
- Options:
  - A. Pass the configured ladder into `manifest_json` and record it.
  - B. Add `configured_timeframes_secs` and define the current field as
    present timeframes.
- Recommendation: A.
- Verification: a short M1 source with no H1 still records H1 in the manifest.
- Status: `Planned`.
- Links: [#312](https://github.com/quant-rule/quant-rule/issues/312).

### AUD-2026-10-31-52 — A measured zero ATR is treated as missing

- Expected invariant: a valid zero is a value; a zero ATR denominator yields
  reason 3, and the fallback target is `origin + direction * 2 * ATR_at_origin`
  (`docs/model/2026-10-21/four-axis-analytical-model.md:63,230,243,255`).
- Observed: `usable_atr` filters to `is_finite() && *value > 0.0`
  (`crates/quantrule-model/src/axes/progression/mod.rs:506-511`). A zero ATR at
  the pivot origin therefore becomes `None` (`:274`); `target.rs:169-179`
  drops the fallback target; `leg.rs:193-203` disables the 0.2×ATR
  invalidation. The contract says a zero denominator produces reason 3, not a
  missing ATR. The test `a_zero_atr_leaves_the_leg_without_a_fallback_target`
  (`tests/progression_golden.rs:551-563`) uses a fully flat series that
  produces no candidates or legs, so it does not exercise the path.
- Root cause: "usable distance" was defined as strictly positive, conflating a
  zero measurement with a missing one.
- Impact: a valid flat-prefix series can publish the wrong target, wrong
  progress reason (4 instead of 3), and the wrong lifecycle.
- Affected contracts/docs: `four-axis-analytical-model.md:63,230,243,255`.
- Dependencies: none.
- Effort: small.
- Options:
  - A. Keep `Some(0.0)` as a valid ATR for target/invalidation, guard
    `displacement_atr` so it is null at zero, and let the degenerate target
    produce reason 3; add a flat-prefix test.
  - B. Amend the contract to declare zero ATR unusable and reason 4, and
    update the oracle and golden fixtures.
- Recommendation: A.
- Verification: a flat prefix followed by a confirmed pivot yields a
  degenerate target, reason 3, and an invalidatable leg.
- Status: `Planned`.
- Links: [#315](https://github.com/quant-rule/quant-rule/issues/315).

### AUD-2026-10-31-53 — `context_lost` exits on unavailable progression

- Expected invariant: a missing or unavailable context does not trigger
  `context_lost`; the position holds
  (`docs/config/2026-10-30/strategy-v3.md:149-151`).
- Observed: `context_lost` is `context_side(...) == Some(false)`
  (`crates/quantrule-backtest/src/strategy/ladder.rs:151-153`). `context_side`
  returns `Some(false)` when the Progression agreement is not Bullish/Bearish
  (`:277-285`). `context_complete` only checks that every context timeframe
  had a closed bar (`context/mod.rs:166-168`); `AxisAccumulator::finish`
  returns `AxisContext::unavailable()` when no valid progression value exists
  (`:739-742`). The unavailable agreement therefore reaches `Some(false)`.
- Root cause: the availability of the progression axis is not distinguished
  from a measured disagreement.
- Impact: a position can be force-closed on a missing measurement with an
  incorrect `context_lost` reason.
- Affected contracts/docs: `strategy-v3.md:149-151`.
- Dependencies: none.
- Effort: small.
- Options:
  - A. Return `None` when the progression direction is unavailable; only a
    measured non-agreement returns `Some(false)`.
  - B. Make an unavailable axis propagate `None` through the whole context
    condition and document the hold.
- Recommendation: A.
- Verification: a complete context with an unavailable progression direction
  returns `Decision::Hold`; a measured Mixed direction still exits.
- Status: `Planned`.
- Links: [#314](https://github.com/quant-rule/quant-rule/issues/314).

### AUD-2026-10-31-54 — Risk-cap step flooring has no boundary tolerance

- Expected invariant: the cap is the largest step multiple that fits the
  entry envelope; representation error does not reject a valid entry or
  underfill one step.
- Observed: the cap path computes `let steps = (capped / step).floor()` and
  rejects below one step (`crates/quantrule-backtest/src/execution/engine.rs:
  642-653`). Portfolio sizing already has a tolerant `floored_steps` with
  `16 × EPS` (`portfolio/sizing.rs:237-249`), but it is private and not used
  here. The test uses `50.0 / 100.02`, not an exact boundary
  (`engine_tests.rs:603-623`).
- Root cause: the cap path reimplements the step floor without the tolerance.
- Impact: a cap at an exact decimal boundary can reject a valid entry or fill
  one step too small; results look configuration-dependent.
- Affected contracts/docs: `docs/config/2026-10-30/policy-and-strategy.md`
  risk row; `execution-v3.md`.
- Dependencies: none.
- Effort: small.
- Options:
  - A. Make `floored_steps` `pub(crate)` and reuse it in the cap path.
  - B. Add the same relative tolerance and assert the largest step multiple.
- Recommendation: A.
- Verification: cap 0.7, step 0.1, price 1.0 → 0.7; price 7.0 → 0.1; both
  accepted.
- Status: `Planned`.
- Links: [#314](https://github.com/quant-rule/quant-rule/issues/314).

### AUD-2026-10-31-55 — Extreme timestamps overflow public arithmetic

- Expected invariant: library code does not panic on invalid input; timestamps
  are bounded to the documented UTC range.
- Observed: `Candle::new` accepts any `i64` timestamp
  (`crates/quantrule-data/src/candle.rs:119-169`); only the CSV parser bounds
  years (`data/time.rs:47-51`). Unchecked arithmetic appears in
  `TimeframeSet` (`data/timeframe_set.rs:164-171`), `SourceCadence`
  (`data/source_cadence.rs:42`), the model aggregate
  (`model/aggregate/progression.rs:218`), the backtest context
  (`backtest/context/mod.rs:556,569,592`), and the runner
  (`backtest/runner/mod.rs:176`). The provider JSONP parser rejects only
  negative milliseconds (`shell/source/jsonp.rs:124-133`), and the CSV
  formatter slices a four-digit year at fixed offsets (`shell/export.rs:
  185-188`).
- Root cause: the timestamp contract is enforced at the text boundary, not at
  the public value boundary; arithmetic assumes the range is valid.
- Impact: a public caller can panic in debug or wrap in release; a provider
  row with a huge millisecond value can produce a malformed date.
- Affected contracts/docs: `docs/config/2026-10-30/data-v3.md`;
  `data/time.rs`.
- Dependencies: none.
- Effort: medium.
- Options:
  - A. Validate the supported timestamp range in `Candle::new`,
    `TimeframeSet::push_values`, `SourceCadence::observe`, and the fetch sink;
    return typed errors.
  - B. Use checked/saturating arithmetic in every bucket, gap, context, and
    runner addition, and make the formatter validate the year.
- Recommendation: A, with B as defense in depth.
- Verification: `i64::MIN`/`i64::MAX` tests return typed errors and do not
  panic; a provider row with an out-of-range millisecond value is rejected.
- Status: `Planned`.
- Links: [#316](https://github.com/quant-rule/quant-rule/issues/316).

### AUD-2026-10-31-56 — Non-finite values are counted as neutral or present

- Expected invariant: a non-finite axis position is excluded, never neutral
  (`docs/model/2026-10-25/dashboard-state.md:363-364`); share denominators are
  valid values (`four-axis-analytical-model.md:376`).
- Observed: `TimeframeConsensus::from_bars` iterates
  `values.into_iter().flatten()`; `Some(NaN)` fails both comparisons and hits
  `neutral += 1` (`crates/quantrule-model/src/consensus.rs:57-69`). The
  cross-timeframe aggregate includes `Some(NaN)` in its values and
  denominators (`aggregate/mod.rs:186-198`). The dashboard excludes non-finite
  positions (`dashboard/assemble/cross_tf.rs:176-182`).
- Root cause: the public consensus/aggregate paths do not apply the dashboard's
  finite filter.
- Impact: a malformed public snapshot produces NaN means and inflated shares;
  normal model output is unaffected.
- Affected contracts/docs: `dashboard-state.md:363-364`;
  `four-axis-analytical-model.md:376`.
- Dependencies: none.
- Effort: small.
- Options:
  - A. Filter `is_finite()` before counting, means, shares, and weakest
    percentile selection; add non-finite tests.
  - B. Validate public bars/snapshots at construction and return `Result`.
- Recommendation: A.
- Verification: NaN direction → not counted neutral; all means/shares finite.
- Status: `Planned`.
- Links: [#315](https://github.com/quant-rule/quant-rule/issues/315).

### AUD-2026-10-31-57 — Latest-event tie-break differs from the report

- Expected invariant: one fixed event key defines "latest"; reordered event
  lists do not change the output.
- Observed: `EventAggregates` uses
  `max_by_key((available_bar_index, occurred_bar_index))`
  (`crates/quantrule-model/src/aggregate/events.rs:186-190`), which returns the
  last maximum on ties. The report uses the full fixed key in `event_cmp`
  (type, subtype, source, reference, ...) (`report/order.rs:13-23`,
  `report/assemble.rs:504`). Two Level events on the same bar can therefore
  publish different latest subtypes in the aggregate and the report.
- Root cause: the aggregate reimplements the latest comparison with a shorter
  key.
- Impact: one report can contain conflicting latest events; the aggregate can
  depend on snapshot event order on ties.
- Affected contracts/docs: `market-report.md` event sections; `event_cmp`.
- Dependencies: none.
- Effort: small.
- Options:
  - A. Use the shared `event_cmp` for `latest_event` and
    `latest_divergence_direction`; add a same-bar tie test.
  - B. Define the aggregate latest as availability/occurrence only and remove
    type/subtype from the carried aggregate.
- Recommendation: A.
- Verification: two same-bar events yield the same latest type/subtype in the
  aggregate and the report.
- Status: `Planned`.
- Links: [#315](https://github.com/quant-rule/quant-rule/issues/315).

### AUD-2026-10-31-58 — Newer-schema database is mutated before rejection

- Expected invariant: a database whose `user_version` is newer than the
  binary's is rejected without being opened read-write or modified
  (`docs/adr/2026-10-24/sqlite-store.md:155-161`).
- Observed: `Store::open` opens read-write, configures, calls `enable_wal`
  (`crates/quantrule-shell/src/store/mod.rs:57-63`, `:219-221`), then `init`
  checks the version (`:234-241`). The test asserts the error only, not that
  the file is unchanged (`store/tests.rs:107-124`).
- Root cause: the version check is after the connection setup that changes the
  journal mode.
- Impact: `store init` or `store ingest` against a newer database changes its
  journal mode before returning `UnsupportedVersion`.
- Affected contracts/docs: `sqlite-store.md:155-161`.
- Dependencies: none.
- Effort: small.
- Options:
  - A. Read `user_version` before `enable_wal`; reject newer versions before
    any persistent PRAGMA.
  - B. Check through a read-only connection, then reopen read-write only after
    the version is supported.
- Recommendation: A.
- Verification: a newer-version fixture is byte-unchanged and has no `-wal`
  after rejection.
- Status: `Planned`.
- Links: [#316](https://github.com/quant-rule/quant-rule/issues/316).

### AUD-2026-10-31-59 — `4h` is accepted although the contract lists four timeframes

- Expected invariant: the store and fetch parsers accept only the documented
  `1m`, `5m`, `15m`, `1h` and reject anything else as a usage error.
- Observed: `Timeframe::from_code` accepts `4h`/`h4`
  (`crates/quantrule-data/src/timeframe.rs:110-111`). `store show`/`db` use it
  (`command/store_query.rs:238-245`) while the error text and help list four
  (`command/help.rs:151-154`, `README.md:280`). `fetch` accepts it
  (`command/fetch.rs:53-57`), then the provider rejects H4
  (`source/dukascopy.rs:83-95`).
- Root cause: the shared five-value parser is used where the command contract
  is four values.
- Impact: `store show ... --timeframe 4h` exits 0 with an empty table;
  `fetch ... --timeframe 4h` fails with a source error, not a usage error.
- Affected contracts/docs: README store/fetch sections; `help all`.
- Dependencies: none.
- Effort: small.
- Options:
  - A. Whitelist the four documented codes in the store and fetch parsers.
  - B. Add H4 store/fetch support and update the docs and the ingest ladder.
- Recommendation: A.
- Verification: both commands reject `4h` as a usage error.
- Status: `Planned`.
- Links: [#316](https://github.com/quant-rule/quant-rule/issues/316).

### AUD-2026-10-31-60 — Lab identity defaults and checks are inconsistent

- Expected invariant: a missing measurement is null, never `false`; every
  provenance identity field named in the help is validated.
- Observed: `lab list` reads `summary.halted` with
  `.and_then(Value::as_bool).unwrap_or(false)`
  (`crates/quantrule-shell/src/command/lab.rs:648-651`). `lab diagnose` reads
  the report `data_version`/`model_version` but only the provenance
  `config_hash` (`:364-410`). `lab summary` checks the provenance
  `data_version` and `config_hash` but not the provenance `model_version`
  (`lab_summary.rs:173-221`). The help says the data version, config hash, and
  model version must match every bundle and the supplied config
  (`command/help.rs:519-520`).
- Root cause: each command validates the fields it happens to read; the
  provenance object is not validated as one identity.
- Impact: a malformed lab is listed as not halted; a provenance whose
  data/model version disagrees with its report passes one command.
- Affected contracts/docs: `lab list`, `lab summary`, `lab diagnose` help.
- Dependencies: none.
- Effort: small.
- Options:
  - A. Make `halted` an `Option<bool>` and render `—`/null; validate every
    provenance identity field against the report and the supplied config in
    both commands.
  - B. Narrow the help to the fields actually checked and document the
    halted default.
- Recommendation: A.
- Verification: a lab without `halted` renders unavailable; a provenance
  model mismatch fails both commands; a valid lab still lists.
- Status: `Planned`.
- Links: [#310](https://github.com/quant-rule/quant-rule/issues/310).

### AUD-2026-10-31-61 — Help metadata is wrong for two options

- Expected invariant: help names each argument and its behavior.
- Observed: `LAB_DIAGNOSE_OPTIONS` uses `BUNDLE_OUT`, whose description is
  "Output root override (default: the run config)"
  (`crates/quantrule-shell/src/command/help.rs:139-142,255`); `lab diagnose`
  defaults to `<lab-dir>/diagnosis`
  (`command/lab.rs:580-583`). `FETCH_OPTIONS` uses `PNG`, "Required PNG
  target" (`help.rs:211-214,267`), while the fetch summary and code treat the
  PNG as optional (`help.rs:715`, `command/fetch.rs:66-72,150-154`). The help
  fixture locks the wrong text (`tests/fixtures/help-all.txt:321,529`).
- Root cause: option descriptions are shared across commands with different
  defaults.
- Impact: a user expects the run config's output root or believes a PNG is
  mandatory.
- Affected contracts/docs: `help all`; `README.md` fetch/lab sections.
- Dependencies: none.
- Effort: trivial.
- Options:
  - A. Add dedicated descriptions for `lab diagnose --out` and optional
    `fetch --png`; update the fixture.
  - B. Change the command behaviors to match the current help.
- Recommendation: A.
- Verification: `help all` names `<lab>/diagnosis` and "Optional PNG target".
- Status: `Planned`.
- Links: [#317](https://github.com/quant-rule/quant-rule/issues/317).

### AUD-2026-10-31-62 — User-facing docs drift from code

- Expected invariant: documentation follows code; links resolve.
- Observed, with the code as the reference:

| Item | Doc | Code |
|---|---|---|
| `research export --root` default | `README.md:250` says `data` | `command/research.rs:48` and `help.rs:176-177` use `data/research` |
| `store show` order | `README.md:286` and `help.rs:644` say primary-key order | `store/query.rs:61-74` orders pivot by occurrence and event by semantic fields |
| Strategy summary fields | `docs/reports/2026-10-28/per-engine-reports.md:137-141` lists `execution_state` in `summary` and `open_position_at_end` | `command/report_run.rs:494-535` puts `execution_state` at the top level and `summary.open_position` |
| Report artifact set | `per-engine-reports.md:47-48` says an extra file fails the build | `tests/report_command.rs:64-75` only asserts required files exist |
| Target-contract status | `docs/README.md:56-57` says the remaining target contracts wait | `docs/README.md:63-65` says the active ladder and `TradeIntent` are implemented |
| v0.2.0 release links | `docs/releases/2026-10-28/v0.2.0.md:7-8,46,89` use `../execution-plans/...` | From `docs/releases/2026-10-28/`, the four links resolve under `docs/releases/`; a link check finds exactly these four broken links |
| Research scope | `docs/analytics/2026-10-24/research-output.md:28-46` lists record files in the export, then says they are not written | `research/mod.rs:180-228` writes only `m1.csv`, the three layers, and metadata; `dataset-report.md:10` agrees |
| MFE/MAE window | `docs/reports/2026-10-31/diagnosis-v1.md:160-162` says entry through exit | `diagnosis/compute.rs:868-875` excludes the exit bar for a strategy exit |

- Root cause: the docs were not updated with the behavior changes, and no link
  check runs in CI.
- Impact: users and consumers follow the wrong contract; four links are
  broken; the dataset scope is ambiguous.
- Affected contracts/docs: the files in the table.
- Dependencies: none.
- Effort: small, but it is a multi-file sweep.
- Options:
  - A. Update every item to the code behavior; add a relative-link check to
    CI. Change code only where the doc is the authoritative contract and the
    human approves the change.
  - B. Change the code to match the docs for the command/report items; keep
    the docs and add the link check.
- Recommendation: A, except for the report summary fields, which need a human
  decision because the doc may be the intended public contract.
- Verification: the link check finds zero broken links; each table row matches
  the code; the help/README fixture tests pass.
- Status: `Planned`.
- Links: [#317](https://github.com/quant-rule/quant-rule/issues/317).

### AUD-2026-10-31-63 — Test and guard coverage gaps

- Expected invariant: a claimed invariant has a test that exercises it; a
  defect found here should have failed a test.
- Observed:
  - Multi-context weights: every context test uses one context timeframe
    (`crates/quantrule-backtest/src/context/tests.rs:65,88,100,171,194`;
    `runner/tests.rs:106`); the strategy golden uses one context. A swap or
    index error across two contexts would pass.
  - Costs: ledger tests pass `0.0` fees and the runner uses fee 0 and funding
    0 (`ledger_tests.rs:56`; `runner/tests.rs:123,140`); the fill-fee
    subtraction, `execution_fees`, and negative funding are untested.
  - Diagnosis checks: the analytics tests do not exercise
    `model.axis_correlation`, `strategy.net_expectancy`,
    `execution.slippage`, `portfolio.cost_drag`, `portfolio.drawdown_limit`,
    or `evidence.rerun` at pass/fail/unknown boundaries
    (`crates/quantrule-analytics/tests/diagnosis.rs`).
  - Lab evidence: the shell tests cover same and distinct OOS versions
    (`tests/diagnosis.rs:209-239`, `tests/lab_summary.rs:286-322`); the
    unreadable branch of `lab_evidence.rs` has no test. The W1 record says the
    analytics test covers the fourth state, but that test maps a status, not
    the file reader (`docs/reports/2026-10-31/w1-oos-evidence-validation.md:40`).
  - Vacuous model tests: the report prefix test alters future candles and then
    builds only `altered[..=199]`, so the altered future is never input
    (`crates/quantrule-model/src/report/tests.rs:450-502`); the detector
    "changes no axis" tests call a pure function with `&Readings` and compare
    a recomputation (`events/divergence.rs:677-693`,
    `events/signal.rs:912-940`).
  - Architecture guard: `tests/architecture.rs:42-51` checks only
    `lab_summary.rs` and `render_state.rs`; `diagnosis.rs:28` imports
    `crate::command::RunError`, an undeclared command↔diagnosis cycle.
  - Duplicate options: some parsers reject a repeated flag (`dashboard.rs:59-67`,
    `report.rs:106-113`, `lab.rs:135-142`), others silently take the last
    (`model.rs:130-146`, `chart.rs:61-74`, `fetch.rs:73-86`,
    `research.rs:53-66`, `store_query.rs:80-86`).
- Root cause: coverage follows the implementation paths that were convenient,
  not the invariants the contracts claim.
- Impact: the Major defects above can regress without a failing test; a
  multi-context weight bug or a fee bug would pass CI.
- Affected contracts/docs: `AGENTS.md` section 4; the named modules.
- Dependencies: none.
- Effort: medium.
- Options:
  - A. Add the named table-driven/boundary tests, expand the architecture
    guard to every renderer and the diagnosis writer, and standardize
    duplicate-option handling with tests.
  - B. Document the gaps and rely on review; this leaves the regressions
    silent.
- Recommendation: A.
- Verification: each named test fails before the corresponding fix and passes
  after it, or has a written reason a fail-before test is impossible.
- Status: `Planned`.
- Links: [#318](https://github.com/quant-rule/quant-rule/issues/318).

### AUD-2026-10-31-64 — Drawdown metric and halt observe different points

- Expected invariant: the maximum drawdown and the halt use the same
  observations, and the contract says which points they are.
- Observed: `refresh_equity_state` updates peak/drawdown/max drawdown and is
  called from `apply_funding` (`portfolio/ledger.rs:210`), `mark` (`:249`),
  `open` (`:349`), and `close` (`:425`). `check_halt` is called only from
  `mark` (`:259`). The record doc says max drawdown is "observed at a mark"
  (`portfolio/records.rs:272`).
- Root cause: the metric was broadened to every equity refresh; the halt was
  left at marks.
- Impact: a funding charge or entry cost can push the drawdown above the limit,
  recover by the next mark, and be published in `max_drawdown` with
  `halted=false` and no `RiskEvent`.
- Affected contracts/docs: `portfolio` report schema; `records.rs:272`.
- Dependencies: none.
- Effort: small.
- Options:
  - A. Call `check_halt` after open/close/funding as well, so a breach latches
    and records.
  - B. Declare the halt mark-only in the contract and stop updating
    `max_drawdown` at fills/funding (or document the broader metric).
- Recommendation: A, unless the human decides the halt is intentionally
  mark-only.
- Verification: a funding/entry-cost breach above the limit yields
  `halted=true` and one `RiskEvent`; or the contract explicitly says
  mark-only and the metric matches it.
- Status: `Planned`.
- Links: [#314](https://github.com/quant-rule/quant-rule/issues/314).

### AUD-2026-10-31-65 — Golden metrics fixture contains runner output

- Expected invariant: a golden fixture is data; the parser fails on an
  unexpected or malformed row.
- Observed: `crates/quantrule-backtest/tests/fixtures/runner_metrics_golden.csv:20`
  is `test runner::tests::dump_goldens_for_regeneration ... ok`, not a metric.
  `metric()` splits each line and returns only on a name match; it has no
  row-count or known-name check (`src/runner/tests.rs:188-208`).
- Root cause: the fixture was regenerated by a test run whose stdout was
  appended; the parser ignores unknown lines.
- Impact: a corrupted or extended fixture can pass; the "hand-derived golden"
  claim is weakened.
- Affected contracts/docs: `AGENTS.md` section 4; runner golden test.
- Dependencies: none.
- Effort: trivial.
- Options:
  - A. Remove the line; make `metric()` reject any row that is not exactly
    `name,value` with a known name, and assert the expected row count.
  - B. Parse the fixture into a map and assert all known names, no duplicates,
    and no unknowns.
- Recommendation: A.
- Verification: the fixture has exactly the expected metric rows; an unknown
  row fails.
- Status: `Planned`.
- Links: [#318](https://github.com/quant-rule/quant-rule/issues/318).

### AUD-2026-10-31-66 — Unreachable public lifecycle and error variants

- Expected invariant: a public lifecycle state or error variant is reachable,
  or it is removed; the contract matches.
- Observed: `PositionState::Closed` is defined and `is_closed()` exists
  (`crates/quantrule-backtest/src/execution/position.rs:159-160,176-180`), but
  the engine sets `Flat` after a strategy exit (`execution/engine.rs:594`) and
  after a bracket (`:819`); no code assigns `Closed`. The trade-intent contract
  says `PositionState` tracks flat, open, and just-closed states
  (`docs/config/2026-10-30/trade-intent.md:51-53`).
  `MissingContextTimeframe` is defined and displayed
  (`context/mod.rs:385-389,431-434`), but `new` maps an absent context to
  `None` (`:512-516`); no code constructs the variant. The `new` doc says it
  returns the error when the run does not contain a configured timeframe
  (`:488-489`).
- Root cause: the lifecycle/error vocabulary was designed ahead of the
  implementation and not reconciled.
- Impact: no shipped path is wrong, but `is_closed()` is never true and the
  docs promise a state/error that cannot occur.
- Affected contracts/docs: `trade-intent.md:51-53`; `context` rustdoc.
- Dependencies: none.
- Effort: small.
- Options:
  - A. Implement the just-closed transition and the context error where they
    are needed.
  - B. Remove `Closed`/`is_closed` and `MissingContextTimeframe`, and correct
    the docs.
- Recommendation: B, unless a caller needs the just-closed state.
- Verification: no `Closed`/`MissingContextTimeframe` remains, or a test
  reaches each.
- Status: `Planned`.
- Links: [#314](https://github.com/quant-rule/quant-rule/issues/314).

### AUD-2026-10-31-67 — Store identity, addressing, and verification scope

- Expected invariant: the same logical dataset produces stable output; every
  stored dataset is addressable; a version-0 SQLite file is not assumed to be
  a QuantRule store; `store verify` states what it proves.
- Observed:
  - `dataset.id` and `event.id` use `AUTOINCREMENT`
    (`store/schema.rs:18-28,134-147`); `db bars`/`db layer3` select `*`
    (`store/query.rs:365`) and expose them. `store remove` deletes the dataset
    but does not reset the sequence (`store/lifecycle.rs:41-43`); `store
    verify` deliberately ignores `event.id` (`store/verify.rs:49-54`).
  - A dataset name beginning with `--` is accepted at ingest
    (`command/store.rs:122-137`) but positional arguments beginning with `--`
    are rejected by `store show`/`remove`/`verify` and `db`
    (`command/store.rs:88-91`, `command/store_query.rs:87-90,190-193`,
    `command/store_verify.rs:53-56`).
  - `store init`/`ingest` migrate any database with `user_version = 0`
    (`store/mod.rs:242-243`); no `application_id` is set.
  - `store verify` rebuilds expected rows with the same writer parameter
    functions (`store/ingest.rs:113-168`) and compares them
    (`store/verify.rs:49-54`); the ADR says the two paths "cannot drift
    silently" (`docs/adr/2026-10-24/sqlite-store.md:199`).
- Root cause: internal database identity is exposed as dataset identity; the
  CLI has no end-of-options escape; store-file identity is only
  `user_version`; verify is a writer/reader consistency check.
- Impact: byte-level output can change across database histories; a stored
  dataset cannot be addressed; an unrelated SQLite file is migrated; verify
  catches drift but not an internally consistent analytics bug.
- Affected contracts/docs: `README.md` store section; `sqlite-store.md`.
- Dependencies: none.
- Effort: medium.
- Options:
  - A. Omit internal ids from query output, validate the dataset-name
    character set, set and verify `PRAGMA application_id`, and document
    `store verify` as consistency-only.
  - B. Keep the current output and document ids as database-local, add a `--`
    escape, and accept the version-0 migration.
- Recommendation: A.
- Verification: the same logical dataset dump is stable across remove/re-ingest;
  a `--` name is rejected at ingest; an unrelated version-0 DB is refused; the
  verify doc states its scope.
- Status: `Planned`.
- Links: [#316](https://github.com/quant-rule/quant-rule/issues/316).

### AUD-2026-10-31-68 — `report_versions` is synthesized on `backtest --diagnose`

- Expected invariant: `identity.report_versions` records the five bundles'
  actual versions.
- Observed: the shell's `report_versions(_config)` returns fixed
  `VIEW_CONTRACT`/`REPORT_VERSION` values and never reads the five
  `report.json` files (`crates/quantrule-shell/src/diagnosis.rs:209-231`).
  `lab diagnose` separately reads and validates the actual versions
  (`command/lab.rs:367-411`); `backtest --diagnose` does not.
- Root cause: the projection was written for the in-process bundles whose
  versions are known constants.
- Impact: on `backtest --diagnose`, a bundle whose report version differs is
  still reported as the constant. The exposure is low because the same binary
  writes the bundles, but the check is not observed evidence.
- Affected contracts/docs: `docs/reports/2026-10-31/diagnosis-v1.md:69`.
- Dependencies: none.
- Effort: small.
- Options:
  - A. Read the five bundle versions from the written files (or the in-memory
    bundle metadata) and pass them to the diagnosis.
  - B. Document the constants and rely on `lab diagnose`'s precheck for
    observed versions.
- Recommendation: A.
- Verification: changing one bundle's report version makes `backtest
  --diagnose` fail the identity check.
- Status: `Planned`.
- Links: [#311](https://github.com/quant-rule/quant-rule/issues/311).

### AUD-2026-10-31-69 — Stale code and module documentation

- Expected invariant: module docs and contracts match the code
  (`AGENTS.md` section 5).
- Observed:
  - `crates/quantrule-shell/src/render.rs:38-41` says "M1, M5, M15, H1" and
    "all four series"; the renderer iterates five series including H4.
  - `crates/quantrule-model/src/timeframe.rs:348-350` says the snapshot carries
    "the full event batch"; it is bounded to `EVENT_SNAPSHOT_LIMIT = 1_024`
    (`retention.rs:16-17`).
  - `docs/model/2026-10-27/market-report.md:86` documents a two-argument
    `MarketReport::compute`; the code takes `(run, ladder, aggregates)`
    (`report/assemble.rs:50`). `:255` describes `ReportLeg` as "the more
    recently created of the two retained scale legs"; the code publishes
    `published_leg()` (`assemble.rs:417`), the newest confirmed leg across
    scales per the base contract.
  - `TargetUnstable` reason 5 is emitted only by the dashboard
    (`dashboard/assemble/progression.rs:290`); the report copies the axis
    reason, so a null stage caused by an unstable target can have no reason
    code. The report contract lists reasons 1..6.
  - `src/source/jsonp.rs:96-99` says a row has "the expected six numeric
    fields"; `:117-121` accepts five with optional volume.
  - `docs/adr/2026-10-24/sqlite-store.md:187` says the default database path is
    `data/quantrule.db`; the code and README use `data/db/quantrule.db`.
- Root cause: behavior changed around the docs; none of these is covered by a
  doc-consistency test.
- Impact: readers cannot trust the module/contract text; the report reason
  vocabulary is incomplete for one case.
- Affected contracts/docs: the named files.
- Dependencies: none.
- Effort: small.
- Options:
  - A. Correct the docs; keep the bounded snapshot and the published-leg rule.
    Add a test or doc lint for the four-series/series-count claim and the
    report constructor.
  - B. Change the code to the documented behavior where the doc is the
    intended contract.
- Recommendation: A.
- Verification: `cargo doc` passes; grep finds five series, the three-argument
  constructor, and the bounded snapshot; the report reason 5 is documented as
  dashboard-only or emitted.
- Status: `Planned`.
- Links: [#317](https://github.com/quant-rule/quant-rule/issues/317).

## Delta review (c8b1dc1..e34693f)

The delta is the remediation of AUD-2026-10-31-31..40, the architecture
baseline, the BTCUSD example move, and documentation cleanup. The ten fixes
remain fixed:

| Prior finding | Check at `e34693f` |
|---|---|
| 31 OOS evidence | `lab_evidence.rs` reads `data_version`; same version `unknown`, distinct `pass`; both commands use it |
| 32 1m gap | `data.long_gap` is `None` → `Unknown` with no value (`compute.rs:985-1015`) |
| 33 direction guard | `checked_add(1)` + range filter (`aggregate/events.rs:128-138`); out-of-range test exists |
| 34 report versions | Requires the five expected bundle names with non-empty versions (`compute.rs:928-961`) |
| 35 zero-trade hold | `Option<f64>` → JSON null and empty CSV cell (`types.rs:288`, `compute.rs:503-507`) |
| 36 manifest self-row | The manifest lists itself with its final byte length (`diagnosis.rs:871-909`) |
| 37 MFE/MAE | Clamped to `mfe >= 0`, `mae <= 0` (`compute.rs:868-905`) |
| 38 daily entry fills | Built from `input.fills` filtered to `entry` (`compute.rs:583-613`) |
| 39 lab module doc | Names `lab summary`, `lab diagnose`, and `lab list` (`command/lab.rs:1-8`) |
| 40 lab list hash | 16-hex validation plus `get(..12)` display (`command/lab.rs:471-491,731`) |

The previous verification record is
[`docs/reports/2026-10-31/improvement-verification-remediation-diagnosis-v1.md`](../../reports/2026-10-31/improvement-verification-remediation-diagnosis-v1.md).
No regression was found in the ten fixes. Two adjacent paths were not covered
by that remediation and are reported here: rerun evidence (43) and the
`checks.csv`/`model-health.csv` null convention (50). The unreadable
`lab_evidence` branch is also untested (63).

## Coverage and limitations

- The full repository was inspected at `e34693f`: the five crates, the shell
  commands, the store, sources, analytics/research, diagnosis, charts, reports,
  configuration, contracts, architecture baseline, CI, and tools.
- The `c8b1dc1..e34693f` delta was reviewed in full: 70 files, +3,873/−273.
- Global gates were run at the head; the results are above. No test was
  modified, removed, or skipped.
- The execution probe is outside the repository
  (`/tmp/opencode/execution-probe`) and is not committed. It reproduces
  AUD-48.
- No live Dukascopy request was made; no full-year source was present. The
  source adapter was inspected, not exercised against the provider.
- No performance benchmark was run; the audit did not identify a new
  performance-sensitive path that requires one.
- Store migrations were inspected and the existing tests were read, but no
  adversarial migration or corruption was run.
- Chart visual correctness and pixel-level diagnosis pages were not inspected
  beyond the shared missing-value paths.
- The project board was not inspected; board state is not part of the audit.
- Data files under `data/` and generated `target/` artifacts were not audited as
  code; `target/report-tests` was used only as observed output.
- This is a sequential read-only pass by one agent. The five exploration
  passes were separate read-only subagent sessions of the same model family;
  the consolidation, severity assignment, and re-verification were performed
  by the main agent. It is not an independent-agent or independent-model
  judgment.

## Positive observations

- The ten prior fixes are present, tested, and unchanged by the delta.
- The execution engine's fill policy is documented and well tested: next-bar
  open, slippage/fees, gap precedence, stop-first ties, zero volume,
  non-contiguous bars, pending expiry, capped quantity, and finish behavior.
- The context builder is causal and as-of; the prefix test and the context
  identity guard are present.
- The ledger enforces `final_equity = initial_cash + net_pnl`, partial-exit
  rejection, exit-quantity reconciliation, monthly-return carry, and
  deterministic ordering.
- The model's pivot availability, event availability, event retention, axis
  extraction, and golden fixtures are strong; no future leak was found in the
  inspected paths.
- The diagnosis is byte-identical across runs; the manifest lists all 17
  artifacts including itself; the 1m-gap, MFE, daily-fill, and report-version
  fixes are correct.
- The shell has no clock, no `HashMap` iteration into output, sorted lab
  listings, typed errors on the reviewed paths, and atomic bundle writes.
- CI runs fmt, clippy with denied warnings, the workspace tests, the Python
  progression oracle, and rustdoc with denied warnings.
- The architecture baseline's kata and characteristics are coherent; the
  crate-graph error in AUD-47 is a factual omission, not a design failure.

## Next step

The Planner pass is written as
[`docs/execution-plans/2026-10-31/full-system-audit.md`](../../execution-plans/2026-10-31/full-system-audit.md)
and is `Approved`. It groups the 29 findings into eleven work items; D1–D17 are
approved. The issue tree is created: W1–W11 are #308–#318, and W1 (#308) is
`In Progress`. No code or test was changed, and no commit or pull request was
made by this audit.
