# AI Transparency — Real HA Accepted Risk

- Date: 2026-11-26 (UTC)
- Work item: `REM-HA-004` ([#1214](https://github.com/quant-rule/quant-rule/issues/1214))
- Tool: OpenCode coding agent; model identifier reported by the harness is
  `DeepSeek V4.1 Flash 1017`.
- Claim type: Behaviour Claim evidence record; no model, strategy, or edge claim.

## 1. Intent and boundary

Record the human-approved D5 accepted risk for Real HA HA-22/HA-23 on the single
laptop. The record states what the real-host gate requires, what evidence is
missing, the residual risk, and the release-note boundary. It does not run a
power event, an AZ loss, or a network partition, does not collect host UUIDs,
and does not claim `Real HA`.

## 2. Implemented

- `docs/reports/2026-11-26/ha-004-real-ha-accepted-risk.md` records the
  requirement, current `Unverified` rows, the accepted risk, the closure path,
  and the release boundary.
- The plan records V-HA-04 as `Accepted risk`.
- No code, test, matrix row, descriptor, or release note was changed.

## 3. Verification

Commands actually run in this worktree:

- `deploy/v3/real-ha.sh --plan` → 9-line operator-only plan; no host contacted.
- `deploy/v3/real-ha.sh --confirm-real-ha --descriptor deploy/v3/reference/three-az.json`
  → `REFUSED: --evidence is required`, exit 1.
- `cargo test -p quantrule-deploy the_ha_matrix_maps_every_row_to_a_rule` →
  1 passed.
- `qr-governance check --all` and `tools/gate.sh --changed main` → recorded in
  the pull request.

No real-host event, composed run, lab run, or `Real HA` verification was
performed. The accepted risk is not evidence that the real gate passes.

## 4. Learning

- The gate is deliberately file-only: it validates a descriptor and an operator
  evidence file and refuses to act on a host itself. A descriptor alone cannot
  satisfy it.
- The matrix rows already carry `physical_hosts=0` and `failure_domains=0`, so
  the record must not upgrade them; HA-006 owns the matrix update after the
  remaining accepted risks are recorded.
- HA-003 (versioned upgrade/rollback) is capacity-gated and remains pending. This
  record does not present the accepted risk as upgrade evidence.

## 5. Next action

Push, pull request, merge and cleanup remain gated. HA-005 records the local
client and the second-machine accepted risk next.
