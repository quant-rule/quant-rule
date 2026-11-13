# AI transparency log — DIST-031 tolerant ingest

- Date: 2026-11-13 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Intent: add a tolerant CSV ingest that skips row-level failures, reports each
  one, and stores the accepted candles and the quality report on the dataset.
- Boundary: the client still uses the strict route; it has no tolerant upload
  control. A coarser-than-M1 source is a whole-file failure, not a row failure.
  The quality report is persisted in migration 0010 but the Postgres path is not
  live-tested.
- Verification: `cargo test --workspace`, clippy with `-D warnings`,
  `cargo fmt --check`, rustdoc with `-D warnings`, `check_structure`,
  `check_legacy`, `check_doc_length`, and the client `check`/`test`/`build`.
- Learning: a tolerant upload must produce the same `data_version` as a strict
  upload of the accepted rows; a test compares the canonical bytes, not just
  the counts. Counting a skip and deciding whether it is a duplicate, an
  out-of-order row, or invalid keeps the report additive and deterministic.
- Rejected: continuing past a 5-minute source. That would label hourly candles
  as M1 and publish a ladder that cannot be built; the whole upload fails.
