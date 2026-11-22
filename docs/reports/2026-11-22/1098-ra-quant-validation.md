# RA-010 Quant Validation — Research Packs and Signal Artifacts

- Date: 2026-11-22 (UTC)
- Status: Sequential Quant Validator pass; not an independent agent and not an
  independent judgment. No edge, profitability, or strategy-validation claim.
- Work item: [#1098](https://github.com/quant-rule/quant-rule/issues/1098)
  (RA-010), parent [#1083](https://github.com/quant-rule/quant-rule/issues/1083)
- Plan: [`1082-v1.6.0-ra-ha.md`](../../execution-plans/2026-11-22/1082-v1.6.0-ra-ha.md)
- Baseline merge commit: `3a722b59` (merge of PR
  [#1150](https://github.com/quant-rule/quant-rule/pull/1150), current `main`)
- Claim type: Behaviour Claim validation record; no edge claim
- Contracts: [`research-artifact-packs.md`](../../config/2026-11-22/research-artifact-packs.md),
  [`signal-artifact.md`](../../config/2026-11-22/signal-artifact.md),
  [`ADR-RA`](../../adr/2026-11-22/1082-research-artifact-layer.md),
  [`research-artifacts.md`](../../architecture/research-artifacts.md)
- Related PRs: [#1145](https://github.com/quant-rule/quant-rule/pull/1145)
  (pack registry), [#1147](https://github.com/quant-rule/quant-rule/pull/1147)
  (signal artifact), [#1149](https://github.com/quant-rule/quant-rule/pull/1149)
  (strategy service), [#1150](https://github.com/quant-rule/quant-rule/pull/1150)
  (client surface)

## 1. Scope and claims checked

The validation reproduces the RA artifact claims that can be checked without a
run:

- strategy, portfolio, and execution pack schema and content identity;
- pack validation and lifecycle: draft, validated, frozen, retired; label
  uniqueness; freeze and retire idempotence; frozen immutability; retired
  terminality;
- signal artifact canonical identity, causality, determinism, and reuse;
- closed signal field surface: no fill, quantity, fee, cash, equity, P&L,
  return, confidence, statistical, or edge field;
- the strategy service's frozen signal-mode precondition, model verification,
  object write, metadata registration, and service boundary;
- the additive client surface: 61 commands, the 49 pre-change help contracts,
  the 40 pinned pre-change requests, both-client reachability, no client-side
  signal computation, and the v2 `503 service.unavailable` create response;
- the run-lineage boundary: the read/write routes exist, but run submission does
  not write lineage and the strategy service does not.

Not checked: the model computation, execution/portfolio replay semantics, a
full run-v5 submission/worker path, the real multi-host or HA topology, the full
Compose stack, Playwright, and any market-data validation or holdout partition.
No strategy result is claimed.

## 2. Inputs and code state

| Item | Value |
|---|---|
| Worktree | `/home/zantgo/Proyectos/quant-rule--1098-quant-validation` |
| Branch | `feat/1098-quant-validation` at `3a722b59` |
| Candle fixture | `crates/quantrule-app/tests/fixtures/signal/model/`, 6 bars, 300 s, 2024-01-01 |
| Signal pack | Synthetic `signal-event-5m-signal` from the public test; mode `signal`, `event_composite`, 5m |
| Model artifact | Fixture `provenance.json`, `readings.csv`, `events.csv` |
| Data partition | Repository fixture only; no `strategy_validation` or holdout access |
| Seed/randomness | None |
| Fast target | `target/debug`; full gate target `target/gate` |
| Web dependencies | Installed with `bun install --frozen-lockfile`; no tracked file changed |

## 3. Observed results

### 3.1 Golden signal identity

The stock test asserts a literal golden artifact id at
[`signal_artifact_public.rs`](../../../crates/quantrule-app/tests/signal_artifact_public.rs)
lines 143–148. There is no separate golden file. To check the id outside the
test binary, I reconstructed the canonical pack, the model artifact digest, and
the canonical signal document from the fixture and the contract, then hashed the
bytes with SHA-256:

| Item | Reconstructed value |
|---|---|
| Canonical pack bytes | 473 bytes |
| Pack id | `sha256:b82fa966c5d06e7700e55135d541a8baab4fb17427798e9c468a94a46e798cc8` |
| Model artifact id | `sha256:f7cdde54c97a1a9a70f9a4e83cc9ec6de75152fd85512b1008afa3254cfc46ef` |
| Canonical signal bytes | 1,443 bytes |
| Reconstructed signal id | `sha256:2551537003307cd1c2b324e3eb006c1a7f14abef6ede728f0cf9510ffaaf7689` |
| Checked-in golden | `sha256:2551537003307cd1c2b324e3eb006c1a7f14abef6ede728f0cf9510ffaaf7689` |
| Match | Yes |

The reconstruction produced the six expected rows: hold, enter, hold, exit
`take_profit`, hold, hold. The matched hash proves the stored document contains
exactly the contract fields and the expected pack, model, timeframe, period, and
row values; an extra or missing field would change the id.

`cargo test -p quantrule-app --lib signal::` passed 24 tests and
`cargo test -p quantrule-app --test signal_artifact_public` passed 6 tests.

### 3.2 Causality and availability

| Test | What it actually does | Result |
|---|---|---|
| `quantrule-app/tests/signal_artifact_public.rs::a_later_bar_cannot_change_an_earlier_row` | Truncates `readings.csv` from 6 to 4 bars and asserts `artifact.rows() == full.rows()[..4]` | Pass |
| `quantrule-app/src/signal/tests.rs::a_later_bar_cannot_change_an_earlier_row` | Truncates the 6 rising bars to 4 and asserts the prefix; then replaces final bar 5 and asserts `changed.rows()[..5] == full.rows()[..5]` | Pass |

The final-bar case compares the five earlier rows, not the full artifact: row 5
is the changed bar and is allowed to differ. That is the correct causality
assertion. The test does not assert that row 5 changes.

Event availability is enforced in
[`bundle.rs`](../../../crates/quantrule-app/src/stages/strategy/bundle.rs)
(`event.available_bar_index <= bar`) and the row parser refuses an
`available_bar_index` earlier than its `bar_index`
([`artifact/mod.rs`](../../../crates/quantrule-app/src/signal/artifact/mod.rs)).
The public fixture event has `occurred_bar = available_bar = 1`; I found no
dedicated signal test with a later availability bar or an earlier available bar.
That is a test-coverage gap, not an observed failure (section 5).

### 3.3 Determinism and reuse

| Claim | Evidence | Result |
|---|---|---|
| Repeated computation gives one id and one byte string | `the_same_inputs_produce_the_same_bytes` (public) and `the_artifact_is_byte_deterministic` (unit) | Pass |
| Reordered events give the same bytes | `reordered_events_produce_the_same_bytes` (unit), two events inserted in opposite order | Pass |
| Execution and portfolio packs do not change signal bytes | `the_signal_is_reusable_under_two_execution_and_portfolio_packs` (public) | Pass, with a scope caveat |

The reuse test builds execution packs with `slippage_bps` 2.0 and 0.5 and
portfolio packs with `fraction` 0.05 and 0.10, asserts each pair has different
ids, then recomputes the signal and compares bytes. It does **not** pass the
packs into `compute`; the computation signature has no execution or portfolio
parameter at all ([`compute.rs`](../../../crates/quantrule-app/src/signal/compute.rs)).
The structural exclusion is therefore stronger than the byte comparison, but the
test is not an end-to-end policy replay.

### 3.4 Closed signal field surface

The parser uses an exact allowed-field list at every level:

- root: `schema`, `strategy_pack_id`, `model_artifact_id`, `data_version`,
  `timeframe`, `period`, `rows`;
- row: `bar_index`, `open_time`, `available_bar_index`, `action`, `side`,
  `intent`, `reason`, `evidence`;
- intent, stop, target, and evidence shapes have their own exact lists.

See [`parse.rs`](../../../crates/quantrule-app/src/signal/artifact/parse.rs)
around the root and row validators. The public golden test scans the canonical
text for `quantity`, `fee`, `cash`, `equity`, `pnl`, and `edge`; the unit test
`the_closed_parser_refuses_a_fill_or_a_quantity_field` injects `quantity` and
expects a refusal. The reconstruction in section 3.1 contains none of the
forbidden fields. `return`, `confidence`, and `statistical` are not separately
scanned, but the closed parser has no field for them, so they cannot appear
without a schema change. No fill field exists; the reference position records a
target state, not a fill.

### 3.5 Pack validation and lifecycle

| Claim | Evidence | Result |
|---|---|---|
| A draft validates | `pack_conformance.rs::conformance`; `packs.rs::freeze_is_explicit_idempotent_and_immutable` | Pass |
| A validated pack freezes | Same tests | Pass |
| Freeze is idempotent and adds no event on repeat | `pack_conformance.rs`, `packs.rs` | Pass |
| Retire is idempotent | `pack_conformance.rs`, `packs.rs` | Pass |
| A frozen pack cannot be re-validated | `packs.rs` HTTP test returns conflict; `pack_conformance.rs` returns `Frozen` | Pass |
| A retired pack is terminal | `pack/lifecycle.rs::retired_is_terminal`; `pack_conformance.rs` freezes a retired pack and gets `Retired`; route maps it to `pack.conflict` | Pass |
| A repeated label with different bytes is refused | `packs.rs::the_same_bytes_are_idempotent_and_a_label_with_different_bytes_is_refused`; `pack_conformance.rs` label race | Pass |
| A retired pack is not deleted | `packs.rs` show still returns 200 and `DELETE` returns 405 | Pass |

The lifecycle tests are at
[`packs.rs`](../../../crates/quantrule-data-service/tests/packs.rs) and
[`pack_conformance.rs`](../../../crates/quantrule-data-service/tests/pack_conformance.rs).
The Postgres conformance body passed in the composed tier; the fast gate also
passed all 7 HTTP pack tests. A direct HTTP validate-after-retire call is not
present, but the store's `Retired` error path and the lifecycle transition test
cover terminality.

### 3.6 Strategy service

| Claim | Evidence | Result |
|---|---|---|
| Only a frozen signal-mode pack is accepted | Code at [`resolve.rs`](../../../crates/quantrule-strategy-service/src/resolve.rs) checks kind, `PackState::Frozen`, matching id/bytes, and `StrategyMode::Signal` before the model call | Pass by inspection; no direct non-frozen test |
| Fold mode is rejected before model call or write | `a_fold_pack_is_refused_before_the_model_call_or_write` asserts 400 `signal.invalid`, zero model calls, zero artifact calls, no object | Pass |
| Pack id/bytes mismatch is refused | `a_pack_whose_bytes_do_not_match_its_id_is_refused` | Pass |
| Model id, dataset, config, and manifest schema are checked | `resolve.rs`; `a_missing_model_object_or_mismatched_manifest_fails_closed` | Pass |
| Every listed model object is hash- and size-checked | `resolve.rs` per-object loop; missing-object test | Pass by inspection and missing-object test |
| The service writes its own canonical bytes and registers metadata only | `one_request_publishes_one_object_and_one_row`; `a_duplicate_returns_the_same_row_with_created_false`; `two_concurrent_duplicates_leave_one_object_and_one_row` | Pass |
| A failed registration leaves only the safe orphan | `a_failed_registration_leaves_only_the_safe_orphan_object` | Pass |
| One pack and one model resolution for 200 bars | `a_200_bar_bundle_makes_one_pack_and_one_model_call` asserts one call to each | Pass |
| No database, queue, research, run, or execution import | `boundary.rs` scans `Cargo.toml` and every `src/*.rs`; seeded-violation test | Pass |

All fixtures in `tests/signal.rs` set `PackState::Frozen`, so the frozen-state
guard itself is not exercised by a failing case. The code check is explicit at
`resolve.rs` line 147; this is a coverage gap to close before any freeze gate
that requires a test for every precondition.

### 3.7 Client surface and parity

| Claim | Evidence | Result |
|---|---|---|
| 61 commands | `help.rs` asserts `COMMANDS.len() == 61`; `client_parity.rs::every_command_is_exercised` | Pass |
| 49 pre-change help contracts are byte-identical | `command_surface.rs::the_49_existing_help_contracts_are_byte_identical`; fixture has 49 keys | Pass |
| 40 pre-change parity requests are pinned | `client_parity.rs` asserts the first 40 of 52 requests match `view-49.json`; fixture has 40 | Pass |
| Both clients reach packs, signals, and lineage | Shell `packs.rs`, `signal.rs`, `run.rs` cases; browser `PacksPanel` and run-detail lineage; shared parity view | Pass |
| No client computes a signal | Browser `signals.ts` carries metadata only and downloads verified bytes; shell `write_verified` writes bytes without parsing a row | Pass |
| v2 signal create is `503 service.unavailable` | `signal_proxy.rs::a_missing_strategy_service_is_unavailable_for_create_but_reads_still_work` | Pass |

The browser parity view reaches 61 commands and 50 requests; the shell reaches
61 commands and 52 requests. The difference is covered by the shared normalized
view, not by a divergent command contract.

### 3.8 Run-lineage boundary

`GET /runs/{id}/lineage` and `PUT /internal/v1/runs/{id}/lineage` exist, and the
lineage set is immutable and idempotent. Run submission at
[`runs.rs`](../../../crates/quantrule-evidence-service/src/routes/runs.rs)
does not call the lineage writer, and the strategy service records no lineage.
No `run:<id>` reference is written by this change. This is the documented
boundary, not a validation failure: a new signal can be unreferenced for the
30-day retention window.

## 4. Gates and composed tier

| Command | Observed |
|---|---|
| `cargo test -p quantrule-app --lib signal::` | 24 passed, 0 failed |
| `cargo test -p quantrule-app --test signal_artifact_public` | 6 passed, 0 failed |
| `cargo test -p quantrule-strategy-service` | 2 unit, 3 boundary, 10 route tests passed |
| `cargo test -p quantrule-data-service --test artifacts` | 8 passed, 0 failed |
| `cargo test -p quantrule-data-service --test packs` | 7 passed, 0 failed (supplementary run) |
| `cargo test -p quantrule-evidence-service --test lineage` | 4 passed, 1 ignored (Postgres; covered by composed) |
| `cargo test -p quantrule-shell --test command_surface --test client_parity` | 2 + 4 passed |
| `cargo test -p quantrule-gateway --test signal_proxy` | 7 passed |
| `cd apps/web && bun test` | 170 passed, 0 failed, 1,497 expectations, 27 files |
| `cd apps/web && bun run check` | 0 errors, 0 warnings |
| `tools/postgres_composed.sh` | 22 tests over 12 targets passed; four-database role boundary passed |
| `tools/gate.sh` | `gate passed: full`; 2,789 passed, 0 failed, 32 ignored |
| `cargo run -q -p quantrule-governance -- check --all` | `governance check ok` |

The full gate includes `cargo fmt --all -- --check`, workspace clippy with
`-D warnings`, workspace tests, rustdoc with `-D warnings`, and the governance
checkers.

The composed script's artifact, pack, and lineage targets were
`artifact_conformance` (1 pass), `pack_conformance` (1 pass), and `lineage`
(1 pass), together with the catalog, D1, leader, outbox, research-ledger, stage,
profile, migration-lock, and simulation-concurrency targets.

The 32 fast-gate ignored tests are 22 disposable-Postgres conformance tests
(executed by the composed script), seven deliberate fixture regenerators, two
release benchmarks, and one RabbitMQ broker test. The fixture regenerators,
benchmarks, and RabbitMQ test were not run in this validation. Playwright and
the full Compose stack were not run.

## 5. Limitations and residual risks

- This is a sequential Quant Validator pass by the same agent that implemented
  and reviewed the work in the same session family. It is not an independent
  agent, an independent model judgment, or a second reviewer.
- No human-owned strategy pack, production model artifact, or run was supplied.
  The golden fixture is a synthetic six-bar bundle. No edge, return, drawdown,
  trade-count, or profitability conclusion is possible or claimed.
- Run-v5 submission, the catalog scan, worker resolution, and `run:<id>` lineage
  are not implemented. The signal artifact is a pre-run object; it is not yet
  consumed by a run.
- The strategy service was tested with in-process pack, model, artifact, and
  object fakes, not against the real data and model services in a composed
  stack. The composed tier proves Postgres conformance, not the service route.
- The frozen-state guard is checked in `resolve.rs` but not exercised by a
  non-frozen test. The model-object hash/size check is exercised by a missing
  object, not a tampered object at the service level.
- The reuse test constructs two execution and two portfolio packs but does not
  feed them to the computation; reuse is proved at the type boundary and by
  byte equality, not by a downstream replay.
- The reference position is not a fill model. It records a target state and no
  price, size, fee, or P&L. An execution/portfolio consumer does not exist yet.
- Event availability and the `available_bar_index >= bar_index` parser rule are
  enforced in code but lack dedicated signal tests with a later availability bar
  or an earlier available value.
- Playwright E2E, the full Compose stack, multi-host, and HA were not run. The
  v2 profile's deliberate lack of the strategy service is covered only by the
  gateway 503 and browser disablement tests.

## 6. Freeze recommendation

**Conditionally freeze the v1 contract.** The `quantrule.pack/v1` envelope and
the `quantrule.signal-artifact/v1` document are ready to freeze for `v1.6.0` as
the research-artifact contract: content identity, causality, determinism, the
closed field set, the pack lifecycle, the strategy-service boundary, and client
parity all reproduced on the validated baseline.

**Do not freeze any specific pack as validated alpha from this record.** No
human-owned pack, production model artifact, or run was supplied. A pack freeze
still requires its own human decision and validation record. The signal is not
yet run-reachable, because run-v5 submission/worker resolution and lineage
recording are later work.

Conditions attached to the recommendation:

- add a failing non-frozen pack test before a freeze gate that requires a test
  for every service precondition;
- add a composed service test against the real data and model services, and a
  Playwright signal/pack/lineage run in the release tier;
- complete run-v5 submission, worker resolution, and lineage before claiming
  end-to-end signal reuse;
- add the availability-specific signal tests described in section 5.

The recommendation is invalidated by any of the following:

- a golden id change without an intentional schema or fixture change;
- a later bar changing an earlier row;
- an execution or portfolio policy entering the computation or changing bytes;
- a fold-mode pack or a giveback exit being accepted;
- a fill, quantity, fee, cash, equity, P&L, return, confidence, statistical, or
  edge field appearing in the signal document;
- a frozen pack mutating, a retired pack re-entering use, or a label conflict
  being accepted;
- the strategy service writing before verification, accepting a non-frozen pack,
  or importing a database, queue, research, run, or execution dependency;
- client parity drift, a client-side signal computation, or signal create not
  returning `503 service.unavailable` on a strategy-less profile.

## 7. Links

- Plan: [`1082-v1.6.0-ra-ha.md`](../../execution-plans/2026-11-22/1082-v1.6.0-ra-ha.md)
- ADR: [`1082-research-artifact-layer.md`](../../adr/2026-11-22/1082-research-artifact-layer.md)
- Architecture: [`research-artifacts.md`](../../architecture/research-artifacts.md)
- Pack contract: [`research-artifact-packs.md`](../../config/2026-11-22/research-artifact-packs.md)
- Signal contract: [`signal-artifact.md`](../../config/2026-11-22/signal-artifact.md)
- RA-007 plan and log: [`1095-signal-artifact.md`](../../ai-transparency-log/2026-11-22/1095-signal-artifact.md)
- RA-008 plan and log: [`1096-strategy-service.md`](../../ai-transparency-log/2026-11-22/1096-strategy-service.md)
- RA-009 plan and log: [`1097-signal-client.md`](../../ai-transparency-log/2026-11-22/1097-signal-client.md)
- Quant Researcher note: [`1095-signal-researcher.md`](1095-signal-researcher.md)
- AI log for this record: [`1098-quant-validation.md`](../../ai-transparency-log/2026-11-22/1098-quant-validation.md)
- Fast evidence: [`signal_artifact_public.rs`](../../../crates/quantrule-app/tests/signal_artifact_public.rs),
  [`signal/tests.rs`](../../../crates/quantrule-app/src/signal/tests.rs),
  [`strategy-service/tests/signal.rs`](../../../crates/quantrule-strategy-service/tests/signal.rs),
  [`packs.rs`](../../../crates/quantrule-data-service/tests/packs.rs),
  [`command_surface.rs`](../../../crates/quantrule-shell/tests/command_surface.rs),
  [`client_parity.rs`](../../../crates/quantrule-shell/tests/client_parity.rs),
  [`signal_proxy.rs`](../../../crates/quantrule-gateway/tests/signal_proxy.rs)
- Composed evidence: [`pack_conformance.rs`](../../../crates/quantrule-data-service/tests/pack_conformance.rs),
  [`artifact_conformance.rs`](../../../crates/quantrule-data-service/tests/artifact_conformance.rs),
  [`lineage.rs`](../../../crates/quantrule-evidence-service/tests/lineage.rs),
  [`tools/postgres_composed.sh`](../../../tools/postgres_composed.sh)
- Web evidence: [`clientParity.test.ts`](../../../apps/web/src/lib/clientParity.test.ts),
  [`signals.ts`](../../../apps/web/src/lib/signals.ts)
