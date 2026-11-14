# Execution Plan — Split the six service binaries

- Status: Proposed
- Date: 2026-11-14 (UTC)
- Claim type: ADR (Architecture Decision)
- Owner: Human
- Workflow: [Execution Plan](../../workflows/execution-plan.md), [Requirements & Design](../../workflows/requirements-design.md)
- Issue: none yet. The number is assigned on approval; this file is renamed to `<issue>-split-service-binaries.md`.
- Branch: `arch/dist-binaries-plan` (plan), `<type>/<issue>-split-service-binaries` (implementation)
- Records: ADR-2026-11-12-01, ADR-2026-11-12-04, ADR-2026-11-12-09; #643, #646, #649, #674, #679, #680

## 1. Intent

The accepted architecture says six services, six binaries, one image. The tree
ships three binaries. `qr-data`, `qr-model`, and `qr-simulation` do not exist,
yet ADR-04, `services.md`, `deployment.md`, #643, and #674 depend on them. #643
cannot package six binaries. #646 cannot proxy a service that is not there.

The gap is structural, not a missing route. #647 and #648 closed as gateway
routes, and #751 recorded the gateway serving the model route "until the six
binaries are split". No open issue owned the missing binaries.

This plan makes the binary inventory true without changing a model, strategy,
execution, or portfolio result. It recommends the smallest option that unblocks
#643 and states, in the repository, what is still not split.

## 2. Current state

| Area | Observed (2026-11-14, `b63b9015`) |
|---|---|
| Binaries | `ls crates/quantrule-server/src/bin` → `qr_backtest.rs`, `qr_evidence.rs`, `qr_gateway.rs` (3) |
| Manifest | `grep -c '\[\[bin\]\]' crates/quantrule-server/Cargo.toml` → 3 |
| Data routes | Served by the gateway: `runtime/server.rs:151-168` (`/catalog/datasets*`, `/catalog/configs`, `/datasets/*`, `/inspect/*`) |
| Model route | Served by the gateway: `runtime/server.rs:160` (`POST /model/compute`); the handler reads catalog and config stores |
| Gateway wiring | `qr_gateway.rs:46-105` connects Postgres, S3, and RabbitMQ and builds every store |
| Worker fast path | `backtest/worker.rs:189-196` calls `AppService::run_backtest` in-process; no data or model HTTP call |
| Simulation | No binary and no route. `quantrule-simulation` is not a `quantrule-server` dependency. #679 and #680 are open |
| Image | No `Dockerfile` and no compose file in the tree; #643 owns both |
| Documents | ADR-04 and `services.md` say six binaries; `module-map.md:19` says "(5 binaries)"; `README.md:72` says "five deployable responsibilities"; `deployment.md:115` says "all eight" containers while §2 lists nine |
| Issues | #643 open (six binaries, nine containers); #646 open (gateway proxy and engine isolation); #649 closed (its plan excluded the split); #674 open (six services, nine containers); #680 depends on #679 |

The contradiction is documented but not owned. #647 and #648 closed as
`quantrule-server` modules served by the gateway, and #751's risk section names
the gateway as the temporary host. No open issue owns the three missing binaries.

## 3. Scope

Included:

- Three `[[bin]]` targets: `qr-data`, `qr-model`, `qr-simulation`.
- Route extraction: data and model handlers move out of the gateway into service
  routers; the gateway keeps only its own routes and the proxy.
- One upstream client with a fake, service URLs, request forwarding, streaming
  upload, response passthrough, and upstream health.
- A gateway source guard: the gateway code calls no data or model handler and no
  engine.
- A `qr-simulation` process that serves `/health` and `/version` only; its
  research surface stays #680.
- Tests for the six-binary inventory, each service route, proxy behavior,
  streaming, health, and isolation.
- Documentation corrections in the touched sections, changelog, AI log, and issue
  comments that record the unblock and the remaining #680 dependency.

Excluded:

