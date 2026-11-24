# Release and Operations Workflow

- Date: 2026-11-07 (UTC)
- Status: Current
- Scope: release readiness, tagging, rollback, source-fetch operations, data-integrity incidents, and determinism incidents
- Owner: Release / Operations Verifier (readiness), Incident Responder (incident), Human (authorization)
- Contract: [`AGENTS.md`](../../AGENTS.md) sections 7 and 9
- Claim type: release and operations process; no model, strategy, or edge claim.

## 1. Purpose

The Release and Operations Workflow answers one question: **is this change safe
to release, and if it fails, how is it recovered?**

QuantRule is a self-hosted, single-operator research platform, so the operational
surface is small but includes a distributed deployment. It still has releases,
source fetches, data integrity, determinism, and rollback. This workflow keeps
those deliberate and recorded.

## 2. When to use

Use it when:

- preparing a release or tag;
- changing the release notes or version;
- rolling back a release;
- a source fetch fails or produces a corrupt file;
- a data-integrity incident is found;
- a determinism incident is found;
- an operational assumption is invalidated.

Do not use it when:

- the change is a defect: use the Audit Workflow;
- the change is a model or strategy hypothesis: use the Quant workflows;
- the change is a data source or partition: use Data Lifecycle.

## 3. Stages

```text
Readiness -> Approval -> Release -> Verify -> Operate -> Incident -> Learn
```

| Stage | Question | Owner | Output | Gate |
|---|---|---|---|---|
| 1. Readiness | Does the release meet the gate? | Release Verifier | Readiness record | Full gate passes |
| 2. Approval | Is the release authorized? | Human | Approval | Explicit human authorization |
| 3. Release | Is the tag and note created? | Release Verifier | Release note and tag | Changelog row present |
| 4. Verify | Does the released binary behave as expected? | Test Engineer | Verification record | The documented checks pass |
| 5. Operate | Is the local workspace healthy? | Release Verifier | Operational note | No unexplained failure |
| 6. Incident | What failed, and what is the impact? | Incident Responder | Incident record | Scope and evidence recorded |
| 7. Learn | What changes next? | Orchestrator | Follow-up issues | Route to Audit/Requirements |

## 4. Release readiness

A release is ready when:

- the full gate passes on the release commit;
- the composed tier has passed against the release commit: `make up`, then
  `tools/test_tiers.sh composed`, which checks the gateway and submits two
  equivalent runs through the gateway and compares their canonical digest;
- every registered port has a fixture and a comparison test, and
  `qr-governance check --checker port-parity` passes;
- the changelog release entry is present;
- the release note names the scope, the verification, and the risks;
- the release image digest, SBOM hash, CVE summary, and signature-bundle hash are
  recorded, and `cosign verify --key deploy/v3/cosign.pub --local-image` verifies
  the saved image;
- no Critical audit finding is open on the changed path;
- no unvalidated model or strategy change is included;
- the command surface, README, help, and tutorials agree;
- the documentation is current;
- the rollback or recovery approach is stated.

A green CI result is evidence, not approval. The human authorizes the release.

### Tag rule

- A release tag is `vMAJOR.MINOR.PATCH` and is annotated.
- It is created from `main` after the release pull request merges.
- The tag requires the full gate, the matching `CHANGELOG.md` section, the release note resolved through the current-tag map (`qr-governance release note --tag <tag>`), no open Critical finding on the changed path, and human authorization.
- A published tag is never moved, deleted, or reused. A bad release gets a new tag and a superseding release note.
- A local tag that was never pushed may be deleted before publication.
- Historical tags are retained. Do not create new tags in their format without a human decision.
- No date-based tags, no `latest`, and no tag per pull request.
- The release index and the full rule are in [`docs/releases/README.md`](../releases/README.md).

## 5. Operations

The local operational surface is:

- source fetch: network failure, partial file, rejected rows, batch cap;
- data integrity: gap, duplicate, out-of-order row, zero volume, volume unit
  change;
- determinism: a rerun that differs, a clock value in output, unordered
  iteration;
- disk: a full or unwritable output root;
- configuration: a missing or invalid file.

An operational incident is recorded in `docs/reports/<date>/` or
`docs/audits/<date>/` and routed to the owning workflow. It is not fixed inside
the release note.

## 6. Roles and boundaries

| Role | Responsibility | Boundary |
|---|---|---|
| Release Verifier | Readiness, release note, rollback | No deployment without authorization |
| Incident Responder | Triage, evidence, containment recommendation | No destructive action without authorization |
| Test Engineer | Verification evidence | No weakening a check |
| Human | Authorizes release, rollback, and recovery | Sole final authority |

