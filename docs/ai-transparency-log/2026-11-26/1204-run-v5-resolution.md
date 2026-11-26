# AI Transparency — ADR-RA-2

- Date: 2026-11-26 (UTC)
- Work item: `REM-RA-001` ([#1204](https://github.com/quant-rule/quant-rule/issues/1204))
- Tool: OpenCode coding agent; model identifier reported by the harness is
  `DeepSeek V4.1 Flash 1017`.
- Claim type: ADR (Architecture Decision) planning record; no model, strategy,
  signal, or edge claim.

## 1. Intent and boundary

The issue asks for an ADR that makes `quantrule.run/v5` executable end to end:
resolve the five references in production, write run lineage at submission, and
let the worker consume a resolved run without a per-bar network call.

The agent read the v5 loader, the reference resolver, `ConfigBundle`, the
evidence run and lineage stores, the data-service pack and signal-artifact
routes, and the backtest worker. It wrote the ADR and this log. It did not
implement code, run tests, commit, or approve the ADR.

## 2. Design selected

- `data` owns strategy, portfolio and execution pack bytes plus the optional
  signal-artifact bytes; it validates those references at config upload.
- `evidence` owns model-stage artifacts, run rows and lineage; it validates the
  model reference against a succeeded stage and writes the run, outbox and
  lineage in one transaction.
- `ConfigBundle` carries the ordered reference set, never a payload.
- `backtest` builds `MemoryReferences` once after claiming the run and calls
  `RunConfig::load_with`; the fold uses the resolved `RunConfig`.
- A v5 run does not require legacy `strategy`, `execution` or `portfolio`
  files; v3/v4 keep the existing path.
- No new database; no per-bar network; no pack or signal body in lineage.

## 3. Verification and limitations

Commands actually run in the ADR worktree:

- `wc -l docs/adr/2026-11-26/1204-run-v5-resolution.md` → 109 lines.
- `qr-governance check --checker doc-length` → pass.
- `qr-governance check --all` → pass.

No unit, integration, composed, lab, or release test was run. The ADR is
`Proposed`; the implementation claims are future work. The ADR does not prove
that the resolver works, only that the design has one owner per content kind and
a fail-closed path.

## 4. Learning

- The clean split is content ownership, not service convenience: evidence must
  not depend on `quantrule-app`, and the worker already has the engine loader.
- `Workspace::create` currently requires a `strategy` file, which v5 does not
  have; the worker must use the resolved `RunConfig` rather than invent a file.
- `ConfigBundle` is the right envelope for references because it already crosses
  the data/evidence/worker boundary and carries no payload.
- The model artifact is not in the data-service artifact catalog; evidence's
  stage store is the natural validator and prevents a second model registry.
- The 110-line ADR cap forced a tighter document; no decision or alternative was
  dropped, only prose was compressed.

## 5. Approval and next action

The human approved ADR-RA-2 on 2026-11-26 (UTC). The ADR and this log are
committed on `arch/rem-ra-001-adr-ra-2`. `REM-RA-002` starts on a stacked branch;
the push, pull request, merge and release gates remain separate.
