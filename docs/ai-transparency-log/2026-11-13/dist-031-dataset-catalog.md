# AI transparency log — DIST-031 dataset catalog

- Date: 2026-11-13 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Intent: implement the catalog slice of DIST-031 — a profile-scoped record over
  content-addressed bytes, with HTTP list, create, and read routes.
- Boundary: the route validates and identifies an upload; it does not store the
  object bytes yet, does not run the timeframe ladder, and does not expose
  repository configs. Those remain in DIST-031.
- Verification: `cargo test --workspace`, clippy with `-D warnings`,
  `cargo fmt --check`, rustdoc with `-D warnings`, `check_structure`,
  `check_legacy`, and `check_doc_length`. No database was available, so the
  Postgres catalog compiles but is not exercised by a test.
- Learning: sqlx 0.9 refuses a dynamically built SQL string, so the two read
  queries are literals. The issue's "duplicate upload is idempotent" conflicts
  with the documented `dataset.exists` conflict; the route returns `200` with the
  existing record, and the contract now carries `dataset.invalid` instead.
- Rejected: defaulting a missing profile header silently. The route refuses it,
  because a record that is not scoped would be indistinguishable from one that
  is.
