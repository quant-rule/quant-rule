# Checkpoint E - Delivery, Observation, Recovery

## Claim

**Claim type: Outcome.**

> For one researcher, a submitted run completes without the client and is
> reproducible from its own provenance, measured by comparing the canonical
> digest of a rerun on a second day, with the limitation that this says nothing
> about the rule's edge.

## Delivery

- One image, six binaries, one digest.
- One command: `make up` -> healthy -> a URL.
- One published port.
- A desktop shell installs from a single release asset and reads no local data.

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
| Worker killed mid-run | message redelivered; recompute is idempotent | no artifact visible; rerun gives the same digest |
| Evidence stage down after compute | finalization retries | run stays `running`, never `succeeded` with a missing artifact |
| Database lost | restore the last backup | every artifact hash verifies |
| Object store lost | restore from backup | missing objects make runs unverifiable, and that is reported |
| Bad image | start the previous digest | migrations are forward-only; the previous version must tolerate the newer schema |

## Limit

No high availability, no failover. A host failure is an outage until the operator
restarts the stack. That is a deliberate trade for a single-operator tool, and it
is stated rather than hidden.

## Decision

Ship the thin shell and profiles with the first release; keep the self-contained
desktop later, behind the adapter traits. Next action: prove the delivered run
survives a client close and a worker kill, then verify the digest on a second
machine.
