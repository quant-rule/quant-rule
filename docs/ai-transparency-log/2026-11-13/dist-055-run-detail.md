# AI transparency log — DIST-055 run detail

- Date: 2026-11-13 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Intent: implement run detail — stage timeline, provenance, artifact list with
  hashes, canonical digest with copy, and a failed run that hides artifacts.
- Boundary: the detail reads the existing `/runs/{id}` route. There is no chart
  or result table because the run contract has no result field; the digest and
  artifacts are the result identity. The API is mocked at the browser boundary.
- Verification: `bun run check`, `bun test` (22), `bun run e2e` (12),
  `bun run build`, `cargo test --workspace`, clippy with `-D warnings`,
  `cargo fmt --check`, rustdoc with `-D warnings`, `check_structure`,
  `check_legacy`, and `check_doc_length`.
- Learning: the detail is keyed by `run_id`, so selecting another run resets the
  copy state. The clipboard is stubbed in the browser test; the assertion
  compares the copied text to the API's digest, not to a second rendering.
- Rejected: showing a stale detail while the selected run disappears. A failed
  detail read clears the detail and shows the error.