## 7. Human gates

| Gate | Human action |
|---|---|
| Release | Authorize the tag and release |
| Rollback | Authorize the rollback |
| Data repair | Authorize any data-changing action |
| Incident closure | Accept the residual risk |

## 8. Green criteria

A release is green when:

- the full gate passes;
- the composed tier has passed against the release commit;
- the port-parity registry check passes and every registered port matches its
  fixture;
- the changelog release entry and release note are present;
- the image digest, SBOM hash, scan summary, and signature-bundle hash are present;
- the Sigstore bundle and saved image verify against `deploy/v3/cosign.pub`, or the note is
  explicitly `pending` before `Status: Released`;
- the verification record is present;
- no Critical finding is open on the changed path;
- the rollback or recovery approach is stated;
- the human authorizes the release.

An incident is green when:

- the impact, timeline, and evidence are recorded;
- containment and recovery are recorded;
- the follow-up work is routed;
- the human accepts the residual risk.

## 9. Artifacts and records

| Artifact | Path |
|---|---|
| Release note | Resolved by `qr-governance release note --tag <tag>`; normally `docs/releases/<date>/<tag>.md` |
| Release index and tag rule | `docs/releases/README.md` |
| Verification record | `docs/reports/<date>/<scope>-verification.md` |
| Incident record | `docs/reports/<date>/<scope>-incident.md` or `docs/audits/<date>/` |
| Changelog | `CHANGELOG.md` |
| Registry and supply chain | `docs/config/2026-11-24/registry.md` |
| AI log | `docs/ai-transparency-log/<date>/<issue>-<slug>.md` |

## 10. Stop conditions

Stop and request human direction when:

- a gate fails;
- a Critical finding is open;
- an unvalidated model or strategy change is included;
- a destructive recovery action is required;
- the release note and the diff disagree;
- the rollback path is unknown;
- the human has not authorized the release.

## 11. Handoff

| To | Packet |
|---|---|
| Audit Workflow | A defect found during release or operations |
| Requirements & Design | A new capability needed to prevent the incident |
| Data Lifecycle | A data defect or source change |
| Recordkeeping | The release tag, incident ID, and follow-up issues |

## 12. Related documents

- [`audit-workflow.md`](audit-workflow.md)
- [`requirements-design.md`](requirements-design.md)
- [`data-lifecycle.md`](data-lifecycle.md)
- [`deprecation-retirement.md`](deprecation-retirement.md)
- [`../../AGENTS.md`](../../AGENTS.md)
- [`../../README.md`](../../README.md)

## Distributed release

The release unit is the server image; the desktop shell and its GitHub Release
assets are retired. GitHub Releases carries the image digest, the SBOM, the CVE
report, and the release note; there is no download page.

| Step | Evidence |
|---|---|
| 1. Gate | `tools/gate.sh` passes |
| 2. Image | Built once for `linux/amd64` from `release-binaries-<sha>`; staging digest recorded |
| 3. SBOM | syft SPDX JSON from the exact staging digest |
| 4. CVE gate | Grype plus `cargo audit`; Critical and fixable High fail before the version image tag |
| 5. Version image | `vX.Y.Z` and `sha-<commit>` pushed; digest equals the scanned staging digest |
| 6. Contract check | `/version` reports the product version; `/health` reports the digest |
| 7. Verification | Composed tier and a clean-host `make verify` |
| 8. Signature | Operator records `cosign.bundle`, `attestation.bundle`, and `cosign save` `signed-image/`; `tools/release.sh verify` passes from the saved image |
| 9. Note | The five supply-chain fields carry the digest and evidence hashes; PLAT-008 blocks the 16 fixture infra pins |
| 10. Tag | Annotated `vMAJOR.MINOR.PATCH` from `main` after the release PR merges; human authorization |

A `workflow_dispatch` with `publish_snapshot` pushes, scans, and records a
`snapshot-<sha>` digest before the tag exists; that is the pre-tag GHCR proof. A
tag run pushes the version image only after the scan, attaches the SBOM and CVE
report, and writes the digest, hashes, manual signing step, and fixture-pin
blocker into the release body.

Rollback is the previous verified image digest. Migrations are forward-only; a
rollback never reverts the schema, so the previous version must tolerate the
newer one. For the first image release there is no earlier digest: recovery is
`make reset` and a rebuild from a known commit.

The retired static download page is recorded in `ADR-2026-11-12-08`.
