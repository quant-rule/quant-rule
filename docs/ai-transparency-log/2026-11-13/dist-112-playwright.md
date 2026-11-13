# AI transparency log — DIST-112 Playwright

- Date: 2026-11-13 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Intent: add the browser tests #689 names — create, switch, reload persistence,
  and the unknown-profile error — against the built client with a mocked API.
- Boundary: the tests mock `/api/v1`; they do not run a gateway, Postgres, or
  S3. They therefore prove the client contract, not the server. The browser is
  Chromium only.
- Verification: `bun run e2e` (4 passed), `bun run check`, `bun test`,
  `bun run build`, `cargo test --workspace`, clippy with `-D warnings`,
  `cargo fmt --check`, rustdoc with `-D warnings`, `check_structure`,
  `check_legacy`, and `check_doc_length`.
- Learning: the first run exposed an `effect_update_depth_exceeded` loop. The
  startup `$effect` called a store loader that synchronously read the `$state`
  it then wrote. `onMount` makes the load a lifecycle action rather than a
  reactive dependency. The Chromium download on this host uses the fallback
  Ubuntu build; CI runs the supported image.
- Rejected: driving the tests against a live gateway. It would require the
  database and object store, and the client contract is what these tests cover.
