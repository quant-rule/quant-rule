# Execution Plan — Whole-Project Audit Remediation

- Status: In Progress
- Proposed: 2026-11-17 (UTC)
- Approved: 2026-11-17 (UTC), human instruction "approved proceed"
- Claim type: Behaviour Claim
- Owner: Human
- Workflow: [Audit Workflow](../../workflows/audit-workflow.md) stage 2
  (Remediation Plan) + [Execution Plan](../../workflows/execution-plan.md)
- Records: `AUDIT-2026-11-17-whole-project`; findings `AUD-2026-11-17-01` to
  `-53`; prior audit `AUDIT-2026-11-16-full-program`
- Audit: [`docs/audits/2026-11-17/whole-project-audit.md`](../../audits/2026-11-17/whole-project-audit.md)
- Baseline: `main` at `a07def5c`; current `main` at `7ae3a984` (docs-only
  release-record commits; no code change)
- Branch / worktree: `docs/whole-project-audit-2026-11-17` in
  `../quant-rule--whole-project-audit` for the plan; each fix issue gets its own
  branch after approval
- Issue number: none exists yet. The Planner assigns the fix issue and
  sub-issue numbers after human approval, as `AGENTS.md` section 11 requires.
- AI log: [`docs/ai-transparency-log/2026-11-17/whole-project-audit-remediation.md`](../../ai-transparency-log/2026-11-17/whole-project-audit-remediation.md)
- Changelog: the audit, this plan, and the triage are records; the fixes carry
  the `CHANGELOG.md` entries. No entry is needed for the plan document alone.

## 1. Intent

The whole-project audit found 53 findings: 4 Critical, 26 Major, 18 Minor, and
5 Info. Sixteen are correctness, provenance, accounting, recovery, or evidence
defects; the rest are contract, test, documentation, client, and operations
defects. This plan turns every finding into a bounded, verifiable remediation
work item, orders the Critical fixes first, and keeps the model, accounting, and
evidence changes under the Quant and Independent Verification gates.

The outcome is a tree where every finding is either `Verified` or has a dated
human `Won't fix`/residual-risk record, the full gate and the composed/browser
tiers pass, and the audit status is a reliable source of truth. The plan does
not claim an edge, a profitability result, or a release.

## 2. Current state

- The audit is at `docs/audits/2026-11-17/whole-project-audit.md`; it records
  the baseline, evidence, options, and verification for all 53 findings.
- The gate passed on the baseline: format, clippy, 2,321 tests (16 ignored),
  rustdoc, 25 checkers, 1,027 linked Markdown files, 49/150/199 registry rows,
  46 commands, 12 manual chapters, and one port-parity entry.
- The audit's 4 Critical findings are:
  `AUD-18` restore schema coverage, `AUD-19` partition enforcement, `AUD-20`
  portfolio cash, and `AUD-38` the cancel/complete race.
- The prior audit `AUDIT-2026-11-16-full-program` is `Fixed` for 22 findings,
  with independent verification pending; its remediation plan is `In Review`
  with unresolved rows. This plan does not mark those findings `Verified`; the
  Improvement Verifier owns that (P3.1).
- Current `main` (`7ae3a984`) updated only the v1.3.0 release note, release
  index, status index, and the stage execution plan. No code, test, config, or
  migration changed from the audit baseline. The audit's dated Planner note
  corrects the v1.3.0 note-status part of `AUD-05`; the audit-closure part
  remains.
- The audit worktree is clean except for the audit document and its AI log;
  no fix has started.

## 3. Scope

Included:

- Every finding `AUD-2026-11-17-01` to `-53`, grouped into the work items in
  section 7. Critical and Major findings are mandatory; Minor and Info findings
  are planned unless the human records `Won't fix` or `Deferred`.
- The tests, docs, records, and release-gate changes those fixes require.
- The triage of the prior audit's 30 Critical/Major `Open`/`Fixed` findings
  (P3.1) with dated notes and a status in the audit vocabulary.
- The closure evidence for each work item: regression guard, gate, review, and
  audit status update.

Explicit non-goals:

- No new feature, indicator, event, strategy, execution rule, portfolio rule,
  or analytics capability.
- No model or strategy profitability claim; the model fixes are correctness
  fixes. No baseline is accepted by this plan.
- No release, tag, deploy, merge, or board change without the separate human
  authorization required by `AGENTS.md` sections 9 and 14.
- No rewrite of historical records; corrections are dated notes.
- No `Won't fix` decision; only the human can record that.
- No bulk cleanup of board items, worktrees, or branches outside the approved
  closure step.