- `Dockerfile`, compose, image digest, and `make up` (#643, #644, #673).
- The full simulation service (#679, #680) and any simulation endpoint.
- Routing the backtest worker through the data and model services; that is the
  worker fast path and remains a recorded limitation under A.
- A crate-per-service split, compile-time engine isolation, and schema-per-service
  enforcement (#651, option B).
- Moving run-record ownership from the gateway to evidence; pre-existing seam,
  outside this plan.
- Any model, strategy, execution, portfolio, client, or command-surface behavior
  change.
- A new crate version; the proxy uses dependencies already in `Cargo.lock`.

## 4. Assumptions and uncertainty

- The six-binary claim is read as a deployable-process claim. If the human reads
  ADR-04 as domain ownership, A is insufficient and B is the answer.
- `qr-simulation` is a process skeleton, not a working simulation service. A must
  not be described as closing #646 or #674.
- The proxy can use `hyper-util` and `http-body-util`, both already resolved in
  `Cargo.lock`. If the required features are not available, the plan stops for a
  dependency decision.
- Service discovery is static: `QR_DATA_URL`, `QR_MODEL_URL`,
  `QR_SIMULATION_URL`. No mesh, no discovery service.
- The gateway keeps read-only catalog/config-store access for run submission.
  Schema ownership is #651; A does not claim it.
- The backtest worker's in-process `AppService::run_backtest` path stays
  byte-identical. No run is re-executed through HTTP.
- #646 is active in its own worktree. The proxy plumbing in this plan and #646's
  remaining gateway work must agree on file ownership before either writes
  `runtime/server.rs`.
- The issue number does not exist yet; on approval the plan is renamed and the
  issue is created from §12.

## 5. Decisions

### 5.1 The architecture choice

| Option | What it does | Consequences | Blast radius | Verdict |
|---|---|---|---|---|
| **A. Structural split** | Six binaries; `qr-data` and `qr-model` serve their routes; the gateway proxies data, model, and simulation; `qr-simulation` is a health-only skeleton; the worker fast path and shared schema reads are recorded | Makes the inventory and the gateway boundary true; unblocks #643; leaves the data-plane choreography, schema ownership, and simulation surface as stated limitations; no engine behavior change | ~20 files, ~1,200–1,600 changed lines; one new error code; no new crate version; #643 unblocked; #646/#674 stay blocked on #680 | **Recommended** |
| **B. Full split** | Every service owns its domain code; the gateway proxies every service call; the backtest worker calls data and model over HTTP; the simulation surface is included | Makes ADR-01/04/09 and `data-plane.md` true at crate and call level; removes the fast path; replaces #649's orchestration; needs a successor issue; multi-week campaign | 6–7 crates, ~4,500 domain lines and ~3,900 test lines moved; worker rewrite; new service clients; #643/#644/#674 delayed; #649 successor | Honest end state, not a blocker fix; defer to a programme |
| **C. Narrow the claim** | Keep three binaries; amend ADR-04, `services.md`, #643, and #674 to three services | Retracts an accepted architecture decision; the gateway keeps computing data and model; #646's engine-isolation and simulation claims are removed; the distributed deliverable shrinks | No code movement; ~12 documents and 8 issues rewritten; fastest schedule; highest architectural cost | **Architecture change; requires explicit human approval** |

Recommendation: **A**. It is the only option that makes the six-binary inventory
true in one reviewable change, unblocks #643, and does not pretend the data plane
or the simulation service is already split. B is the correct end state but is a
programme, not a blocker. C is a retraction of an accepted decision and is listed
so the human can choose it deliberately, not by omission.

C is not a plan edit. It amends ADR-01, ADR-04, ADR-09, ADR-12, `services.md`,
`deployment.md`, `data-plane.md`, `http-api.md`, `module-map.md`, the
`README.md` glossary, `AGENTS.md` §2, and issues #643, #644, #646, #651, #674,
#679, and #680. It retires the sixth service from v1.0.0. That is an architecture
change and requires explicit human approval.

### 5.2 Decisions inside A

| ID | Decision | Options | Recommendation | Approval |
|---|---|---|---|---|
| D1 | Architecture option | A, B, C | A | **Human, this plan** |
| D2 | Proxy transport | (a) direct `hyper-util` + `http-body-util`, both already in the lock; (b) add `reqwest`; (c) no proxy (C) | (a); no new crate version | With the plan |
| D3 | `qr-simulation` scope | (a) health/version skeleton, research surface #680; (b) full #680 now | (a); #646 stays open | With the plan |
| D4 | Worker fast path | (a) keep and record, follow-up issue; (b) route through data/model now (B) | (a); no run behavior change | With the plan |
| D5 | Schema ownership | (a) keep shared reads, record, #651; (b) move every read to HTTP now | (a); A is a process split, not an ownership split | With the plan |
| D6 | Unreachable upstream | (a) add `service.unavailable` 503; (b) reuse `internal` 500 | (a); one stable code, documented in `http-api.md` | With the plan |
| D7 | Gateway guard | (a) source-level guard with a seeded violation; (b) compile-time crate split (B) | (a); the limit is stated | With the plan |

No model, strategy, execution, or portfolio definition changes. The Quant
Validator is not required; the model route test asserts byte-identical output for
the same fixture.

## 6. Blast radius

### 6.1 Issues

| Issue | Today | Under A | Under B | Under C |
|---|---|---|---|---|
| **#643** image | Blocked: three binaries, no Dockerfile/compose | Unblocked: six binaries exist; #643 still owns the image and compose | Unblocked later, after the crate split | Scope narrowed to three binaries and five containers |
| **#644** Makefile | Depends on #643 | Unchanged; `make up` waits for #643 | Unchanged, later | `scale` and `ps` describe three services |
| **#646** gateway | Blocked: gateway serves data/model; no simulation service | Engine-isolation half satisfied at source level; data/model proxied; simulation proxy can be wired and tested with a fake, but every endpoint does not answer until #680 | All service calls proxied; simulation still needs #680 unless B includes it | Scope rewritten: gateway keeps data/model compute and drops the simulation proxy; #646 cannot close as written |
| **#649** backtest | Closed; its plan excluded the six-binary split | Stays closed; worker unchanged; fast path recorded | Needs a successor issue; worker orchestration changes; #649 stays Done | Stays closed; the exclusion becomes permanent |
| **#674** release | Blocked: six services, nine containers | Image topology can be true; release still cannot claim a working simulation surface or a split data plane; blocked on #646/#680 | Can claim the real split after a longer campaign | Release evidence and claim narrowed to three services |
| **#651** guard tests | Open; one binary per service and schema ownership | Inventory guard and gateway source guard pass; schema ownership deferred; compile-time isolation not claimed | Real ownership guard possible | "One binary per service" and simulation isolation removed |
| **#680 / #679** simulation | Open; #680 depends on #679 | `qr-simulation` health skeleton; #680 fills the surface; #679 still required for parity | Included in the campaign | Deferred or retired; ADR-09 amended |
| **#673** digest | Depends on #643 | Unchanged; image digest still #643/#673 | Unchanged, later | Digest covers three binaries |

#649 quantification: A changes 0 lines of `backtest/worker.rs` and
`quantrule-app`. B changes the one call site, the 412-line worker, and the
~1,000-line coupled path (`stages/backtest.rs` 548, `pipeline.rs` 171,
`stages/model.rs` 267), and needs a successor issue because #649 is Done.

### 6.2 What moves

| Kind | Under A |
|---|---|
| Binaries | `qr-gateway` loses the data and model handlers and gains the proxy; `qr-data`, `qr-model`, `qr-simulation` are added; `qr-backtest` and `qr-evidence` are unchanged |
| HTTP routes | `/catalog/datasets*`, `/catalog/configs`, `/datasets/*`, `/inspect/*` move to `qr-data`; `/model/compute` moves to `qr-model`; `/simulation/*` is proxied but answers 404 until #680 |
| Commands | No shell command changes. The shell is still local (#668); when it becomes an HTTP client it uses the same gateway paths. No command contract changes |
| Crates | `quantrule-server` (routes, config, upstream, tests), `quantrule-api` (one error code); `quantrule-shell` and `apps/web` no behavior change |
| Dependencies | `quantrule-server` adds direct `hyper-util` and `http-body-util`; both are already in `Cargo.lock`; no new package version. `quantrule-simulation` is not added until #680 |
| Docs | `module-map.md` (5→6), `README.md` glossary (five→six), `services.md` (proxy and limitation), `data-plane.md` (worker fast-path note), `deployment.md` (three URLs and count corrections), `http-api.md` (`service.unavailable`), `CHANGELOG.md` |
| Tests | New binary inventory, service route, proxy, streaming, health, and guard tests; existing gateway/model/config/ladder/runs tests adapt to the upstream fake |
| Estimate | ~20 files, ~1,200–1,600 changed lines; no engine, schema, artifact, or digest change |

### 6.3 What remains a limitation under A

1. **Worker fast path.** `qr-backtest` still calls `AppService::run_backtest`
   in-process. `qr-data` and `qr-model` are not on a run's path. Scaling `model`
   does not speed a run, and a model outage does not fail a run. `data-plane.md`
   §1 and `services.md` §5 describe the target, not this behavior. A records this
   and opens a follow-up; it does not change #649.
2. **Simulation skeleton.** `qr-simulation` answers `/health` and `/version` only.
   #646's "every documented endpoint answers" stays false; #674 cannot close. The
   release note must not claim the research plane is served.
3. **Shared schema reads.** The gateway and the model service read the catalog and
   config stores directly; every binary applies every migration. A does not
   enforce schema-per-service. #651 owns that gap.
4. **Source-level guard.** The gateway binary links the whole `quantrule-server`
   library, so a compile-time engine call is still possible. The guard is a source
   test with a seeded violation. B is the compile-time fix.
5. **Run-record ownership.** The gateway still writes the `runs` schema on
   submission and cancel. Evidence owns that schema in the documents. A does not
   move it.
6. **No image evidence.** A creates the binaries; #643 creates the image, compose,
   and the no-Python/no-Bun checks. A cannot prove the image.

## 7. Acceptance criteria

1. `cargo build -p quantrule-server --bins` builds exactly six binaries:
   `qr-gateway`, `qr-data`, `qr-model`, `qr-backtest`, `qr-evidence`,
   `qr-simulation`.
2. A guard test asserts the six-name inventory and fails when one `[[bin]]` is
   removed.
3. `qr-data` serves every moved data/config/inspection route with the same status,
   body, and ETag as the current gateway.
4. `qr-model` serves `POST /model/compute` and produces the same model-object
   SHA-256 for the same fixture as the current gateway.
5. The gateway proxies method, path, query, profile header, and body to the right
   service; it passes status, content type, ETag, and error envelope through
   unchanged.
6. A streaming upload is forwarded without buffering the whole body; the
   configured limit still returns `413 upload.too_large`, and the config upload
   keeps its 4 MiB route limit.
7. The gateway `/health` lists `data`, `model`, and `simulation`, and reports
   `down` when an upstream is down; a missing URL fails fast with a named
   variable.
8. Gateway source calls no data or model handler and no engine; a seeded violation
   fails the guard.
9. `qr-simulation` starts, applies its startup checks, and answers `/health` and
   `/version`; `/simulation/runs` returns 404 until #680.
10. No client path is accepted, no label is rendered by a service, and no new
    crate version appears in `Cargo.lock`.
11. `cargo test`, clippy, fmt, rustdoc, and the repository checkers pass.
12. The docs named in §6.2 are corrected, the changelog has an entry, and the AI
    log records the work.
13. The issue is created on approval, the plan is renamed, and the board follows
    `Todo → In Progress → In Review → Done`.
14. #643 and #646 receive a comment stating what is unblocked and what remains
    (#680 for #646); no issue is closed by the implementation PR unless the human
    approves it.

## 8. Work breakdown

| Phase | Work | Role | Paths | Depends |
|---|---|---|---|---|
| P0 | Human approves A; create the issue from §12; rename this file to `<issue>-split-service-binaries.md`; set the plan `Approved` | Human + Planner | `docs/execution-plans/2026-11-14/`, GitHub | This plan |
| P1 | Split the router: data handlers move to `data/routes.rs`; the model router moves to `model/routes.rs`; the gateway keeps its own routes; no behavior change | Implementation | `crates/quantrule-server/src/runtime/server.rs`, `src/data/`, `src/model/` | P0 |
| P2 | Add `qr-data`, `qr-model`, `qr-simulation` with `[[bin]]` entries; each wires only its stores; each serves `/health` and `/version`; inventory guard | Implementation + Test | `crates/quantrule-server/Cargo.toml`, `src/bin/`, `tests/boundary.rs` | P1 |
| P3 | Add `runtime/upstream.rs` (trait, `hyper-util` client, fake) and `runtime/proxy.rs`; service URLs; proxy handlers; streaming; passthrough; upstream health; `service.unavailable` | Implementation + Security | `src/runtime/`, `crates/quantrule-api/src/error.rs` | P2 |
| P4 | Remove data/model handlers and their stores from the gateway binary; point existing client tests at the fake upstream; add the source guard with a seeded violation | Implementation + Test | `src/bin/qr_gateway.rs`, `tests/gateway.rs`, `tests/model.rs`, `tests/ladder.rs`, `tests/configs.rs`, `tests/runs.rs`, `tests/boundary.rs` | P3 |
| P5 | Documentation and records: module map, README glossary, services, data plane, deployment, http-api, changelog, AI log, #643/#646 comments | Orchestrator | `docs/architecture/`, `CHANGELOG.md`, `docs/ai-transparency-log/` | P4 |
| P6 | Independent review: diff, proxy security, streaming, error mapping, no path, no label; run the gate | Code Reviewer + Security + Test | PR | P5 |
| P7 | Merge after human acceptance; move the issue to `In Review` then `Done`; remove the worktree and branches | Human + Orchestrator | GitHub, git | P6 |

The gateway proxy is the intersection with active #646. P3–P4 must agree on file
ownership with that stream before writing `runtime/server.rs`; if #646 owns all
gateway changes, P3–P4 move to #646 and this issue ends at P2.

## 9. Verification

The plan changes no behavior. The behavioral rows stay `Pending` until the
implementation PR. Gate commands run through `tools/gate.sh` with a target
directory inside the worktree.

## Verification table

| ID | Plan item | Acceptance criterion | Evidence / command | Expected | Observed | Status | Reviewer | Date | Notes |
|---|---|---|---|---|---|---|---|---|---|
| V1 | Plan checks | Plan structure, length, and links pass | `python3 tools/check_execution_plans.py && python3 tools/check_doc_length.py && python3 tools/check_links.py` | pass | pass | Pass | agent | 2026-11-14 | Plan-only PR |
| V2 | P1 route split | Data and model handlers compile from their modules and existing behavior is unchanged | `cargo test -p quantrule-server` | pass | — | Pending | — | — | No behavior change |
| V3 | P2 binaries | Six binaries are declared and built | `grep -c '\[\[bin\]\]' crates/quantrule-server/Cargo.toml`; `cargo build -p quantrule-server --bins` | 6; six files | — | Pending | — | — | |
| V4 | P2 inventory guard | A removed binary fails the guard | `cargo test -p quantrule-server --test boundary six_binaries` | pass; seeded removal fails | — | Pending | — | — | |
| V5 | P2 qr-data | Moved data routes answer with the same status, body, and ETag | `cargo test -p quantrule-server --test services data_` | pass | — | Pending | — | — | |
| V6 | P2 qr-model | Same fixture gives the same model-object SHA-256 | `cargo test -p quantrule-server --test services model_` | equal | — | Pending | — | — | |
| V7 | P2 qr-simulation | Starts and answers `/health` and `/version`; no simulation endpoint claimed | `cargo test -p quantrule-server --test services simulation_` | 200; 404 for `/simulation/runs` | — | Pending | — | — | #680 owns the surface |
| V8 | P3 proxy | Method, path, query, profile, status, content type, ETag, and error envelope pass through | proxy tests with a fake upstream | equal | — | Pending | — | — | |
| V9 | P3 streaming | A 2 MiB upload is forwarded incrementally and the limit returns 413 | streaming test with a paused producer | first chunk forwarded before the last is produced; 413 over limit | — | Pending | — | — | |
| V10 | P3 health | `/health` lists data, model, simulation; a down upstream yields `down` | gateway health test | names present; down | — | Pending | — | — | |
| V11 | P3 config | A missing URL fails fast with its name | config test | error names `QR_DATA_URL` | — | Pending | — | — | |
| V12 | P4 isolation | Gateway source calls no data/model handler and no engine; a seeded violation fails | boundary test | pass; seeded fail | — | Pending | — | — | Source-level |
| V13 | P4 no path or label | No client path is accepted and no label is rendered by a service | proxy tests and manual read | pass | — | Pending | — | — | |
| V14 | P5 docs | Module map says six; README glossary says six; limitations and URLs are recorded | `grep` checks in `docs/architecture/` | present | — | Pending | — | — | |
| V15 | P5 changelog | An entry exists under `## [Unreleased]` | `python3 tools/check_changelog.py` | pass | — | Pending | — | — | |
| V16 | P5 AI log | The implementation log exists and names the work | `ls docs/ai-transparency-log/2026-11-14/` | file present | — | Pending | — | — | |
| V17 | Gates | Tests, clippy, fmt, rustdoc, checkers | `tools/gate.sh` | pass | — | Pending | — | — | |
| V18 | Closure | PR merged, issue `Done`, comments posted, worktree and branches removed | `gh pr view`; `git worktree list` | merged; comments; removed | — | Pending | — | — | |
| V19 | Honesty | A's three limitations are recorded, not claimed fixed | manual read of §6.3 and the touched docs | present | — | Pending | — | — | |
| V20 | No new crate | No new package version in the lock | `git diff Cargo.lock` | no new package | — | Pending | — | — | Direct edges only |

## 10. Risks and recovery

| Risk | Impact | Mitigation |
|---|---|---|
| The proxy buffers a large upload | Memory and the 512 MiB limit fail on a real dataset | Stream the body; a paused-producer test proves incremental forwarding; the limit stays on the gateway |
| A forwarded header is unsafe | Header injection or a hop-by-hop leak | Forward an allowlist; never accept a host or path from the client; security review and tests |
| Data/model route behavior drifts when moved | A client sees a different status or body | Reuse the same handlers; equality tests for status, body, ETag; golden model SHA-256 |
| The gateway becomes a single point of failure | All client calls fail if it is down | It is already the only published port; keep it stateless; upstream health and 503 `service.unavailable`; compose restarts in #643 |
| `qr-simulation` is mistaken for a working service | #646/#674 close on a false claim | §6.3, the release note, and V7 state the surface is #680 |
| The worker fast path is mistaken for a split data plane | Scaling and failure behavior overclaimed | §6.3, `data-plane.md` note, and a follow-up issue; no run change in this plan |
| The proxy implementation overlaps #646 | Two streams edit `runtime/server.rs` | Agree on file ownership before P3; the fallback is to move P3–P4 to #646 |
| The guard is source-level only | A compile-time engine call is still possible | The guard is seeded; B is the compile-time fix; the limitation is stated |
| A new crate is needed | The plan becomes a dependency decision | Stop; use only `hyper-util` and `http-body-util` from the lock, or return for approval |
| The implementation changes a model output | A behavior regression | Byte-identical model test; no engine change; the reviewer checks the fixture |

Recovery: the change is additive until P4 and behavior-preserving. A revert
restores the gateway handlers and removes the three binaries; no schema, data,
artifact, or digest migrates. The image is one digest, so a rollback is the
previous digest. Data and model objects are content-addressed and unchanged.

## 11. Stop conditions

- The human does not choose A, B, or C, or chooses B or C; that is a different
  plan and scope.
- The proxy needs a crate that is not already in `Cargo.lock`, or a different
  data-plane transport.
- Moving the model route changes a model object's bytes or needs an engine change.
- The split needs a change to run identity, artifact bytes, or the canonical
  digest.
- A streaming upload cannot be proxied without buffering the whole body.
- `qr-simulation` would have to answer a research endpoint to satisfy an
  acceptance criterion; stop and leave #646 open until #680.
- The worker fast path must be removed to satisfy a criterion; that is B.
- A file is owned by #643, #646, #668, #680, or another active stream and cannot
  be coordinated.
- Any test, gate, or review fails in a way that changes the approved scope.

## 12. Issue body draft (10 lines)

```text
Goal: Make the six-binary claim true by adding qr-data, qr-model, and qr-simulation, moving the data and model routes into their binaries, and proxying them through the gateway without changing a model or strategy result.
Scope:
- Add three [[bin]] targets; split the data/model route modules out of the gateway.
- Add one upstream client, service URLs, gateway proxy routes, and upstream health; no client path, no label.
- Keep the backtest worker's in-process data/model path and the simulation surface (#680) out; record both as limitations.
Done when:
- cargo build -p quantrule-server --bins produces six binaries and a guard test fails when one is removed.
- Gateway proxy tests pass for method, path, profile, streaming upload, status/body/ETag, and a down upstream.
- qr-data and qr-model serve their moved routes with byte-identical output; qr-simulation answers /health only.
Claim type: ADR (Architecture Decision)
```

## 13. Review checklist

- [ ] The human chose A, B, or C.
- [ ] The issue body is exactly the 10 lines in §12 and carries one claim type.
- [ ] The plan is renamed on approval and the old name is not linked.
- [ ] The limitations in §6.3 are in the docs before the PR is called complete.
- [ ] No file owned by #643, #646, #668, #680, or the shell stream is changed
      without coordination.
- [ ] No model, strategy, execution, or portfolio result changed.
