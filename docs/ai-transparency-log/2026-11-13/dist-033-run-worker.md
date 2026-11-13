# AI transparency log — DIST-033 run worker

- Date: 2026-11-13 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Intent: implement the approved worker core — claim, materialize, execute,
  artifact CAS, canonical digest, finalize, publish; and add the queue consumer
  and `qr-backtest` binary.
- Boundary: the worker finalizes the run in the run store because `qr-evidence`
  does not exist yet; the finalizer is evidence-shaped and will move. The
  database and broker paths compile but are not live-tested. Cancellation is
  checked before and after the coupled backtest, not inside it. The Compose stack
  is #643 and is not exercised.
- Verification: `tools/gate.sh` passed in the worktree, including tests, clippy,
  fmt, rustdoc, file size, structure, legacy, plans, links, and board
  permanence. Worker tests cover a full fixture run, duplicate delivery, cancel,
  missing dataset, and invalid config. The consumer has a fake-implementation
  test; RabbitMQ is not live.
- Learning: the catalog stores canonical candles as Unix seconds without a
  header, while the app's CSV loader expects `Date,Open,High,Low,Close,Volume`
  and `YYYY-MM-DD HH:MM:SS+00:00`. The workspace reconstructs that source form.
  The SR1 1m strategy also required the execution fill to be 1m; the copied
  example had 5m, so the v4 example was corrected.
- Rejected: running the coupled backtest directly from the catalog bytes.
  It takes paths and validates its own source; a private materialization is the
  smaller, testable seam.
