# AI transparency log — DIST-040 run state machine

- Date: 2026-11-13 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Intent: close the remaining DIST-040 gaps — the memory run store accepted a
  `complete` or `fail` from any state, and the acceptance criteria had no
  API-level test for "a changed config yields a new run".
- Boundary: the Postgres store already guarded the transitions with
  `status = 'running'`; this change makes the memory fake agree so unit and
  integration tests cannot accept a state production rejects. No schema,
  endpoint, or response shape changed.
- Verification: `tools/gate.sh` passed in the worktree. The new store test
  proves complete and fail are no-ops on a queued run and fail is a no-op on a
  succeeded run. The new runs test uploads a second valid bundle and proves the
  two config ids produce two run ids; the existing duplicate and invalid-config
  tests still pass.
- Learning: a fake store that is more permissive than its production adapter is
  a test hole, not a convenience. The guard belongs in both implementations.
