# Execution Plan — Multi-Host Deployment Profile (#959)

- Status: Approved
- Approved: 2026-11-20
- Claim type: Behaviour Claim
- Owner: Human
- Workflow: [Execution Plan](../../workflows/execution-plan.md) + [Requirements & Design](../../workflows/requirements-design.md)
- Records: #958 (ADR), #959 (this plan), #960–#965 (profile), #966–#970 (desktop install/docs)
- ADR: [`958-multi-host-deployment.md`](../../adr/2026-11-20/958-multi-host-deployment.md)
- Branch: `arch/958-multi-host`

## 1. Intent

Make the six-service server stack deployable across several hosts from one
versioned descriptor, with only the gateway reachable from the public side, one
image digest on every host, and the same canonical digest as the single-host
profile.

The engines already take URLs and environment variables. The work is a
deployment profile: a descriptor, a validator/renderer, per-role Compose files,
runtime hardening for a host boundary, backup/restore, tests, and docs. It is
not a second architecture and it does not change model or strategy behavior.

## 2. Current state

| Area | Observed | Evidence |
|---|---|---|
| Topology | Six services, six binaries, one image, one Compose stack | `compose.yaml`, `docs/architecture/services.md` |
| Public boundary | Only the gateway publishes `:8080`; the rest stay on the bridge network | `docs/architecture/deployment.md` §2 |
| Split host | Server runs all nine containers; clients run none | `docs/architecture/deployment.md` §6 |
| Multi-host | Explicitly not provided | `docs/architecture/deployment.md` §8 |
| Configuration | Environment only; no deployment config file | `docs/architecture/deployment.md` §4 |
| Service URLs | `QR_DATA_URL`, `QR_MODEL_URL`, `QR_EVIDENCE_URL`, `QR_SIMULATION_URL`, `QR_DATABASE_URL`, `QR_OBJECT_STORE_URL`, `QR_QUEUE_URL` | `crates/quantrule-runtime/src/config.rs` |
| Trust boundary | Host and container network trusted; LAN/internet only the gateway behind a gate | `docs/architecture/security.md` §1–2 |
| Image identity | `QR_IMAGE_DIGEST` is required for a run/stage and recorded in provenance | `docs/architecture/deployment.md` §4 |
| Tests | Fast and composed tiers exist; no multi-host tier | `tools/test_tiers.sh` |

The gap is deployment, not domain logic. `compose.yaml` hardcodes service names
and same-host `depends_on`; internal services have no application
authentication; no descriptor exists; no host-boundary test exists.

## 3. Scope and non-goals

In scope:

- `quantrule.deploy/v2` descriptor, schema, example, and `qr-deploy`.
- Per-role Compose files for gateway, core, compute, and infrastructure.
- Private-overlay network gate, private-IP binds, firewall guidance.
- One image digest across hosts; distribution with `docker save`/`load`.
- Runtime hardening: startup/retry, healthchecks, `QR_INTERNAL_ONLY`, migrations.
- Multi-host backup and restore.
- Local multi-network harness and a real three-host LAN verification.
- Architecture, manual, agent runbook, and deployment README.

Non-goals:

- Cloud provider deployment, HA, autoscaling, Kubernetes, mTLS, multi-region.
- RPM and DEB packaging.
- Model or strategy behavior changes.
- Multiple model hosts behind a load balancer; one model endpoint in v1.
- Windows multi-host servers; the server profile is Linux.

## 4. Assumptions and uncertainty

- Three Linux hosts or VMs are available for the real LAN verification. If not,
  the local harness runs and the LAN row is `Blocked`, not `Pass`.
- The existing image builds and runs; no new Rust dependency is needed for
  `qr-deploy` beyond the workspace's existing `serde_json`.
- The private overlay is WireGuard; Tailscale is documented as an alternative.
- The operator supplies firewall rules and secrets; the descriptor validates
  placement, not the host firewall.
- Cross-host latency is LAN-like; WAN/cloud placement is not claimed.

## 5. Decisions

| Decision | Value | Status |
|---|---|---|
| Network gate | WireGuard; Tailscale alternative | Approved |
| Image distribution | `docker save`/`load` over SSH, one pinned digest | Approved |
| Descriptor | JSON, `quantrule.deploy/v2` | Approved |
| Tool | Rust `qr-deploy` (`validate`, `render`, `check`) | Approved |
| v1 scope | Hardened: backup/restore, failure tests, bind guard | Approved |
| Model scaling | One model endpoint in v1 | Approved |
| Internal bind guard | `QR_INTERNAL_ONLY=1` refuses a non-private bind | Approved |

