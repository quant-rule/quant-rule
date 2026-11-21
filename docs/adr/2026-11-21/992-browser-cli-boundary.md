# ADR — Browser GUI and Terminal CLI Boundary

- Date: 2026-11-21 (UTC)
- Status: Accepted
- Owner: Human
- Claim type: ADR (Architecture Decision)
- Plan: [`992-development-speed-v1.5.0.md`](../../execution-plans/2026-11-21/992-development-speed-v1.5.0.md)
- Epic: #992

## Context

QuantRule has one server contract and two client shapes: a browser SPA and a
headless command shell. The desktop wrapper is retired. The remaining question
is whether the CLI should also move into the browser as a console.

GitHub is the reference: `github.com` is the browser GUI, `gh` is the terminal
CLI, and the public API is the contract. GitHub has no general browser console
for `gh`. Codespaces has a browser terminal, but that is a separate development
environment. GitHub Desktop is an optional GUI app, not the platform.

## Decision

**The browser is the only GUI. The CLI remains a headless client.**

| Surface | Role |
|---|---|
| Browser SPA | Human GUI on Windows and Linux |
| `quantrule` binary | Agent and automation client on Linux and Windows |
| Gateway API | The one contract both clients use |
| Browser console | Not built |
| Desktop app | Not built |
| macOS | Unsupported in v1.5.0 |

- The CLI holds no engine, database, queue, or model.
- It stores only local connection state: API URL, token, and default profile.
- Every PR tests the CLI on Linux through the workspace gate.
- The release gate tests the Linux and Windows CLI before publish.
- Windows and macOS CLI jobs do not run on normal PRs.
- A browser terminal would be a new security surface and needs a separate ADR.

## Alternatives considered

| Alternative | Why rejected |
|---|---|
| Browser-only CLI via a PTY | Adds server-side process execution and session security |
| Generated TypeScript command console | Duplicates the 49-command parser and output contract |
| Remove the CLI; agents use HTTP directly | Loses the stable agent command interface |
| Desktop GUI app | Reintroduces the packaging path just retired |

## Consequences

- The 49-command surface stays the stable agent interface.
- Browser/shell parity remains a tested contract.
- The CLI is distributed as source, not an installer.
- A browser terminal can be added later without changing this boundary.

## Verification

- Architecture docs name browser + CLI only.
- `cargo test -p quantrule-shell` passes on Linux.
- The release gate builds and tests the Linux and Windows CLI.
- No browser console or desktop app exists in the current tree.
