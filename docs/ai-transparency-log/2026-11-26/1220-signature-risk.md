# AI Transparency — Image Signature Accepted Risk

- Date: 2026-11-26 (UTC)
- Work item: `REM-REL-002` ([#1220](https://github.com/quant-rule/quant-rule/issues/1220))
- Tool: OpenCode coding agent; model identifier reported by the harness is
  `DeepSeek V4.1 Flash 1017`.
- Claim type: Behaviour Claim evidence record; no model, strategy, or edge claim.

## 1. Intent and boundary

Record the human-approved D6 accepted risk for the image signature. The host has
no `cosign` binary, no public key file, and no offline private key, and there is
no candidate digest to sign. The record states the missing evidence and the
release boundary. It does not install tools, sign an image, or claim a signature.

## 2. Implemented

- `docs/reports/2026-11-26/rel-002-signature-accepted-risk.md` records the
  requirement, the missing tool/key/digest, the accepted risk, the closure path,
  and the release-note boundary.
- The plan records V-REL-02 as `Accepted risk`.
- No code, tool, image, tag, or release note was changed.

## 3. Verification

Commands actually run in this worktree:

- `command -v cosign` → no output.
- `ls deploy/v3/cosign.pub` → missing.
- `env | grep -E 'COSIGN|QR_RELEASE_(PRIVATE|PUBLIC)_KEY'` → no key.
- `tools/release.sh --plan` → manual signing step and pinned tool versions; no
  key or image read.
- `qr-governance check --all` and `tools/gate.sh --changed main` → recorded in
  the pull request.

No image was signed, no bundle was produced, and no verification ran. REL-001 is
capacity-gated, so no candidate digest exists.

## 4. Learning

- Installing `cosign` alone would not close the gate; the offline key and the
  exact candidate digest are both missing.
- A signature is a claim about a digest; without the candidate image a signature
  record would be meaningless.
- The v1.6.0 note already says `Image signature: pending`; the v1.6.1 note must
  carry the same boundary rather than silently inheriting a signed claim.

## 5. Next action

Push, pull request, merge and cleanup remain gated. REL-001, HA-002, and HA-003
remain the release blockers; the human must decide whether to provide capacity
and the offline key or record a release boundary.