## 6. Acceptance criteria

- `qr-deploy validate` accepts the example and rejects every unsafe case.
- Gateway/core/compute/infra roles start on separate hosts.
- Only the gateway port is publicly reachable.
- The private overlay is required to reach an internal service.
- A run submitted from the laptop completes; stages are `done`.
- The multi-host canonical digest equals the single-host digest.
- One and two backtest workers produce the same digest.
- A killed worker leaves no partial artifact and no `succeeded` run.
- Concurrent migrations succeed; a rollback to the previous digest works.
- Backup/restore reproduces the digest.
- Docs checkers and the full gate pass.
- Release note and changelog are updated.

## 7. Phases and work breakdown

| Phase | Work | Issues | Estimate |
|---|---|---|---|
| 0 | ADR, plan, issues, worktrees, schema freeze | #958, #959 | 1–2 d |
| 1 | Descriptor and `qr-deploy` | #960 | 2–3 d |
| 1 | Per-role Compose, infra, image distribution | #961 | 2–3 d |
| 1 | Runtime hardening | #962 | 1–2 d |
| 2 | Backup and restore | #963 | 1 d |
| 2 | Multi-host test tier and LAN verification | #964 | 3–4 d |
| 2 | Architecture, manual, agent docs | #965 | 2 d |
| 2 | Desktop install and agent runbook | #966–#970 | 4–6 d |
| 3 | Fixes, full gate, release PR | #959 | 2–3 d |
| 4 | Release and stabilization | #959 | 1–2 d |

Parallel tracks:

| Track | Owns | Starts after |
|---|---|---|
| 1 | `crates/quantrule-deploy/**`, descriptor example/schema | #958 approved |
| 2 | `deploy/multi-host/compose*.yaml`, `infra/**`, deployment README | schema freeze |
| 3 | `crates/quantrule-runtime/**`, service `main.rs`, guard tests | #958 approved |
| 4 | `docs/manual/13-desktop.md`, AppImage install evidence | — |
| 5 | `docs/manual/agent-runbook.md`, Windows/LAN/remote tutorial docs | — |
| 6 | `deploy/multi-host/test/**`, `tools/test_tiers.sh` | schema freeze |

One owner per file. Shared files (`README.md`, `CHANGELOG.md`, manual index) are
serialized in the closure PRs.

## 8. PR map

| PR | Branch | Contents | Merge after |
|---|---|---|---|
| PR-B1 | `arch/958-multi-host` | ADR, this plan, AI log | Human approval |
| PR-B2 | `feat/960-deploy-tool` | `qr-deploy`, schema, example, tests | PR-B1 |
| PR-B3 | `feat/961-multi-host-artifacts` | Compose, infra, image distribution, README | PR-B1 |
| PR-B4 | `feat/962-multi-host-runtime` | startup/health, `QR_INTERNAL_ONLY`, migration tests | PR-B1 |
| PR-B5 | `feat/963-multi-host-backup` | backup/restore scripts and procedure | PR-B3 |
| PR-B6 | `test/964-multi-host-tier` | harness, LAN/failure/scale tests | PR-B2–B4 |
| PR-B7 | `docs/965-multi-host-docs` | architecture, manual, agent docs | PR-B3–B6 |
| PR-A1 | `docs/966-desktop-install` | manual chapter 13, AppImage evidence | IN-1.1 |
| PR-A2 | `docs/969-agent-runbook` | agent runbook, Windows, LAN, remote fix | — |
| PR-A3 | `docs/970-install-closure` | index, README, changelog, checks | PR-A1, PR-A2 |
| PR-R | `chore/959-multi-host-release` | changelog, release note, version | all |

## 9. Verification

Every PR runs `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D
warnings`, `cargo test`, `tools/gate.sh`, `python3 tools/check_manual.py`, and
`python3 tools/check_doc_length.py`. Multi-host PRs add
`qr-deploy validate`, `qr-deploy check`, `tools/test_tiers.sh composed`, and
`tools/test_tiers.sh multi-host`.

## Verification table

