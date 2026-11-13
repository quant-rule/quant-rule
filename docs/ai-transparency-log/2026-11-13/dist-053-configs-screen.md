# AI transparency log — DIST-053 configs screen

- Date: 2026-11-13 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Intent: implement the configs screen — list the catalog, show origin and
  validity, render the parser message verbatim, and submit a run from a valid
  config with a chosen dataset.
- Boundary: the catalog is repository-only; upload is still planned. The screen
  uses the existing `/catalog/configs`, `/catalog/datasets`, and `/runs` routes.
  The API is mocked at the browser boundary in the E2E tests.
- Verification: `bun run check`, `bun test` (21 pass), `bun run e2e` (10 pass),
  `bun run build`, `cargo test --workspace`, clippy with `-D warnings`,
  `cargo fmt --check`, rustdoc with `-D warnings`, `check_structure`,
  `check_legacy`, and `check_doc_length`.
- Learning: a submit button per row needs an accessible name that includes the
  config path; a generic "Submit" repeats and cannot be addressed. The invalid
  row's message is the server's text, not a client paraphrase.
- Rejected: a free-text data version field. A selector from the dataset catalog
  keeps the client from sending an identifier the server cannot resolve.
