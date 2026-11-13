# AI transparency log — DIST-031 candle stream

- Date: 2026-11-13 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Intent: serve the canonical bytes a run consumes at
  `GET /datasets/{version}/candles`, with the digest re-checked and an ETag.
- Boundary: the stream is M1 only. Timeframe selection and the stored ladder
  remain; a client cannot ask for `5m` on this route yet.
- Verification: `cargo test --workspace`, clippy with `-D warnings`,
  `cargo fmt --check`, rustdoc with `-D warnings`, `check_structure`,
  `check_legacy`, and `check_doc_length`. No S3 endpoint was available, so the
  stream is tested against the in-memory object store.
- Learning: the inspection loader and the stream share one resolver, so the
  profile check, the catalog lookup, and the digest check cannot diverge between
  the two routes.
- Rejected: streaming straight from the object store without re-hashing. The
  ETag would then describe bytes nobody verified.
