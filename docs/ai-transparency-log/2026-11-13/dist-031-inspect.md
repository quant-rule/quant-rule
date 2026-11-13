# AI transparency log — DIST-031 inspection

- Date: 2026-11-13 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Intent: add `GET /inspect/candles` and `GET /inspect/timeframes` over the stored
  canonical bytes, and make ingest reject a series the ladder cannot build.
- Boundary: the timeframes response counts bars and partial buckets; it does not
  publish events, because events are model output and the model service is not
  wired into the gateway yet.
- Verification: `cargo test --workspace`, clippy with `-D warnings`,
  `cargo fmt --check`, rustdoc with `-D warnings`, `check_structure`,
  `check_legacy`, and `check_doc_length`. No database or S3 endpoint was
  available; the Postgres catalog and the S3 adapter compile but are not
  exercised by a test.
- Learning: `TimeframeSet` is a bounded inspection window, not a full-dataset
  ladder. Counting through `LadderSummary` reuses the same `Bucket` and is
  tested for equality against `TimeframeSet` on a fixture, so the two cannot
  drift silently. The ingest had accepted out-of-order rows; inspection made
  that defect visible.
- Rejected: silently inspecting a dataset by trusting the catalog row. The
  route re-hashes the object against `data_version` before parsing it.
