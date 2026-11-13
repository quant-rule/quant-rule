# AI transparency log — DIST-030 static client

- Date: 2026-11-13 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Intent: serve the built Svelte client from the gateway at the same origin as
  the API, with an SPA fallback and no API fallthrough.
- Boundary: the image build and the deployment that sets `QR_STATIC_DIR` are not
  part of this slice. Without the variable the gateway serves no client, which
  keeps tests and probes working. The assets are read with tokio's filesystem;
  no new crate is added.
- Verification: `cargo test -p quantrule-server` (72 unit, 6 static, 28 gateway,
  4 ladder), clippy with `-D warnings`, `cargo fmt --check`, rustdoc with
  `-D warnings`, `check_structure`, `check_legacy`, and `check_doc_length`.
- Learning: canonicalizing both the root and the resolved file, then requiring
  `starts_with(root)`, refuses `..` and a symlink that leaves the tree. The API
  check must come before the SPA fallback, or an unknown `/api/v1` path would
  return HTML with 200.
- Rejected: adding `tower-http` for `ServeDir`. The standard library and tokio
  already cover a read-only file response, and a dependency needs its own
  justification.
