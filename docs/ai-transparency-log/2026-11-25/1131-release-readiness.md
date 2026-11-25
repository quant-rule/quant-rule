# AI Transparency Log — v1.6.0 Release Readiness

- Date: 2026-11-25 (UTC)
- Work item: DOC-004 [#1131](https://github.com/quant-rule/quant-rule/issues/1131)
- Tool: OpenCode, DeepSeek V4.1 Flash 1017

## Intent and boundary

Prepare the v1.6.0 release-readiness record and stop at the human tag gate. No
release tag, changelog section, SBOM, CVE report, or signature was created; those
are release-workflow outputs after authorization.

## Work performed

- Summarized the 2026-11-25 evidence: fast gate, composed tier, HA lab, restore
  and cutover, security, soak/scale/client parity, and docs.
- Recorded the open gates: HA-22/HA-23, 72-hour soak, versioned rolling upgrade,
  SBOM/CVE/signature, commercial licence text, real remote TLS client.
- Wrote `docs/reports/2026-11-25/release-readiness.md` with the tag checklist and
  rollback boundary.
- Updated the plan to 45 Done/1 In Progress and V24 pending the human gate.

## Verification and evidence

- The report cites the merged evidence reports and the live Composed digest
  `sha256:a0372f02…`.
- `qr-governance check --all`: pass.
- No tag exists; `git tag` shows `v1.5.2` as the latest.

## Learning

- The release note cannot be written as a released record before the tag and
  supply-chain artifacts exist; a readiness report is the honest pre-tag record.
- DOC-004's done condition includes the human tag authorization, so the leaf
  stays In Progress while all technical evidence is complete.

## Residual risk

- The human must accept HA-22/HA-23, the 72-hour soak, the versioned upgrade, and
  the commercial licence text before the tag. The release workflow then produces
  the SBOM, CVE report, and signature.
