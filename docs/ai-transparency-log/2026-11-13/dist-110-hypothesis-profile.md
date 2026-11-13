# AI transparency log — DIST-110 hypothesis scoping

- Date: 2026-11-13 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Intent: scope sealed hypotheses by profile, so the research record follows the
  same rule as the catalog and the routes refuse an unknown profile.
- Boundary: runs and simulations still lack `profile_id`. A run is
  content-addressed and may be shared by two profiles, so it needs an ownership
  mapping rather than a column; that decision is not made here.
- Verification: `cargo test --workspace`, clippy with `-D warnings`,
  `cargo fmt --check`, rustdoc with `-D warnings`, `check_structure`,
  `check_legacy`, and `check_doc_length`. No database was available, so the
  Postgres hypothesis store compiles but is not exercised by a test.
- Learning: the fingerprint stays the content identity while the row key becomes
  `(profile_id, fingerprint)`, matching the dataset catalog: the record is
  scoped, the content identity is not. The memory store uses the same pair.
- Rejected: making the fingerprint globally unique across profiles. It would
  force one profile's record to hide another profile's identical record, and the
  two are independent research lines.
