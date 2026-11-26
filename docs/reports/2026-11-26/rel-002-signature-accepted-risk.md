# REL-002 — Image Signature Accepted Risk

- Date: 2026-11-26 (UTC)
- Status: **Accepted risk** (human, D6); no signed image and no signature claim.
- Work item: [#1220](https://github.com/quant-rule/quant-rule/issues/1220)
  (`REM-REL-002`, V-REL-02)
- Plan: [`rem-000-v1.6.1-remediation.md`](../../execution-plans/2026-11-26/rem-000-v1.6.1-remediation.md)
- Claim type: Behaviour Claim evidence record; no model, strategy, or edge claim.
- Boundary: one laptop; no `cosign`, no offline key, no signed image, no
  attestation bundle.

## 1. What the signature gate requires

The operator signs the exact `linux/amd64` image digest with the offline cosign
key and records `cosign.bundle`, `attestation.bundle`, and the `signed-image/`
directory. `tools/release.sh verify` must pass from the saved image, and the
release body must carry the digest, bundle hashes, and the signing step.

## 2. Current evidence

| Check | Observed |
|---|---|
| `command -v cosign` | no `cosign` binary |
| `deploy/v3/cosign.pub` | absent |
| `COSIGN_PRIVATE_KEY` / `QR_RELEASE_PRIVATE_KEY` / `QR_RELEASE_PUBLIC_KEY` | unset |
| `tools/release.sh --plan` | prints the manual offline signing step and pinned tool versions; reads no key or image |
| Candidate image digest | none; REL-001 is capacity-gated, so there is no exact digest to sign |
| `v1.6.0` note | `Image signature: pending`; no signed claim |

The release tool pins `cosign v3.1.4`, `syft 1.57.2`, and `grype 0.122.1`, but
none of them is installed on this host. Installing them would not close the gate
without the offline key and an exact candidate digest.

## 3. Accepted risk

The human approved D6 in `REM-000`: if no key or `cosign` binary is available,
record `Accepted risk` and make no signed claim. The accepted risk is:

- the image digest is not signed;
- there is no `cosign.bundle`, `attestation.bundle`, or `signed-image/`;
- there is no offline `cosign verify --key` proof;
- supply-chain trust rests on the build workflow, the digest, the SBOM, the CVE
  report, and the recorded manual step, not on a signature.

This is a material residual risk for the release. It does not waive the gate; it
records that the gate was not met.

## 4. What would close it

With the offline key and the exact candidate digest:

1. Install the pinned `cosign` binary or use the operator's copy.
2. `COSIGN_PASSWORD=<vault> tools/release.sh sign <reference@sha256:...> <out> <sbom>`
3. `tools/release.sh verify <reference@sha256:...> <out>`
4. `tools/release.sh bundle <reference@sha256:...> <bundle-out> <out>`
5. Attach the SBOM, CVE report, bundles, and signed image; record their hashes in
   the release note.

## 5. Release boundary

The `v1.6.1` note must say `Image signature: pending` or `accepted risk`, must
not say `signed`, and must carry the residual risk above. The released `v1.6.0`
note is not rewritten; its pending signature remains part of the history.

## 6. Verification commands actually run

- `command -v cosign` → no output.
- `ls deploy/v3/cosign.pub` → missing.
- `env | grep -E 'COSIGN|QR_RELEASE_(PRIVATE|PUBLIC)_KEY'` → no key.
- `tools/release.sh --plan` → manual signing step and pinned tools; no key read.
- `qr-governance check --all` and `tools/gate.sh --changed main` → recorded in
  the pull request.

## 7. Verdict

**Accepted risk.** The image is not signed, no bundle exists, and no verification
was run. The v1.6.1 release must carry the pending/accepted-risk boundary and no
signed-image claim. The gate remains open for the operator's offline key.
