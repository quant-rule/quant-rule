# AI transparency log — DIST-031 stored ladder

- Date: 2026-11-13 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Intent: build one timeframe's ladder from the stored canonical bytes and keep
  it under `ladder/{data_version}/full/{timeframe}` so a repeat read reuses the
  bytes.
- Boundary: the key uses the literal `full` because a configurable ladder needs
  the config catalog, which is not implemented. The route builds one timeframe
  at a time and buffers that timeframe's output; it is not a streaming object
  upload. The model service does not consume the key yet.
- Verification: `cargo test --workspace`, clippy with `-D warnings`,
  `cargo fmt --check`, rustdoc with `-D warnings`, `check_structure`,
  `check_legacy`, and `check_doc_length`. No S3 endpoint was available; the
  ladder is exercised against the in-memory object store.
- Learning: `LadderSummary` and the new `LadderWriter` now share one fold. The
  summary is the writer plus counters, so the count and the emitted bars cannot
  drift. The writer's emissions are compared to `TimeframeSet` in a test.
- Rejected: materializing the ladder at upload. It would slow every upload for
  a derived object a run may never ask for, and the full ladder is not the only
  possible selection.
