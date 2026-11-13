# AI transparency log — DIST-052 tolerant upload

- Date: 2026-11-13 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Intent: expose the tolerant ingest in the browser — a "Skip bad rows" control
  and the quality report — so the capability is not API-only.
- Boundary: the report is shown after an upload and is not a separate screen.
  The strict route remains the default. The client figures are the server's
  counts; it does not recompute them.
- Verification: `bun run check`, `bun test` (23), `bun run e2e` (13),
  `bun run build`, `cargo test --workspace`, clippy with `-D warnings`,
  `cargo fmt --check`, rustdoc with `-D warnings`, `check_structure`,
  `check_legacy`, and `check_doc_length`.
- Learning: the quality notice is cleared by every list load and set only after
  the post-upload load, so switching profiles cannot leave another profile's
  report on screen.
- Rejected: showing the reason as a sentence guessed by the client. The server's
  stable code is rendered as-is, and the line number comes from the report.
