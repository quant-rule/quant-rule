# AI Transparency Log — Research Governance and Directional Program Closeout

- Date: 2026-11-04 (UTC)
- Tool: OpenCode agent session (DeepSeek V4.1 Flash)
- Work item: close the failed directional event-family program and establish the
  research mandate and program rules
- Issues: [#386](https://github.com/quant-rule/quant-rule/issues/386),
  [#387](https://github.com/quant-rule/quant-rule/issues/387)
- Plan: [`docs/execution-plans/2026-11-04/386-research-governance.md`](../../execution-plans/2026-11-04/386-research-governance.md)

## Intent and boundary

The human approved the full implementation of the research governance work.
The agent was asked to:

- close the five-family directional program as no edge;
- preserve the five local run records;
- commit the five candidate docs, plans, and AI logs;
- add the research mandate, program-charter template, and AGENTS.md rules;
- update the baseline and run registries;
- clean the five research worktrees and branches after preservation.

No model or strategy code changed. No validation was opened. No baseline was
accepted.

## Proposed work versus implemented work

| Item | Status |
|---|---|
| Preserve five run records | Done |
| Copy candidate docs, plans, AI logs | Done |
| Closeout document | Done |
| Research mandate and charter template | Done |
| AGENTS.md research rules | Done |
| Workflow program-charter section | Done |
| Baseline and index updates | Done |
| Verification | Pending at the time of writing; see the plan |
| Worktree/branch cleanup | Pending until the governance branch is pushed |

## Verification and evidence

The run records were copied from the five worktrees to the main checkout:

- `MODEL-2026-11-04-13` — pivot break
- `MODEL-2026-11-04-14` — EMA50 test
- `MODEL-2026-11-04-15` — DI cross
- `MODEL-2026-11-04-16` — MACD histogram
- `MODEL-2026-11-04-17` — trend continuation

The candidate docs, plans, and AI logs were copied from the five local branches.
The results and limitations are recorded in the closeout document and the five
candidate records. The agent did not rerun the gates or change any config.

## Learning and rejected suggestions

- A larger parameter sweep was rejected: the five families were a finite,
  pre-registered batch, and the premise failed. More combinations would add
  multiple-testing risk without a new premise.
- Auto-selection was rejected: a batch can report a candidate, but only the
  human promotes it to one-shot validation.
- Deleting the five branches before copying their docs was rejected: the run
  records are local and the docs are the committed evidence.
- The research mandate distinguishes exploration from confirmation and makes
  the data budget explicit; it does not promise an edge.