- No new dependency, schema, or architecture decision without a separate
  approval.

## 4. Assumptions and uncertainty

- The findings are reproducible at `a07def5c`; the current `main` changes are
  docs-only, so no finding is invalidated by them. The Planner re-reads each
  finding against `7ae3a984` in P0.1; a stale finding is marked `Stale` with a
  dated note, not fixed.
- The operator can run the web, composed, and Postgres tiers on the host or CI.
  If a tier cannot run, the row is `Blocked` with the reason; the plan is not
  `Verified`.
- The model/accounting fixes can be implemented without a new dependency. A
  new dependency stops the work and returns to the human.
- The Quant Validator is independent of the implementer. If no independent
  agent is available, the sequential pass is disclosed and does not count as
  independent validation.
- The partition fix may reject existing research records or configs. A
  migration/backfill plan is part of P1.2; no historical record is deleted.
- The remote-browser fix may require a JS SHA-256 fallback or HTTPS; the human
  chooses the option in D4.
- The Postgres conformance suites can run against a disposable database; the
  release tier is extended only after the human approves the infrastructure
  change (D5).

## 5. Decisions

| ID | Decision | Options | Recommendation | Approval |
|---|---|---|---|---|
| D1 | Execution order | A. Critical P1 first, then P2-P6 one leaf at a time; B. parallel phases | A; parallel only with explicit human authorization | Proposed |
| D2 | Model and accounting changes | A. Quant Researcher + Quant Developer + Quant Validator, deliberate golden regeneration and invalidation; B. implement without quant roles | A | Proposed |
| D3 | Partition enforcement | A. Reject a config outside the declared partition at create/submit; B. warn and record | A | Proposed |
| D4 | Remote browser | A. Pure-JS SHA-256 fallback for non-secure contexts; B. require HTTPS and show a clear error | A, with B as the deployment rule for public exposure | Proposed |
| D5 | Postgres conformance | A. Add a disposable Postgres job to the composed/release tier; B. keep the suites opt-in and record the exception | A | Proposed |
| D6 | Audit/release closure | A. Improvement Verifier verifies the prior audit and this audit before the next release; B. release with a recorded residual-risk acceptance | A | Proposed |
| D7 | Config upload confinement | A. Reject absolute/`..` references and store only submitted bundle bytes; B. document the current read | A | Proposed |
| D8 | Issue creation | A. Create one fix issue/sub-issue per work item after approval; B. create during execution | A; no board change before approval | Proposed |

## 6. Acceptance criteria

- Every Critical and Major finding is `Verified`, or the human records a dated
  `Won't fix` with residual-risk acceptance; every Minor and Info finding is
  `Verified`, `Won't fix`, `Stale`, or `Duplicate` with a dated note.
- Each fix has a regression guard that fails before and passes after, or a
  written reason why a fail-before test is impossible.
- `tools/gate.sh`, the web suite, the composed tier with Postgres, the port
  parity tier, and every current doc checker pass on the merged tree.
- The canonical digest, config identity, promotion, partition, portfolio cash,
  cancel, restore, health, queue, and client fixes pass their specific tests.
- No test is weakened, skipped, removed, or made less strict to obtain a pass.
- The audit status table and the prior-audit statuses are current; the release
  and audit-closure records are complete.
- The changelog, AI log, and record map are updated; the fix worktrees and
  branches are removed after merge.

## 7. Work breakdown

The work is ordered. P0.1 is the Planner's verification; P1 starts only after
approval. Each work item is one issue or sub-issue with one branch and one
reviewer. The `Branch` column is the proposed slug; the Planner prepends the
issue number after approval.

### P0 — Triage and verification

| Work item | Findings | Scope | Roles | Branch | Depends on | Acceptance |
|---|---|---|---|---|---|---|
| P0.1 | All 53 | Re-read each finding against `7ae3a984`; mark `Stale`/`Duplicate` with a dated note; map each finding to a work item; add the Planner note to `AUD-05`; update the audit statuses to `Triaged`/`Planned` on approval | Planner | same docs branch | human approval of this plan | every finding has a work item and a disposition; no `Open` Critical/Major without a decision |

### P1 — Critical safety and integrity

