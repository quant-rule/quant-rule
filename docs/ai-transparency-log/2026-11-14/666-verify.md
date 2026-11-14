# AI transparency log — DIST-063 reviewer verification

- Date: 2026-11-14 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Intent: make reviewer verification four commands and one line, using the
  canonical digest only.
- Boundary: tooling and a checked-in mini fixture. No engine, model, strategy,
  or service behavior changed. The script uses the shell's local `backtest run`
  and does not read provenance, logs, or images.
- Verification: `tools/verify.sh` printed one digest twice; `--expected` with
  that digest printed `verified`; a wrong expected digest failed with both
  values; `make verify` printed the same line. `tools/gate.sh` passed.
- Learning: the full-year example is too heavy for a quick reviewer loop, so
  the default is a 250-bar checked-in fixture. The release run points the same
  script at the full example with `--config` and `--strategy`.
- Rejected: computing the digest in Python. The shell already has `sha256sum`,
  `find`, `sort`, and `printf`; the four-command procedure should not require a
  second runtime. The encoding is the same path, NUL, digest wire, newline as
  `Digest::canonical`.
- Deferred: `make up` and the distributed `make verify` are #643/#644; this
  target is the local data-plane procedure they will call.
