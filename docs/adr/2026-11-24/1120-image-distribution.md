# ADR-1120: Image Distribution, Signing, and Supply-Chain Evidence

- Date: 2026-11-24 (UTC)
- Status: Accepted
- Issue: [#1120](https://github.com/quant-rule/quant-rule/issues/1120)
- Decision owner: Human
- Related: [HA deployment profile](../2026-11-22/1082-ha-deployment-profile.md), [registry contract](../../config/2026-11-24/registry.md), [deployment architecture](../../architecture/deployment.md)

## Context

The platform runs one application image for seven services. The previous release
flow built a local image, pushed it to a registry, and recorded a digest, but it
did not publish a package path a host could pull by digest without credentials,
did not sign the digest, and produced no SBOM or CVE evidence. The v3 sample
pointed at `registry.quantrule.dev/quantrule/server` with a fixture digest, which
looks deployable but is not.

The operator runs one deployment on owned or lab hosts. There is no multi-tenant
registry, no cloud runtime, and no account system. A release must be pullable by
digest, attributable to a key the operator controls, and blocked when a known
Critical or fixable High vulnerability is present.

## Decision

1. Publish the application image as the public GHCR package
   `ghcr.io/quant-rule/quantrule-platform`. A host pulls
   `reference@sha256:<digest>` and needs no registry credential. The package is
   public; a private package would need a host credential and is an operator
   override, not the default.
2. Keep the 16 infrastructure images as separate digest pins. They remain
   fixtures in the sample and lab; a production descriptor supplies real pins.
3. Sign the pushed digest with a cosign key pair. The private key is offline and
   is never in CI, a workflow secret, the repository, or an agent environment.
   CI does not sign. The operator signs locally with `tools/release.sh sign`,
   which writes `cosign.bundle`/`attestation.bundle` and a `cosign save`
   `signed-image/`; the public key is committed at `deploy/v3/cosign.pub`, and a
   missing key fails verification closed.
4. Verify the signature before `tools/swarm.sh bootstrap` and `upgrade`.
   `swarm-ha` and `single-az` fail closed. `lab-single-host` may continue only
   with an explicit `--allow-unverified` and prints `Unverified`, never a pass.
   `qr-deploy check` stays independent of cosign, compares every running task
   image, and `--runtime` proves all seven service digests.
5. Generate an SPDX JSON SBOM with syft from the exact pushed digest. Attach it
   to the GitHub release and record its SHA-256 in the release note. When the
   operator signs, `cosign attest --type spdxjson` binds the SBOM to the digest.
   The SBOM body is not committed to Git.
6. Gate CVEs with Grype on the syft SBOM plus `cargo audit` on `Cargo.lock`.
   Critical always fails. High fails when a fix is available. An unfixed High
   needs a valid, expiring allowlist entry with an owner, reason, and ticket.
   Medium is report-only. The gate runs before the version image tag is pushed.
7. Pin cosign, syft, Grype, skopeo, and cargo-audit by release checksum, exact
   version, or image digest. Do not add a Rust dependency or a `qr-release`
   binary. The orchestration is shell plus CI; deterministic equality remains in
   `qr-deploy`.
8. For an offline host, `tools/release.sh bundle` writes an OCI layout and copies
   the Sigstore bundles plus `signed-image/`; `bundle.json` hashes every file. A
   transient local `registry:2@sha256:a3d8…` serves a `reference@digest` pull. No
   permanent registry, HA registry, or backup unit is introduced.
9. Add `Image`, `Image digest`, `Image signature`, `SBOM`, and `CVE scan` to
   release notes at or after `v1.5.10`. The signature-bundle, SBOM, and scan
   values may start as `pending`; a `Status: Released` note must carry the hashes.

## Alternatives considered

| Alternative | Consequence | Rejected because |
|---|---|---|
| Self-hosted registry | Full control of retention and offline access | New HA/backup unit and a second service to operate; GHCR plus an offline bundle covers both paths |
| `docker save`/`docker load` only | Simplest offline path | No registry reference or digest pull; insufficient for a host that expects `reference@digest` |
| Keyless cosign (OIDC) | No long-lived key | The operator is offline and CI must not hold an identity; key-based custody is simpler here |
| CI-held signing key | Automatic signatures | A compromise of CI or a workflow secret signs arbitrary images |
| Unsigned digest only | Least work | No provenance beyond the registry; the deploy gate cannot distinguish a replaced digest |
| No CVE gate | Faster releases | Critical and fixable High findings would ship silently |
| Commit the SBOM | Reviewable in Git | It is derived evidence, grows with every release, and duplicates the release asset |

## Consequences

Positive:

- A host pulls the same public digest without a credential.
- The operator can verify the image before deploy, and the lab cannot silently
  claim a verified state.
- SBOM and CVE evidence are attached to the release and hashed in the note.
- No new runtime service or Rust dependency is added.

Negative:

- The operator must hold and rotate an offline private key, and signing is a
  manual post-CI step.
- A public package is visible to anyone; the image contains no secret, but its
  layers and SBOM are public.
- The 16 infrastructure pins remain fixtures until PLAT-008; a production
  descriptor is not complete without real infrastructure digests.
- The CVE gate depends on Grype's vulnerability database at release time; a
  database update can change the result without a code change.
- No bit-for-bit reproducible rebuild is claimed; the SBOM and scan describe the
  pushed digest, not a rebuild.

## Residual risks

- A compromised operator key can sign a malicious digest. Rotation and an
  offline key limit, but do not remove, this risk.
- GHCR availability and retention are outside the operator's control; the
  offline OCI bundle is the fallback, not a second HA registry.
- A vulnerability disclosed after the release is not covered by the release
  scan; a new release and scan are required.
- The live lab, a real registry push, a real signature, and a real scan are
  `Unverified` in this work item. `Real HA` remains HA-014.
