# AI transparency log — DIST-032 model service

- Date: 2026-11-13 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Intent: compute the model ladder behind a reference boundary — load the dataset
  and config by id, shard by timeframe, merge sorted, and store one CAS object.
- Boundary: the route is served by the gateway until the six binaries are split.
  It computes the existing flat JSON report, not a new model contract. The model
  service binary and cross-service HTTP are future. Strategy, execution, and
  portfolio are untouched.
- Verification: `cargo test --workspace`, clippy with `-D warnings`,
  `cargo fmt --check`, rustdoc with `-D warnings`, `check_structure`,
  `check_legacy`, `check_doc_length`. Tests compare full-pass bytes to merged
  shard bytes and exercise the route with a 250-bar fixture.
- Learning: the source timestamp parser accepts `YYYY-MM-DD HH:MM:SS+00:00`, not
  the RFC 3339 `T...Z` form; the model test first exposed that `is_header`
  treated any first cell with a letter as a header, so a malformed date was
  skipped instead of rejected. The header test now requires an all-alphabetic
  first cell.
- Rejected: serializing `ModelRun` directly. It has no `Serialize`; the existing
  flat report envelope is the published numeric contract and avoids inventing a
  second shape.
