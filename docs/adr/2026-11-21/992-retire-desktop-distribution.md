# ADR — Retire the Desktop Distribution and Embedded Profile

- Date: 2026-11-21 (UTC)
- Status: Accepted
- Owner: Human
- Claim type: ADR (Architecture Decision)
- Supersedes: [`2026-11-12/12-desktop-distribution.md`](../2026-11-12/12-desktop-distribution.md)
- Plan: [`992-development-speed-v1.5.0.md`](../../execution-plans/2026-11-21/992-development-speed-v1.5.0.md)
- Epic: #992

## Context

The desktop release path is the slowest and most failure-prone part of a
release. In the v1.4.1 recovery run, Linux spent 11 minutes and Windows 18
minutes before failing 8–17 second smoke tests. The desktop package is not on
the HA path: the target topology is a browser client reaching one HTTPS name
through a load balancer.

The Tauri shell, its supervisor, the installers, and the embedded profile also
keep SQLite, `rusqlite`, a filesystem CAS, and an in-process queue alive. That is
a second deployment shape with no remaining product consumer.

## Decision

**Retire the desktop distribution completely.**

- Delete `apps/desktop-shell/**`: Tauri app, supervisor, UI, icons, scripts,
  package tests, and staged sidecars.
- Delete the `desktop` release job, matrix, artifacts, smoke tests, and
  release-note text.
- Delete `QR_PROFILE=desktop`, `QR_DATA_DIR`, SQLite adapters and migrations,
  `rusqlite`, `FilesystemStore`, and `InProcessQueue`.
- Delete `tools/check_schema_prefixes.py`, its test, and
  `tools/parity_desktop.sh`.
- Keep the browser GUI, `crates/quantrule-shell`, the Linux server image, and
  Windows/Linux browser access.
- Historical releases and assets are immutable.

## Alternatives considered

| Alternative | Why rejected |
|---|---|
| Keep the Tauri source and defer packaging | Leaves dead code and a second client surface |
| Keep the embedded profile as a developer mode | No product consumer; keeps SQLite and its tests alive |
| Split and cache the desktop build | Still 8–10 minutes and still a second release artifact |
| Drop only Windows | Linux remains a client wrapper with no HA role |

## Consequences

- The no-Docker single-machine mode disappears. Local use is Compose on Linux
  or Docker Desktop on Windows.
- The release loses two jobs and the 18-minute blocker.
- The gate loses one checker: 27 → 26.
- The Rust build loses bundled SQLite.
- macOS is unsupported in v1.5.0; it can return as a separate browser-client
  decision later.
- SQLite-only coverage must be replaced before deletion; no live store contract
  loses its only test.

## Verification

- `test ! -e apps/desktop-shell`.
- No Tauri, AppImage, NSIS, `QR_PROFILE=desktop`, SQLite, or `rusqlite` in the
  current tree.
- Release publishes the image and note only.
- Historical release tags and assets are unchanged.
