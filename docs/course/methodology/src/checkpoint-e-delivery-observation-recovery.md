# Checkpoint E - Delivery, Observation, Recovery

## Spoken script

The current release is v1.2.0. It ships as one image with six binaries, one published port, and nine containers. `make up` builds the stack, waits for health, and prints a URL. The desktop package installs from a release asset and starts the same six sidecars on SQLite, a filesystem CAS, and an in-process queue. It needs no Docker, and the shell reads no local engine data.

I observe the system in three places. `/health` reports each dependency, queue depth, worker count, and the image digest. Run detail reports the stage, provenance, canonical digest, and artifacts. `make verify` submits two equivalent runs through the gateway and prints one canonical digest. The image digest is part of the evidence identity: a different build is a different run. The run record is the source of truth. A dead-lettered message shows up as a failed run, not as a silent loss.

Here is one recovery story. I kill a worker in the middle of a run. The message is redelivered. The recompute is idempotent. No partial artifact becomes visible. A rerun gives the same digest. The queue also covers lease expiry, retry, dead-letter, and cancellation. If the evidence stage goes down after compute, the run stays running and finalization retries; it never becomes succeeded with a missing artifact.

The server profile and the desktop profile share the same six services and the same canonical digest. The release note records the v1.2.0 gate: 2,118 passing tests, 0 failed, 14 ignored, and 24 checkers. For backup, the archive contains Postgres and the object store; restore verifies every hash before it writes.

The limit is clear. There is no high availability and no failover. A host failure is an outage until I restart the stack. That is a deliberate trade for a single-operator tool. Second-day and second-machine reproducibility are reviewer-executed; the release record claims the local gate, the two equivalent runs on the release host, and the packaged parity smokes, not a second day. A digest also says nothing about the rule's edge. The desktop artifacts are unsigned.

When a client reconnects, the run record is the first place I look. The release note is the record I compare against. The next action is to run the reviewer verification on a second host and a second day, and record the result. The current evidence is the release-host two-run comparison, the packaged Linux and Windows parity smoke, and the queue and CAS tests.

```{=latex}
\newpage
```

## Evidence appendix

### Delivery

| Item | Observed |
|---|---|
| Delivery | one image, six binaries; one published port `8080`; nine containers: six services, Postgres, MinIO, RabbitMQ |
| Desktop | six sidecars, SQLite, filesystem CAS, in-process queue; installer from the release |
| Current release | v1.2.0; gate 2,118 passed, 0 failed, 14 ignored; 24 checkers |

### Observation

| Signal | Where | What I do |
|---|---|---|
| Service state | `/health`, per dependency | restart the failing dependency |
| Queue depth and workers | `/health` | add a worker |
| Run state and failed stage | run detail | fix the input, resubmit |
| Canonical digest | run detail, `make verify` | compare; a mismatch is a defect |
| Image digest | `/health`, provenance | a different build is a different run |

The run record is the source of truth. A dead-lettered message appears as a failed run, not as a silent loss.

### Recovery

| Failure | Recovery | Evidence |
|---|---|---|
| Worker killed mid-run | message redelivered; recompute is idempotent | queue and CAS suite; rerun digest is the same |
| Evidence stage down after compute | finalization retries | run stays `running`; never `succeeded` with a missing artifact |
| Database lost | restore the last backup | `tools/test_backup.sh` |
| Object store lost | restore from backup | `tools/restore.sh`; missing objects make runs unverifiable |
| Bad image | start the previous release digest (`v1.1.0`) | release note; `v0.9.0` is the server-platform baseline |

Limit: no high availability, no failover; second-day and second-machine reproducibility are reviewer-executed. Link: [current release note](../../../releases/2026-11-16/v1.2.0.md).
