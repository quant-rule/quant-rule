# AI transparency log — DIST-031 config upload

- Date: 2026-11-13 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Intent: accept an uploaded run-config bundle, validate it with the same
  loader as the repository mount, and catalogue it with origin `upload`.
- Boundary: the bundle is a JSON object of relative paths to file bodies; there
  is no archive format and no client upload form yet. The staged directory is
  under the system temp root and is removed by a drop guard. Postgres is not
  live-tested.
- Verification: `cargo test --workspace`, clippy with `-D warnings`,
  `cargo fmt --check`, rustdoc with `-D warnings`, `check_structure`,
  `check_legacy`, and `check_doc_length`.
- Learning: a config id is content-derived, so an uploaded bundle identical to a
  repository config is a lookup rather than a second row. The test changes the
  run file's name to force a distinct hash. Path components are checked before
  any write; the loader never sees an absolute path.
- Rejected: accepting a filesystem path or a zip archive. A path is a client
  path the API forbids, and an archive needs a format and an unpack policy that
  are not specified.
