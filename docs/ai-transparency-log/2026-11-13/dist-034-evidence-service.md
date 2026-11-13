# AI transparency log — DIST-034 evidence service

- Date: 2026-11-13 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Intent: extract run finalization from the worker into evidence: verify the
  artifact hash chain, recompute the canonical digest, and mark a computed run
  succeeded; publish and consume the run id only.
- Boundary: the run store, queue, and object store are the in-process fakes in
  tests; Postgres, RabbitMQ, and S3 compile behind the existing traits but are
  not live-tested. Evidence has no HTTP route in this slice. Research memory
  remains where #650's scope puts it: the move from SQLite to Postgres is not in
  this change. The queue still binds one queue per routing key; the named
  `runs.backtest`/`runs.evidence` topology and dead-lettering are #653.
- Verification: `tools/gate.sh` passed in the worktree. Evidence tests cover a
  successful finalize, a missing artifact, one changed artifact, a digest
  mismatch, an unfinished stage, a second finalize that skips, a cancelled run,
  and two profiles over one finalized run. The worker tests finalize through
  evidence and assert the `run.computed` payload is `{run_id}`.
- Learning: the worker's `running` state now carries two meanings — executing and
  computed-but-unfinalized — and the recorded digest is the discriminator. A
  redelivery of a computed run republishes `run.computed` instead of re-running
  it, which closes the lost-publish gap without a new state.
- Rejected: adding an evidence HTTP route before a client needs it. The service
  owns finalization; the browser and shell read the finalized run through the
  existing run detail route.
