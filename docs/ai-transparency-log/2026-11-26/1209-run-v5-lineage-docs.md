# AI Transparency — Run v5 and Lineage Documentation

- Date: 2026-11-26 (UTC)
- Work item: `REM-RA-006` ([#1209](https://github.com/quant-rule/quant-rule/issues/1209))
- Tool: OpenCode coding agent; model identifier reported by the harness is
  `DeepSeek V4.1 Flash 1017`.
- Claim type: Behaviour Claim; documentation only, no model, strategy, signal,
  or edge claim.

## 1. Intent and boundary

Remove the documentation's "later work item" language now that RA-002–RA-005
have landed, and state the v5 submission, full-run, promoted-run and worker
paths as current. The change is documentation only; it does not create the
v1.6.1 release note, which belongs to REL-003 after the candidate is verified.

## 2. Implemented

- `research-artifacts.md` §5 now states that a fold-mode v5 run without a model
  artifact computes the model, and that a cited model artifact is reused byte for
  byte. The status line records catalog/worker reachability as current.
- `cross-database-consistency.md` states that submission writes the run row,
  outbox intent and lineage set in one transaction and the worker resolves the
  set once; the old "signal writer lands" caveat is gone.
- The architecture README and config index no longer say v5 reachability targets
  the next release.
- The pack and signal contract status lines are current.
- Manual chapter 6 adds the v5 submission, full-versus-promoted table and
  `run lineage` behavior; chapter 9 adds the v5 note; chapter 15 records that the
  HA verifier now submits v4 and v5 parity runs.
- The plan records the release-note input for REL-003.

## 3. Verification

Commands actually run in the RA-006 worktree:

- `cargo run --quiet -p quantrule-governance -- check --all` → `governance check ok`.
- `tools/gate.sh --changed main` → `gate passed: docs`.
- `grep` over the current architecture, manual and config documents → no
  remaining "later work item" or "do not yet resolve" language in the run-v5 and
  lineage path. Historical AI logs and dated execution plans are left unchanged.

No code, test, composed, lab, or release run was performed.

## 4. Learning

- The manual's chapter 15 is the HA deployment chapter, not a run-v5 chapter.
  The v5 procedure belongs in chapter 6, and chapter 15 records only the
  deployed-stack verifier. The PR states this correction rather than adding v5
  text to the wrong chapter.
- The v1.6.1 release note cannot be created before REL-003 because the release
  date, tag, image digest, SBOM and CVE evidence do not exist yet. The plan
  records the required input so the release note does not omit v5 reachability.
- A living document may not name an untagged release version; the governance
  checker rejected `v1.6.1` in the architecture README. The status now says
  current without naming the unreleased tag.

## 5. Next action

Push, pull request, merge and cleanup remain gated. The next leaf is RA-007, the
independent Quant Validator reproduction.
