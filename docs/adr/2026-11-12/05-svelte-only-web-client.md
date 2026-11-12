# ADR — Svelte-Only Web Client

- Date: 2026-11-12 (UTC)
- Status: Accepted
- Owner: Human
- Claim type: ADR (Architecture Decision)
- Supersedes: the Tauri desktop surface of [`2026-11-11/adopt-local-first-tauri-platform.md`](../2026-11-11/adopt-local-first-tauri-platform.md)
- Related: [`01`](01-adopt-distributed-service-based-architecture.md)

## Context

Once the pipeline runs behind a gateway, the desktop application becomes a pure
HTTP consumer. A native shell then adds a second transport path, a second build,
and a second set of parity rules for no functional gain.

## Decision

The human-facing client is a **Svelte 5 single-page application**, built with
Vite, served by the gateway at the same origin as the API.

- Location: `apps/web/`.
- Transport: `fetch` to `quantrule.api/v1`. No IPC, no native bridge.
- `apps/desktop/` and the Tauri dependency are removed.
- The mock backend becomes an HTTP fixture router so browser tests stay
  deterministic.
- The shell remains the agent interface and speaks the same API.

## Alternatives considered

| Alternative | Why rejected |
|---|---|
| Keep Tauri as a thin shell | Two transports, two builds, no capability gained |
| Server-rendered pages | No offline build story; the SPA is smaller to maintain |
| Separate static host (nginx) | One more container for no benefit; the gateway already serves the build |

## Consequences

- The client runs on any machine with a browser; nothing installs.
- Same-origin serving removes CORS and cookie complexity.
- Native file dialogs are replaced by an HTML upload control.
- The desktop parity programme is retired; the shell–web parity test replaces it.
- The client can no longer read the local filesystem, by design.

## Revisit trigger

Revisit only if a capability genuinely requires native OS access, such as a
tray daemon or a local hardware integration. None is planned.

## Verification

- No Rust crate depends on a webview runtime.
- `apps/web` builds to static assets and is served by the gateway.
- The shell and the web client call the same endpoints.
