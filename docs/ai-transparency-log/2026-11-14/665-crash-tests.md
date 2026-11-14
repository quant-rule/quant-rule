# AI transparency log — DIST-062 crash and partial-publish tests

- Date: 2026-11-14 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Intent: prove the failure semantics of the two publish boundaries — the
  backtest manifest and the evidence `complete` — including a restart.
- Boundary: test-only. The crash is injected at the object-store write and the
  run-store `complete` call; no real process is killed. A real `SIGKILL` needs
  a durable Postgres, S3, and RabbitMQ stack, which is #643; the in-process
  fault injection exercises the same state transition and object assertions.
- Verification: the worker suite has 10 passing tests, including the
  manifest-write failure that leaves blobs but no `runs/{run_id}/` prefix, then
  restarts and finalizes. The evidence suite has 11 passing tests, including a
  `complete` failure that leaves the run `running` with a complete manifest and
  then finalizes on restart. `tools/gate.sh` passed.
- Fail-before note: the #654 stopped-write test already fails on the old
  per-name layout. The evidence crash has no earlier implementation to fail
  against, because `complete` was the only finalizer; the test guards the
  current invariant instead.
- Learning: the backtest crash before the manifest is invisible at the run
  prefix; the evidence crash after verification leaves a complete artifact set,
  which is correct, and only the run state distinguishes it. The restart path is
  the proof that both are recoverable rather than lost.
- Rejected: a child process over in-memory fakes. A child cannot share the
  parent's fakes, and a durable stack is the missing prerequisite, not a reason
  to skip the state assertions.
