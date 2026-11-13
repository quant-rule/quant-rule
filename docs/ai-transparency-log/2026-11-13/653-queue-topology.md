# AI transparency log — DIST-041 queue topology plan

- Date: 2026-11-13 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Intent: write the execution plan for the queue topology, retries, dead-letter
  queue, and cancellation, and surface the decisions that need human approval.
- Boundary: planning only. No program code, test, issue, or pull request behavior
  changed. The plan reads the current adapter, run store, worker, gateway, and
  control-plane document; it does not assume a live broker.
- Verification: `tools/check_execution_plans.py`, `tools/check_links.py`, and
  `tools/check_doc_length.py` were run on the plan PR. The behavioral
  verification rows are Pending by design because this PR changes no behavior.
- Learning: the current `consume(queue, handler)` conflates the queue name with
  the routing key, and `claim` cannot distinguish a live holder from a killed
  worker's redelivery. The plan makes both explicit before retry policy can be
  correct.
- Rejected: native TTL retry queues as the first step. They are the better
  long-term shape, but proving them needs the Compose stack from #643, and the
  issue's tests can be met with a policy the fake can exercise. The plan records
  the TTL queue as the fallback if the blocking sleep is not acceptable.

## Implementation — 2026-11-14 (UTC)

- Intent: implement the approved plan — attempt accounting, redelivery reclaim,
  named queues, bounded retries, dead-letter, cancellation publication, and the
  attempt count on the run row.
- Boundary: RabbitMQ and Postgres compile behind the traits but are not
  live-tested; the composed broker test remains #643. Per-stage cancellation
  inside the coupled backtest remains #649. No model, strategy, execution, or
  portfolio behavior changed.
- Verification: `tools/gate.sh` passed. The new tests cover the retry policy,
  claim/reclaim/exhaust, a stage failure that retries 5 s then 25 s and
  dead-letters on the third, redelivery after a simulated killed worker, a
  computed run that is never re-executed, cancellation after claim with no CAS
  object, the gateway's `run.cancelled` payload, and queue-name distinctness.
- Learning: the approved plan said the handler republishes after the backoff.
  A republished message arrives with `redelivered = false`, so the claim could
  not tell a legitimate retry from a duplicate submission while a live worker
  held the run. The handler now holds the unacked delivery, sleeps, and retries
  in place with the redelivery flag set. The budget, delays, and terminal state
  are unchanged; the plan records this deviation.
- Rejected: adding a retry header or a second payload field. The run row already
  owns the attempt count, and the broker's redelivery flag is the proof the
  holder is gone.
