# AI transparency log — Split the six service binaries (plan)

- Date: 2026-11-14 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash. S2 planning subagent.
- Intent: draft the execution plan for the missing `qr-data`, `qr-model`, and
  `qr-simulation` binaries, the gateway proxy choice, and the blast radius on
  #643, #646, #649, #674, #679, and #680. The plan recommends an option; it does
  not implement one.
- Boundary: planning only. No Rust, Cargo, Dockerfile, compose, Makefile, CI,
  app, or non-plan documentation was changed. No issue was created and no code
  pull request was opened. Worktree
  `/home/zantgo/Proyectos/quant-rule--dist-binaries-plan`, branch
  `arch/dist-binaries-plan`, base `b63b9015`.

## What was read

`AGENTS.md` sections 2, 3, 4, 7, 8, 9, 11, 13, and 14; the execution-plan
workflow; ADR-04; `services.md`; `deployment.md`; issues #643, #644, #646, #649,
and #674. The hidden blocker required #647, #648, #651, #661, #673, #679, #680
and pull requests #750/#751 as well.

## Findings

- The tree has three binaries: `qr-gateway`, `qr-backtest`, `qr-evidence`.
- The data and model HTTP routes are served by the gateway. The model route's
  pull request #751 says this is temporary "until the six binaries are split".
- The backtest worker calls `AppService::run_backtest` in process; it never calls
  a data or model service. That is the worker fast path.
- There is no simulation binary and no simulation route. #680 depends on #679 and
  both are open.
- #649 is closed; its plan explicitly excluded the six-binary split. A rewrite of
  the worker under option B needs a successor issue, because a Done board item is
  not reopened.
- `module-map.md` says "5 binaries", the architecture README glossary says "five
  deployable responsibilities", and `deployment.md` says "all eight" containers
  while its own topology lists nine.
- No open issue owned the binary inventory. #647 and #648 were allowed to close
  as gateway routes, which is how the contradiction survived.

## Verification

| Command | Observed |
|---|---|
| `python3 tools/check_execution_plans.py` | pass |
| `python3 tools/check_doc_length.py` | pass, 32 files within cap |
| `python3 tools/check_links.py` | pass, 922 Markdown files, no broken relative links |
| `git diff --check` | pass |

The behavioral verification rows in the plan are `Pending` by design because this
PR changes no behavior.

## Learning and corrections

- The blocker is not a missing route; it is a claim that was never given an
  owner. The plan must name the closure pattern or the same gap returns.
- The first draft treated #646 as fully unblocked by option A. It is not: A
  satisfies the engine-isolation half and wires the proxy, but the simulation
  surface remains #680. #646 and #674 stay blocked. The plan states this in §6.3
  and in the acceptance criteria.
- Option A's worker fast path changes the failure semantics that `services.md`
  describes: a model outage does not fail a run. The plan records that instead of
  claiming a split data plane.
- Option C is a real option, but it retracts ADR-01, ADR-04, and ADR-09. The plan
  lists it so the human can reject it deliberately; it is not a planning
  convenience.

## Rejected

- Adding simulation endpoints to make `qr-simulation` "real". That is #679/#680
  and would hide a parity gap.
- Adding `reqwest` for the proxy. `hyper-util` and `http-body-util` are already
  in `Cargo.lock`; a direct edge is enough.
- Creating the issue or inventing a number before human approval.
- Editing `module-map.md`, the README glossary, `deployment.md`, or the other
  implementation documents in this plan PR. The task boundary allows only the
  plan and this log; the plan lists those edits for the implementation issue.
- Opening a code PR or closing any issue. The plan is the only deliverable.

No credentials, secrets, or sensitive operational data were recorded.

## Revision — Option B selected — 2026-11-14 (UTC)

- Decision: the human selected **Option B — full split** on 2026-11-14. Option A
  (six binaries, gateway proxies data and model, worker fast path recorded) is
  recorded as rejected because it leaves the data plane and schema ownership
  false. Option C (narrow the claim to three binaries) is recorded as rejected
  because it retracts ADR-01, ADR-04, and ADR-09.
- Revised scope: the plan is now the B implementation plan, not a comparison.
  It defines the target crate graph, the exact `quantrule-server` module moves,
  the gateway proxy surface, the worker HTTP path, the #649 successor, schema
  ownership, per-service migrations, internal contracts, transport, health,
  cancellation, retries, streamed uploads, phases, file ownership, and
  downstream gates.
- The six-binary inventory is now milestone M1 inside B. It unblocks the #643
  binary prerequisite but is explicitly not the end state.
