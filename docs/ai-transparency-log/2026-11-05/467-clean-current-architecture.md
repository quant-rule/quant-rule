# AI Transparency Log — Clean Current Architecture P0.1

- Date: 2026-11-05 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Work item: [#467](https://github.com/quant-rule/quant-rule/issues/467)
- Plan: [`467-clean-current-architecture.md`](../../execution-plans/2026-11-05/467-clean-current-architecture.md)
- Scope: P0.1 execution plan and file-by-file deletion manifest

## Intent and boundary

The human asked the agent to proceed with the revised clean-current-architecture
plan. The agent created a dedicated worktree and branch
(`quant-rule--467-p0-1-deletion-manifest`, `docs/467-p0-1-deletion-manifest`) and
wrote the plan of record, the exact deletion manifest, the manifest summary, and
the index links.

No code, test, config, fixture, or model behavior was changed. No path was
deleted. P1–P8 were not started.

## Verification and evidence

| Check | Observed result |
|---|---|
| `python3 tools/check_terms.py` | `checked 340 file(s); no banned terms` |
| `python3 tools/check_registry.py` | `registry ok: 46 indicators, 143 events, 110 parameters` |
| `python3 tools/check_links.py` | First run failed on the not-yet-written AI log; after the log was added: 544 Markdown files, no broken relative links |
| `git status --short --branch` | Branch `docs/467-p0-1-deletion-manifest`; only the plan, manifest, log, and index edits are present |
| Manifest count | 352 paths: 126 delete, 5 move, 49 rewrite, 169 decide, 3 keep |

## Learning

- The revised plan said “50 indicators and 10 axes.” The registry has 46
  indicators, 4 retired axes, 10 information areas, and 143 events. The plan and
  manifest use the verified counts.
- The current model config is still `quantrule.model/v4`; `model-spec/v2` is a
  separate measurement spec. Deleting all four model parsers would delete the
  current parser. The manifest marks the current schema decision as D2.
- The strategy ladder still reads `AxisContext` and the backtest context imports
  the axes module. “Remove axes” and “preserve the ladder” cannot both be true
  without a rewrite. The manifest marks this as D3.
- `config/compression-range*` are accepted M1.0 evidence, not clear legacy.
  The manifest marks them `decide`, with keep recommended, as D4.
- The human signed the 37/37 indicator checklist. P6 must correct the wording
  without revoking the signature; the plan records that constraint.

## Rejected or corrected suggestions

- Rejected: treating “50 indicators” and “10 axes” as literal counts. Corrected
  to 46 indicators, 4 retired axes, 10 areas, 143 events.
- Rejected: deleting `quantrule.model/v4` with v1–v3 without a replacement
  decision. Corrected to D2.
- Rejected: deleting `config/compression*` as old machinery. Corrected to D4
  because the prior approved inventory keeps them as accepted M1.0 evidence.
- Rejected: marking row 37 unverified. Corrected to keep the human signature and
  fix the wording.
- Not claimed: any test of model or strategy behavior. P0.1 is documentation
  only.
