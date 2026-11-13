# AI transparency log — DIST-031 config catalog

- Date: 2026-11-13 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Intent: implement the repository half of the config catalog — scan the mount,
  validate each run config, hash the five files, and list the entries.
- Boundary: a catalog entry is a **run config**, the composition root. Uploads
  are not accepted: validating an uploaded run file would resolve its referenced
  paths, potentially outside the mount, and a bundle format is not specified.
  The endpoint table now marks `POST /catalog/configs` as planned.
- Verification: `cargo test --workspace`, clippy with `-D warnings`,
  `cargo fmt --check`, rustdoc with `-D warnings`, `check_structure`,
  `check_legacy`, and `check_doc_length`. No database was available, so the
  Postgres config store compiles but is not exercised by a test.
- Learning: the run config's own `config_hash` is a 16-digit FNV fingerprint,
  not a SHA-256, so the catalog hashes the five files with `Digest::sha256`
  instead. A valid config's id changes when any referenced file changes; an
  invalid config's id covers just its run file so it can be listed and fixed.
- Rejected: cataloguing every individual config file. The run config is the
  unit a run references, and the table's `config_hash` names the five-file
  composition, not one JSON document.
