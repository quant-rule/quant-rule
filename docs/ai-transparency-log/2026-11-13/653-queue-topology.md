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
