# AI Transparency Log — Multi-Host Deployment Profile (#959)

- Date: 2026-11-20 (UTC)
- Work item: #958 ADR, #959 plan, #960–#965 profile, #966–#970 desktop install/docs
- Tool: OpenCode agent (DeepSeek V4.1 Flash 1017)

## Intent and boundary

The operator approved a multi-host server deployment profile and the desktop
install documentation. The agent drafted the ADR, the execution plan, the issue
tree, and the implementation tracks. It did not approve the ADR, merge a pull
request, publish a release, or claim a verification result that was not run.

## Work performed

- Read the existing architecture, deployment, security, service, ADR, and
  execution-plan documents.
- Created the issue tree #958–#970 with claim types and board fields.
- Drafted the ADR and this plan from the approved conversation.
- Prepared the parallel worktree and PR map.

## Verification and evidence

- Repository preflight: clean `main` at `fc2f5d97`; no other worktrees.
- GitHub CLI authenticated as `zantgo` with project scope.
- Issues created and added to the QuantRule project.
- ADR and plan are documentation only; no test result is claimed here.
- The implementation verification is recorded in the plan's verification table
  and in each pull request.

## Learning and corrections

- The first draft treated "split host" as configuration only. The review
  corrected that: a supported multi-host profile needs a descriptor, a
  validator, a network gate, tests, and an ADR.
- RPM and DEB were rejected because the operator uses Fedora and the locked
  decision is AppImage plus a user-level desktop entry.
- The packaged Tauri app starts local sidecars, so it is not a remote client;
  the manual must direct remote users to the browser or shell.
- The remote-compute tutorial omitted `QUANTRULE_API_URL`; that is recorded as a
  documentation defect to fix in #969.

## Implementation evidence (2026-11-20)

The implementation ran in parallel worktrees. The agent disclosed sequential and
parallel passes; no independent human review is claimed.

| Track | Pull request | Evidence |
|---|---|---|
| ADR and plan | #971 | Checker gate and link checks pass |
| `qr-deploy` | #976 | 18 unit tests; validator and render exercised against the example |
| Compose and infra | #974 | `docker compose config` for all four role files; checker gate pass |
| Runtime guard | #973 | 104 runtime tests; clippy clean |
| Backup wrappers | #978 | `bash -n`; `--help`; application-stopped refusal; `COMPOSE_ENV_FILES` config check |
| Multi-host harness | #979 | Two runs, same digest `sha256:a0372f02…`, one and two workers |
| Desktop install | #972 | Chapter 13; checkers pass |
| Agent runbook and LAN | #975 | Runbook, Windows snippet, remote tutorial; checkers pass |
| Multi-host docs | #977 | Architecture, manual 14, public boundary; checkers pass |

Desktop AppImage evidence:

- Built `QuantRule_1.3.0_amd64.AppImage` (175,839,736 bytes).
- `check-package.sh` -> `package ok: shell, supervisor, six sidecars, client, no interpreter`.
- Installed under `~/Applications` with a user-level `.desktop` entry and icon.
- `gtk-launch quantrule` opened the window; `/ready` reported sqlite, data,
  model, evidence, and simulation healthy; package digest
  `sha256:071007a5…`.

Multi-host local harness evidence:

- `deploy/multi-host/test.sh` started the six services on one Docker network
  with separate addresses and explicit URLs.
- Only the gateway published a host port.
- Two verification runs, one with one worker and one with two workers, printed
  the same canonical digest `sha256:a0372f02db7bc65460906eba3242943da8c56f26c46fde470726cee856bc4cea`
  (13 canonical artifacts, 3,560,384 bytes each).

The real three-host LAN rows (host firewall, overlay isolation, cross-host
failure, live backup/restore) remain blocked until the hosts exist. They are not
claimed as passing.

## Security hardening (2026-11-20)

The independent security review of the integrated profile requested changes.
The agent fixed the blocking findings in #980:

- public/unspecified/IPv6 hosts, external endpoints, template secrets,
  placeholder digests, port zero, and unsupported role splits are rejected;
- the infrastructure Compose file requires `QR_BIND_IP` and the role files use
  `pull_policy: never` and healthy-state healthchecks;
- `qr-deploy check` no longer sends the bearer token on open probes and reports
  the backtest worker as `SKIP`;
- the runtime rechecks the internal bind at the serve boundary and redacts
  secrets from `Config` debug output;
- the backup wrappers source `infra.env` so custom database, bucket, and queue
  names reach the archive tools.

Verification: 26 `quantrule-deploy` tests, 18 runtime config tests, clippy,
`tools/checkers.sh`, and Compose config tests for all four role files. The real
three-host LAN rows remain blocked.

## Closure evidence (2026-11-20)

- PRs #971–#982 are merged; issues #958–#970 and #980 are `Done`.
- The workspace gate passed on `main` at `b78e674a` with
  `CARGO_TARGET_DIR=target/gate`; the checker gate reports 27 checkers and the
  manual reports 14 chapters.
- The Linux AppImage was built, checked, installed under `~/Applications`, and
  opened with `gtk-launch quantrule`; `/ready` reported sqlite, data, model,
  evidence, and simulation healthy.
- The local multi-host harness printed the same canonical digest with one and
  two backtest workers: `sha256:a0372f02…`.
- The real three-host LAN rows (host firewall, overlay isolation, cross-host
  failure, live backup/restore) remain `Blocked`; no host was available.
- Release note, version bump, and tag are not created. That is a separate
  release decision.
