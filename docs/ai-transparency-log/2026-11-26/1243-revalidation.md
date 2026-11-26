# AI Transparency — Run v5 Re-validation After Remediation

- Date: 2026-11-26 (UTC)
- Work item: `REM-RA-007A` ([#1243](https://github.com/quant-rule/quant-rule/issues/1243))
- Tool: OpenCode coding agent; model identifier reported by the harness is
  `DeepSeek V4.1 Flash 1017`.
- Claim type: Behaviour Claim validation record; no edge, profit or release
  claim.

## 1. Intent and boundary

A fresh independent pass over the remediation in #1239 (D1/D2) and #1240 (D3).
The pass re-ran the core v5 suites and checked the upload, submission and lineage
paths directly. It did not run Docker, the composed tier, a live Postgres, the
full gate, HA, soak, upgrade, signature or release.

## 2. Implemented

- The validator added `## Re-validation (2026-11-26)` to
  `docs/reports/2026-11-26/rem-ra-quant-validation.md`.
- Verdict for `8b54e62d`: **Pass**; D1, D2 and D3 are `Verified`; no new Critical
  or Major finding.
- D4 (Minor wording in the pack contract) was corrected in the same PR; the
  `V-RA-07` row is now `Pass`.
- G1–G8 remain documented gaps; the composed and live-Postgres evidence remains
  for REL-001.

## 3. Verification

Commands actually run by the re-validator in its worktree:

- `cargo test -p quantrule-data-service --test configs_v5` → 4 passed.
- `cargo test -p quantrule-evidence-service --test promoted_runs` → 8 passed.
- `cargo test -p quantrule-app --test run_v5_e2e` → 2 passed; golden unchanged.
- `cargo test -p quantrule-backtest-service --test worker` → 48 passed.
- `cargo test -p quantrule-model-service` → 32 passed.
- `cargo test -p quantrule-app --test signal_artifact_public` → 6 passed.
- `cargo test -p quantrule-evidence-service --lib` → 82 passed.
- `cargo run --quiet -p quantrule-governance -- check --all` → ok.
- Additional app lib 348, data lib 72, backtest lib 15, pack filter 119, identity
  17, `run_v5_public` 4, `run_v5_identity` 2, lineage 5 + 1 ignored; clippy and
  fmt clean.

The digest was independently reconstructed from the emitted bundle:
`sha256:4d5f6a8ad385989d8a6c335bb1bd66620bd7bb3fd5a3505438d7bd6cab256b79`
with `portfolio/metrics.json`, matching the golden.

## 4. Learning

- The upload path and the full loader are different seams: the upload resolves
  bytes and state, while pack-kind and signal cross-field checks remain at
  submission/loader. The contract now says that precisely.
- The repo-scanned v5 `valid` flag is still structural and origin-scoped; the
  submission read is the run-safety gate. This is a documented residual.
- A second independent pass can clear the findings without rewriting the
  historical Fail verdict; the report keeps both dates and commits.

## 5. Next action

The remaining plan work is HA/REL/GOV/STR. Commit, push, PR, merge and cleanup
remain gated.