| Work item | Findings | Scope | Roles | Branch | Depends on | Acceptance |
|---|---|---|---|---|---|---|
| P1.1 | 18 | Include `gateway` and `stage` in the restore count, drop, and round-trip test; refuse a non-empty target unless `--force`; test on a disposable stack | Implementation, Test, Release/Operations | `fix/audit-restore-schemas` | P0.1 | all six schemas restored; a partial target cannot be silently destroyed |
| P1.2 | 19 | At `research create`/`submit`, compare the run config's `data.from`/`data.to` (or dataset coverage) with the partition window; reject a non-contained config; add the regression test | Data Steward, Implementation, Test, Quant Validator | `fix/audit-partition-window` | P0.1 | a `strategy_dev` + 2024 config is refused; a contained config passes |
| P1.3 | 26 | Admit 2025 COPPER (and any missing TSLA) or remove the uncovered instruments from `portfolio_holdout`; update the data-change record and the coverage check | Data Steward, Implementation, Test | `data/audit-holdout-coverage` | P0.1, D3 | every holdout instrument has 2025 coverage; the data record and partition agree |
| P1.4 | 20, 34 | Cap the entry notional at `equity × leverage`; reject a fill that makes cash negative; reject an entry while halted; mark funding at the charge price; add tests and record any baseline invalidation | Implementation, Test, Quant Validator | `fix/audit-portfolio-cash` | P0.1, D2 | cash never negative; halt and funding tests pass; baselines are invalidated or re-run |
| P1.5 | 38 | Make the Postgres cancel update conditional on `queued`/`running`, inspect `rows_affected`, return `Conflict` on zero; add a concurrent cancel/complete test | Implementation, Test | `fix/audit-cancel-race` | P0.1 | a succeeded run is never cancelled; a second cancel is a conflict |
| P1.6 | 04 | Refuse a server run/stage when `QR_IMAGE_DIGEST` is unset; set it in `make up`; keep the desktop package digest; add the two-build test | Release/Operations, Implementation, Test | `fix/audit-image-identity` | P0.1 | `/health` and provenance agree; two builds produce different run ids |

### P2 — Model, evidence, and provenance

| Work item | Findings | Scope | Roles | Branch | Depends on | Acceptance |
|---|---|---|---|---|---|---|
| P2.1 | 45 | Track session volume completeness; publish `session_vwap` null from the first missing-volume bar; add the session edge tests | Quant Researcher, Quant Developer, Quant Validator | `fix/audit-session-vwap` | P1.4, D2 | a missing-volume session publishes null; the session event cannot fire |
| P2.2 | 46 | Reject or clamp a divergence RSI window above the pivot lookback; add the prefix test | Quant Researcher, Quant Developer, Quant Validator | `fix/audit-divergence-window` | P2.1 | a too-wide window is refused; the event is unchanged by later bars |
| P2.3 | 47 | Guard the Donchian/Keltner `bar - 1` at bar 0, or require a window of at least two; add the boundary test | Quant Developer, Test, Quant Validator | `fix/audit-donchian-window` | P2.2 | no panic at bar 0; the event is correct or safely absent |
| P2.4 | 49 | Change the VWAP slope prior index to `bar - window`; regenerate the golden deliberately | Quant Researcher, Quant Developer, Quant Validator | `fix/audit-vwap-slope` | P2.1 | the formula matches the contract; the golden is regenerated |
| P2.5 | 48 | Plumb `structure.scale` into the structure readings/events, or remove the field and update the registry | Quant Researcher, Quant Developer, Quant Validator | `fix/audit-structure-scale` | P2.1 | a minor-scale config changes the output; the registry matches |
| P2.6 | 50, 51 | Decouple VWAP bands/slope and range expansion from ATR; register `session_vwap` | Quant Developer, Test, Quant Validator | `fix/audit-reading-availability` | P2.1 | non-ATR readings publish without ATR; the registry lists every reading |
| P2.7 | 17, 24 | Add per-event edge tests and a retest fixture; name the proof in the verification map; extend the checker to reject a readings-only event edge claim | Test, Quant Developer | `fix/audit-event-edge` | P2.1-P2.6 | a diagnostic/edge fixture constructs events; the retest event has a golden |
| P2.8 | 21, 22, 23 | Unify the config identity on SHA-256; include `metrics.json` in the canonical digest; add a model-input hash for promotion; add the cross-crate tests | Implementation, Test, Quant Validator | `fix/audit-evidence-identity` | P1.4, P2.4 | run record, artifacts, and manifests agree; a metrics-only change changes the digest; a strategy-only change can reuse the model |
| P2.9 | 32, 33 | Filter the strategy event snapshot by role and remove/register `ContextBar`; add the constructed-snapshot test; enforce the v4 strategy/fill timeframe check and reject v3 at `run submit` | Implementation, Test, Quant Validator | `fix/audit-strategy-boundary` | P2.8 | a diagnostic event cannot satisfy a rule; a mismatched/v3 config is refused at submission |
| P2.10 | 31 | Define and test the stop-entry bracket path convention, or evaluate the bracket from the next bar; regenerate the golden deliberately | Quant Researcher, Quant Developer, Quant Validator | `fix/audit-execution-path` | P2.9 | the contract states the rule; the fixture covers a pre-trigger low |
| P2.11 | 52, 53 | Remove the retired policy-universe clause from data-v3; build the event batch/history from `merged_events` or document the base-only view | Implementation, Data Steward, Test | `fix/audit-data-events` | P0.1 | the data doc and loader agree; the batch/history includes extended events |

