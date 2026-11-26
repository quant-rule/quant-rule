# ADR-STR-001: Object DR — One-Way Archive, Not Replication

- Date: 2026-11-26 (UTC)
- Status: Accepted (2026-11-26 UTC)
- Issue: [#1227](https://github.com/quant-rule/quant-rule/issues/1227)
- Decision owner: Human
- Approval: human, D3 in [REM-000](../../execution-plans/2026-11-26/rem-000-v1.6.1-remediation.md), 2026-11-26 (UTC)
- Related: [MinIO cluster](../../config/2026-11-23/minio-cluster.md),
  [backup contract](../../config/2026-11-24/backup-restore.md),
  [ADR-HA-013](../2026-11-24/1111-backup-restore.md),
  [HA architecture](../../architecture/ha.md)
- Claim type: ADR (Architecture Decision)

## Context

The v3 profile runs one distributed MinIO deployment: four nodes × one drive
with EC:2 in the reference, three nodes × two drives with EC:3 in the lab. One
node or drive loss preserves the write quorum and every committed object.
Versioning, a least-privilege `qr_app` policy, and TLS are in place.

That protects an object inside the cluster. It does not protect a site loss, an
operator error that removes a bucket, or a cluster-wide corruption. The v1.6.0
plan named "object replication" as part of HA-013 without choosing between
MinIO bucket/site replication and an off-host archive. The contract now refuses
`backup.object_replication: true` because no second site exists, and
`tools/object-archive.sh` already implements a one-way off-host copy. The
decision was implicit; this ADR makes it explicit and records its boundary.

The deployment is a single-operator research platform on one cluster. It has no
second site, no second MinIO deployment, and no operator commitment to run one
in this release. A replication claim without those is a false HA claim.

## Decision

1. **Do not implement MinIO bucket or site replication in v1.6.1.** Keep
   `backup.object_replication: false`; the validator and check continue to
   refuse `true` as a replication claim without a second site.
2. **Use the one-way off-host archive as the object DR path.**
   `tools/object-archive.sh` lists and gets from the read-only
   `qr_archive_reader` identity and runs `mc mirror --overwrite --summary` to an
   operator-provided target. It has no `--remove`, no `--watch`, and no
   `mc replicate`.
3. **Restore through the drill.** `deploy/v3/dr.sh` restores the archive into a
   fresh scratch bucket before rows, reruns the same run/config, and fails on a
   canonical-digest mismatch. The in-place lab path remains destructive and
   `--confirm`-gated.
4. **Keep the contract authoritative.** The backup contract and the MinIO
   contract state the archive boundary; the descriptor keeps
   `object_replication` false by default and the check emits the no-replication
   row.
5. **State the release boundary forward.** The released `v1.6.0` note is not
   rewritten. The `v1.6.1` note must say archive-only, no replication, no second
   site, and no object-lock claim; the plan records that input for REL-003.
6. **Defer replication to a separate programme.** A second site needs its own
   ADR, descriptor fields, credentials, bandwidth and monitoring budget, tests,
   and a restore drill. It is not a silent extension of this release.

## Alternatives considered

| Alternative | Consequence | Rejected because |
|---|---|---|
| MinIO bucket replication | Continuous copy to a remote bucket | Needs a second site and credentials; no such target exists; it would turn an unverified path into an HA claim |
| MinIO site replication | Multi-site namespace and failover | Requires a second MinIO deployment and a new topology; out of the single-cluster release scope |
| No off-host object copy | Simpler operations | A site or cluster loss has no object recovery path; the archive already exists and is tested |
| One-way off-host archive | A recoverable copy without a second live site | Chosen: it matches the single-cluster deployment and makes no HA claim |
| Object lock or version-history archive | Ransomware/immutability story | The target is operator-owned and versioning is not part of the archive contract; a later programme can add it |

## Consequences

Positive:

- The object DR path is explicit, testable, and consistent with the descriptor,
  the check rows, and the tools.
- A site or cluster loss has a documented recovery path: restore the archive
  into a fresh bucket, then rows, then the digest drill.
- The release makes no replication, second-site, object-lock, or `Real HA` claim.

Negative:

- The archive is one-way and periodic; it is not continuous replication, and a
  key changed after the last run is not in the target.
- A target key removed by the operator stays removed; the archive does not
  delete, watch, or heal the target.
- Site loss recovery depends on the operator's target and the last successful
  archive run; RPO is the archive interval, not zero.
- A second site remains a separate programme with its own approval.

## Invariants

- `backup.object_replication` is false; a true value is refused before render.
- The archive source identity may only list and get; no delete, watch, or
  replicate flag exists.
- The archive target differs from the application bucket.
- The DR drill restores objects before rows and fails on a digest mismatch.
- No service, command, or release note claims replication, a second site, object
  lock, or `Real HA` for this path.

## Verification

`cargo test -p quantrule-deploy` covers the descriptor refusal, the check row,
the archive plan/confirmation gate, the distinct-bucket refusal, the absence of
a destructive or replication flag, and the DR plan/gates. `qr-governance check
--all` enforces the document caps. A live archive, restore, or DR drill is not
part of the fast gate; the 2026-11-25 in-place lab DR is `Measured`, and
`Real HA` remains HA-014.
