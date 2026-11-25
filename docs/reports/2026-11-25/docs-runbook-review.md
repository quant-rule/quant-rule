# Documentation and Runbook Review

- Date: 2026-11-25 (UTC)
- Status: Current; review of the living docs against the 2026-11-25 evidence
- Issues: [#1128](https://github.com/quant-rule/quant-rule/issues/1128) (DOC-001), [#1129](https://github.com/quant-rule/quant-rule/issues/1129) (DOC-002)
- Claim type: Behaviour Claim
- Related: [HA failure matrix](ha-failure-matrix.md), [cutover and restore](cutover-restore-drill.md), [soak and client parity](soak-upgrade.md), [security baseline](security-baseline.md), [v1.6.0 plan](../../execution-plans/2026-11-22/1082-v1.6.0-ra-ha.md)

## 1. DOC-001 — architecture, manual and command reference

| Area | Document | Check |
|---|---|---|
| Research artifacts and run identity | [`architecture/research-artifacts.md`](../../architecture/research-artifacts.md) | Current; links to the RA contracts |
| HA, failure and evidence boundary | [`architecture/ha.md`](../../architecture/ha.md) | Current; 2026-11-25 `Simulated HA`/`Measured`/`Unverified` split |
| Services and data/control planes | [`services.md`](../../architecture/services.md), [`data-plane.md`](../../architecture/data-plane.md), [`control-plane.md`](../../architecture/control-plane.md) | Current |
| Deployment | [`deployment.md`](../../architecture/deployment.md) | Current; rollback/PITR evidence updated |
| Security | [`security.md`](../../architecture/security.md) | Current; loopback local bind and remote token split |
| Manual chapters 15 and 16 | [`manual/15-ha-deployment.md`](../../manual/15-ha-deployment.md), [`manual/16-observability.md`](../../manual/16-observability.md) | Current; lab status updated |
| Command reference | [`manual/09-command-reference.md`](../../manual/09-command-reference.md), [`architecture/command-surface.md`](../../architecture/command-surface.md) | Same 61-command registry; checker passes |
| README | [`README.md`](../../../README.md) | Current; `make up`/`make remote` bind split |

The review found stale `Unverified` lines left by the HA-013/HA-014 lab and the
2026-11-25 Compose drills. They now name the measured or simulated evidence and
keep HA-22/HA-23 as the only real-host `Unverified` rows. The same pass carries
the #1180 loopback bind and the #1125 restore ownership/fresh-bootstrap
corrections into the architecture and manual.

## 2. DOC-002 — install, bootstrap, upgrade, failover and restore

| Runbook step | Where it is executable | Evidence |
|---|---|---|
| Install and bootstrap | [manual 15](../../manual/15-ha-deployment.md) steps 1-12, [`deploy/v3/README.md`](../../../deploy/v3/README.md) | `tools/stateful.sh`, `tools/swarm.sh`, `qr-deploy validate/render/check`; 2026-11-25 lab |
| Upgrade and rollback | Manual 15 step 13, manual 08 migrations, `tools/swarm.sh upgrade`/`rollback` | Scripts and the cutover/restore drill rollback; versioned upgrade is a release gate |
| Failover | Manual 15 Lab tier, [`chaos-matrix.md`](../../config/2026-11-24/chaos-matrix.md) | HA-014 `Simulated HA` rows; HA-22/23 real-host gate |
| Restore and PITR | Manual 08 backup/restore, [`backup-restore.md`](../../config/2026-11-24/backup-restore.md) | In-place lab DR `Measured`; Compose restore `Measured`; queued-run rebuild |
| Incident and recovery | Manual 15 Failure table, manual 12 troubleshooting, manual 08 | Each code has a cause and next action |

The operator can follow the install/bootstrap sequence from the document alone:
each step names the command, the host role, and the expected output. The restore
path is executable from `make backup` / `make restore FILE=…` and the
`tools/restore.sh --check` / `--force` pair. The v3 failure table covers the
preflight, signature, secret, stateful, queue, object and lab codes.

## 3. Gaps found and fixed

- The architecture and manual said the live backup/restore/PITR/archive/DR and
  HA-18..HA-23 were `Unverified`. The 2026-11-25 lab records 13 `Simulated HA`
  rows and a `Measured` in-place DR; the docs now say so and keep HA-22/HA-23
  `Unverified`.
- The manual 08 restore row described only a clean stack. It now names the
  `--force` fresh-bootstrap path and the fixed `qr_*` role requirement.
- The manual 15 failure row said the phase 3 archive drill was `Unverified`; it
  now names the 2026-11-25 in-place `Measured` drill.
- The local/remote bind split from #1180 is reflected in the README and security
  architecture.

## 4. Verification

| Command | Observed |
|---|---|
| `qr-governance check --all` | Pass (doc-length, command-surface, links, records) |
| `cargo test` / CI gate | Pass on the merged evidence PRs |
| `qr-deploy validate/render/check` | Pass; 180 lab rows |
| 2026-11-25 lab | 13 `Simulated HA`, 8 static, 2 real-host `Unverified` |
| 2026-11-25 Compose drills | Restore/queued-run/rollback, security and soak reports |

## 5. Boundary

- This is a documentation review; it does not re-run the v3 lab or a real remote
  client. It cites the measured 2026-11-25 evidence.
- A versioned rolling upgrade and the 72-hour soak remain release risks, recorded
  in the soak report.
- HA-22 and HA-23 are real-host gates; no document claims `Real HA`.