### P3 — Release, audit, and records

| Work item | Findings | Scope | Roles | Branch | Depends on | Acceptance |
|---|---|---|---|---|---|---|
| P3.1 | 05, 06, 25 | Improvement Verifier verifies the 2026-11-16 audit and this audit; triage the 30 prior Critical/Major findings with dated notes; make the release gate fail on an unverified Critical; push the image only after the composed suite; add the browser/tooling release job | Planner, Improvement Verifier, Release/Operations | `docs/audit-closure` | P1, P2 | the audit statuses are current; a release with an unverified Critical fails; no push before composed |
| P3.2 | 01, 02, 03, 07, 08, 12, 14, 43 | Correct `AGENTS.md` (crate list, workflow narrative, status tables), the stage-surface status table, the HTTP API stage routes, the improvement index table, the config cap, the index length, and the service-catalog digest sentence | Technical writer, Architect, Test | `docs/audit-current-docs` | P0.1 | the named documents and the tree agree; the checkers pass |
| P3.3 | 16 | Add or link the missing AI logs for the nine plans; add a `- AI log:` link if a sub-plan uses a different name | Planner, Orchestrator | `docs/audit-ai-logs` | P3.2 | every current plan resolves to one log |

### P4 — Services, operations, and input hardening

| Work item | Findings | Scope | Roles | Branch | Depends on | Acceptance |
|---|---|---|---|---|---|---|
| P4.1 | 39 | Aggregate service health, queue depth, and a worker heartbeat; return `down`/`503` when a required dependency is down, or narrow the contract and add `/ready` | Implementation, Security, Release/Operations | `fix/audit-health` | P1.6 | killing `qr-data` makes the gateway report down; a healthy stack reports workers/queue |
| P4.2 | 40, 41 | Enable RabbitMQ publisher confirms; open a channel per consumer with its own QoS; wire `QR_BACKTEST_CONCURRENCY`; add the composed timing test | Implementation, Test, Release/Operations | `fix/audit-queue-concurrency` | P4.1 | a closed channel returns an error; a stage does not block a run; the concurrency knob works |
| P4.3 | 09, 42 | Add a disposable Postgres job to the composed/release tier and run the ignored suites; run stage retention on an interval or document startup-only | Implementation, Test, Release/Operations | `fix/audit-postgres-retention` | D5 | the eight suites run; an expired stage is removed without a restart or the docs say startup-only |
| P4.4 | 10, 11, 13, 44 | Confine every `run.json` reference to the bundle; apply `safe_key` to `data_key`/`strategy_key`; fix the stage retention check-then-act; pin the infrastructure images and remove the default store credentials; use route-specific client timeouts and correct the contract drifts | Security, Implementation, Test | `fix/audit-input-hardening` | P0.1, D4, D7 | a `../`/absolute/dev-zero reference is rejected; a concurrent stage keeps its blob; the compose digests/credentials pass; the error table matches |

### P5 — Clients and command parity

| Work item | Findings | Scope | Roles | Branch | Depends on | Acceptance |
|---|---|---|---|---|---|---|
| P5.1 | 27 | Add a pure-JS SHA-256 fallback or require HTTPS and show a clear secure-context error | Implementation, Test, Security | `fix/audit-remote-browser` | D4 | a remote HTTP browser downloads/verifies; a non-secure context shows the error |
| P5.2 | 28 | Guard each detail read by the selected id or a per-selection token; add the out-of-order browser test | Implementation, Test | `fix/audit-selection-race` | P5.1 | a late A response cannot replace B; actions use B |
| P5.3 | 29 | Generate the server and client labels from the registry; remove the service timeframe words | Implementation, Test | `fix/audit-labels` | P3.2 | one table; a gate compares the words |
| P5.4 | 30 | Add `simulation create --inputs` and a bundle producer; correct the manual/tutorial; add the parity test | Implementation, Test, Technical writer | `fix/audit-simulation` | P3.2 | both clients create the same inputs digest; the tutorial runs |
| P5.5 | 36 | Fix the stale config detail, the `cfg_abc` defaults, the desktop profile env, the parity release hook, the malformed-token help, the one-call/flag docs, the simulation check output, and extend the parity fixture to all 46 | Implementation, Test, Technical writer | `fix/audit-client-parity` | P5.1-P5.4 | a 46/46 parity test; a profile switch and first submit pass; the docs match |

