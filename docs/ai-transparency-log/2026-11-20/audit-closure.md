# AI transparency log — Audit closure and release gate (AUD-05, AUD-06, AUD-25)

- Date: 2026-11-20 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: [Audit Workflow](../../workflows/audit-workflow.md) execution for
  `AUD-2026-11-17-05`, `-06`, and `-25`; P3.1.
- Claim type: Behaviour Claim.
- Intent: close the audit/release records and make the release gate enforce the
  audit state and the full verification suite.
- Boundary: branch `docs/audit-closure`, based on `main` `cba85434`.

## What changed

- The 2026-11-16 audit's 22 findings are `Verified` by a sequential Improvement
  Verifier pass; the v1.3.0 note carries a dated audit-status addendum.
- The old Open Major findings are triaged with dated notes; the native-window
  `-11` is a Duplicate, `-12`/`-13` are Stale, and the data/target findings are
  Stale.
- `tools/check_release_audit.py` requires the latest release note to name a
  verified audit status and rejects an unverified Critical/Major.
- `tools/check_audit_backlog.py` fails when an audit leaves a Critical/Major
  Open.
- The release image job runs `tools/test_tiers.sh composed` on the locally
  built image before `docker push`; the gate includes the local image ID.
- A `quality` job runs the browser check, unit tests, build, and Playwright;
  `desktop` and `publish` require it.
- `test_check_release_workflow.py` rejects a missing or reordered composed step
  and a missing quality job.

## Verification observed

| Command | Result |
|---|---|
| `tools/gate.sh` | pass; 27 checker suites and the workspace tests |
| `python3 tools/check_release_audit.py` | v1.3.0 audit status present |
| `python3 tools/check_audit_backlog.py` | no Critical/Major Open |
| `python3 -m unittest test_check_release_workflow test_check_release_audit test_check_audit_backlog` | pass |
| release workflow YAML parse | pass |

## Independence

The verifier pass is sequential and single-agent; it is not an independent
verifier. The human remains the final acceptance authority. The current 53
findings are `Fixed`, not `Verified`.

## Quant validation

Not required: audit records, release workflow, and checker rules only.

## Learning

- A release gate needs a machine-readable audit status; a prose note is not
  enough to fail a release.
- The composed tier can run before the push if the local image is tagged and
  the run identity is the local image ID; the canonical digest excludes
  provenance, so the published digest is the same.
