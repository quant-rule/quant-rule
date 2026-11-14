# AI transparency log — DIST-061 scale invariance

- Date: 2026-11-14 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Intent: prove the model object is invariant to shard count and shard
  completion order, and show the harness would catch an unsorted merge.
- Boundary: test-only. The production `compute_sharded` now routes through one
  `merge_reports` function that sorts by timeframe and deduplicates; its output
  is unchanged. No model, strategy, execution, or portfolio behavior changed.
- Verification: six compute tests pass, including one pass vs four shards,
  reversed shard order, reversed requested ladder, and an unsorted-merge
  counterexample. `tools/gate.sh` passed.
- Learning: the existing one-shard test compared the full pass with per-timeframe
  shards but never varied the merge order or shard count. Exposing `merge_reports`
  makes the ordering rule a single place the test can attack.
- Rejected: adding a real thread pool for the test. The risk is merge order, not
  concurrency; a deterministic shard split and a reversed completion order prove
  the same invariant without a flaky scheduler.
