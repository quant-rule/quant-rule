# AI transparency log — DIST-031 config scoping

- Date: 2026-11-13 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Intent: make config rows follow the profile rule while keeping the file bundle
  shared by content id, so every catalog row carries a profile and identical
  files are stored once.
- Boundary: the repository scan writes a row only for the requesting profile; a
  new profile gets its rows on its first list or on a run submission miss.
  Existing rows backfill to `default`. Postgres is not live-tested.
- Verification: `cargo test --workspace`, clippy with `-D warnings`,
  `cargo fmt --check`, rustdoc with `-D warnings`, `check_structure`,
  `check_legacy`, `check_doc_length`, and the client `check`/`test`/`e2e`.
- Learning: normalizing the rows and the bundle avoids copying five files per
  profile. The run submission must resolve the config in the requesting
  profile, so a miss triggers a scan before the 400; otherwise a valid
  repository config would be unknown until the configs screen was opened.
- Rejected: one global config list. ADR-11 says every catalog row carries a
  profile, and a profile's run must reference a config row it can see.
