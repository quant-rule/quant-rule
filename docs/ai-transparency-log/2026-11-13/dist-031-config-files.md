# AI transparency log — DIST-031 config files

- Date: 2026-11-13 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Intent: persist a config's whole bundle so a catalogued configuration can be
  reproduced after an upload's staging directory is gone.
- Boundary: the files are stored but no route returns them yet; the run worker
  will read the store or a future detail endpoint. Existing rows have an empty
  map until the next scan or upload; Postgres is not live-tested.
- Verification: `cargo test --workspace`, clippy with `-D warnings`,
  `cargo fmt --check`, rustdoc with `-D warnings`, `check_structure`,
  `check_legacy`, and `check_doc_length`.
- Learning: the invalid branch of the shared record loader had hard-coded the
  repository origin, so an invalid upload was mislabelled. Passing the origin
  through both branches and asserting the upload origin fixes it. The run file's
  references are the bundle keys, not the resolved paths.
- Rejected: storing only the run body and re-reading references from the mount.
  An uploaded config has no mount, and a deleted staging directory would make it
  unreproducible.
