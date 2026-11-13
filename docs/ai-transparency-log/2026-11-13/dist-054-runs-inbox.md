# AI transparency log — DIST-054 runs inbox

- Date: 2026-11-13 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Intent: implement the runs inbox — server truth, status, config, data version,
  submission time, duration, 2 s/30 s polling, cancel only on an active run, and
  a disconnected state without stale rows.
- Boundary: the detail screen (#660) and submission form (#658) are not part of
  this slice. The list reads the existing `/runs` route; there is no client-side
  run cache. The E2E API is mocked at the browser boundary.
- Verification: `bun run check`, `bun test` (17 pass), `bun run e2e` (8 pass),
  `cargo test --workspace`, clippy with `-D warnings`, `cargo fmt --check`,
  rustdoc with `-D warnings`, `check_structure`, `check_legacy`, and
  `check_doc_length`.
- Learning: Playwright's clock lets the 2 s poll advance without a real wait.
  A killed preview server from an earlier worktree held port 4173 and made the
  web server time out; the fix is to stop the old process, not to change the
  test.
- Rejected: keeping the rows when a read fails. The inbox is evidence; a
  disconnected client shows the error and no rows, never a stale status.
