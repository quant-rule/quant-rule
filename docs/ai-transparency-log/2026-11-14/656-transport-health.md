# AI transparency log — DIST-051 transport and connection health

- Date: 2026-11-14 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Intent: give the browser client one transport seam and one truthful
  connection state — connected, degraded, mismatch, or unreachable — and add a
  fixture-driven Playwright router for the shell endpoints.
- Boundary: client-only. `call<T>` already existed and is unchanged. No Rust
  code, endpoint, model, strategy, or engine behavior changed. The expected API
  version is a client constant until the generated contract table (#662).
- Verification: `bun test` 27 pass, `svelte-check` 0 errors, the full Playwright
  suite 17 pass including the four new connection tests. `tools/gate.sh` passed.
- Learning: the existing connection store treated any parseable `/version` as
  connected. A different API version can still parse and then mean something
  else, so the mismatch must be decided before the health state; a mismatch
  outranks a degraded service.
- Rejected: reporting a gateway that answers with a down dependency as
  `unreachable`. The transport is reachable; the dependency is not, and the
  health rows already say which one is down.
