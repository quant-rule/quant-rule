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

## Update 2026-11-26 — release record and snapshot

The readiness record above was the pre-tag state. After the human continued
through the CVE findings, the following work was performed:

- The release scan and the image gate were fixed (CVSS vector parsing, the
  unused AWS legacy TLS graph, the curl-free runtime health probe, the pinned
  bases, and the ten-container stack check). Those changes are separate merged
  pull requests, not part of this docs record.
- Snapshot workflow `45726933695` passed at `64003970`: the composed canonical
  digest is `sha256:a0372f02…`, the image digest is
  `sha256:ba77ad9f…`, and `/version` reports `1.6.0`. The CVE gate reports
  0 Critical and 72 allowlisted Highs.
- The `v1.6.0` release note was written with the snapshot SBOM/CVE hashes, the
  `Unreleased` section was promoted to `## [1.6.0] - 2026-11-26`, and the
  current-tag map now resolves `v1.6.0`.
- The release body still needs the annotated tag, and the image signature
  remains a manual operator step with no key available.

Residual risk: the 72 Debian trixie High findings are accepted until
2027-02-28; `Real HA`, the 72-hour soak, and the versioned upgrade remain
unclaimed. The tag is the remaining human gate.

## Final 2026-11-26 — tag and release

- Annotated tag `v1.6.0` was created at `8eaa44c1` (tag object
  `8be23c38…`) and pushed. Tag workflow `45730024955` passed every job and
  published the image `sha256:a000bce9…` and the GitHub release
  `https://github.com/quant-rule/quant-rule/releases/tag/v1.6.0`.
- The release note on `main` now records the tag object, commit, published image,
  SBOM hash, and CVE hash; the status remains `Prepared` because the offline
  cosign signature is still operator-owned.
- The post-tag record updates V24/V25 in the plan; the remaining plan gaps
  (`Real HA`, the 72-hour soak, the versioned upgrade, and the strategy replica
  kill) are recorded as release risks rather than claimed as passed.
