# Checkpoint E - Delivery, Observation, Recovery

## Claim

**Claim type: Outcome.**

> For one researcher, a submitted run completes without the client and is
> reproducible from its own provenance, measured by comparing the canonical
> digest of two equivalent runs on the release host, with the limitation that
> second-day and second-machine reruns are reviewer-executed and not claimed as
> observed, and that this says nothing about the rule's edge.

## Delivery

- Released as v1.0.0: one image, six binaries, one digest, nine containers.
- One command: `make up` -> healthy -> a URL.
- One published port.
- A self-contained desktop package installs from a release asset, starts the
  same six sidecars on SQLite/filesystem CAS/in-process queue, and reads no
  local engine data.
- Release record: [`docs/releases/2026-11-15/v1.1.0.md`](../../../releases/2026-11-15/v1.1.0.md)
  (current tag `v1.0.0`). It records 2,134 passing tests; the earlier
  distributed-platform note (now `v0.9.0`) recorded 1,995.

## Observation

| Signal | Where | Action when it moves |
|---|---|---|
| Service state | `/health`, per dependency | restart the failing dependency |
| Queue depth, workers | `/health` | add a worker |
| Run state and failed stage | run detail | fix the input, resubmit |
| Canonical digest | run detail, `make verify` | compare; a mismatch is a defect |
| Image digest | `/health`, provenance | a different build is a different run |

The run record is the source of truth. A dead-lettered message appears as a
failed run, not as a silent loss.

## Recovery

| Failure | Recovery | Evidence it worked |
|---|---|---|
| Worker killed mid-run | message redelivered; recompute is idempotent | release note queue/CAS suite: lease expiry, redelivery, retry, dead-letter, cancellation, killed-writer invisibility; rerun digest verified |
| Evidence stage down after compute | finalization retries | run stays `running`, never `succeeded` with a missing artifact; release note records the run lifecycle and no-partial-publish guard |
| Database lost | restore the last backup | `tools/test_backup.sh`; release note backup/restore over Postgres and the object store |
| Object store lost | restore from backup | `tools/test_backup.sh` and `tools/restore.sh`; missing objects make runs unverifiable, and that is reported |
| Bad image | start the previous digest | release note rollback: server starts `v0.9.0`; migrations are forward-only, so the previous version must tolerate the newer schema |

## Limit

No high availability, no failover. A host failure is an outage until the operator
restarts the stack. That is a deliberate trade for a single-operator tool, and it
is stated rather than hidden. Second-day and second-machine reproducibility are
reviewer-executed; the release record claims the local gate, the two equivalent
runs on the release host, and the packaged parity smokes, not a second day.

## Decision

Shipped in v1.0.0: the server platform and the self-contained desktop package
share the same six services and the same canonical digest; the server profile
remains the default and the oracle. The thin shell, profiles, and desktop are
released, not deferred.

Next action: run the reviewer verification on a second host and a second day and
record the result; the current evidence is the release-host two-run comparison,
the packaged Linux/Windows parity smoke, and the queue/CAS tests.