### P6 — Test and gate hardening

| Work item | Findings | Scope | Roles | Branch | Depends on | Acceptance |
|---|---|---|---|---|---|---|
| P6.1 | 35 | Add reordered-input determinism tests, a checker meta-test, a use-tree boundary, a matrix regeneration check, base-reading null assertions, and the one-vs-N worker digest test | Test, Implementation | `test/audit-evidence` | P2, P4 | each claim names a test that exercises it; a seeded regression fails |
| P6.2 | 35, 37 | Make the port registry cover all eight modules; exclude `#[cfg(test)]` from the emitter checker; add the seeded-failure tests | Test, Implementation | `test/audit-checkers` | P6.1 | a removed marker/emitter fails; the registry and checker agree |
| P6.3 | 15 | Wire `test_worktree_cleanup.py` into the gate/CI | Test, Implementation | `test/audit-worktree-guard` | P0.1 | the gate runs the eight tests; a seeded cleanup regression fails |

### P7 — Closure

| Work item | Findings | Scope | Roles | Branch | Depends on | Acceptance |
|---|---|---|---|---|---|---|
| P7.1 | all | Full gate, web suite, composed+Postgres, port parity, doc checkers, changelog, AI log, audit statuses, worktree/branch cleanup | Reviewer, Release/Operations | merged `main` | P1-P6 | every verification row `Pass` or human `Accepted risk`; no worktree/branch remains |

## 8. Verification

| Gate | Command | Expected |
|---|---|---|
| Fast gate | `tools/gate.sh` | format, clippy, all tests, rustdoc, 25 checkers pass |
| Web | `cd apps/web && bun run check && bun test && bun run build && bun run e2e` | type check, unit, build, Playwright pass |
| Composed + Postgres | `tools/test_tiers.sh composed` with a disposable `QR_TEST_DATABASE_URL` | health, version, two-run digest, eight Postgres suites pass |
| Port parity | `tools/test_tiers.sh parity` | one registered port, fixture match |
| Docs | `python3 tools/check_links.py`; `check_terms.py`; `check_doc_length.py`; `check_workflows.py`; `check_manual.py`; `check_command_surface.py`; `check_labels.py`; `check_execution_plans.py` | all pass |
| Specific fixes | The regression guard named in each work item | fails before, passes after |
| Audit closure | `tools/check_releases.py` + the Improvement Verifier record | every Critical/Major `Verified` or human `Won't fix`; no release with an unverified Critical |
| Cleanup | `python3 tools/worktree_cleanup.py check <worktree>`; `git worktree list`; `git branch -a` | no task worktree or local/remote branch remains |

Limitations: the composed and web tiers need the operator's host/CI; cross-machine
determinism remains reviewer verification; the Postgres suites need a disposable
database; the desktop installer build needs the release runner. A tier that does
not run is `Blocked`, not `Pass`.

## Verification table

