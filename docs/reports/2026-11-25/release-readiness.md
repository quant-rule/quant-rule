# v1.6.0 Release Readiness

- Date: 2026-11-25 (UTC)
- Status: Release candidate; no release tag exists
- Issue: [#1131](https://github.com/quant-rule/quant-rule/issues/1131) (DOC-004)
- Claim type: Behaviour Claim
- Related: [v1.6.0 plan](../../execution-plans/2026-11-22/1082-v1.6.0-ra-ha.md), [Composed tier](composed-tier.md), [HA failure matrix](ha-failure-matrix.md), [cutover and restore](cutover-restore-drill.md), [security baseline](security-baseline.md), [soak and client parity](soak-upgrade.md), [docs review](docs-runbook-review.md)

## 1. Technical evidence

| Area | Evidence | Tier |
|---|---|---|
| Fast gate | CI `Gate` green on #1177/#1180 and the docs PRs; `qr-governance check --all` pass | Fast |
| Composed tier | Two equivalent runs, 13 canonical artifacts each, digest `sha256:a0372f02…`; queue isolation, four DBs/roles/no cross-DB, 12 Postgres targets | Composed |
| HA failure matrix | 13 `Simulated HA` rows, 8 static alert/check rows, HA-22/HA-23 `Unverified` | Simulated HA |
| Backup/restore and cutover | Compose archive/restore, 29/29 artifacts, queued-run rebuild, rollback digest parity; lab in-place DR `Measured` (RPO 2 s, 219 s total) | Measured / Simulated HA |
| Security | No open Critical; local bind fixed to `127.0.0.1:8080`, remote token matrix, roles/TLS/secrets checked | Measured |
| Soak, scale, clients | 6 cycles over 21 min, 1-vs-2-worker digest parity, shell/browser gateway-restart recovery | Measured / sample |
| Documentation | Architecture, manual 15/16, command reference, runbooks, support matrix and licence boundary | Docs |

## 2. Open release gates

| Gate | State | Why it stays open |
|---|---|---|
| HA-22 power loss/reboot on real hosts | `Unverified` | Requires the operator-only real-host gate |
| HA-23 AZ/disk/network failure | `Unverified` | AZ1 MinIO/ingress limitation is a human-accepted risk, but the row is not a pass |
| 72-hour soak | Not run | The 2026-11-25 sample is 6 cycles over about 21 minutes |
| Versioned rolling upgrade | Not run | No release image exists; the cutover/rollback drill is the closest evidence |
| SBOM, CVE scan, image signature | Not produced | The release workflow produces them at tag time |
| Licence | Resolved | MIT OR Apache-2.0 chosen and declared in the crate metadata, image labels, and LICENSE-MIT/LICENSE-APACHE |
| Real remote TLS client | Not run | The v3 lab is `Simulated HA`; the remote-client row is `Unverified` |

## 3. Tag checklist

The human authorization opens this sequence:

1. Create the release PR with the `## [1.6.0] - 2026-11-25` changelog section and
   `docs/releases/2026-11-25/v1.6.0.md`.
2. Run the release workflow on `main`: full gate, staging image, SPDX SBOM,
   Grype/`cargo audit`, offline cosign signature, version tag only after the
   scan passes.
3. Verify the tag digest equals the scanned staging digest; publish the GitHub
   release with the SBOM, CVE report, and signature bundle.
4. Record the release note, changelog, and tag in the board and close DOC-004.

Rollback remains the previous published image (`v1.5.2`) and the previous
volumes/stack; no schema downgrade is performed.

## 4. Recommendation

The technical evidence is complete for a **v1.6.0 release candidate** with the
gates above recorded honestly. It does not justify a `Real HA` claim, a 72-hour
soak claim, or a versioned-upgrade claim. The human must accept the remaining residual
risks and authorize the tag; the release workflow then produces the supply-chain
evidence.
