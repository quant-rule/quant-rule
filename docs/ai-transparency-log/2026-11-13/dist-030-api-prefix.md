# AI transparency log — DIST-030 API prefix

- Date: 2026-11-13 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Intent: serve the contract's `/api/v1` base, which the Svelte transport already
  calls, without breaking the root routes used by probes and the existing tests.
- Boundary: the static client is still served by Vite in development; the
  gateway does not embed the built assets yet.
- Verification: `cargo test --workspace`, clippy with `-D warnings`,
  `cargo fmt --check`, rustdoc with `-D warnings`, `check_structure`,
  `check_legacy`, and `check_doc_length`.
- Learning: the contract already named `/api/v1`; the server was the side that
  had not implemented it. Nesting a second copy of the routes is enough, and a
  route with state (`/api/v1/profiles`) proves the state is propagated.
- Rejected: changing the client to call the root paths. That would make the
  client the authority on the base and leave the documented `/api/v1` unused.