- Deferred at that revision, corrected by the accuracy pass below: embedded
  desktop v1.1, streamed artifact downloads, and a `quantrule-app` crate split.
  The shell HTTP rewrite (#668) and simulation client screens (#681-#683) were
  later confirmed as parallel v1.0.0 work, not deferred. The compute services
  remain thin adapters over the shared app crate; the gateway is the
  compile-time isolated binary.
- Verification: `python3 tools/check_execution_plans.py`,
  `python3 tools/check_doc_length.py`, `python3 tools/check_links.py`, and
  `git diff --check` were run on the revised plan. The behavioral rows remain
  Pending because this PR changes no behavior.
- Parked S1 input: `/home/zantgo/Proyectos/quant-rule--646-gateway` was inspected
  read-only. It has uncommitted bearer-seam, route-module, artifact/digest,
  event-inspection, profile-count, migration-13, and route-table work. B maps it
  as a reuse candidate in P3, P5, and P8; it is not landed and is not evidence.
  The plan requires a salvage map before P3 and a re-test in the owning service
  crate for every reused hunk.
- Learning: B is not an incremental route change. The three hard parts are the
  model bundle protocol (the public model route returns a report, while the
  worker needs the staged model inputs), run-record ownership (the gateway and
  worker currently write the `runs` schema), and the worker's new staged
  application use case. The six-binary milestone is easy by comparison.
- Rejected: reopening #649 (a Done board item stays Done; the successor is a new
  issue), inventing an issue number, implementing any phase in this PR, creating
  the issue, or merging the plan PR.
- Correction: the first plan's A recommendation treated the six-binary inventory
  as the goal. B treats it as a milestone and makes the data plane, schema
  ownership, and simulation surface the acceptance frame.
- Parked S1: the `#646` worktree carries uncommitted gateway work. The plan maps
  it to P3 (data routes and events), P5 (run artifacts, digest, count, migration
  13), and P8 (bearer seam, route table, boundary tests). It is a reuse
  candidate, not landed. Copying it wholesale would duplicate routes and import
  the mixed `AppState` into services that no longer have it.

## Amendment — #772 merged findings — 2026-11-14 (UTC)

- Input: PR #772 (`DIST-022`) merged at `f3e258f0`. It builds the image and the
  Compose stack for the three existing binaries and records two findings in
  `docker/README.md` and its AI log.
- Migration lock defect: the runtime acquires a session-level advisory lock and
  releases it through the pool, so lock and unlock can land on different
  connections. Three migrating services deadlocked; Compose works around it by
  making the gateway the sole migrator (`QR_MIGRATE=1`, workers `0`). The plan
  records this in §5.5 and adds the P1 fix: one `PoolConnection`, acquire,
  migrate, and release on it. A composed two-migrator test with a timeout is
  V30. The Compose workaround stays until that test passes.
- MinIO registry: public MinIO images were withdrawn on 2026-09-11. #772 pins
  `bitnamilegacy/minio@sha256:451fe6858cb770cc9d0e77ba811ce287420f781c7c1b806a386f6896471a349c`
  as an amd64 stopgap, overridable with `QR_OBJECT_STORE_IMAGE`. The plan records
  the permanent registry as a human decision under P9 and §9.1, not as an
  implementation choice; the legacy archive is never described as permanent.
- Verification: `python3 tools/check_execution_plans.py`,
  `python3 tools/check_doc_length.py`, `python3 tools/check_links.py`, and
  `git diff --check` were run on the amended plan.
- Learning: the image work exposed defects that belong to the split, not to the
  image. The B runner must own the migration lock and the release must own the
  registry decision, or the Compose workaround becomes the architecture.
- Rejected: treating the gateway sole-migrator rule as the fix; treating the
  legacy Bitnami archive as the permanent object store; implementing the
  migration fix or the registry decision in this plan-only PR.

## Accuracy pass — parallel v1.0.0 work — 2026-11-14 (UTC)

- Inputs inspected read-only: #771 (`quantrule-shell` HTTP client), #770
  (`PORT-001`/simulation meta), #769 (simulation chart slice), #773 (desktop
  shell Linux slice), #772 (image and Compose).
- State recorded: #668 is Done via #771; `quantrule-shell` depends only on
  `quantrule-api`, `serde_json`, and `ureq` and exposes 27 commands. #770 merged
  the meta stage and `PORT-001`; #679 stays open on historical SIM-01..SIM-10
  parity. #769 merged the chart and mark-capture client slice; #681 stays open on
  the real backend round trip and #682/#683 remain open. #773 merged the Linux
  AppImage; Windows NSIS and published assets remain #673/#674, and #691 stays
  open. #772 merged the image and six-container default stack; #643 stays open on
  six binaries and the MinIO registry.
- Plan changes: §4.3 now lists only true v1.1/limitation deferrals; a new §4.4
  records the parallel v1.0.0 work and states B does not implement it. Current
  state, P7/P8/P9 ownership, downstream gates, acceptance criteria, risks,
  blast radius, verification V32/V33, and the review checklist were updated.
  The issue body line 5 now names the parallel streams.
- Learning: v1.0.0 is a portfolio of parallel streams. Calling the shell rewrite
  or simulation screens "deferred" would be false: the shell is done and the
  chart slice is merged. B must claim only the service split and preserve the
  shell contract, not absorb or defer its peers.
- Rejected: claiming #681 or #691 closed (only slices merged), claiming #679
  closed (historical parity open), claiming #643 closed (six binaries and
  registry open), and implementing any parallel client work in this plan.
- Verification: `python3 tools/check_execution_plans.py`,
  `python3 tools/check_doc_length.py`, `python3 tools/check_links.py`, and
  `git diff --check` were run on the accuracy pass.

No credentials, secrets, or sensitive operational data were recorded.