| ID | Plan item | Acceptance criterion | Evidence / command | Expected | Observed | Status | Reviewer | Date | Notes |
|---|---|---|---|---|---|---|---|---|---|
| V01 | P0.1 triage | Every finding is re-read against `7ae3a984` and has a work item or a dated `Stale`/`Duplicate` note | audit review; `tools/check_execution_plans.py` | 53 findings mapped; no `Open` Critical/Major without a decision |  | Pending |  |  |  |
| V02 | P1.1 restore | All six schemas are counted, dropped, and restored; a partial target is refused or safely replaced | `tools/test_backup.sh` on a disposable stack; seeded `gateway`/`stage` rows | round trip includes `gateway` and `stage`; no partial destroy | `bash -n` passes; round-trip updated, not run (needs a disposable stack) | Pending |  |  |  |
| V03 | P1.2 partition | A `strategy_dev` manifest with a 2024/2025 config is refused; a contained config passes | `cargo test -p quantrule-evidence-service`; new partition test | non-contained config rejected at create/submit | `cargo test -p quantrule-api`: 76 pass; evidence tests pass; a 2024 config on `model_dev` is refused | Pending |  |  |  |
| V04 | P1.3 holdout | Every `portfolio_holdout` instrument has 2025 coverage, or the uncovered instrument is removed | `data/partitions.json`; data-change record; coverage check | no holdout instrument without 2025 data | `portfolio_holdout` now lists five instruments; COPPER/TSLA removed; the MR7 record has a dated correction; `check_partitions` passes | Pending |  |  |  |
| V05 | P1.4 portfolio | Cash never goes negative; a halted ledger refuses an entry; funding uses the charged price | `cargo test -p quantrule-backtest`; new cash/halt/funding tests | no negative cash; tests pass | `quantrule-backtest`: 210 pass; `quantrule-app`: 164 pass; all-in entry refused; funding mark and halt tests pass; golden short unchanged | Pending |  |  |  |
| V06 | P1.5 cancel | A succeeded run is not overwritten by a concurrent cancel; a second cancel is a conflict | Postgres concurrency test | terminal state monotonic | SQL predicate added; Postgres race test is `#[ignore]` until `QR_TEST_DATABASE_URL`; fast tests pass | Pending |  |  |  |
| V07 | P1.6 image identity | A server without `QR_IMAGE_DIGEST` refuses a run/stage; two builds differ | `cargo test`; two-build fixture | `/health` and provenance agree | server-without-digest test passes; `make up` records the built image id; docs updated | Pending |  |  |  |
| V08 | P2.1 session VWAP | A missing-volume session publishes null; the session event cannot fire | `cargo test -p quantrule-model`; session edge test | no fabricated VWAP | 469 lib + all model suites pass; `a_missing_volume_makes_the_session_vwap_unavailable` covers the session reset; gate 2,322 pass; commit `cb85f495` | Pass | Sequential Quant Validator | 2026-11-17 | not independent |
| V09 | P2.2 divergence | A window above the pivot lookback is refused; the event is prefix-stable | model config test; prefix test | no look-ahead | minor/6 prefix test passes; v4 parser refuses major/6 and minor/3, accepts major/5; gate 2,323 pass; commit `736d781e` | Pass | Sequential Quant Validator | 2026-11-17 | not independent |
| V10 | P2.3 Donchian | `donchian_window = 1` is safe (refused or no panic) | model boundary test | no panic; event correct or absent | `a_one_bar_donchian_window_is_safe_at_bar_zero` passes; no event before bar 1; gate 2,322 pass; commit `671fc9dc` | Pass | Sequential Quant Validator | 2026-11-17 | not independent |
| V11 | P2.4 VWAP slope | The prior index is `bar - window`; the golden is regenerated deliberately | unit test; golden diff | formula matches contract | unit test pins `vwap[3]-vwap[1]`; readings diff is only the 214 `vwap_slope` cells; events diff is only six `signal.vwap_slope` rows; coupled worker oracle `81e73e07…` stable twice; gate 2,322 pass; commit `d12505a2` | Pass | Sequential Quant Validator | 2026-11-17 | not independent |
| V12 | P2.5 structure.scale | A minor-scale config changes the structure output, or the field is removed | model test; registry check | the config affects the documented output |  | Pending |  |  |  |
| V13 | P2.6 registry/ATR | Non-ATR readings publish without ATR; `session_vwap` is registered | model test; `check_registry.py` | registry and availability agree |  | Pending |  |  |  |
| V14 | P2.7 event edge | An event edge test constructs events; the retest event has a golden; the checker rejects a readings-only claim | `check_verification.py`; model tests | every event has real edge evidence |  | Pending |  |  |  |
| V15 | P2.8 identity/digest/promotion | Run record, artifacts, and manifests use one SHA-256; metrics changes the digest; a strategy-only change reuses the model | cross-crate tests; promotion test | identities agree; digest covers metrics; model reuse works |  | Pending |  |  |  |
| V16 | P2.9 strategy/config | A diagnostic event cannot satisfy a rule; a 5m/15m mismatch and v3 are refused at submission | snapshot test; config test | boundary and loader enforced |  | Pending |  |  |  |
| V17 | P2.10 execution path | The stop-entry bracket convention is stated and tested, or evaluated from the next bar | execution test; contract diff | the rule is explicit and the fixture covers it |  | Pending |  |  |  |
| V18 | P2.11 data/events | data-v3 matches the loader; the batch/history includes extended events or is documented base-only | doc check; model test | docs and API agree |  | Pending |  |  |  |
| V19 | P3.1 audit/release closure | Prior and current Critical/Major findings are `Verified` or human `Won't fix`; the release gate fails on an unverified Critical; no push before composed | Improvement Verifier record; `check_releases.py`; release workflow test | audit status current; release gate enforces |  | Pending |  |  |  |
| V20 | P3.2 docs/status/API | The named documents and the tree agree; the stage routes are in the HTTP contract; the index table renders | `check_links.py`; `check_command_surface.py`; `check_doc_length.py` | no current-doc drift |  | Pending |  |  |  |
| V21 | P3.3 AI logs | Every current plan resolves to one AI log or a recorded link | checker; plan/log listing | no orphan plan |  | Pending |  |  |  |
| V22 | P4.1 health | A required-service outage makes `/health` down/`503` or `/ready` down; queue/workers are real | composed kill test; health test | no false healthy |  | Pending |  |  |  |
| V23 | P4.2 queue/concurrency | A closed channel errors; a stage does not block a run; the concurrency knob works | composed test; queue test | no silent drop; isolation works |  | Pending |  |  |  |
| V24 | P4.3 Postgres/retention | The eight Postgres suites run in the tier; retention runs or is documented startup-only | `test_tiers.sh`; long-running retention test | database tested; TTL or docs correct |  | Pending |  |  |  |
| V25 | P4.4 input hardening | A `../`/absolute/dev-zero reference is rejected; a concurrent stage keeps its blob; images/credentials and the error table are correct | upload test; stage concurrency test; `docker compose config` | confinement and hardening hold |  | Pending |  |  |  |
| V26 | P5.1 remote browser | A remote HTTP origin downloads/verifies, or shows a clear secure-context error | Playwright HTTP-origin test | no silent failure |  | Pending |  |  |  |
| V27 | P5.2 selection race | A late A response cannot replace selected B; actions use B | browser test | no wrong-record action |  | Pending |  |  |  |
| V28 | P5.3 labels | Server and client labels come from one table; a gate compares them | `check_labels.py`; artifact test | no word drift |  | Pending |  |  |  |
| V29 | P5.4 simulation | Both clients create the same inputs digest; the documented bundle producer works | shell/browser parity test; tutorial run | simulation create is reproducible |  | Pending |  |  |  |
| V30 | P5.5 client/parity | The 46/46 parity test, stale-state, first-submit, packaged-launch, malformed-token help, and output tests pass | web/shell tests; Playwright | no untested command |  | Pending |  |  |  |
| V31 | P6.1 test/checker | Reordered determinism, seeded checker failures, use-tree boundaries, matrix regeneration, base-reading nulls, one-vs-N digest | gate; seeded tests | every claim is exercised |  | Pending |  |  |  |
| V32 | P6.2 port/emitter | All eight port markers are checked; the emitter checker ignores `#[cfg(test)]` | checker tests | no silent coverage loss |  | Pending |  |  |  |
| V33 | P6.3 worktree test | The gate runs `test_worktree_cleanup.py`; a seeded regression fails | `tools/gate.sh`; seeded test | cleanup guard covered |  | Pending |  |  |  |
| V34 | P7.1 full gate | `tools/gate.sh` passes on the merged tree | `tools/gate.sh` | pass | `tools/gate.sh` passed on the fix branch: 2,325 tests, 17 ignored, 25 checkers | Pending |  |  |  |
| V35 | P7.2 web/composed/parity | The web suite, composed+Postgres, and port parity pass | `bun` suite; `test_tiers.sh composed`; `parity` | pass |  | Pending |  |  |  |
| V36 | P7.3 audit statuses | Every finding is `Verified` or human `Won't fix`; the audit status table is current | audit review; Improvement Verifier record | no unresolved Critical/Major |  | Pending |  |  |  |
| V37 | P7.4 changelog/AI log | The changelog and AI logs record the fixes; no unrelated change | `check_changelog.py`; review | current history |  | Pending |  |  |  |
| V38 | P7.5 cleanup | No task worktree or local/remote branch remains; the `Done` board items remain | `git worktree list`; `git branch -a`; cleanup check | clean |  | Pending |  |  |  |
| V39 | P7.6 release | If a release is cut, it has the full gate, audit closure, changelog, note, and human authorization | release checklist | no release with an unverified Critical |  | Pending |  |  |  |

