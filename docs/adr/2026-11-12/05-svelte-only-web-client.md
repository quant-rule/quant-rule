# ADR — Svelte-Only Web Client

- Date: 2026-11-12 (UTC)
- Status: Accepted
- Owner: Human
- Claim type: ADR (Architecture Decision)
- Supersedes: the Tauri desktop surface of [`2026-11-11/adopt-local-first-tauri-platform.md`](../2026-11-11/adopt-local-first-tauri-platform.md)
- Related: [`01`](01-adopt-distributed-service-based-architecture.md), [`12`](12-desktop-distribution.md)

## Context

Once the pipeline runs behind a gateway, the desktop application becomes a pure
HTTP consumer. A native shell then adds a second transport path, a second build,
and a second set of parity rules for no functional gain.

## Decision

The human-facing client is a **Svelte 5 single-page application**, built with
Vite, served by the gateway at the same origin as the API.

- Location: `apps/web/`.
- Transport: `fetch` to `quantrule.api/v1`. No IPC, no native bridge.
- **Migrate, do not rewrite.** The Svelte source in `apps/desktop/src` moves to
  `apps/web/src` before anything is deleted. The `api.ts` seam keeps its
  signature; the body becomes `fetch`. Panels keep their markup and lose their
  path props.
- The Tauri shell (`apps/desktop/src-tauri`) and the Tauri dependency are
  deleted only after the move merges.
- The mock backend becomes an HTTP fixture router so browser tests stay
  deterministic.
- The shell remains the agent interface and speaks the same API.

### Deletion order

| Order | Removed | By |
|---|---|---|
| 1 | `site/` | `DIST-016` |
| 2 | Svelte source moved, not deleted | `DIST-050` |
| 3 | `apps/desktop/src-tauri/` | `DIST-050` |
| 4 | empty `apps/desktop/` | `DIST-050` |

A deletion never precedes its replacement.

### Amendment — 2026-11-12

A **Tauri shell** returns as an optional desktop client, and it is not the
retired application. The retired artifact embedded `quantrule-app`, owned an IPC
command boundary, and read the local filesystem. The shell loads the gateway URL,
owns no domain boundary, and reads nothing local. It is recorded in
[`12`](12-desktop-distribution.md).

The Svelte client remains the single implementation of the interface. The shell
is a window onto it.

## Alternatives considered

| Alternative | Why rejected |
|---|---|
| Keep Tauri as a thin shell | Two transports, two builds, no capability gained |
| Server-rendered pages | No offline build story; the SPA is smaller to maintain |
| Separate static host (nginx) | One more container for no benefit; the gateway already serves the build |

## Consequences

- The client runs on any machine with a browser; nothing installs.
- Same-origin serving removes CORS and cookie complexity.
- Native file dialogs are replaced by an HTML upload control. With D16, configs
  still come from the repository mount, so the editing loop is unchanged.
- The desktop parity programme is retired; the shell–web parity test replaces it.
- The client can no longer read the local filesystem, by design.
- The Tauri build stays in CI until `DIST-050` removes it; it is dormant, not
  developed.

## Revisit trigger

Revisit only if a capability genuinely requires native OS access, such as a
tray daemon or a local hardware integration. None is planned.

## Verification

- No Rust crate depends on a webview runtime.
- `apps/web` builds to static assets and is served by the gateway.
- The shell and the web client call the same endpoints.
- `apps/desktop/src` no longer exists after `DIST-050`; its files exist under
  `apps/web/src` with the same component names.
