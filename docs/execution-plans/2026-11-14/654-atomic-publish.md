# Execution Plan — Atomic artifact publish

- Status: Verified
- Approved: 2026-11-14
- Claim type: Behaviour Claim
- Owner: Human
- Workflow: [Execution Plan](../../workflows/execution-plan.md)
- Records: DIST-042 (#654); epic #630
- Branch: `docs/654-atomic-publish-plan` (plan), `feat/654-atomic-publish` (implementation)

## 1. Intent

Guarantee that a succeeded run has a complete, verified artifact set, and that a
worker stopped mid-write leaves no visible artifact. Today the worker writes each
artifact directly to `runs/{run_id}/{name}`; a kill after the first write leaves
partial final keys even though the run row lists nothing.

The publish point becomes one manifest object. Artifact bytes live under their
own SHA-256, so a retry reuses them and a partial write cannot publish a set.

## 2. Current state

| Piece | Today |
|---|---|
| Worker write | `collect_artifacts` PUTs `runs/{run_id}/{name}` if absent |
| Run record | `computed` stores the refs and canonical digest after all writes |
| Evidence | reads `runs/{run_id}/{name}`, verifies size/hash, recomputes digest |
| Kill mid-write | some final keys exist; the run row lists nothing; run `running` |
| Download | `GET /runs/{id}/artifacts/{name}` is documented, not implemented |
| Store atomicity | S3 PUT is atomic per object; the in-memory fake is; the trait does not say so |

The canonical digest over sorted `(name, sha256)` is already correct and stays.

## 3. Scope and non-goals

In scope:

- Content-addressed artifact blobs at `artifacts/{sha256}`.
- One manifest at `runs/{run_id}/manifest.json`, written once, atomically.
- Worker: write blobs, build and PUT the manifest, then `computed` with the manifest entries.
- Evidence: read the manifest, verify every blob and the manifest/DB agreement, recompute the digest.
- `ObjectStore::put` atomicity contract in the trait docs.
- Fail-injection test for a mid-write stop; success, missing, changed, mismatch tests.

Non-goals:

- No artifact download endpoint; its future implementation resolves names through the manifest.
- No garbage collection of unreferenced blobs; they are content-addressed and safe, and retention is a follow-up.
- No real `SIGKILL` subprocess test; that is the composed tier (#665).
- No change to the canonical digest algorithm, the run identity, or any engine behavior.
- No live S3.

## 4. Assumptions and uncertainty

- An object PUT is atomic per key: a reader sees the old object or the complete new one, never a partial body. S3 provides this; the trait will state it and the filesystem implementation, when it exists, uses temp file plus rename.
- The manifest is small (one entry per artifact) and is written after every blob.
- A kill after the manifest PUT but before `computed` leaves a complete manifest and no DB list; a retry reuses it and records the same entries. Evidence never sees it without `run.computed`.
- A kill after `computed` but before the publish is already covered by #653: the redelivery republishes.

## 5. Decisions needing approval

| ID | Decision | Recommendation | Alternative |
|---|---|---|---|
| D1 | Artifact layout | Blobs at `artifacts/{sha256}` plus `runs/{run_id}/manifest.json`; `runs/{run_id}/{name}` retired | One `runs/{run_id}/bundle.tar`; atomic but evidence must untar and download must extract |
| D2 | Publish point | The manifest PUT is the publish; nothing is visible before it | DB `computed` as the publish; rejected because final keys still appear before it |
| D3 | Manifest schema | `quantrule.run-manifest/v1` with run id, canonical digest, and sorted entries `{name, bytes, sha256}`; no timestamp | Ad hoc JSON; rejected because evidence and the future download route need one schema |
| D4 | Evidence trust | Verify manifest entries against the DB list and every blob; reject any disagreement | Trust the DB list only; rejected because the manifest is the published set |
| D5 | Kill test | Fail-injection at the object store now; a real `SIGKILL` subprocess is composed and deferred to #665 | Real kill now; needs the Compose stack and a child binary |
| D6 | Orphan blobs | Keep them; content-addressed and reused by a retry | Delete on failure; rejected because a delete can race a shared blob |

## 6. Acceptance criteria

- A successful run writes one manifest whose entries equal its run artifacts; every blob verifies; the digest is unchanged.
- A worker stopped after N writes leaves no manifest, no recorded artifact, and no succeeded run.
- No final `runs/{run_id}/{name}` object is written on success or failure.
- Evidence rejects a missing manifest, a missing blob, a changed blob, a manifest/DB disagreement, and a digest mismatch.
- A fail-before/pass-after test asserts the no-final-key invariant; on the current layout it fails, after the change it passes.
- Existing worker, evidence, queue, and run tests pass.

## 7. Work breakdown

| Step | Work | Paths |
|---|---|---|
| 1 | State and test the `put` atomicity contract | `runtime/adapters/mod.rs` |
| 2 | `RunManifest` type: encode/decode, sorted entries, digest | `evidence/manifest.rs` or `backtest/manifest.rs` |
| 3 | Worker: write blobs, PUT manifest, record `computed` | `backtest/worker.rs` |
| 4 | Evidence: load and verify the manifest, then the blobs and DB list | `evidence/artifacts.rs`, `evidence/finalize.rs` |
| 5 | Fail-injection and kill-boundary tests; update existing worker/evidence tests | `tests/worker.rs`, `tests/evidence.rs` |
| 6 | Architecture docs, changelog, AI log | `docs/architecture/data-plane.md`, `CHANGELOG.md`, `docs/ai-transparency-log/` |

Responsible roles: Orchestrator, Implementation, Test Engineer, Code Reviewer.
No model or strategy behavior changes; no Quant Validator record is required.

## 8. Verification

The plan changes no behavior, so the rows below are `Pending` by design. The
real-kill row is accepted as deferred to the composed tier.

## Verification table

| ID | Plan item | Acceptance criterion | Evidence / command | Expected | Observed | Status | Reviewer | Date | Notes |
|---|---|---|---|---|---|---|---|---|---|
| V1 | Manifest | A successful run writes one manifest and no final per-name key | worker test | manifest only | worker test: manifest written, no per-name key | Pass | Orchestrator | 2026-11-14 | — |
| V2 | Partial write | A stop after N writes leaves no manifest and no run artifact | fail-injection test | nothing published | stopped write leaves blobs, no manifest, no run artifact | Pass | Orchestrator | 2026-11-14 | — |
| V3 | Fail-before | The old per-name layout fails the no-final-key assertion | regression test | fails before, passes after | the no-`runs/{id}/` assertion fails on the old layout | Pass | Orchestrator | 2026-11-14 | — |
| V4 | Evidence | Manifest, blobs, and DB list agree and the digest verifies | evidence test | succeeded | evidence test: manifest, blobs, and record agree | Pass | Orchestrator | 2026-11-14 | — |
| V5 | Rejection | Missing/changed blob and manifest/DB disagreement fail | evidence tests | failed | missing/changed/mismatch/disagreement tests fail | Pass | Orchestrator | 2026-11-14 | — |
| V6 | Digest | The canonical digest is unchanged | golden/unit test | equal | canonical digest tests unchanged | Pass | Orchestrator | 2026-11-14 | — |
| V7 | Gates | Tests, clippy, fmt, rustdoc, checkers | `tools/gate.sh` | pass | `tools/gate.sh` passed | Pass | Orchestrator | 2026-11-14 | — |
| V8 | Real kill | A real subprocess kill leaves nothing visible | composed test | deferred to #665 | real kill composed test deferred to #665 | Accepted risk | Orchestrator | 2026-11-14 | — |

## 9. Risks and recovery

- Unreferenced blobs accumulate until a retention policy exists. They are
  content-addressed, immutable, and reused by a retry; this plan records the
  follow-up rather than deleting shared bytes.
- The manifest is a new contract. The download route is not implemented; when it
  is, it resolves a name through the manifest and reads the blob.
- `put` atomicity is a contract on the adapter. S3 satisfies it; the in-memory
  fake satisfies it; a filesystem implementation must use temp file plus rename.
- A manifest written by an older worker is absent; the evidence path must fail
  closed (missing manifest) rather than fall back to per-name keys. Existing
  running runs from before the change are not migrated; there are no production
  runs yet.
- The composed kill test remains open; the fail-injection test proves the write
  boundary, not the process boundary.

## 10. Stop conditions

- A change requires altering the canonical digest, the run identity, or an
  engine result.
- The manifest cannot be made the sole visibility point without an artifact
  download contract change that needs a client decision.
- A real kill test is required before merge; it needs the Compose stack and is a
  separate issue.
- A new dependency is required.
