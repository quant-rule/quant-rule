# AI Transparency Log — Dating the board so the timeline can show each item's life

- **Issue:** [#42](https://github.com/quant-rule/quant-rule/issues/42)
- **Date:** 2026-10-21 (UTC)
- **Execution plan:** [`docs/execution-plans/2026-10-21/42-timeline-dates.md`](../../execution-plans/2026-10-21/42-timeline-dates.md)

## Entry

- **Date and tool:** 2026-10-21 (UTC; 2026-10-20 local time). opencode (MiniMax M3.1 Flash), running in a local shell with `git`, `gh`, and filesystem tools.
- **Intent and boundary:** Give the board the date fields the Timeline view needs, keep them current, and fill them in for the work already done. The boundary was the project's field configuration, one script, two lines of `AGENTS.md`, and this plan and log. The view's own configuration was left to the human, because the API does not expose it.
- **Verification and evidence:**
  - The project's fields were read before the change: fifteen of them, of which the three built-in date fields carried no value on any item, which the API confirms. The Timeline view's own message — that it needs at least one date or iteration field — matches.
  - The `createProjectV2Field` mutation was called twice, and the responses returned the two fields with their ids, `PVTF_lADOFJhfMs4Bn-OgzhmjWZU` for `Start date` and `PVTF_lADOFJhfMs4Bn-OgzhmjWaA` for `Closed date`, both of type `DATE`. A read-back shows seventeen fields, the two new ones among them.
  - `bash -n .github/scripts/project-sync.sh` — no syntax errors.
  - The script was run against a closed issue, a merged pull request, and an open issue. Issue #1 took the closed date `2026-10-20`, which is the day its own `closedAt` records. The merged pull requests #2 and #4 took the same treatment, and no open item took a date.
  - Every closed and merged item on the board was then run through the script, and the read-back reports twenty-one items carrying a closed date, none missing, and no open item carrying one.
  - `git diff --check` — no whitespace errors; the script ends with a newline and carries the executable bit.
- **Learning:**
  - **The built-in date fields exist and are empty, and that is not the same as a field the view can draw.** The board has had `Created`, `Updated`, and `Closed` since the board was set up, and the API returns no value for any of them. The Timeline's refusal is therefore about what the view will draw, not about what the project holds, and the fix is a field of the project's own.
  - **A merged pull request is not closed.** It reports `MERGED`, and the first version of the date path tested for `CLOSED` and left every merged pull request without a date. The test became "anything that is not open", which is also what the `State` field means to a reader.
  - **The date came from the issue's own record rather than from a clock.** `closedAt` is on the issue, so the board's date is the date of the event. A workflow that read its own clock would have dated a close by when it noticed, which for a backlog of days is a different date.
  - **The start date was not backfilled, and that was a decision rather than an omission.** The issue's creation is when it was filed, not when the work began, and a field that means one thing going forward and another in the history misleads in exactly the place a reader looks. The closed items are dated by their close, which is exact.
  - **Rejected:** a `Target date` field. It would put a commitment on the board that nobody had made, and the repository holds no record of the human's commitments to take it from.
  - **Rejected:** writing the `Start date` from the workflow. Moving a leaf to `In Progress` is a board write and raises no issue event, so no workflow can learn that the work has begun; the agent is the only party that knows.
