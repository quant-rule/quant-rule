# AI Transparency — Canonical-Digest Contract Correction

- Date: 2026-11-26 (UTC)
- Work item: `REM-RA-009` ([#1240](https://github.com/quant-rule/quant-rule/issues/1240))
- Origin: RA-007 D3 (Moderate), [validation record](../../reports/2026-11-26/rem-ra-quant-validation.md)
- Tool: OpenCode coding agent; model identifier reported by the harness is
  `DeepSeek V4.1 Flash 1017`.
- Claim type: Behaviour Claim; documentation contract correction, no code or
  digest change.

## 1. Intent and boundary

Make the written canonical-digest contract match the code and the changelog.
The service already includes every `metrics.json`; `AGENTS.md`, the manual and
the glossary omitted it, so a reviewer following them would compute a different
digest. No code, test, golden or behavior was changed.

## 2. Implemented

- `AGENTS.md` §12 now lists `results.json`, `*.csv`, every `metrics.json`, and
  `*.ndjson`.
- Manual chapter 6 now lists every `metrics.json`, includes
  `portfolio/metrics.json` in the example set, and says the other JSON reports
  are not canonical.
- The glossary's canonical-digest row is corrected.
- The canonical-digest ADR carries a dated addendum recording
  `AUD-2026-11-17-22` and its fix (#890); the historical decision text is not
  rewritten.
- The validation record marks D3 fixed and the plan records V-RA-09.

## 3. Verification

Commands actually run in the RA-009 worktree:

- `cargo run --quiet -p quantrule-governance -- check --all` → `governance check ok`.
- `tools/gate.sh --changed main` → the classifier's docs/governance scope.
- The checked-in golden digest is unchanged; no code or test was edited.

No composed, lab, or release run was performed.

## 4. Learning

- The audit fix #890 changed the code and changelog but left the manual, the
  glossary and the decision record behind. A behavior change has to update every
  contract that restates it, not only the implementation.
- An ADR is a historical decision; the correction is an addendum, not a rewrite
  of the accepted text.
- `AGENTS.md` is a normative contract, so the fix required the human approval
  recorded in #1240.

## 5. Next action

A fresh independent validation is required before `V-RA-07` can move from `Fail`
to `Pass`. Commit, push, PR, merge and cleanup remain gated.
