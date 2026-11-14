# AI transparency log — Split the six service binaries (plan)

- Date: 2026-11-14 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash. S2 planning subagent.
- Intent: draft the execution plan for the missing `qr-data`, `qr-model`, and
  `qr-simulation` binaries, the gateway proxy choice, and the blast radius on
  #643, #646, #649, #674, #679, and #680. The plan recommends an option; it does
  not implement one.
- Boundary: planning only. No Rust, Cargo, Dockerfile, compose, Makefile, CI,
  app, or non-plan documentation was changed. No issue was created and no code
  pull request was opened. Worktree
  `/home/zantgo/Proyectos/quant-rule--dist-binaries-plan`, branch
  `arch/dist-binaries-plan`, base `b63b9015`.

## What was read

`AGENTS.md` sections 2, 3, 4, 7, 8, 9, 11, 13, and 14; the execution-plan
workflow; ADR-04; `services.md`; `deployment.md`; issues #643, #644, #646, #649,
and #674. The hidden blocker required #647, #648, #651, #661, #673, #679, #680
and pull requests #750/#751 as well.

## Findings

- The tree has three binaries: `qr-gateway`, `qr-backtest`, `qr-evidence`.
- The data and model HTTP routes are served by the gateway. The model route's
  pull request #751 says this is temporary "until the six binaries are split".
- The backtest worker calls `AppService::run_backtest` in process; it never calls
  a data or model service. That is the worker fast path.
- There is no simulation binary and no simulation route. #680 depends on #679 and
  both are open.
- #649 is closed; its plan explicitly excluded the six-binary split. A rewrite of
  the worker under option B needs a successor issue, because a Done board item is
  not reopened.
- `module-map.md` says "5 binaries", the architecture README glossary says "five
  deployable responsibilities", and `deployment.md` says "all eight" containers
  while its own topology lists nine.
- No open issue owned the binary inventory. #647 and #648 were allowed to close
  as gateway routes, which is how the contradiction survived.

## Verification

| Command | Observed |
|---|---|
| `python3 tools/check_execution_plans.py` | pass |
| `python3 tools/check_doc_length.py` | pass, 32 files within cap |
| `python3 tools/check_links.py` | pass, 922 Markdown files, no broken relative links |
| `git diff --check` | pass |

The behavioral verification rows in the plan are `Pending` by design because this
PR changes no behavior.

## Learning and corrections

- The blocker is not a missing route; it is a claim that was never given an
  owner. The plan must name the closure pattern or the same gap returns.
- The first draft treated #646 as fully unblocked by option A. It is not: A
  satisfies the engine-isolation half and wires the proxy, but the simulation
  surface remains #680. #646 and #674 stay blocked. The plan states this in §6.3
  and in the acceptance criteria.
- Option A's worker fast path changes the failure semantics that `services.md`
  describes: a model outage does not fail a run. The plan records that instead of
  claiming a split data plane.
- Option C is a real option, but it retracts ADR-01, ADR-04, and ADR-09. The plan
  lists it so the human can reject it deliberately; it is not a planning
  convenience.

## Rejected

- Adding simulation endpoints to make `qr-simulation` "real". That is #679/#680
  and would hide a parity gap.
- Adding `reqwest` for the proxy. `hyper-util` and `http-body-util` are already
  in `Cargo.lock`; a direct edge is enough.
- Creating the issue or inventing a number before human approval.
- Editing `module-map.md`, the README glossary, `deployment.md`, or the other
  implementation documents in this plan PR. The task boundary allows only the
  plan and this log; the plan lists those edits for the implementation issue.
- Opening a code PR or closing any issue. The plan is the only deliverable.

No credentials, secrets, or sensitive operational data were recorded.
