# REM-RA-007 Quant Validation — Run v5 and Research Artifacts

- Date: 2026-11-26 (UTC)
- Status: Independent child agent session; same model family, fresh context; not a human reviewer and not an independent model judgment.
- Re-validation: 2026-11-26 (UTC), base commit `8b54e62d`; the §7 verdict remains the historical result for `d19e6047`. The new verdict is in the Re-validation section.
- Work item: [#1210](https://github.com/quant-rule/quant-rule/issues/1210) (`REM-RA-007`), parent `REM-RA` / `REM-000`
- Claim type: Behaviour Claim validation record; no edge claim
- Plan: [`rem-000-v1.6.1-remediation.md`](../../execution-plans/2026-11-26/rem-000-v1.6.1-remediation.md) (`RA-007`, `V-RA-07`)
- ADR: [`1204-run-v5-resolution.md`](../../adr/2026-11-26/1204-run-v5-resolution.md)
- Contracts: [`research-artifact-packs.md`](../../config/2026-11-22/research-artifact-packs.md), [`signal-artifact.md`](../../config/2026-11-22/signal-artifact.md), [`research-artifacts.md`](../../architecture/research-artifacts.md)
- Evidence boundary: no edge, profitability, holdout, composed-stack, or live-Postgres claim is made. No live Postgres, Docker stack, full gate, soak, upgrade, lab, or release run was performed.

## 1. Scope and claims checked

The validation reproduces the run-v5 and research-artifact path that landed in
`RA-002` through `RA-006`:

1. Strategy, portfolio, and execution pack parsing, canonical bytes, content id,
   wrong-kind and missing-hash rejection, and the freeze/retire lifecycle.
2. Signal artifact canonical identity, causality and availability, determinism,
   reuse under execution and portfolio pack changes, and the closed field
   surface.
3. Run-v5 loading with `MemoryReferences`, missing and mismatched reference
   refusal, fold-mode versus signal-mode rules, model-artifact reuse identity,
   and the full fold path.
4. The v5 worker tests, the app end-to-end test, the checked-in golden digest,
   and app-fold versus worker-fold agreement.
5. Lineage submission atomicity by code inspection, idempotent same set,
   different-set refusal, the five reference kinds, and the public/internal read
   shape.
6. No partial publish on the missing-pack v5 case.
7. The v4 regression and the coupled baseline.
8. A documentation spot-check for untested paths, edge claims, or partial
   release claims.

Not checked: live Postgres, the composed service stack, the gateway HTTP path,
a signal-mode v5 end-to-end execution, a v5 promoted-model worker execution, a
holdout or strategy result, a multi-worker HTTP run, release signing, or any
market-data claim.

## 2. Inputs and code state

| Item | Value |
|---|---|
| Worktree | `/home/zantgo/Proyectos/quant-rule--1210-quant-validation` |
| Branch | `docs/1210-quant-validation` |
| Base commit | `d19e6047` (merge of PR #1237; `main` at the same commit) |
| Working tree | Clean before this report; only this report was added by the validator |
| Toolchain | `cargo 1.99.1 (5f94df478 2026-08-27)`, `rustc 1.99.1 (58cf8f9c3 2026-10-13)` |
| Build | `CARGO_BUILD_JOBS=2`; `target/debug` |
| Fixtures | `crates/quantrule-app/tests/fixtures/run-v5/`, `crates/quantrule-app/tests/fixtures/signal/model/`, `config/examples/verify-v5/` |
| Network / Docker | None; the full gate and Docker were deliberately not run |
| Seed / randomness | None; no wall-clock value enters a signal or digest check |

## 3. Observed results

All commands below were actually run in the worktree. The counts are the
observed `cargo` output, not the implementer's claims.

| Command | Observed result |
|---|---|
| `CARGO_BUILD_JOBS=2 cargo test -p quantrule-app --test run_v5_e2e` | exit 0; **2 passed**, 0 failed (0.21 s) |
| `CARGO_BUILD_JOBS=2 cargo test -p quantrule-backtest-service --test worker` | exit 0; **48 passed**, 0 failed (6.03 s) |
| `CARGO_BUILD_JOBS=2 cargo test -p quantrule-model-service` | exit 0; **32 passed**, 0 failed across 13 lib + 4 bundle + 5 events + 2 health + 1 isolation + 7 model; two empty targets (`qr_model` bin, doc-tests) |
| `CARGO_BUILD_JOBS=2 cargo test -p quantrule-app --lib pack::` | exit 0; **119 passed**, 0 failed (229 filtered) |
| `CARGO_BUILD_JOBS=2 cargo test -p quantrule-app --test signal_artifact_public` | exit 0; **6 passed**, 0 failed |
| `CARGO_BUILD_JOBS=2 cargo test -p quantrule-evidence-service --lib` | exit 0; **82 passed**, 0 failed |
| `CARGO_BUILD_JOBS=2 cargo run --quiet -p quantrule-governance -- check --all` | exit 0; `governance check ok` |
| `CARGO_BUILD_JOBS=2 cargo test -p quantrule-app --lib` | exit 0; **348 passed**, 0 failed |
| `CARGO_BUILD_JOBS=2 cargo test -p quantrule-app --test run_v5_public` | exit 0; **4 passed**, 0 failed |
| `CARGO_BUILD_JOBS=2 cargo test -p quantrule-backtest-service --test run_v5_identity` | exit 0; **2 passed**, 0 failed |
| `CARGO_BUILD_JOBS=2 cargo test -p quantrule-evidence-service --test lineage` | exit 0; **5 passed, 1 ignored** (Postgres; no `QR_TEST_DATABASE_URL`) |
| `CARGO_BUILD_JOBS=2 cargo test -p quantrule-app --test strategy_pack_identity --test execution_pack_identity --test portfolio_pack_identity` | exit 0; **17 passed** (5 + 6 + 6), 0 failed |
| `CARGO_BUILD_JOBS=2 cargo test -p quantrule-data-service --test configs` | exit 0; **9 passed**, 0 failed |
| `CARGO_BUILD_JOBS=2 cargo test -p quantrule-data-service --test internal` | exit 0; **7 passed**, 0 failed |

No command failed. The pack filter is a subset of the 348 app-lib tests; the
counts are per command and are not additive. I re-ran the governance check after
writing this report; it returned `governance check ok` and exit 0.

### 3.1 Pack identity, canonical bytes, and lifecycle

- `cargo test -p quantrule-app --lib pack::` passed 119 tests covering the
  envelope, canonical JSON, duplicate labels, every lifecycle transition, and
  the strategy, portfolio, and execution payload parsers.
- The public identity suites passed 17 tests. They assert literal canonical
  hashes for all three kinds (strategy `sha256:dcbe8d…20ad6`, execution
  `sha256:b526f41…469be`, portfolio `sha256:4cae9bd…9c0c9`), reject key-order
  changes, reject a label reused with different bytes, keep the id and bytes
  across freeze and retire, and reject a wrong kind.
- Canonicalisation is once at creation; `from_stored_bytes` re-identifies the
  stored bytes and rejects a non-canonical spelling. The pack parsers are
  closed; unknown fields are refused.
- The freeze and retire state machine is correct in the parser and registry:
  a frozen pack is immutable, a retired pack is terminal, and the registry
  routes refuse an unlawful transition. **However, a new run does not consult
  the lifecycle state.** This is Defect D1 below, not a passing claim.

### 3.2 Signal artifact

- The public signal suite passed 6 tests: the golden identity
  `sha256:2551537003307cd1c2b324e3eb006c1a7f14abef6ede728f0cf9510ffaaf7689`,
  the expected rows, byte determinism, a truncated model producing the same
  earlier rows, an unchanged final bar producing the same earlier rows, reuse
  of the same bytes under two different execution packs and two different
  portfolio packs, fold-pack refusal, and mismatched model-artifact refusal.
- The closed parser allows only the documented top-level, row, intent, and
  evidence fields. A fill, quantity, cash, equity, P&L, return, confidence,
  statistical, or edge field has no field to travel in; the public test also
  scans the canonical text for forbidden names.
- `ModelBundle::input`/`events_for` filters events by
  `available_bar_index <= execution_bar` and by the event window, and the row
  writer sets `available_bar_index = bar_index`. A later bar cannot change an
  earlier row; the truncation and changed-final-bar tests prove the direct
  causality requirement.
- The computation takes the model bundle, the strategy pack, and the model id.
  It takes no execution pack, no portfolio pack, no clock, and no random source.
  The reuse test is a byte-level proof; it is not a second end-to-end run under
  each policy.

### 3.3 Run v5 loader and worker

- `cargo test -p quantrule-app --lib` passed 348 tests, including the v5 loader
  cases: a fold run materializes every pack, a signal run carries the signal
  artifact, a structural load returns the reference set and hash, a missing
  reference names the field, an id must equal its hash, stored bytes must match
  the hash, a wrong-kind pack is rejected, signal mode requires a signal
  artifact, fold mode rejects one, the signal artifact must name the strategy
  pack, model artifact, and execution timeframe, signal mode requires a model
  artifact, the model artifact must match the model version and run ladder, the
  execution timeframe must match the strategy, the universe must contain the
  instrument, the account stays in the run, legacy fields are rejected, and the
  legacy config hashes are unchanged.
- `run_v5_public` passed 4 tests; `run_v5_identity` passed 2 tests. Every pack
  and artifact hash changes the config hash and therefore the derived `RunId`;
  the signal artifact changes the identity; a v3/v4 config keeps its hash.
- The worker suite passed 48 tests, including the three v5 cases:
  a full fold run resolves the packs, publishes artifacts, and records a
  digest; a missing strategy pack fails closed with no digest, no artifacts, and
  no run manifest; one worker and two concurrent workers produce the same
  digest with one model call.
- The app `run_v5_e2e` passed 2 tests: the fold matches the checked-in golden
  digest, and one fold and two concurrent folds produce the same digest.
- Model service: 32 tests passed, including the v5 loader/resolver cases. The
  service refuses a promoted model artifact reference (the worker consumes that
  bundle directly), resolves the pack and signal references once, verifies the
  hashes, and loads the run with `RunConfig::load_with`.
- The loader and worker fail closed on a missing or hash-mismatched reference.
  The semantic cross-checks (signal `strategy_pack_id`, `model_artifact_id`,
  timeframe; fold/signal presence) are enforced by `RunConfig::load_with` in
  the worker/model service, not by run submission. See G2 in section 4.

### 3.4 Golden digest and app-versus-worker agreement

The checked-in golden is
`sha256:4d5f6a8ad385989d8a6c335bb1bd66620bd7bb3fd5a3505438d7bd6cab256b79`.
I reconstructed it independently from the emitted bundle, not by repeating the
test assertion:

1. I ran the app test binary with a syscall delay on deletion and copied the
   emitted `one/backtest` bundle before its temporary directory was removed.
   This is a verification harness only; it does not alter the repository.
2. The canonical set contained 13 files:
   `execution/fills.csv`, `execution/orders.csv`, `execution/rejections.csv`,
   `model/events.csv`, `model/legs.csv`, `model/levels.csv`,
   `model/readings.csv`, `portfolio/equity.csv`, `portfolio/funding.csv`,
   `portfolio/metrics.json`, `portfolio/risk_events.csv`,
   `portfolio/trades.csv`, `strategy/decisions.ndjson`.
3. I hashed each file with `sha256sum`, built the sorted
   `path NUL sha256:<hex> newline` byte stream, and hashed that stream. The
   result is the golden value above, byte for byte.
4. I ran the worker v5 test the same way. Its canonical set is the same 13
   paths; every canonical file is byte-identical to the app bundle; the
   computed digest is the same `sha256:4d5f…b79`. The worker test itself does
   not assert the checked-in golden, so this capture is the direct
   app-versus-worker agreement evidence.
5. For contrast, omitting `portfolio/metrics.json` gives
   `sha256:8fa9d752d4e6faffaaa1d6f44656a7d5ed9a864576d71b49d6811f85dcf3f3a5`.
   This is the difference behind documentation Defect D3.

The digest reconstruction is one fixture. It proves that for the tested
250-bar fold the app and worker paths agree; it is not a proof for every
possible run.

### 3.5 Lineage

- The evidence library passed 82 tests. The lineage integration test passed 5
  tests and ignored 1 Postgres test because no live database was supplied.
- The memory route test records a set, reads it back through both
  `GET /internal/v1/runs/{id}/lineage` and the public
  `GET /runs/{id}/lineage`, repeats the same set idempotently, and receives
  `409 run.conflict` for a different set. A submission writes the lineage that
  the public read returns. An unknown run and an unrecorded lineage are clean
  404s. A malformed set is refused.
- By inspection of `PgRunStore::submit`, the run row, the lineage header, the
  lineage references, and the `run.requested` outbox intent commit in one
  Postgres transaction. `PgLineage::record` writes the header and references in
  one transaction and compares a repeat. **No live Postgres run is claimed.**
- The memory `MemoryRuns::submit` inserts the run under the run lock and then
  records lineage under the lineage lock; it is not one transaction like the
  Postgres path. It is the test/local fake, not evidence of Postgres
  atomicity.
- `ReferenceKind` defines the five kinds (model artifact, strategy pack,
  signal artifact, portfolio policy, execution model). The route test uses two
  references and the promoted-v5 test uses four; a five-reference signal-mode
  route round-trip is not directly exercised. The type and the loader cover the
  fifth kind.
- The public and internal read shapes are the same `RunLineage` object
  (`run_id`, ordered `references`, `recorded_at`); the route test compares them
  byte for byte.

### 3.6 No partial publish and v4 regression

- The worker missing-pack v5 test passed: the run is `failed`, the digest is
  `none`, the artifact list is empty, and no run manifest object exists. The
  worker resolves the references before the fold and publishes nothing on the
  failure path. The test does not assert that the model service was not read;
  it asserts the run artifact set is absent.
- The v4 worker regression passed inside the 48-test worker suite:
  `a_queued_run_matches_the_coupled_baseline` asserts the coupled baseline
  `sha256:32dc2c72bfa54569067b0beedacd2c97e06d87532da6f32930f068732e73e26f`
  and the worker digest equals it. The 348-test app library includes the legacy
  config-hash golden test. No v4 behavior change was observed.

### 3.7 Documentation spot-check

- `qr-governance check --all` passed.
- The current docs do not claim an edge, a profitability result, a holdout
  result, or a completed v1.6.1 release. There is no `v1.6.1` release note;
  the `v1.6.0` note is historical and keeps its published boundary.
- Two current claims do not match the observed implementation: the upload-time
  reference resolution claim (D2) and the canonical-digest set in the manual
  (D3). The lifecycle contract text is correct, but the new-run path does not
  enforce it (D1).

## 4. Defects and gaps

| ID | Severity | Finding | Status |
|---|---|---|---|
| D1 | Major | A new run may cite a `draft`, `validated`, or `retired` pack; the frozen-pack new-run gate is not enforced. | Fixed by #1239 |
| D2 | Moderate | The docs and ADR say the data service validates every v5 reference at upload; the code marks a v5 config valid without resolving packs/signal and resolves them later at submission. | Fixed by #1239 |
| D3 | Moderate | The manual's canonical digest set omits every `metrics.json`; the service includes them, and the checked-in golden depends on one. | Fixed by #1240 |
| G1 | Gap | No fast test exercises the production data-service `config_references` missing/mismatch path at submission; no composed stack or live Postgres. | Open |
| G2 | Gap | No signal-mode v5 end-to-end execution and no v5 promoted-model worker end-to-end test; the fast v5 folds are fold mode. | Open |
| G3 | Gap | No five-reference lineage route round-trip; the route tests use two and four references. | Open |

### D1 — New-run pack lifecycle is not enforced

Evidence:

- `crates/quantrule-api/src/pack.rs:114-120`: `PackState::allows_new_runs`
  returns true only for `Frozen`, and the doc comment says only a frozen pack is
  usable for a new run.
- `crates/quantrule-data-service/src/configs/references.rs:68-74` records the
  registry state as `PackProvenance.state` but does not reject a non-frozen
  state.
- `crates/quantrule-evidence-service/src/lineage.rs:74-107` (`normalize`)
  checks id/hash consistency, pack-kind provenance, duplicate kinds, and
  non-empty labels; it never checks `pack.state`.
- `crates/quantrule-app/src/pack/core.rs:33-42` starts every parsed pack in
  `PackState::Draft`; the run loader parses bytes only and has no registry
  state. `crates/quantrule-backtest-service/src/backtest/resolve.rs:22-105`
  resolves pack bytes by id/hash without a state check.
- The v5 public loader test (`crates/quantrule-app/tests/run_v5_public.rs:113-139`
  defines the pack document; `:244-251` loads it) succeeds with a pack document
  that the loader parses as `Draft`. The app e2e and worker v5 fixtures use
  canonical pack documents that were never frozen.
- `tools/verify.sh:307-330` (`prepare_v5`) creates the three composed v5 packs
  but never calls `pack validate` or `pack freeze`; the pack records stay
  `draft`. The comment at `tools/verify.sh:309-311` says the data service
  validates the run against the registered packs at upload, but the upload
  route does not resolve the packs (see D2), and no layer checks their state.

Impact: the freeze decision is a human gate backed by a Quant Validator record;
the run path bypasses it. A retired pack can re-enter use in a new run, which
the prior RA validation listed as an invalidation condition. A run's lineage
records the non-frozen state, so the record is honest, but the run and its
evidence are still produced.

Recommended remediation (not implemented by this validator): enforce
`allows_new_runs` for a newly created v5 run at submission (allow an existing
run to read its lineage after retirement), and make the composed v5 fixture
freeze the packs it registers. Add a regression test that a draft, validated,
and retired pack are each refused for a new run and accepted only by an
existing run's read. This is a behaviour change and requires the owning
workflow and human approval.

### D2 — Upload-time reference validation claim

Evidence:

- `docs/architecture/research-artifacts.md:82-85` says "The data service
  resolves every data-owned reference at upload".
- `docs/config/2026-11-22/research-artifact-packs.md:250-257` says "A v5 config
  is validated through the reference resolver at upload (`RunConfig::load_with`);
  the data service verifies the pack and signal bytes".
- `crates/quantrule-data-service/src/configs/upload.rs:60-64` builds the record
  through `scan::record`; for a v5 document `crates/quantrule-data-service/src/configs/scan.rs:92`
  calls `RunConfig::load_v5_references`, which does not take a resolver and does
  not read pack or signal bytes; it marks the record `valid: true`.
- `crates/quantrule-data-service/src/routes.rs:309-320` stores the record
  without calling `config_references`. The resolution happens on the internal
  read at `crates/quantrule-data-service/src/routes.rs:384-385`, which is the
  submission/worker path.
- `tools/verify.sh:309-311` repeats the upload-time claim in a comment.

Impact: a v5 config can appear `valid` while a referenced pack or signal object
is absent; the refusal happens when evidence fetches the bundle at submission,
and the client sees a config/service error rather than an upload-time rejection.
The observable run-row safety is still preserved (the submission fails before
`runs.submit`), so this is a contract/documentation mismatch and a UX gap, not a
data-integrity defect. The plan's `V-RA-02` says submission; the ADR says upload;
the two documents conflict.

Recommended remediation: choose one contract and apply it. Either move the
resolution into the upload route and make `valid` mean resolved, or amend the
ADR/architecture/contract/script comment to say resolution happens at
submission. Add a test with a registered pack whose object is missing and a test
with a missing pack reference.

### D3 — Canonical digest documentation omits `metrics.json`

Evidence:

- `docs/manual/06-backtest.md:87` says the canonical digest covers
  `results.json`, `*.csv`, and `*.ndjson`; `:90-94` says "the JSON reports are
  not canonical".
- `crates/quantrule-service-api/src/manifest.rs:169-190` includes every
  `metrics.json` (basename match) in `canonical_digest`.
- `CHANGELOG.md:400` records that the canonical digest covers every
  `metrics.json`.
- The captured v5 bundle contains `portfolio/metrics.json`; the product digest
  is `sha256:4d5f…b79` with it and `sha256:8fa9…3a5` without it. The
  checked-in golden is the former, so the manual's set produces a different
  identity.
- `AGENTS.md` section 12 has the same omission as the manual.

Impact: a reviewer following the manual will compute a different digest for a
correct run. This is a documentation defect, not a code defect; the code and
the changelog agree.

Recommended remediation: update the manual and the AGENTS.md claim boundary to
include every `metrics.json`, or make the code exclude it. The changelog records
the inclusion as intentional, so the documentation update is the smaller
change. This is a human-owned contract edit.

### Gaps

- **G1:** The fast tests use `MemoryDataClient` and `MemoryReferences`; the real
  data-service `config_references` path (registry lookup, object hash, missing
  pack, missing signal, pack provenance) has no direct fast test. The composed
  tier is not run. The workspace was deliberately not started.
- **G2:** The app e2e and worker v5 tests are fold mode. The signal-mode loader
  is tested, but no signal-mode v5 run executes through the worker. The worker
  v5 fixture uses a model bundle derived from the v4 config, not a promoted v5
  model stage. The model service resolves a v5 set, but its tests are not a
  gateway run.
- **G3:** The lineage route test uses two references; the promoted-v5 test uses
  four. The five-kind type and the `RunReferences::each` order are inspected,
  but a five-reference route round-trip is not directly tested.
- **G4:** No live Postgres. `the_postgres_lineage_store_conforms` is ignored
  without `QR_TEST_DATABASE_URL`. The Postgres code is inspected only; the
  memory submit is not one transaction. The composed tier remains the evidence
  for the Postgres transaction and crash/retry behavior.
- **G5:** No gateway HTTP submit/fetch round trip in this validation; the app
  e2e is an application-service call, and the worker test uses in-process fakes.

## 5. Limitations

- This is one validator pass by a child agent; it is not a human review, not an
  independent model judgment, and not a self-approval.
- The artifact capture used a syscall delay and a copy loop to preserve the
  test's temporary bundle. It is a verification harness, not a product path or
  a change to the test.
- The digest agreement is demonstrated for one 250-bar fold fixture. It does not
  prove every future run, and it does not replace the composed tier.
- No edge, profitability, holdout, strategy-baseline, composed-stack,
  live-Postgres, HA, soak, upgrade, signature, or release claim is made.
- The `cargo` commands were fast-tier tests in the configured worktree. The
  full gate, Docker stack, and multi-host tiers were not run.
- The model-service count includes two empty test targets; the meaningful
  suites are the 13 lib tests plus the four integration targets listed in
  section 3.

## 6. Remediation update (2026-11-26)

- D1 and D2 are fixed by #1239: the data service resolves every data-owned
  reference at upload and refuses a non-frozen pack; run submission re-checks
  `allows_new_runs`; the composed v5 fixture validates and freezes its packs.
  Regression tests cover draft, validated and retired packs, a missing pack, a
  missing pack object, and the no-run-row result.
- D3 is fixed by #1240: `AGENTS.md` section 12, the manual chapter 6 table and
  prose, the glossary, and the ADR addendum now include every `metrics.json`.
  The code and the checked-in golden are unchanged.
- The verdict below remains the historical result for commit `d19e6047`. A fresh
  independent pass is required before `V-RA-07` moves to `Pass`.

## 7. Verdict

**Fail — remediation required before `V-RA-07` can be marked `Pass`.** The
core run-v5 mechanics hold in the fast evidence: pack canonical identity,
signal-artifact identity, causality, determinism, reuse and closed fields; the
loader's hash/kind/mode checks; the worker's one-resolution fold; the
independently reconstructed golden digest; app-fold and worker-fold agreement;
the v4 coupled baseline; the memory lineage idempotence and conflict rules;
and the missing-pack no-partial-publish case. The new-run path does not enforce
the frozen/retired pack lifecycle (D1), so the pack lifecycle claim is not met;
the upload-time validation claim (D2) and the canonical-digest documentation
(D3) also fail their stated contracts. The composed tier remains for `G1`, `G2`,
and `G4` after the lifecycle fix. No code, test, config, contract, or script was
modified; only this report was written.

## Re-validation (2026-11-26)

- Base commit: `8b54e62d` (merge of PR #1242; `main` at the same commit).
- Worktree: `/home/zantgo/Proyectos/quant-rule--1243-revalidation`; branch `docs/1243-revalidation`.
- Toolchain: `cargo 1.99.1 (5f94df478 2026-08-27)`, `rustc 1.99.1 (58cf8f9c3 2026-10-13)`.
- Independence: a separate child-agent session with the same model family and a
  fresh context. This is not a human reviewer and not an independent model
  judgment; it is a second separate pass, not a self-approval.
- Scope: D1, D2 and D3 from the original record, the core v5 evidence, and a
  new Critical/Major scan of the #1239/#1240 fix.
- Evidence boundary: no edge, profitability, holdout, composed-stack,
  live-Postgres, HA, soak, upgrade, signature or release claim is made. Docker,
  the composed tier, the full gate and a live database were not run.
- Change boundary: no code, test, config, contract or script was modified; only
  this report was written.

### Verdict for `8b54e62d` plus this record

**Pass — D1, D2 and D3 are `Verified`; no new Critical or Major finding was
observed.** The §7 `Fail` remains the historical result for `d19e6047` and is
not rewritten. The new verdict applies to `8b54e62d` plus this record. The Minor
D4 wording was corrected after this pass in the same PR; the fast-test gaps
(G6–G8) remain, and the prior gaps G1–G5 also remain open. The fix is a behaviour
gate, not an edge, profit or release claim. This section supports `V-RA-07`
`Pass`; the plan row is updated by the implementer after this record.

### Findings

| ID | Severity | Original finding | Re-validation status | Evidence |
|---|---|---|---|---|
| D1 | Major | A new run may cite a `draft`, `validated` or `retired` pack; the frozen-pack new-run gate is not enforced. | **Verified** | Upload resolves and refuses non-frozen packs; submission re-checks before the run row; the lineage read is not gated. Tests below. |
| D2 | Moderate | The docs/ADR claimed upload-time resolution; the upload marked a v5 config valid without resolving packs or signal. | **Verified** for the named gap | `upload_config` calls `config_references` before the idempotent lookup and before storing; the docs/ADR describe resolution at upload. A Minor wording precision remains (D4). |
| D3 | Moderate | The manual's canonical digest set omitted every `metrics.json`; the service includes them and the golden depends on one. | **Verified** | `AGENTS.md` §12, manual chapter 6, the glossary and the ADR addendum include every `metrics.json`; code and golden unchanged; the independently reconstructed digest matches. |

### D1 — frozen pack lifecycle

Upload path:

- `crates/quantrule-data-service/src/routes.rs:309-328`: for a valid record,
  `config_references` runs before `store.get` and `store.upsert`; a pack whose
  state does not `allows_new_runs` returns `pack.invalid`, so no record is
  written.
- `crates/quantrule-data-service/src/configs/references.rs:19-122`: every pack
  is read from the registry and object store; the object bytes are checked
  against the registry hash and length; a signal artifact is resolved the same
  way. A missing pack is `pack.not_found`, a missing object is `internal`, and a
  non-frozen pack is `pack.invalid`.
- `configs_v5` passed 4 tests: a draft pack refused with no upload; a frozen set
  accepted; the same payload re-uploaded after retirement refused with
  `pack.invalid`; a missing pack refused with 404; a missing pack object refused
  with 500; the internal bundle returns the three references with frozen
  provenance.
- The retired re-upload is the duplicate-bypass regression: the check runs
  before the idempotent lookup, so an existing valid record does not bypass it.

Submission path:

- `crates/quantrule-evidence-service/src/routes/runs.rs:45-68` defines
  `require_frozen_packs`; `:176` calls it before `derive` and before
  `:281-308` `runs.submit`. A missing provenance, draft, validated or retired
  pack returns `pack.invalid`.
- `promoted_runs::a_v5_run_refuses_a_non_frozen_pack` passed for `draft`,
  `validated` and `retired`; each returned 400 `pack.invalid`, and `GET /runs`
  was empty, so no run row was written.
- `research create` uses the same `submit_validated_pipeline` helper, so the
  same check covers the research route.
- The signal path is not a bypass: the strategy service requires a frozen
  signal-mode strategy pack before it computes
  (`crates/quantrule-strategy-service/src/resolve.rs:144-153`).
- The worker does not re-check the pack state; the lifecycle boundary is the
  run row. A pack retired after the row exists belongs to an existing run, which
  the lifecycle permits. The window in G8 is the only gap.

Lineage read:

- `crates/quantrule-evidence-service/src/routes/lineage.rs:95-114`: the read
  checks the run is visible and the lineage store; it never reads the pack
  registry or the pack state. An existing run's lineage is therefore not gated
  by a later retirement.
- Not directly tested: submit a run while frozen, retire the pack, then read the
  same run's lineage. The code path is independent of pack state, but the
  regression test named in the prior recommendation is absent (G6).

Composed fixture:

- `tools/verify.sh:312-348` validates and freezes each v5 pack before uploading
  the rewritten config; `bash -n tools/verify.sh` reports no syntax error. The
  composed tier was not run (G1/G2).

Residual D1 risks:

- The frozen check is a read in the data/evidence services followed by a write
  in evidence. A pack retired between the read and the run-row write can still
  be cited; no lock or lease spans the two services. This is a narrow
  cross-service TOCTOU, not the original unconditional gap, and it is untested
  (G8).
- The data-service tests exercise `draft` and `retired` at upload. The
  `validated`-at-upload case is covered by the same `allows_new_runs` predicate
  and by the evidence submission test, but not by a direct upload test. A
  missing signal artifact is handled by `config_references` but not covered by
  `configs_v5`. The plan's `V-RA-08` observed text describes the data-service
  upload cases as including `validated`, and the #1239 AI log says the upload
  refuses a missing signal; the named data-service tests do not exercise either
  case. The behaviour is not disproved, but the record is broader than the test
  (G6).

### D2 — upload-time resolution

- Before #1239, `upload_config` built the record and stored it without calling
  `config_references`; the v5 scan marked it valid from `load_v5_references`
  alone. That old gap is gone: `routes.rs:309-328` resolves every data-owned
  reference and returns an error before `store.upsert` when a pack, signal
  artifact, object or frozen state is missing.
- The docs now describe the same behaviour:
  `docs/architecture/research-artifacts.md:82-87` and
  `docs/config/2026-11-22/research-artifact-packs.md:240-244` say the data
  service resolves every data-owned reference at upload and that upload and
  submission refuse a draft, validated or retired pack;
  `docs/manual/06-backtest.md:50-54` says the same for upload and `run submit`;
  `docs/manual/09-command-reference.md:47-50` says `run submit` refuses a
  missing, mismatched or non-frozen pack. ADR decision 3
  (`docs/adr/2026-11-26/1204-run-v5-resolution.md:42-45`) says the data service
  parses, resolves and verifies each id and hash before it marks the bundle
  valid; that is the implemented behaviour.
- The comment at `tools/verify.sh:309-311` now matches the resolved upload; the
  script also freezes the packs before the upload.
- D4 (Minor, fixed in this PR): `docs/config/2026-11-22/research-artifact-packs.md`
  said the upload validates through `RunConfig::load_with`. The upload route uses
  `parse_v5_references` plus `config_references`, not the full loader; it checks
  existence, state, object hash and length, while pack-kind and signal
  cross-fields remain at submission and the loader. The paragraph now names the
  upload path and the full loader's location. This is a documentation precision
  issue, not a reopened D2.
- G7: a v5 config discovered by the read-only mount (`sync_configs`/`scan`) is
  still marked `valid: true` without resolving its packs or signal
  (`crates/quantrule-data-service/src/configs/scan.rs:91-107`;
  `routes.rs:344-362`). Submission's `internal_config` resolves the set and
  refuses a missing or non-frozen pack before a run row, so the run-safety
  property holds. The `valid` flag for repository origin is not the same as
  upload validity; the docs should say so or the scan should resolve. This is
  pre-existing, not introduced by #1239.
- The direct duplicate-bypass evidence is the `configs_v5` retired re-upload;
  the check precedes `store.get`.

### D3 — canonical digest documentation

- `AGENTS.md` §12 line 1123, `docs/manual/06-backtest.md:88-94`,
  `docs/manual/11-glossary.md:18`, and the dated addendum at
  `docs/adr/2026-11-12/07-canonical-digest-and-claim-boundary.md:59-63` all list
  `results.json`, `*.csv`, every `metrics.json`, and `*.ndjson`.
- `crates/quantrule-service-api/src/manifest.rs:169-190` still includes a
  basename `metrics.json`; `git diff d19e6047..8b54e62d` is empty for
  `crates/quantrule-service-api` and
  `crates/quantrule-app/tests/fixtures/run-v5/golden-digest.txt`. #1240 changed
  docs only.
- I captured the app `one/backtest` bundle from a fresh `run_v5_e2e` run (a
  syscall-delay verification harness, not a repository change) and recomputed
  the digest from the 13 canonical files with the documented
  `path NUL sha256:<hex> newline` stream, sorted by path:
  - with `portfolio/metrics.json`:
    `sha256:4d5f6a8ad385989d8a6c335bb1bd66620bd7bb3fd5a3505438d7bd6cab256b79`,
    matching the checked-in golden and the service;
  - without it:
    `sha256:8fa9d752d4e6faffaaa1d6f44656a7d5ed9a864576d71b49d6811f85dcf3f3a5`,
    the same contrast the prior record reported.
- This is one 250-bar fold fixture. It is not a proof for every run.

### Commands and observed results

All build and test commands below were run in this worktree with
`CARGO_BUILD_JOBS=2`; `cargo fmt` and `bash -n` do not use it. The counts are the
observed `cargo` output, not the implementer's claims. Counts are per command and
are not additive.

| Command | Observed result |
|---|---|
| `cargo test -p quantrule-data-service --test configs_v5` | exit 0; **4 passed**, 0 failed |
| `cargo test -p quantrule-evidence-service --test promoted_runs` | exit 0; **8 passed**, 0 failed |
| `cargo test -p quantrule-app --test run_v5_e2e` | exit 0; **2 passed**, 0 failed (0.23 s) |
| `cargo test -p quantrule-backtest-service --test worker` | exit 0; **48 passed**, 0 failed (5.84 s) |
| `cargo test -p quantrule-model-service` | exit 0; **32 passed**, 0 failed across 13 lib + 4 bundle + 5 events + 2 health + 1 isolation + 7 model; two empty targets (`qr_model` bin, doc-tests) |
| `cargo test -p quantrule-app --test signal_artifact_public` | exit 0; **6 passed**, 0 failed |
| `cargo test -p quantrule-evidence-service --lib` | exit 0; **82 passed**, 0 failed |
| `cargo run --quiet -p quantrule-governance -- check --all` | exit 0; `governance check ok` |
| `cargo test -p quantrule-app --lib pack::` | exit 0; **119 passed**, 0 failed (229 filtered) |
| `cargo test -p quantrule-app --test strategy_pack_identity --test execution_pack_identity --test portfolio_pack_identity` | exit 0; **17 passed** (5 + 6 + 6), 0 failed |
| `cargo test -p quantrule-app --test run_v5_public` | exit 0; **4 passed**, 0 failed |
| `cargo test -p quantrule-backtest-service --test run_v5_identity` | exit 0; **2 passed**, 0 failed |
| `cargo test -p quantrule-evidence-service --test lineage` | exit 0; **5 passed, 1 ignored** (Postgres; no `QR_TEST_DATABASE_URL`) |
| `cargo test -p quantrule-data-service --test configs` | exit 0; **9 passed**, 0 failed |
| `cargo test -p quantrule-app --lib` | exit 0; **348 passed**, 0 failed |
| `cargo test -p quantrule-backtest-service --lib` | exit 0; **15 passed**, 0 failed |
| `cargo test -p quantrule-data-service --lib` | exit 0; **72 passed**, 0 failed |
| `cargo test -p quantrule-model-service v5` | exit 0; **2 passed**, 0 failed (11 filtered); a subset of the 32 above |
| `cargo clippy -p quantrule-data-service -p quantrule-evidence-service --all-targets` | exit 0; no warnings |
| `cargo fmt --all -- --check` | exit 0; no diff |
| `bash -n tools/verify.sh` | exit 0; syntax ok |

The pack filter is a subset of the 348 app-lib tests, and the model-service v5
filter is a subset of its 32 tests; neither is added to the totals.

### New Critical/Major scan

| Check | Observed result |
|---|---|
| A valid v4 config still uploads | `configs` 9 passed, including upload, duplicate lookup, invalid bundle, path refusal and profile scoping; `config_references` returns an empty set for a non-v5 schema. No v4 failure was observed. |
| An existing v5 config whose pack retired | The duplicate upload is refused with `pack.invalid`; submission with a retired pack is refused with `pack.invalid` and writes no run row. The existing run's lineage read does not consult the pack state. |
| A duplicate upload bypasses the check | The check is before `store.get`; the third `configs_v5` upload after retirement is the direct test and returns 400. |
| A new run row on a refused submission | The evidence test lists runs after each non-frozen attempt and finds none. |
| Code scope of the fix | `git diff --stat d19e6047..8b54e62d -- crates` shows only `quantrule-data-service/src/routes.rs` (+21) and `quantrule-evidence-service/src/routes/runs.rs` (+29); the fold, model, service-api and golden files are unchanged. |
| Formatting and lints | `cargo fmt --check` clean; the two changed crates' clippy clean. |

No new Critical or Major finding was observed. D4 (Minor wording) is corrected
in this record's PR. The residuals are G6 (fast-test coverage), G7
(repository-scanned v5 `valid` flag) and G8 (cross-service TOCTOU). None writes
a run row for a refused submission or changes the v4 path.

### Gaps

| ID | Status | Gap |
|---|---|---|
| G1 | Open | No fast test exercises the production data-service `config_references` missing/mismatch path at submission; no composed stack or live Postgres. |
| G2 | Open | No signal-mode v5 end-to-end execution and no v5 promoted-model worker end-to-end test; the fast v5 folds are fold mode. |
| G3 | Open | No five-reference lineage route round-trip; the route tests use two and four references. |
| G4 | Open | No live Postgres. `the_postgres_lineage_store_conforms` is ignored without `QR_TEST_DATABASE_URL`; the Postgres path is not exercised in this pass. |
| G5 | Open | No gateway HTTP submit/fetch round trip in this validation. |
| G6 | Open | No direct upload test for a validated-only pack or a missing signal artifact; no retire-after-submit lineage read test. The code path is inspected, but those cases are not exercised. |
| G7 | Open | Repository-scanned v5 configs still get `valid: true` without resolution; submission resolves and refuses, but the `valid` flag is origin-scoped and undocumented. Pre-existing. |
| G8 | Open | The frozen-state check is not atomic with pack retirement across data and evidence; the narrow TOCTOU race is untested. |

### Limitations

- This is a second validator pass by a child agent; it is not a human review, not
  an independent model judgment, and not a self-approval.
- The bundle capture used a syscall delay on deletion and a copy loop. It is a
  verification harness, not a product path or a change to the test.
- The digest reconstruction is one 250-bar fold fixture. It does not prove every
  future run, and it does not replace the composed tier.
- I did not repeat the prior syscall-delay capture of the worker bundle. The
  worker suite passed, its test asserts the staged digest equals the manifest
  digest and every artifact hash, and the fold/canonical code is unchanged since
  `d19e6047`; the prior record's app-versus-worker byte agreement remains the
  evidence for that comparison.
- No edge, profitability, holdout, strategy-baseline, composed-stack,
  live-Postgres, HA, soak, upgrade, signature or release claim is made.
- The full gate, Docker stack, multi-host and HA tiers were not run. The
  `cargo` commands were fast-tier tests in this worktree.

The re-validation does not reopen D1, D2 or D3. It records the Minor D4 wording
and G6–G8 so the next change can correct them without treating the fix as
unfinished.