## 10. Traceability

| Work item | Findings |
|---|---|
| P0.1 | 01–53 (triage) |
| P1.1 | 18 |
| P1.2 | 19 |
| P1.3 | 26 |
| P1.4 | 20, 34 |
| P1.5 | 38 |
| P1.6 | 04 |
| P2.1 | 45 |
| P2.2 | 46 |
| P2.3 | 47 |
| P2.4 | 49 |
| P2.5 | 48 |
| P2.6 | 50, 51 |
| P2.7 | 17, 24 |
| P2.8 | 21, 22, 23 |
| P2.9 | 32, 33 |
| P2.10 | 31 |
| P2.11 | 52, 53 |
| P3.1 | 05, 06, 25 |
| P3.2 | 01, 02, 03, 07, 08, 12, 14, 43 |
| P3.3 | 16 |
| P4.1 | 39 |
| P4.2 | 40, 41 |
| P4.3 | 09, 42 |
| P4.4 | 10, 11, 13, 44 |
| P5.1 | 27 |
| P5.2 | 28 |
| P5.3 | 29 |
| P5.4 | 30 |
| P5.5 | 36 |
| P6.1 | 35 |
| P6.2 | 35, 37 |
| P6.3 | 15 |
| P7.1 | all closure |

All 53 findings are covered exactly once or in the P0.1 triage. A finding that
P0.1 marks `Stale`/`Duplicate` is removed from the fix row with a dated note;
the work item remains for the surviving findings.

