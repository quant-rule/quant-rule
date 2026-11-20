# AI Transparency Log — Runtime History and Governance Program

- Date: 2026-11-20 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Work item: runtime history, repository history CLI, checker speedup
- Epic: #937
- Plan: [`docs/execution-plans/2026-11-20/937-runtime-history-program.md`](../../execution-plans/2026-11-20/937-runtime-history-program.md)
- Claim type: Behaviour Claim

## Intent and boundary

Record the approved execution plan for the runtime-history capability, the
repository-history CLI, and the local checker speedup. The Python retirement
campaign is deferred: no checker port, no current-tool port, no Python archive,
and no zero-Python policy in this program.

No code was changed by the planning commit. The implementation issues #938–#947
are open.

## Verification and evidence

| Command | Result |
|---|---|
| `gh issue view 937` | Epic open; sub-issues #938–#947 linked |
| `python3 tools/check_execution_plans.py` | Plan structure valid |
| `python3 tools/check_links.py` | No broken links |
| `python3 tools/check_terms.py` | No banned terms |

## Learning

- Runtime history is a product capability for both clients. Repository evidence
  is source control and stays in the governance CLI; that is the parity boundary,
  not a missing client feature.
- The Python retirement is too large for this program. The local checker speed
  fix is a Python scan-set change, not a port.
- The destructive endpoints need a design and security review before code; the
  evidence/data/simulation lanes start after #938.
- The checker speedup and repository CLI are disjoint and can run in parallel
  with the design.

## Rejected or corrected suggestions

| Suggestion | Disposition |
|---|---|
| Port all 27 checkers now | Deferred; too long for this program |
| Add a GUI filesystem-admin service for repository evidence | Rejected without a separate ADR |
| Hard-delete without a tombstone | Rejected; archive then erase |
| Aggregate every kind in one shell call | Rejected; one kind per command, one gateway call |
