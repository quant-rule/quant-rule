# ADR — Desktop Distribution: Shell Now, Embedded Later, In-Process Never

- Date: 2026-11-12 (UTC)
- Status: Accepted
- Owner: Human
- Claim type: ADR (Architecture Decision)
- Related: [`01`](01-adopt-distributed-service-based-architecture.md), [`05`](05-svelte-only-web-client.md), [`08`](08-retire-static-download-site.md), [`10`](10-python-boundary.md)

## Context

The distributed stack installs with Docker. That is right for developers and for
a reviewer who must reproduce a run, and wrong for a casual user who wants a
double-click install. The distributed architecture is not negotiable.

Four shapes were considered.

| Shape | What it is | Keeps six services | Self-contained | Cost |
|---|---|---|---|---|
| **A. Thin shell** | A window over the gateway URL | yes | no | days |
| B. Shell + bundled daemons | Spawns six binaries plus Postgres, MinIO, RabbitMQ | yes | yes | very high, fragile |
| C. Shell + embedded backends | Spawns six binaries; SQLite, filesystem CAS, in-process queue | yes | yes | high |
| D. In-process app | One binary, engines linked in | **no** | yes | medium, destroys the architecture |

Discord is shape A: a web client in a native shell with a remote server. It is a
desktop app and it is not self-contained.

## Decision

**Ship A in v1.0.0. Design for C. Never do D.**

1. **Desktop shell.** `apps/desktop-shell/` is a Tauri 2 window that loads the
   gateway URL. It owns no IPC domain, no local file access, and no engine. It
   embeds the built Svelte assets only as a fallback when the gateway is
   unreachable.

2. **Adapter traits from Phase 1.** Database, object store, and queue sit behind
   traits. v1.0.0 ships the Postgres, S3, and RabbitMQ implementations only.

3. **Embedded desktop profile is v1.1.** SQLite, filesystem CAS, and an
   in-process queue implement the same traits. It is a deployment profile, not a
   second architecture: same six services, same planes, same determinism.

4. **Never in-process.** A single binary that links the engines and reads local
   files is the shape this programme removed. It does not return.

**Why this preserves the distributed claim.** Distribution is process boundaries
and network contracts, not Docker. A local profile with six sidecars over
localhost is still six services.

## Alternatives considered

| Alternative | Why rejected |
|---|---|
| Bundled Postgres, MinIO, RabbitMQ | Three daemons, three data directories, antivirus interference, no clean upgrade |
| Docker bundled inside the app | Requires a container runtime; the install problem returns |
| A maintained download page | Retired by ADR-08; GitHub Releases carries artifacts |
| Desktop as the primary client | The browser remains the zero-install path; the shell is additive |

## Consequences

- The thin shell is small: a window, a URL setting, an update check, and a build
  matrix in GitHub Actions.
- The adapter traits are a Phase 1 requirement, so the embedded profile is
  additive rather than a rewrite. Doing this after the fact is where desktop
  ports usually die.
- The desktop artifact contains no compute. One image still pins the compute
  stack; the shell is a separate artifact.
- The self-contained desktop is a real programme: a second SQL dialect, a
  filesystem CAS, an in-process queue, sidecar lifecycle, and a parity suite.
  `DIST-200` owns it and it does not gate v1.0.0.
- Signing and notarization become part of the release when Windows or macOS
  artifacts are published. Linux first.

## Revisit trigger

Revisit if the thin shell proves insufficient for a real research workflow, or if
an embedded database can be shipped without a dialect fork.

## Verification

- The shell loads the gateway URL and reads no local file.
- No IPC command boundary exists in the shell.
- The six service binaries are the only compute; the shell has no engine
  dependency.
- `docker run --rm <image> sh -c 'command -v python3'` is empty.