## 11. Risks and recovery

- **Model and accounting changes invalidate results.** The portfolio cash fix,
  session VWAP, divergence window, Donchian guard, VWAP slope, and structure
  scale can change published values. A Quant Validator reproduces the old and
  new behavior; an affected baseline is recorded in
  [`docs/improvement/INVALIDATIONS.md`](../../improvement/INVALIDATIONS.md) and
  re-run or retired before the next claim. The shipped `verify-mini` portfolio
  is all-in (10,000 cash, 10,000 fixed notional); the composed tier must re-size
  it to a funded notional and record the new canonical digest in the next
  release note. The golden short fixture is unchanged.
- **The partition fix rejects existing records.** P1.2 adds a preflight check;
  existing research records are not deleted. A record outside its declared
  partition is marked `Stale` with a dated note; no new validation access is
  consumed.
- **The restore test is destructive.** It runs only on a disposable Compose
  project. No production volume is touched. The test refuses a non-test project
  name.
- **The Postgres tier adds infrastructure.** It is a release-tier requirement,
  not a fast-tier requirement. A missing database is `Blocked`, not `Pass`.
- **The health aggregation changes readiness.** A required-service outage
  should fail `/health`; the deployment smoke must start the full stack. The
  change is tested with a killed service.
- **The queue confirms and concurrency change throughput.** A publisher
  timeout returns `503`; the client may resubmit. The composed timing test
  records the new behavior.
- **The remote browser fix is a security/UX trade-off.** A JS hash fallback is
  code in the client; HTTPS is the deployment rule. The human chooses D4.
- **The audit closure needs an independent verifier.** The implementer cannot
  mark its own finding `Verified`; if no independent agent is available, the
  pass is disclosed and the finding stays `Fixed`, not `Verified`.
- **The configuration upload fix may reject a repository config.** The
  repository configs use relative file names for `run.json`; the check is
  applied only to uploaded bundles and the worker's materialized paths. A
  false rejection is a stop condition.
- **Rollback.** Each fix is one branch and one PR; a failed fix is reverted by
  its PR, not by a schema rollback. Migrations are forward-only; a schema change
  needs its own recovery note. The canonical digest and existing evidence are
  not rewritten by a code fix.

## 12. Stop conditions

- A finding does not reproduce on `7ae3a984`; mark it `Stale` with a dated note
  and do not implement it.
- A fix changes a model or strategy baseline, a signal value, or an accepted
  evidence run; stop and route to the Quant Validator/Improvement Workflow.
- A fix requires a new dependency, a schema change, a new service, or an
  architecture decision; stop for human approval.
- A test cannot be written or a tier cannot run; record `Blocked` and the
  limitation; the plan is not `Verified`.
- A release, tag, merge, board change, or destructive Git operation is needed;
  stop for the separate approval gate.
- A finding is proposed as `Won't fix`; only the human can decide.
- The gate, web, composed, or port parity tier fails on a clean tree; stop and
  fix the cause before adding the next work item.

## 13. Approval

This plan was approved by the human on 2026-11-17 (D1–D8 and the order). The
plan is `In Progress`; the first work items are P1.1, P1.2, P1.5, and P1.6. On
approval:

1. The Planner marks the plan `Approved`, creates the fix issues/sub-issues,
   and moves the audit findings to `Planned` (or `Stale`/`Duplicate` with a
   dated note).
2. The Implementer creates one branch/worktree per issue, starting with P1.1.
3. The Reviewer and Quant Validator verify each fix; the Improvement Verifier
   closes the audit statuses in P7.
4. The plan moves to `In Progress` on the first issue, `In Review` when the
   first PR is open, and `Verified` only when every row is `Pass` or the human
   records an `Accepted risk`.

No code, test, config, schema, issue, or board change starts before that
approval.
