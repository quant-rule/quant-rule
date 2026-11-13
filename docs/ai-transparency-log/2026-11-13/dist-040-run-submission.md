# AI transparency log — DIST-040 run submission

- Date: 2026-11-13 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Intent: implement the run state machine's submission half — content-addressed
  `run_id`, duplicate lookup, per-profile lists, detail, cancellation, and the
  `run.requested` publish.
- Boundary: no worker consumes the message yet, so a run stays `queued`. A
  cancel changes the row but does not remove a message already in the queue; the
  future worker must check the run state before each stage. A missing broker
  returns `503` after the run row is stored; a retry publishes again. The
  Postgres run store compiles but is not live-tested.
- Verification: `cargo test --workspace`, clippy with `-D warnings`,
  `cargo fmt --check`, rustdoc with `-D warnings`, `check_structure`,
  `check_legacy`, and `check_doc_length`.
- Learning: a `profile_id` column on `runs.runs` would duplicate a shared run.
  A `runs.submissions` join keyed by `(profile_id, run_id)` keeps the run
  content-addressed and gives each profile its own list. Publishing on a
  duplicate submission is deliberate: if the first publish was lost, the retry
  is the only chance to recover, and the worker is expected to be idempotent.
- Rejected: deriving `run_id` by plain concatenation. The fields are
  newline-delimited with a version tag, so a boundary cannot be forged; a test
  covers `"ab"` versus `"a\nb"`.