| ID | Plan item | Acceptance criterion | Evidence / command | Expected | Observed | Status | Reviewer | Date | Notes |
|---|---|---|---|---|---|---|---|---|---|
| V01 | ADR | ADR approved before code | ADR record | Approved | Approved 2026-11-20 | Pass | Human | 2026-11-20 | — |
| V02 | Descriptor | Valid example passes; unsafe cases fail | `cargo test -p quantrule-deploy` | Typed errors | 26 passed, 0 failed | Pass | Test | 2026-11-20 | Hardening #980 |
| V03 | Image identity | One digest on every host | `qr-deploy check` | All equal | Local harness sets one digest; real LAN blocked | Blocked | Test | 2026-11-20 | Needs the three hosts |
| V04 | Public boundary | Only the gateway is public | Port scan / firewall test | One port | — | Blocked | Security | 2026-11-20 | Local harness: only the gateway published; host firewall blocked |
| V05 | Internal boundary | Internal services need the overlay | Network test | Unreachable without overlay | — | Blocked | Security | 2026-11-20 | Overlay isolation needs the three hosts |
| V06 | Auth | Gateway returns 401 without token | `curl` matrix | 401, 401, 200 | — | Blocked | Test | 2026-11-20 | Token matrix needs the deployed gateway |
| V07 | Run | `verify-mini` completes across hosts | Stage output | Stages `done` | Local harness: 13 artifacts, 3,560,384 bytes | Pass | Test | 2026-11-20 | Local multi-network approximation |
| V08 | Parity | Multi-host digest equals single-host | `verify` | Same digest | `sha256:a0372f02…` matches the documented single-host digest | Pass | Test | 2026-11-20 | — |
| V09 | Scale | One vs two workers same digest | `verify` | Same digest | Both runs `sha256:a0372f02…` | Pass | Test | 2026-11-20 | — |
| V10 | No partial publish | Killed worker leaves no visible run | Failure test | No `succeeded` run | — | Blocked | Test | 2026-11-20 | Failure test needs the LAN workers |
| V11 | Migrations | Concurrent start succeeds | Start-all test | No lock failure | — | Blocked | Test | 2026-11-20 | Concurrent cross-host start needs the LAN |
| V12 | Rollback | Previous digest restores | Upgrade test | Healthy | — | Blocked | Release | 2026-11-20 | Rollback needs the LAN |
| V13 | Backup | Restore reproduces the digest | Restore test | Same digest | — | Blocked | Data Steward | 2026-11-20 | Live backup/restore needs the LAN |
| V14 | Docs | Checkers and gate pass | `tools/gate.sh` | Green | `workspace gate passed`; 27 checkers; 14-chapter manual | Pass | Docs | 2026-11-20 | Gate on `main` at `b78e674a` |
| V16 | Security | Blocking review findings closed | #980 tests and Compose config | Negative tests pass | 26 deploy tests; Compose config tests; runtime redaction | Pass | Security | 2026-11-20 | Remaining risks documented |
| V17 | AppImage | Installed app opens from the menu | `gtk-launch quantrule`; `/ready` | Window opens | AppImage installed; `/ready` healthy | Pass | Test | 2026-11-20 | Local desktop profile |
| V15 | Release | Release note and changelog | Review | Present | — | Pending | Release | — | — |

## 10. Risks and recovery

| Risk | Control | Recovery |
|---|---|---|
| Internal service exposed | Validator, `QR_INTERNAL_ONLY`, firewall, Security review | Stop the host service, rotate secrets, record an audit finding |
| Mixed image digests | One pinned digest; `qr-deploy check` | Stop the run path; redeploy the correct digest |
| Single infrastructure host fails | Explicit non-goal; backup/restore | Restore on a new infra host |
| Startup ordering | Restart policies, start order, tests | Restart in order; read `/ready` |
| Parallel file conflicts | Ownership matrix | Serialize the PRs |
| LAN hardware unavailable | Local harness | Mark the LAN row `Blocked`; do not claim Pass |
| Scope creep | ADR non-goals | Stop and request a new decision |

## 11. Stop conditions

- ADR or plan rejected.
- Security Critical finding open.
- Digest mismatch in the LAN run.
- A public internal port cannot be closed.
- Required LAN hosts are unavailable for the real verification.
- A change requires cloud, HA, Kubernetes, mTLS, RPM/DEB, or a model/strategy
  decision outside this plan.

## 12. Authorization

This plan authorizes implementation, commits, pushes, and pull requests for
#958–#970. Merging, tagging, and release publication remain separate human
gates.
