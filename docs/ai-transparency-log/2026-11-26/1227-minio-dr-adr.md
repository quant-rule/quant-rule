# AI Transparency — MinIO Object DR ADR

- Date: 2026-11-26 (UTC)
- Work item: `REM-STR-001` ([#1227](https://github.com/quant-rule/quant-rule/issues/1227))
- Tool: OpenCode coding agent; model identifier reported by the harness is
  `DeepSeek V4.1 Flash 1017`.
- Claim type: ADR (Architecture Decision); no model, strategy, signal, or edge
  claim.

## 1. Intent and boundary

Record the human-approved D3 decision on object disaster recovery: keep
`object_replication: false` and use the one-way off-host archive plus the DR
drill, rather than implement MinIO bucket/site replication for a single-cluster
release. The work records the decision and aligns the contracts and the
release-note input. It does not implement replication, add a second site,
change a descriptor default, or rewrite the released `v1.6.0` note.

## 2. Implemented

- `docs/adr/2026-11-26/1227-minio-object-dr.md` records the context, decision,
  alternatives, consequences, invariants, and verification, with D3 as the
  human approval.
- The MinIO contract now says second-site replication is deferred by
  ADR-STR-001; the backup contract and `ha.md` link the ADR and keep the
  one-way archive wording.
- The plan records V-STR-01 as `Pass` and adds the archive-only boundary to the
  REL-003 release-note input.
- No tag, descriptor default, validator rule, archive tool, or DR script was
  changed.

## 3. Verification

Commands actually run in this worktree:

- `cargo test -p quantrule-deploy` → 284 unit tests passed (4 ignored), plus 22
  `cli`, 10 `chaos_harness`, 6 `dr_tool`, 30 `ha_harness`, 10 `minio_harness`,
  7 `object_archive`, 6 `reference`, and the remaining suites all passed.
- `cargo run --quiet -p quantrule-governance -- check --all` →
  `governance check ok`.
- `wc -l docs/adr/2026-11-26/1227-minio-object-dr.md` → 105 lines, under the
  110-line ADR cap; `docs/architecture/ha.md` is 169 lines, under 170.
- `tools/gate.sh --changed main` → recorded in the pull request.

No live archive, restore, DR drill, composed, lab, or `Real HA` run was
performed. The ADR claims no second site, no object lock, and no replication.

## 4. Learning

- The contract and the validator already implemented the archive-only choice;
  the missing piece was the accepted decision record, not new behavior.
- A released note must not be rewritten. The correct place for the v1.6.1
  boundary is the plan's REL-003 input and the eventual v1.6.1 note.
- The architecture document was at its 170-line cap, so the MinIO bullet was
  condensed to stay within it while adding the ADR link.
- Replication needs a second site, credentials, monitoring, and a drill; calling
  the one-way archive "replication" would be a false HA claim.

## 5. Next action

Push, pull request, merge and cleanup remain gated. STR-002 implements the
chosen object DR path if the approved scope requires a code or descriptor
change; this ADR changes no behavior.
