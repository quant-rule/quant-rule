# AI transparency log — DIST-057 generated labels

- Date: 2026-11-14 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Intent: give the client one generated code-to-word table from the model
  registry, and make drift fail the gate.
- Boundary: tooling and a generated client file. No model, strategy, endpoint,
  or engine behavior changed. The registry JSON remains the source; the Rust
  constants remain the runtime identity and are checked by `check_registry.py`.
- Verification: `python3 tools/check_labels.py` passes on the checked-in table.
  A deliberate label change made the checker fail, and the file was restored.
  `bun test` passes with the new label test, `svelte-check` is clean, and
  `tools/gate.sh` passes with `check_labels` in the loop.
- Learning: the registry already carries indicator names and event IDs, so the
  label table needs no second source. Generating both maps from the same JSON
  keeps the client from re-declaring a code.
- Rejected: generating the table from the Rust constants by parsing `codes.rs`.
  The registry checker already proves the constants and the JSON agree; parsing
  the JSON is simpler and keeps the generator in the same governance layer.
