# S3.x Stage-Surface Audit

- Date: 2026-11-07 (UTC)
- Audit ID: `AUDIT-2026-11-07-s3-stage-surface`
- Status: Published; findings 01, 02, and 04–10 `Fixed` in PR #536 at `e8f99cc`; `AUD-2026-11-07-03` `Won't fix` by the human decision of 2026-11-07; independent verification pending
- Baseline: `main` at `fbc38a0`
- Scope: the S3.x strategy, execution, portfolio, backtest, qualification, and
  analytics commands and engines; their artifacts, provenance, boundary tests,
  determinism, and current documentation.
- Limitations: one sequential audit pass; no independent agent or human review;
  the live source path, the data engine, and the model formulas were not
  re-audited except where the strategy handoff depends on them; the run data is
  the repository fixture; no profitability or edge claim is made.

## Summary

| Severity | Count |
|---|---:|
| Critical | 3 |
| Major | 2 |
| Minor | 5 |
| Info | 0 |

`AUD-2026-11-07-03` is `Won't fix` by the human decision of 2026-11-07: the
`allow_short` rule is removed from the current contract, so a short fill is now
the expected behavior. The other nine findings are `Fixed` in PR
[#536](https://github.com/quant-rule/quant-rule/pull/536) at `e8f99cc`;
independent Improvement Verifier closure is pending.

The first Independent Verifier pass returned `Not verified`. The AUD-02, AUD-04,
AUD-05, and AUD-08 guards were strengthened on
`fix/audit-verification-remediation`; that follow-up is not yet merged, so the
findings stay `Fixed` until it lands and the verifier reruns.

The S3.x stages run, their gates pass, and their bundles are byte-deterministic
on the fixture. The strategy handoff is not consistent: the coupled runner
ignores the strategy's declared execution timeframe, reads a different event set
than the standalone strategy, and does not enforce the portfolio's short-entry
rule. The remaining findings are a registry/role gap, an event-window contract
drift, and test/documentation defects.

The existing gates do not catch these defects: every finding below is a
semantic or cross-stage defect, and the gate suite is green on the baseline.

> The paragraphs above describe the audit baseline. The handoff defects were
> fixed in PR [#536](https://github.com/quant-rule/quant-rule/pull/536); the
> current status is in the Summary and Remediation evidence sections.

## Findings

| ID | Severity | Area | Finding | Evidence | Impact | Options | Recommended | Verification | Status | Links |
|---|---|---|---|---|---|---|---|---|---|---|
| AUD-2026-11-07-01 | Critical | backtest/strategy | `backtest run` evaluates the strategy on the run's `fill_timeframe`, ignores the strategy's `execution_timeframe`, and skips the condition/context validation that `strategy explain` and `strategy run` perform | `crates/quantrule-shell/src/backtest/run.rs:130-176`; `crates/quantrule-shell/src/strategy/run.rs:113-131`; observed 15m strategy run on 5m with a 15m provenance label; a 15m condition silently held while `strategy run` refused it | Silent run on the wrong timeframe; the decisions and their provenance disagree; a typo in a condition timeframe silently disables the strategy | A. Reject a mismatch and call the context validation before the run (recommended); B. make the runner use the strategy's declared execution timeframe; C. record both timeframes and document the override | A | A 15m strategy against a 5m run is refused; a condition timeframe outside execution plus context is refused | Fixed | [#536](https://github.com/quant-rule/quant-rule/pull/536) |
| AUD-2026-11-07-02 | Critical | strategy/backtest | The standalone strategy reads `events.csv` (core, extended, and diagnostic events) while the coupled runner reads only the core `timeframe.events()`, so the same model bundle, strategy, and bar can produce different decisions | `crates/quantrule-shell/src/strategy/bundle.rs:374-418`; `crates/quantrule-shell/src/model_bundle/artifacts.rs:151-179`; `crates/quantrule-backtest/src/strategy/event_input.rs:157-181`; observed `strategy explain` `enter_bracket` at bar 41 against the coupled runner's `hold` | The coupled run cannot be reproduced from the model bundle it writes; the stage handoff contract is broken; one path sees evidence the other cannot | A. Use one merged event stream in both paths and gate roles (recommended); B. use core events only in both paths; C. publish an explicit strategy event view in the model bundle | A | `an_extended_event_reaches_both_strategy_paths` uses the extended level event `(1,3,19)`; the standalone path enters and the coupled path matches, while the pre-fix coupled path held | Fixed | [#536](https://github.com/quant-rule/quant-rule/pull/536) |
| AUD-2026-11-07-03 | Critical | portfolio/backtest | `allow_short: false` did not prevent a short fill: `portfolio replay` opened a short trade, and the coupled runner sized every bracket as long | `crates/quantrule-backtest/src/portfolio/ledger.rs:152-169,316-340`; `crates/quantrule-shell/src/backtest/run.rs:234-250`; observed `portfolio replay` trade `side=short`, `net_pnl=198.24` with `allow_short: false` in provenance | A disallowed short position and its P&L entered the account; the risk envelope was bypassed | A. The ledger refuses a disallowed short open and the runner rejects the lower bracket leg; B. add `allow_short` to the execution engine and reject the leg; C. refuse two-sided brackets when shorts are disallowed; D. human: remove the rule | Human: remove the rule | `portfolio replay` accepts a short fill under `portfolio/v2`; no `allow_short` field exists | `Won't fix` | [plan](../../execution-plans/2026-11-07/remove-allow-short.md) |
| AUD-2026-11-07-04 | Major | strategy/config/registry | The strategy event selector is never validated against the registry: unregistered identities are accepted silently, diagnostic and non-compression events are accepted, and the cited boundary test guards `StrategyInput` rather than the `EventStrategyInput` the engines use | `crates/quantrule-shell/src/config/strategy.rs:456-501`; `crates/quantrule-backtest/src/strategy/event.rs:278-317`; `crates/quantrule-backtest/src/strategy/bracket.rs:49-56,202-228`; `crates/quantrule-backtest/src/strategy/input.rs:100-140`; observed unregistered `(1,1,2)` config all-holds and a diagnostic bracket decision | A typo silently disables a strategy; a diagnostic event can be the sole trigger of a decision; the "diagnostic never enters a decision" invariant is false; the boundary claim is untested | A. Validate against the registry at parse and restrict the bracket to the registered compression-range event (recommended); B. publish a role/engine code on the event and filter at the input; C. keep range checks and document human responsibility | A | Unregistered and diagnostic configs are rejected; the registry role test proves the diagnostic identity is not strategy-readable; the bracket whitelist is tested | Fixed | [#536](https://github.com/quant-rule/quant-rule/pull/536) |
| AUD-2026-11-07-05 | Major | strategy/backtest | `event_window_bars` is documented in execution bars but applied to each timeframe's own bars, so a 20-bar window at 5m over a 1h context reads 20 hours instead of 100 minutes | `docs/config/2026-11-06/strategy-config.md:67`; `crates/quantrule-shell/src/strategy/bundle.rs:184-204,226-234`; `crates/quantrule-backtest/src/strategy/event_input.rs:160-170`; `crates/quantrule-shell/src/backtest/run.rs:152` | A context event can be much older than the human intended; the strategy's published meaning drifts | A. Convert the execution-bar window to each context timeframe (recommended); B. document the per-timeframe semantics and add tests; C. remove context events or make the window per timeframe explicit | A | `the_context_window_is_expressed_in_execution_bars` and `the_context_view_window_is_converted_from_execution_bars`; the strategy contract documents the rounded conversion | Fixed | [#536](https://github.com/quant-rule/quant-rule/pull/536) |
| AUD-2026-11-07-06 | Minor | backtest/provenance | The backtest manifest omits the strategy config identity the artifact contract names | `docs/config/2026-11-06/engine-artifacts.md:352-359`; `crates/quantrule-shell/src/backtest/run.rs:manifest_bytes`; observed manifest keys `code_commit, config_id, data_version, period, run, schema, seed, stage_order, stages` | A manifest reader cannot identify the strategy config without opening the strategy provenance; the documented manifest is incomplete | A. Add `strategy_config_id` to the manifest (recommended); B. amend the contract to say the strategy identity is only in the stage provenance | A | Manifest test asserts the field and matches `strategy/provenance.json` `config_id` | Fixed | [#536](https://github.com/quant-rule/quant-rule/pull/536) |
| AUD-2026-11-07-07 | Minor | docs | The S3.x status docs still say Phase 8 is unpublished, while the Phase 8 PR is merged on the baseline | `docs/execution-plans/2026-11-06/s3-command-surface.md:4,668,790`; `docs/architecture/command-surface.md:243`; `docs/README.md:35`; `git log` shows PR #515 merged at `5e2ab6c` | Readers and agents believe the surface is unpublished and Phase 1 is current | A. Update the current status lines and index (recommended); B. keep the historical phase records and add a dated status note at the top | A | Current status lines match Git; `check_links.py` and `check_terms.py` pass | Fixed | [#536](https://github.com/quant-rule/quant-rule/pull/536) |
| AUD-2026-11-07-08 | Minor | tests | The strategy rule has no causality test that appending, changing, or removing later bars leaves an earlier decision unchanged; the execution engine has one, the strategy does not | `crates/quantrule-backtest/src/strategy/event.rs` test module; `crates/quantrule-backtest/src/strategy/bracket.rs:365` (determinism only); `crates/quantrule-shell/tests/strategy_command.rs` (command determinism); `AGENTS.md` §4 causality tests | The causality claim is not directly tested; a future stateful change can regress it silently | A. Add a causal test over `EventStrategyInput` and `CompressionBracketInput` (recommended); B. document why the stateless engine makes the test unnecessary | A | `later_bars_do_not_change_an_earlier_snapshot`; `the_snapshot_boundary_keeps_both_engine_decisions_causal` | Fixed | [#536](https://github.com/quant-rule/quant-rule/pull/536) |
| AUD-2026-11-07-09 | Minor | backtest/output | `backtest run` does not refuse an existing non-empty `backtest` root when its stage directories are absent; it writes beside the pre-existing files | `crates/quantrule-shell/src/backtest/run.rs:138-142`; `crates/quantrule-shell/src/model_bundle/bundle.rs:90-92`; `crates/quantrule-shell/tests/backtest_command.rs:204-217`; observed `junk.txt` retained in a successful run | A target that is not a clean bundle can be mixed with new stage artifacts | A. Check `ensure_available(&root)` before the model write (recommended); B. document that only each stage directory is checked | A | A `backtest` root containing an unrelated file is refused | Fixed | [#536](https://github.com/quant-rule/quant-rule/pull/536) |
| AUD-2026-11-07-10 | Minor | docs/strategy | The evidence table documents `level` and `progression` evidence kinds, but the strategy emits only `event` and `state`; no condition kind can produce the other two | `docs/config/2026-11-06/engine-artifacts.md:201-213`; `crates/quantrule-shell/src/strategy/run.rs:460-495`; `crates/quantrule-shell/src/config/strategy.rs:448-505` | A reader expects evidence and conditions the rule language does not support | A. Remove the unused kinds from the table (recommended); B. add the conditions and evidence as a new approved rule-language change | A | The evidence table lists only kinds the engine can emit; a search for `"kind":"level"` and `"kind":"progression"` returns none | Fixed | [#536](https://github.com/quant-rule/quant-rule/pull/536) |

## Detail

### AUD-2026-11-07-01 — the coupled runner ignores the declared execution timeframe

- Expected invariant: the strategy is evaluated on the timeframe it declares, or the run is refused before a bundle is written. A condition timeframe must be the execution timeframe or a configured context timeframe.
- Observed: a copy of `crates/quantrule-shell/tests/fixtures/strategy/event-composite.json` with `execution_timeframe: "15m"`, `context_timeframes: ["1h"]`, and conditions still on `5m` ran with `backtest run` against the fixture run config whose `execution.fill_timeframe` is `5m`. The command exited 0, the first decision record carried `"timeframe_secs": 300`, and `strategy/provenance.json` recorded `"execution_timeframe": "15m"`. In a second case, a condition on `15m` with no context produced a successful run with 52 holds, while `strategy run` over the same model bundle exited 1 with `a strategy condition names 900s, but it is not the execution timeframe or a configured context timeframe`.
- Root cause: `backtest/run.rs` takes `config.execution.fill_timeframe` for the model series and the strategy input, but never compares it with `strategy_config.execution_timeframe`; it also never calls the `validate_context` logic in `strategy/run.rs`. The strategy metadata uses the declared timeframe, so the decisions and the provenance disagree.
- Impact: a strategy can silently run on a timeframe it was not written for, with different states, events, context, and execution bars. A condition typo silently becomes a no-trade strategy instead of an error.
- Affected contracts/docs: `docs/architecture/command-surface.md` §2-3; `docs/config/2026-11-06/strategy-config.md` §2-3; `docs/config/2026-11-06/engine-artifacts.md` §5; `AGENTS.md` §2 (strategy reads published measurements) and §3 (causality/config ownership).
- Dependencies: the run composition root and the strategy parser. No model or strategy formula change is required.
- Effort: S-M.

#### Options

A. Reject in `backtest run` when `strategy_config.execution_timeframe` is not the run's `execution.fill_timeframe`, and validate every condition timeframe against execution plus context.
B. Make `backtest run` select the strategy's declared execution timeframe for the model series, the execution engine, and the portfolio marks, requiring the run config to agree or refusing.
C. Keep the run's timeframe as an explicit override and record both the declared and effective timeframes in the strategy provenance and manifest.

#### Recommendation

A. The strategy declares the timeframe; a run that disagrees is a configuration error, not an override. A and B are compatible later if a deliberate override becomes a product decision.

#### Verification

A binary-level test with a 15m strategy and a 5m run must fail before a bundle is written. A binary-level test with an entry condition on an unconfigured timeframe must fail with the same message `strategy run` emits.

#### Links

— (no issue or PR yet; the Planner assigns the remediation).

### AUD-2026-11-07-02 — standalone and coupled strategy inputs read different event sets

- Expected invariant: the same model bundle, strategy file, and bar produce the same decision whether the strategy is run through `strategy explain`, `strategy run`, or `backtest run`. The model bundle is the handoff.
- Observed: using the model bundle written by `backtest run` and a compression-bracket strategy whose event is `(event_type 3, subtype 21, source 41)`, `strategy explain --bar 41` printed an `enter_bracket` decision with evidence `event_code 120, subtype 21, source 41`; the coupled `backtest run` decision record at bar 41 was `hold`. The event is present in `events.csv` and is a diagnostic-role event in the registry. The same asymmetry exists for any extended event a bracket can name.
- Root cause: `strategy/bundle.rs` parses `events.csv`, which `model_bundle/artifacts.rs` fills from `timeframe.all_events()` (core plus extended plus diagnostic). `event_input.rs::view` reads `timeframe.events()`, which is the core batch only. The standalone path and the coupled path therefore splice different event streams into the strategy input.
- Impact: the coupled run is not reproducible from its own model bundle. A later `strategy explain` over the bundle can disagree with the decision the runner made. The stage handoff contract says a stage reads the artifact it is given; here the artifact and the in-memory run disagree.
- Affected contracts/docs: `docs/config/2026-11-06/engine-artifacts.md` §4-5 and §11; `docs/architecture/command-surface.md` §1-2 and §4; `AGENTS.md` §2 (availability, model publishes measurements), §3 (causality), §12 (determinism/reproducibility).
- Dependencies: the model event batch and the strategy event view. A model or event-definition change is not required.
- Effort: M.

#### Options

A. Make both paths use `timeframe.all_events()`, and gate roles at the strategy boundary so diagnostics never enter a decision.
B. Make both paths use the core `timeframe.events()` batch, and document that extended/session events are not strategy inputs.
C. Publish an explicit strategy event view in the model bundle and have both paths read it.

#### Recommendation

A if the extended/session events are intended to be strategy inputs; B if they are not. Either choice must name the canonical event set in the model/strategy contract and add the cross-path test.

#### Verification

`an_extended_event_reaches_both_strategy_paths` uses the extended level event `(1,3,19)`; the standalone path enters and the coupled path matches, while the pre-fix coupled path held.

#### Links

[PR #536](https://github.com/quant-rule/quant-rule/pull/536).

### AUD-2026-11-07-03 — `allow_short: false` does not prevent a short fill

- Expected invariant: `portfolio.allow_short: false` means no short position can be opened, regardless of whether the fill came from the coupled runner or a standalone fills bundle.
- Observed: `portfolio replay` accepted a fills bundle whose entry side was `sell` and whose exit side was `buy`, with a run config whose `portfolio.allow_short` is `false`. The command exited 0, `trades.csv` recorded `side=short` with `net_pnl=198.24211369439251`, and `portfolio/provenance.json` recorded `"allow_short": false`. In the coupled path, `backtest/run.rs` sizes every `Decision::EnterBracket` as `ledger.size_entry(Side::Long, ...)`; the execution engine applies that quantity to either boundary and has no `allow_short` input, so the lower leg can fill as a sell.
- Root cause: `allow_short` is enforced only inside `PortfolioConfig::size_entry`; `PortfolioLedger::apply_fill`/`open` does not consult it, and the execution engine's bracket path does not know the portfolio rule. The standalone execution command does use `sizing.size(intent.side, ..., allow_short)`, so an event-composite short is correctly rejected there; the bracket and replay paths bypass it.
- Impact: a risk rule in the human-owned portfolio configuration is not enforced. A short trade, its P&L, and its exposure can enter the account.
- Affected contracts/docs: `docs/config/2026-10-30/policy-and-strategy.md` (portfolio `allow_short`); `docs/config/2026-11-06/engine-artifacts.md` §6-7; `docs/architecture/command-surface.md` §2 (portfolio owns accounting and risk) and §6; `AGENTS.md` §2 (execution/portfolio boundaries) and §3 (risk rules).
- Dependencies: portfolio ledger, execution engine, and the coupled runner. No model or strategy formula change is required.
- Effort: S-M.

#### Options

A. Make the ledger refuse an opening fill whose side is disallowed by `allow_short`, and make the coupled runner reject the lower bracket leg before submission. This catches both the replay and the coupled path.
B. Add `allow_short` to the execution engine and reject the lower leg there; keep the ledger as accounting only.
C. Refuse a two-sided bracket entirely when shorts are disallowed, so only the long side is available.

#### Recommendation

A at the time of the audit. The human subsequently chose a different remediation:
remove the rule entirely.

#### Decision (2026-11-07)

The human instructed: “take out from the whole repo this rule, we always operate
in both directions, we never need this rule.” The current portfolio contract is
now `quantrule.portfolio/v2`; `allow_short` and `SizingRejection::ShortNotAllowed`
are removed, and a short fill is expected behavior. The finding is `Won't fix`,
not because a short fill under the old rule is acceptable, but because the rule
no longer exists. The remediation is recorded in
[`docs/execution-plans/2026-11-07/remove-allow-short.md`](../../execution-plans/2026-11-07/remove-allow-short.md).

#### Verification

A binary-level `portfolio replay` with a short entry and a `portfolio/v2` config
must accept the fill and write `side=short`. A config containing `allow_short`
must be rejected as an unknown field. The current tree implements both.

#### Links

[Removal plan](../../execution-plans/2026-11-07/remove-allow-short.md).

### AUD-2026-11-07-04 — event identities are not registry-validated

- Expected invariant: a strategy condition and a compression-bracket event name a registered event with an allowed role and an engine-appropriate meaning. A diagnostic event is never readable by a strategy. A typo is rejected, not silently inert.
- Observed: a strategy with entry event `(event_type 1, subtype 1, source 2)` — an identity not present in `docs/registry/indicators.json` — ran through `backtest run` and produced 52 holds. A compression-bracket strategy naming `(3, 21, 41)`, the diagnostic `signal.direction_changes`, produced `enter_bracket` in `strategy explain`. The config parser and the engine both range-check only. The cited boundary test in `strategy/input.rs` inspects `StrategyInput`, while the event-composite and compression engines consume `EventStrategyInput`; the test therefore does not guard the actual input.
- Root cause: the strategy config parser does not load the registry, and the engine's `validate_condition` checks numeric ranges (`event_type 1..=3`, `subtype 1..=7` for event-composite, `subtype 1..=23` for bracket, `source 1..=54`) without a registry lookup. The bracket's `range()` accepts any event with finite `primary_value > reference_value > 0` and `direction == 0`, so a level, divergence, extended, or diagnostic event can act as a range. The boundary test was written against the legacy narrow `StrategyInput` and never moved to `EventStrategyInput`.
- Impact: a typo silently disables a strategy; a diagnostic event can be the sole trigger of a bracket decision; the rule-purity and role invariants are not enforced. The contract's claim that a boundary test proves the strategy cannot read a diagnostic is false.
- Affected contracts/docs: `docs/registry/README.md` §4 (role rules; diagnostics never readable); `docs/config/2026-11-06/strategy-config.md` §3 (registered subtype/source); `docs/config/2026-11-06/engine-artifacts.md` §5.2 (diagnostic never in evidence); `docs/architecture/command-surface.md` §6; `AGENTS.md` §2 (indicator/event roles, no double counting), §3 (indicator roles and claims), §4 (boundary tests).
- Dependencies: the registry is a repository artifact; the shell may read it (shell boundary) or the model may publish a role code. A model event-schema change is not required for a parser-only check.
- Effort: S-M for a parser/registry check; M-L for a published role code and a structural filter.

#### Options

A. Validate every configured event identity against the registry at parse time, reject diagnostic roles, and restrict the compression bracket to the registered compression-range event. Add a boundary test over `EventStrategyInput`.
B. Publish a role and engine-family code on each event record and filter at the strategy input, so no future condition can read a diagnostic even if the config parser is bypassed.
C. Keep the numeric range checks and document that the human owns the role and identity choice.

#### Recommendation

A for the immediate fix, with B as the structural guard if the model event schema can carry a role code. C is not acceptable for a system whose contract says a diagnostic never enters a decision.

#### Verification

Configs with `(1,1,2)`, `(3,21,41)`, and a non-compression event in the bracket are rejected before a decision or a bundle. `a_diagnostic_identity_is_not_strategy_readable` proves the registered diagnostic identity is not a strategy role. The diagnostic event in the input test remains a boundary assertion; it passes before the fix because exact-identity matching already excluded it.

#### Links

[PR #536](https://github.com/quant-rule/quant-rule/pull/536).

### AUD-2026-11-07-05 — the event window is not in execution bars

- Expected invariant: `event_window_bars` is a window in execution bars, as the strategy configuration contract states.
- Observed: the same `window_bars` value is passed to the execution view and to every context view, and each view subtracts it from that view's own bar index. With 5m execution, 1h context, and `event_window_bars: 20`, the context view includes 20 one-hour bars (20 hours), not the 100 minutes that 20 execution bars represent.
- Root cause: `strategy/bundle.rs::input` calls `events_for(context, context_row.bar, window_bars)` and `event_input.rs::with_view` calls `view(timeframe, bar_index, window_bars)`; neither converts the execution-bar window to the context timeframe. The docs and the code disagree.
- Impact: a context condition can match an event far older than the human intended, changing the strategy's meaning without an error. The same strategy file can be read differently by a reader who trusts the contract.
- Affected contracts/docs: `docs/config/2026-11-06/strategy-config.md:67`; `docs/config/2026-11-06/engine-artifacts.md` §5.2; `docs/architecture/command-surface.md` §2.
- Dependencies: the strategy input builder; no model formula change.
- Effort: S.

#### Options

A. Convert the execution-bar window to each context timeframe's bar count and document the conversion.
B. Keep the per-timeframe meaning, change the contract, and add a test that states the context window is in context bars.
C. Remove context timeframes from the event window or require a separate per-timeframe window.

#### Recommendation

A if the documented "execution bars" meaning is the intended one; otherwise B with an explicit contract change. The choice is a strategy-language decision for the human.

#### Verification

`the_context_window_is_expressed_in_execution_bars` covers the arithmetic, including 20 execution bars at 5m/1h = 2 context bars. `the_context_view_window_is_converted_from_execution_bars` asserts the 5m/15m view reaches back only 7 context bars and fails when the conversion is removed. The strategy contract documents the rounded conversion.

#### Links

[PR #536](https://github.com/quant-rule/quant-rule/pull/536).

### AUD-2026-11-07-06 — the manifest omits the strategy config identity

- Expected invariant: the backtest manifest records the fields the artifact contract names, including the strategy config identity.
- Observed: the generated manifest keys are `code_commit`, `config_id`, `data_version`, `period`, `run`, `schema`, `seed`, `stage_order`, and `stages`; there is no `strategy_config_id`. The strategy provenance does carry `config_id` (the strategy config identity), but the manifest does not name it.
- Root cause: `backtest/run.rs::manifest_bytes` writes the run config identity and the stage provenance hashes, but not the strategy config identity.
- Impact: a manifest-only reader cannot identify the strategy configuration; the documented manifest is incomplete. The information is recoverable from the hashed strategy provenance, so this is not a silent result change.
- Affected contracts/docs: `docs/config/2026-11-06/engine-artifacts.md:352-359`; `docs/execution-plans/2026-11-06/s3-command-surface.md` §6.5; `docs/architecture/command-surface.md` §4.
- Dependencies: none.
- Effort: S.

#### Options

A. Add `strategy_config_id` to the manifest and assert it matches `strategy/provenance.json` `config_id`.
B. Amend the contract to say the strategy identity is recorded only in the stage provenance.

#### Recommendation

A; the manifest is the run identity document and the field is already known at write time.

#### Verification

A manifest test asserts the field and its equality with the strategy provenance.

#### Links

[PR #536](https://github.com/quant-rule/quant-rule/pull/536).

### AUD-2026-11-07-07 — the S3.x status documents are stale

- Expected invariant: the current status lines and index match the merged tree.
- Observed: `docs/execution-plans/2026-11-06/s3-command-surface.md` still says "Phase 8 verified locally; publication pending" (line 4), and its Phase 8 record says "publication pending" (line 668 and 790); `docs/architecture/command-surface.md:243` says Phase 8 is "Implemented and verified locally; publication pending"; `docs/README.md:35` says "Phase 1 implemented and verified locally, publication pending". `git log` shows the Phase 8 PR #515 merged at `5e2ab6c`, which is an ancestor of the audit baseline.
- Root cause: the phase records were written before publication and the status lines were not updated after the merges.
- Impact: readers and agents believe the surface is unpublished and Phase 1 is current; the documentation contradicts Git.
- Affected contracts/docs: the three files above; `docs/README.md` §Current architecture.
- Dependencies: none.
- Effort: S.

#### Options

A. Update the current status lines and the index, leaving the dated phase records as history.
B. Add a dated top-of-file note that Phase 8 is merged and the per-phase publication lines are historical.

#### Recommendation

A; the phase records are historical, but the document header and index are current status.

#### Verification

`rg "publication pending" docs/execution-plans/2026-11-06/s3-command-surface.md docs/architecture/command-surface.md docs/README.md` returns only historical phase-record lines; `check_links.py` and `check_terms.py` pass.

#### Links

[PR #536](https://github.com/quant-rule/quant-rule/pull/536).

### AUD-2026-11-07-08 — no strategy causality test

- Expected invariant: a rule that consumes a series has a test proving its output for a given bar does not change when later bars are appended, changed, or removed.
- Observed: the execution engine has `later_bars_do_not_change_earlier_fills` in `crates/quantrule-backtest/src/execution/engine_tests/determinism.rs`; the strategy has no equivalent. `bracket.rs:365` tests determinism (the same input twice), not causality. `strategy_command.rs` tests command byte-determinism, not the effect of later bars. The event-composite engine has no determinism unit test; the shell command test covers its output.
- Root cause: the S3.x test campaign added command-level determinism and boundary tests but did not add the section 4 causality test for the strategy rule.
- Impact: the causality claim is not directly tested; a future stateful or windowed change can regress it without a failing test.
- Affected contracts/docs: `AGENTS.md` §4 (causality tests); `docs/execution-plans/2026-11-06/s3-command-surface.md` §6.2 (boundary and determinism tests); `docs/config/2026-11-06/strategy-config.md` §6 (determinism and causality).
- Dependencies: none.
- Effort: S.

#### Options

A. Add a causal test that builds an `EventStrategyInput` and a `CompressionBracketInput` at bar N, appends later bars, and asserts the decision at N is unchanged; change a later bar and assert the same.
B. Document why the stateless, input-scoped engine makes the test unnecessary.

#### Recommendation

A; the test is cheap and matches the execution engine's existing pattern.

#### Verification

`later_bars_do_not_change_an_earlier_snapshot` compares the bar-40 snapshot across 60 bars, 80 bars, and a changed later bar. `the_snapshot_boundary_keeps_both_engine_decisions_causal` compares both the event-composite and compression-bracket decisions on those snapshots.

#### Links

[PR #536](https://github.com/quant-rule/quant-rule/pull/536).

### AUD-2026-11-07-09 — a non-empty backtest root is not refused

- Expected invariant: a rerun writes a new target and refuses a non-empty target.
- Observed: a `backtest` root containing `junk.txt` but no `model/`, `strategy/`, `execution/`, or `portfolio/` directory is reused. The command exits 0, writes the stage bundles and the manifest, and leaves `junk.txt` in the bundle. The existing test only covers the case where the first run's `model/` directory exists, so the model stage refuses the rerun.
- Root cause: `backtest/run.rs` checks `ensure_available` on each stage directory through the stage writers, but not on the `backtest` root itself.
- Impact: a target that is not a clean bundle can be mixed with new stage artifacts; a reader cannot assume the directory contains only one run.
- Affected contracts/docs: `docs/config/2026-11-06/engine-artifacts.md` §1; `docs/execution-plans/2026-11-06/s3-command-surface.md` §6.5; `docs/architecture/command-surface.md` §4.
- Dependencies: none.
- Effort: S.

#### Options

A. Call `ensure_available(&root)` before the model write, so the whole backtest target is one bundle.
B. Document that the guarantee is per stage directory, not per run root.

#### Recommendation

A; the backtest root is the run bundle and the rule is written at the run level.

#### Verification

A `backtest` root containing an unrelated file is refused before any stage is written. The current tree exits 0 and leaves the file.

#### Links

[PR #536](https://github.com/quant-rule/quant-rule/pull/536).

### AUD-2026-11-07-10 — two documented evidence kinds are not produced

- Expected invariant: the evidence kinds documented in the artifact contract are the kinds the strategy can emit.
- Observed: the contract lists `event`, `state`, `level`, and `progression`; `strategy/run.rs::evidence` and `condition_evidence` emit only `event` and `state`. The strategy configuration supports only `kind: "event"` and `kind: "state"`; there is no level condition, and progression is read through a state field. A search for `"kind":"level"` and `"kind":"progression"` in the crate sources returns none.
- Root cause: the evidence table describes a broader rule language than the implemented `Condition` enum.
- Impact: a reader expects a level or progression evidence record and a condition that can produce it; the contract and the implementation disagree.
- Affected contracts/docs: `docs/config/2026-11-06/engine-artifacts.md:201-213`; `docs/config/2026-11-06/strategy-config.md` §3; `docs/architecture/command-surface.md` §2.
- Dependencies: none for the documentation fix; a new condition kind is a strategy-language change requiring approval.
- Effort: S.

#### Options

A. Remove the `level` and `progression` rows from the evidence table until a condition kind exists.
B. Add the conditions and evidence records as a new approved rule-language change.

#### Recommendation

A; the table should describe the shipped language. B is a product decision, not an audit fix.

#### Verification

The evidence table lists only the emitted kinds; a documentation read and a term check pass.

#### Links

[PR #536](https://github.com/quant-rule/quant-rule/pull/536).

## Gate evidence

Observed on the audit baseline before the audit document was written:

| Check | Observed |
|---|---|
| `git status --short --branch` | `## main...origin/main`; clean |
| `cargo fmt --all -- --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| `cargo test --workspace` | 1,312 passed, 0 failed, 8 ignored |
| `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps` | pass |
| `python3 tools/check_command_surface.py` | 15 implemented, 0 planned; 15 README rows; 8 tutorial files |
| `python3 tools/check_registry.py` | 46 indicators, 143 events, 101 parameters |
| `python3 tools/check_validation.py` | 189 claims for 46 indicators and 143 active events |
| `python3 tools/check_verification.py` | row=1091 family=329 tool=332 n/a=327 |
| `python3 tools/check_verification_matrix.py` | 46 indicators, 143 events |
| `python3 tools/check_event_emitters.py` | 143 registered codes referenced or family-computed |
| `python3 tools/check_links.py` | 597 Markdown files; no broken relative links |
| `python3 tools/check_terms.py` | 362 files; no banned terms |
| `python3 tools/check_legacy.py` | 737 current files |
| `python3 tools/check_file_size.py` | every Rust source at or under 1,000 lines |
| `python3 tools/check_structure.py` | module docs, engine purity, naming, layout, data allowlist pass |
| `git diff --check` | pass |

## Reproduction notes

The observed runs below were performed in a scratch directory outside the
repository, against the checked-in fixtures. They are recorded so the Planner can
reproduce them; the audit itself did not modify the repository outside this
document.

### Execution timeframe and context

```bash
# 15m strategy with 1h context, conditions still on 5m
cp crates/quantrule-shell/tests/fixtures/strategy/event-composite.json /tmp/strategy-15m.json
# edit execution_timeframe=15m, context_timeframes=["1h"]
target/debug/quantrule backtest run \
  --config crates/quantrule-shell/tests/fixtures/config-run/run.json \
  --strategy /tmp/strategy-15m.json --out /tmp/out15
# observed: exit 0; decisions time 300s; strategy/provenance.json execution_timeframe "15m"

# 15m condition with no context
# edit execution_timeframe=5m, context_timeframes=[], entry/exit timeframe=15m
target/debug/quantrule backtest run ... --out /tmp/out-cond
# observed: exit 0; 0 entries
target/debug/quantrule strategy run --model /tmp/out-cond/backtest/model \
  --strategy /tmp/cond-15m.json --out /tmp/stand-cond
# observed: exit 1; "a strategy condition names 900s, but it is not the execution timeframe or a configured context timeframe"
```

### Event set and diagnostic bracket

```bash
# compression-bracket strategy whose event is (3, 21, 41), the diagnostic signal.direction_changes
target/debug/quantrule backtest run ... --strategy /tmp/diag-bracket.json --out /tmp/out-diag2
target/debug/quantrule strategy explain \
  --model /tmp/out-diag2/backtest/model \
  --strategy /tmp/diag-bracket.json --bar 41
# observed: explain -> "decision":"enter_bracket", evidence event_code 120 subtype 21 source 41
# observed: backtest decisions at bar 41 -> "decision":"hold"
```

### Unregistered event

```bash
# event-composite entry (1, 1, 2); not a registry identity
target/debug/quantrule backtest run ... --strategy /tmp/unregistered.json --out /tmp/out-unreg
# observed: exit 0; 52 holds
```

### Short fill under allow_short: false

```bash
# copy a valid fills bundle; make the entry sell and the exit buy; adjust price to ref ± slippage
target/debug/quantrule portfolio replay \
  --fills /tmp/short-fills --config /tmp/cfg/run.json --out /tmp/out-short-portfolio
# observed: exit 0; trades.csv side=short; net_pnl=198.24211369439251
# portfolio/provenance.json: "allow_short": false
```

### Non-empty root

```bash
mkdir -p /tmp/out-junk/backtest
touch /tmp/out-junk/backtest/junk.txt
target/debug/quantrule backtest run ... --out /tmp/out-junk
# observed: exit 0; junk.txt remains beside the new stage bundles
```

## Additional observations (not findings)

These are recorded as observations, not defects, because the current contract
does not require a different behavior or the impact is limited.

- The manifest and stage provenance hashes cover the provenance files, not the
  data files named by the bundles. A byte change in `decisions.ndjson`,
  `fills.csv`, or `portfolio/metrics.json` is not detected by `qualify` or
  `analytics`; the manifest is an identity aid, not a data-integrity check.
- The analytics provenance uses `"artifact": "analytics"`; the artifact contract
  lists `metrics` as the analytics artifact example. The bundle directory is
  `analytics`, so the value is defensible, but the contract could name the field
  more precisely.
- The analytics `exposure` null reason is `no_marks` when the mark count is
  positive but the equity sum is zero. The reason code does not distinguish the
  zero-denominator case.
- `portfolio replay` records the fills bundle's `config_id` as a source artifact
  but does not refuse a fills bundle produced under a different run
  configuration; the contract only requires the data version to match.
- The strategy decision record's `available_bar_index` is the decision bar, not
  the maximum availability of the specific evidence used. It is conservative
  (never later than the decision bar), but it is not the "first bar" the
  artifact contract describes.

## Remediation evidence

The nine open findings were fixed in PR
[#536](https://github.com/quant-rule/quant-rule/pull/536), merged at `e8f99cc`.
The merged `main` tree passes the full gate: 1,323 tests, 0 failed, 8 ignored;
fmt, clippy, doc, the contract checks, the structure check, and
`git diff --check`.

| ID | Fix | Regression guard |
|---|---|---|
| `AUD-2026-11-07-01` | Shared strategy timeframe/context validation; the coupled runner rejects a mismatch before the model bundle. | `backtest_rejects_a_strategy_execution_timeframe_mismatch`; `backtest_rejects_a_condition_outside_execution_and_context`. |
| `AUD-2026-11-07-02` | Both strategy paths use `all_events()`; one canonical event set. | `the_standalone_and_coupled_strategy_decisions_agree_on_the_signal`; `an_extended_event_reaches_both_strategy_paths`. |
| `AUD-2026-11-07-04` | Build-time registry validation; unregistered and diagnostic identities rejected; bracket restricted to `(3,8,5)`. | `an_unregistered_event_condition_is_rejected`; `a_diagnostic_event_is_rejected_as_the_bracket_event`; `a_diagnostic_identity_is_not_strategy_readable`. |
| `AUD-2026-11-07-05` | The execution-bar window is converted to each context timeframe; the rounded conversion is documented. | `the_context_window_is_expressed_in_execution_bars`; `the_context_view_window_is_converted_from_execution_bars`; `strategy-config.md` window contract. |
| `AUD-2026-11-07-06` | The manifest records `strategy_config_id`. | `backtest_writes_the_stage_bundles` manifest assertion. |
| `AUD-2026-11-07-07` | Current S3.x status lines and index corrected. | `check_links.py`; `check_terms.py`. |
| `AUD-2026-11-07-08` | Later-bar causality test for the strategy snapshot and both engine decisions. | `later_bars_do_not_change_an_earlier_snapshot`; `the_snapshot_boundary_keeps_both_engine_decisions_causal`. |
| `AUD-2026-11-07-09` | The backtest root is checked before the model write. | `backtest_refuses_an_unrelated_file_in_the_root`. |
| `AUD-2026-11-07-10` | The evidence table lists only the emitted event/state kinds. | Manual read; `check_terms.py`. |

## Status and next stage

All findings are `Fixed`, except `AUD-2026-11-07-03`, which the human marked
`Won't fix` on 2026-11-07 by removing the `allow_short` rule. The Audit
Workflow's next stage is Verification: the Improvement Verifier checks the merged
fix, the regression guards, the diff, the tests, the absence of unrelated
changes, and the Quant Validator record, then moves each finding to `Verified` or
returns it for remediation.

The fixes are in PR [#536](https://github.com/quant-rule/quant-rule/pull/536),
merged at `e8f99cc`; the plan is PR
[#535](https://github.com/quant-rule/quant-rule/pull/535), and the removal is PR
[#534](https://github.com/quant-rule/quant-rule/pull/534). The audit did not
create the fix issues; no issue number was invented. The `allow_short` removal is
recorded in
[`docs/execution-plans/2026-11-07/remove-allow-short.md`](../../execution-plans/2026-11-07/remove-allow-short.md).
