# Image Registry, Signing, SBOM, and CVE Contract

- Date: 2026-11-24 (UTC)
- Status: Current; PLAT-007 phases 1–2 and PLAT-008 phase 1
- Issue: [#1120](https://github.com/quant-rule/quant-rule/issues/1120), [#1121](https://github.com/quant-rule/quant-rule/issues/1121)
- Claim type: Behaviour Claim
- Related: [ADR-1120](../../adr/2026-11-24/1120-image-distribution.md), [reference topologies](reference-topologies.md), [release workflow](../../workflows/release-operations.md), [releases](../../releases/README.md), [deploy/v3](../2026-11-22/deploy-v3.md), [HA deployment](../../manual/15-ha-deployment.md)

The release unit is one application image, addressed by a registry reference and
a `sha256:` manifest digest. The image is pullable from a public package, signed
by an offline key, described by an SBOM, and scanned before the version image tag
is pushed. This contract defines the reference, the evidence, the gate, and the
boundary.

## 1. Reference and digest

| Item | Value |
|---|---|
| Registry | `ghcr.io` |
| Package | `ghcr.io/quant-rule/quantrule-platform` (public) |
| Primary pull | `ghcr.io/quant-rule/quantrule-platform@sha256:<64 lowercase hex>` |
| Version image tag | `ghcr.io/quant-rule/quantrule-platform:vX.Y.Z` |
| Build tag | `ghcr.io/quant-rule/quantrule-platform:sha-<commit>` |
| Staging tag | `ghcr.io/quant-rule/quantrule-platform:staging-<commit>` |
| Descriptor field | `image.reference` plus `image.digest`; render emits `reference@digest` |
| Platform | `linux/amd64`, built once from `Dockerfile.release` |

The digest is the manifest digest returned by
`docker buildx imagetools inspect`. A tag is a convenience name; a deployment
uses `reference@digest`. A host pulls anonymously because the package is public.
A private package is an operator override and needs a host registry credential;
it is not the default.

The sample and lab descriptors use
`ghcr.io/quant-rule/quantrule-platform` with the fixture digest
`sha256:cdace4a996c06cd6142fb63eb23a52136c071d9450fe0e07f5556fc26b9edabe`.
That digest is a labelled example, not a published image. `qr-deploy check
--release` and `QR_DEPLOY_RELEASE=1` reject it; the default sample/lab check stays
green. An operator writes a production descriptor with
`tools/release.sh pin`.

The 16 infrastructure image pins (`images.*`) are separate from the app pin.
PLAT-008 resolved 14 of them from registry metadata and recorded the index and
linux/amd64 digests in section 11. Two remain exact blockers because anonymous
registry metadata is denied here: `minio` and `minio_client`. The app digest is
also still the labelled fixture. A production descriptor is incomplete until an
operator supplies those three digests; `qr-deploy validate --release` names every
remaining fixture at its exact field. PLAT-007 did not refresh the infra pins;
PLAT-008 does not guess the blockers.

## 2. GHCR package and permissions

The release workflow logs in with `GITHUB_TOKEN` under the image job's
`packages: write` permission. The release job keeps `contents: write` and
`actions: read`; it does not push or sign. A package is public by operator
choice in the GitHub package settings; the workflow cannot make it public.

A host needs only network access to `ghcr.io`. No pull secret, Docker config, or
registry account is required. A deployment host that cannot reach GHCR uses the
offline path in section 3.

## 3. Offline bundle and transient registry

`tools/release.sh bundle <reference@digest> <out-dir> [evidence-dir]` uses
skopeo to write an OCI layout and, when the sign evidence directory is passed,
copies the real `cosign.bundle`, `attestation.bundle`, `signature.json`, and
`signed-image/` into the bundle. It hashes every bundled file in `bundle.json`,
starts no daemon, and deletes nothing. Without an evidence directory it still
writes the OCI layout and labels the signed-image proof absent.

On the offline host:

```bash
# the proof is the saved image, not a metadata file:
cosign verify --key deploy/v3/cosign.pub --local-image bundle/signed-image \
  ghcr.io/quant-rule/quantrule-platform@sha256:<digest>
# or pull through the transient local registry:
docker run -d --rm -p 127.0.0.1:5000:5000 \
  registry:2@sha256:a3d8aaa63ed8681a604f1dea0aa03f100d5895b6a58ace528858a7b332415373
skopeo copy oci:bundle:latest docker://127.0.0.1:5000/quantrule-platform@sha256:<digest>
# or copy directly into the host store:
skopeo copy oci:bundle:latest docker-daemon:quantrule-platform@sha256:<digest>
```

The local registry is the pinned `registry:2@sha256:a3d8…`, transient and
loopback-only. It is not an HA registry, is not backed up, and is not a
deployment dependency. The release unit remains the digest; the bundle is
transport, and the Sigstore bundle plus saved image are the proof.

## 4. Signing and key custody

cosign is key-based. The operator generates one key pair offline:

```bash
cosign generate-key-pair
# keep cosign.key offline; commit only cosign.pub as deploy/v3/cosign.pub
```

The private key is never in Git, CI, a workflow secret, an agent environment, or
a log. CI does not sign. After the release image is pushed, the operator runs:

```bash
COSIGN_PASSWORD='<from the operator vault>' \
  tools/release.sh sign ghcr.io/quant-rule/quantrule-platform@sha256:<digest> out/release sbom-<tag>.spdx.json
```

`sign` uses `COSIGN_PRIVATE_KEY` or `--key` for the key path and
`COSIGN_PASSWORD` from the environment; neither value is printed. It runs
`cosign sign --key <key> --bundle cosign.bundle --yes <target>`, attests the
SBOM with `cosign attest --type spdxjson --bundle attestation.bundle`, and
saves the image and its signatures with `cosign save --dir signed-image`.
`signature.json` is metadata only: it records the public-key fingerprint, the
signature- and attestation-bundle SHA-256 values, and `signed_image`.
`tools/release.sh verify <reference@digest> [bundle]` uses
`deploy/v3/cosign.pub`, fails closed when the key is absent, verifies
`--local-image <bundle>/signed-image` when that directory exists, and otherwise
verifies from the registry. It also checks the metadata digest and any recorded
bundle hash. The key path may be overridden with `QR_RELEASE_PUBLIC_KEY` for a
rotation or a test; the default is the committed public key.

Rotation: generate a new pair, sign the new release with the new key, add the
new public key at `deploy/v3/cosign.pub`, and keep prior public keys under
`deploy/v3/keys/<YYYY-MM-DD>/cosign.pub` for old digests. Never delete a public
key that a supported release still cites. A lost private key cannot be recovered;
the next release uses a new key and records the rotation.

`deploy/v3/cosign.pub.example` documents the expected file; it is not a key.
The real `deploy/v3/cosign.pub` is generated by the operator and is the only file
`verify` trusts by default.

## 5. Deploy gate

`tools/swarm.sh bootstrap` and `upgrade` verify the app image before rendering or
deploying. `swarm-ha` and `single-az` fail closed when the key is missing, cosign
fails, or the digest does not match. `lab-single-host` may continue only with
`--allow-unverified`; it prints `Unverified` and never a pass. The lab flag does
not weaken a production profile.

`qr-deploy check` remains independent of cosign. It checks the descriptor, local
files, placement, and (with `--remote docker`) every running task image against
the descriptor; a stopped task is ignored, and a running task with a foreign
image or no running task fails. `--remote docker --runtime` runs the all-service
digest probe: the descriptor image's `curl` on the overlay reads `/version` for
six services and `/health` for backtest, and all seven `image_digest` values must
equal the descriptor digest. The probe fails closed and is `Unverified` when
Docker, the image, or the overlay is unavailable; `ha` runs it after deploy and
after restore.

## 6. SBOM

```bash
tools/release.sh sbom ghcr.io/quant-rule/quantrule-platform@sha256:<digest> out/sbom.spdx.json
```

Syft writes SPDX JSON from the exact digest. The command refuses an unpinned
reference, prints the subject digest and the file SHA-256, and does not commit
the body. The release workflow attaches `sbom-<tag>.spdx.json` to the GitHub
release. The release note records the digest, the SBOM asset name, and its
SHA-256. When signing, `cosign attest --type spdxjson` binds the SBOM to the
digest; the attestation is evidence, not a second SBOM.

An SBOM describes the pushed layers. It is not proof of a reproducible rebuild,
and it does not replace the image digest.

## 7. CVE gate and allowlist

```bash
tools/release.sh scan out/sbom.spdx.json out/cve
```

`scan` runs Grype on the SBOM and `cargo audit` on `Cargo.lock`. It writes
`grype.json`, `grype.txt`, `cargo-audit.json`, and `summary.json`; it prints the
counts and exits non-zero on a violation. No secret is read or printed.

| Severity | With a fix | Without a fix |
|---|---|---|
| Critical | Fail | Fail |
| High | Fail | Allowlist entry required |
| Medium | Report only | Report only |
| Low | Report only | Report only |

An allowlist entry is a JSON object in `deploy/release/cve-allowlist.json` with
exactly `id`, `package`, `version`, `severity`, `reason`, `owner`, `expires`, and
`ticket`; `severity` is `High` or `Critical`. `expires` is `YYYY-MM-DD`; an
expired or malformed entry fails the
scan. The file ships with an empty `entries` array. A real exception requires
human approval and a ticket; it is never added by an agent to make a release
pass. The `cargo audit` path treats a RustSec vulnerability conservatively as
High unless its CVSS score is 9.0 or above, in which case it is Critical.

The scan runs before the version image tag is pushed. A failed scan stops the
release job: no version image tag, no GitHub release, no digest claim. The
staging tag may remain in the registry, but it is not the release.

## 8. Release workflow and evidence

| Step | Evidence |
|---|---|
| 1. Gate | `tools/gate.sh` and the release tag/note checks |
| 2. Build | One `linux/amd64` image from `Dockerfile.release` |
| 3. Local checks | No Python/Bun, version contract, composed tier |
| 4. Staging push | `staging-<commit>` digest inspected |
| 5. SBOM | `sbom-<tag>.spdx.json` and SHA-256 |
| 6. Scan | `cve-report-<tag>.json` and counts; gate passes |
| 7. Version push | `vX.Y.Z` and `sha-<commit>`; digest equals staging |
| 8. Release | Assets attached; body names digest, signature step, SBOM/scan hashes, and the fixture-pin blocker |
| 9. Signature | Manual operator step after the push: `cosign.bundle`, `attestation.bundle`, and `signed-image/`; the note records the bundle SHA-256 |

A release note at or after `v1.5.10` carries:

```markdown
- Image: `ghcr.io/quant-rule/quantrule-platform:vX.Y.Z`
- Image digest: `sha256:<64 lowercase hex>`
- Image signature: `cosign bundle sha256:<64 lowercase hex>` or `pending`
- SBOM: `sbom-vX.Y.Z.spdx.json sha256:<64 lowercase hex>` or `pending`
- CVE scan: `cve-report-vX.Y.Z.json sha256:<64 lowercase hex>` or `pending`
```

`pending` is allowed only before `Status: Released`. The signature value is the
Sigstore signature-bundle SHA-256 (or the cosign signature digest), never the
public-key fingerprint. The governance release checker validates the fields for
releases at or after `v1.5.10`; historical notes are unchanged.

## 9. Verification commands

```bash
tools/release.sh --plan
tools/release.sh sbom ghcr.io/quant-rule/quantrule-platform@sha256:<digest> out/sbom.spdx.json
tools/release.sh scan out/sbom.spdx.json out/cve
tools/release.sh sign ghcr.io/quant-rule/quantrule-platform@sha256:<digest> out/release
tools/release.sh verify ghcr.io/quant-rule/quantrule-platform@sha256:<digest> out/release
tools/release.sh bundle ghcr.io/quant-rule/quantrule-platform@sha256:<digest> out/bundle out/release
qr-deploy check deploy.json --remote docker --runtime
tools/release.sh pin --descriptor deploy.json \
  --reference ghcr.io/quant-rule/quantrule-platform --digest sha256:<digest>
qr-deploy validate deploy.json --release
tools/swarm.sh bootstrap            # verifies first; fails closed
tools/swarm.sh --allow-unverified bootstrap   # lab only; prints Unverified
```

## 10. Tool pins

| Tool | Pin | Source |
|---|---|---|
| cosign | `v3.1.4`, `cosign-linux-amd64` SHA-256 `424e2c97d0b45fd7a6b608a87f090f35633f4c23ece5f092d48bba11aaf4bbfa` | sigstore/cosign GitHub release |
| syft | `1.57.2`, `syft_1.57.2_linux_amd64.tar.gz` SHA-256 `ad92bd3ec1b94a3df01d4d0aec199afa9c42b72e86da436ff5f837914d32706b` | anchore/syft GitHub release |
| Grype | `0.122.1`, `grype_0.122.1_linux_amd64.tar.gz` SHA-256 `f1ba7e2917ee1395618de20a98da0aa7c0fb1788927797651047c50502a37538` | anchore/grype GitHub release |
| skopeo | `quay.io/skopeo/stable:v1.25.1` image digest `sha256:7e7d5b9c9f30896e8fd256d33ba351408299399442f04b8ef3a9452f3c2fa50e` | quay.io/skopeo/stable |
| transient registry | `registry:2@sha256:a3d8aaa63ed8681a604f1dea0aa03f100d5895b6a58ace528858a7b332415373` | Docker Hub `registry:2` |
| cargo-audit | `0.23.0` | crates.io |

`tools/release.sh install-tools <dir>` downloads the three release artifacts,
verifies the checksum before extraction, and never uses `curl | sh`. CI installs
cargo-audit with `cargo install cargo-audit --version 0.23.0 --locked`. A pin is
changed only with a reviewed release-tool change.

## 11. Infrastructure pins

Fourteen infrastructure images were resolved from registry metadata on
2026-11-24 with `skopeo inspect --raw` and `skopeo inspect --format '{{.Digest}}'`.
No layer was pulled. The descriptor pins the multi-arch index digest; the
linux/amd64 manifest digest is recorded so a single-platform audit is possible.
The tag is metadata for review; the deployment always uses `reference@digest`.

| Key | Reference | Tag | Index digest |
|---|---|---|---|
| `traefik` | `traefik` | `v3.7.20` | `sha256:8301d6f60957a422f98918aacf2b01482abde3e74ead3719aabdb7c64045fe28` |
| `postgres` | `postgres` | `17.12` | `sha256:156d40cfa063d6babb15d329a8e15b26a8d0690ebb6084c956084f3b025c3b30` |
| `patroni` | `batonogov/patroni-docker` | `main-17.10-4.0.7-trixie` | `sha256:0de7dc8bc5919c76721bee1475ed30155b1848c0a6bac7298605bf39eef1a7eb` |
| `etcd` | `quay.io/coreos/etcd` | `v3.6.17` | `sha256:316f5d367455f75a327f9dc72f669cbf645651859fa2e272f692a1f2dee6f5c9` |
| `haproxy` | `haproxy` | `3.4.8` | `sha256:8ab12982db911f3f13e53c8e4c11312f65b8367301787acbab86459cddf288ea` |
| `keepalived` | `osixia/keepalived` | `2.4.3` | `sha256:4bf02d569ed240e6da272242b368ce5a25cf0f7d92114536367ee2c963c318a2` |
| `rabbitmq` | `rabbitmq` | `4.4.1` | `sha256:183272aa2ff82718cabeef343de8d9a900931ea65fda4e563fed2a6d9efde578` |
| `prometheus` | `prom/prometheus` | `v3.9.1` | `sha256:1f0f50f06acaceb0f5670d2c8a658a599affe7b0d8e78b898c1035653849a702` |
| `grafana` | `grafana/grafana` | `13.3.1` | `sha256:0615ec4fbd4072004b7d86b46ded63013a74b7d6eb1119273dd89e8fa25c13cf` |
| `loki` | `grafana/loki` | `3.7.10` | `sha256:b43c257ccfc925d4a50b8717cac61a3b51df8c48d01847936057e5c4075a24a7` |
| `alertmanager` | `prom/alertmanager` | `v0.35.0` | `sha256:e638797febd034838eca8b98cdbd46f951e9bf7f178777aa8669167406a9946c` |
| `node_exporter` | `prom/node-exporter` | `v1.13.1` | `sha256:7296504d4ada44b7d7ab4431e27e9f9900d8488f8d55842f4b12ae2704e16dfe` |
| `promtail` | `grafana/promtail` | `3.6.11` | `sha256:a761cb834cfaeee29745440d4884d6748f0a08d8f68928db1d707018c1dcfbe9` |
| `blackbox_exporter` | `prom/blackbox-exporter` | `v0.29.1` | `sha256:ea1c7aa3ef3f307cd03e85f34abb604478ccb90b567fc690da0cd9776da76b13` |

| Key | linux/amd64 manifest digest |
|---|---|
| `traefik` | `sha256:cb035e329ea90292290e0199a062aa9bbc78e427356a63a0f8d31e40b058b3bb` |
| `postgres` | `sha256:e711be2c7ff64c9eb8883fc87571621be6caa2f943d825a49e5d20ac5b0221d7` |
| `patroni` | `sha256:f7f34a52db6218941315d40cc1008156c4694fe93b486efe99f33fd15499534f` |
| `etcd` | `sha256:c521f860433f4f1696e95ff2124c90e8a979766c8443f5f487bdf1b02d238de2` |
| `haproxy` | `sha256:6ad3bce68cc37443c16f0a1be909d82af126e4ebf419aaa9b31db8ee9d6e8d94` |
| `keepalived` | `sha256:3cf650f7570f5e3f1b0a7fe33a900af09cda47569845cdd12947fa8e4553a941` |
| `rabbitmq` | `sha256:cf6a49ed990149e548f13fa4e02e9446aab2c6e5d51980f0d9a88fe4160fece2` |
| `prometheus` | `sha256:2024d7bdfc559f6254de22d65e62d799eaa8eb818fe5dbeca212d3f200b70890` |
| `grafana` | `sha256:1c88285f1dd391a62232c010e6c224846e6a73bdc583b3365be2eba614b7919e` |
| `loki` | `sha256:6d789ab9a603ee4429d0fdc4d11512bf2423e8a8c5fc898a9e5aa2d8471ce459` |
| `alertmanager` | `sha256:0f6727a39ff00b6cf95e8a5ebaf7442c55af855b79c689fb9a060e2bada1ecbf` |
| `node_exporter` | `sha256:f99f6e8dcad00fb95ff9fe81ceb1e6442fd9b4180f177da02e8437c845d570eb` |
| `promtail` | `sha256:90a93ba8969799344f197d408334b57d4b86b8b11f32506f840606fc9588366d` |
| `blackbox_exporter` | `sha256:66eae352a79e6f677aeddd8c25a9bdb017586dc5fbd230f22329213d891f94bd` |

The remaining blockers are exact, not guessed:

| Field | Reference | Digest | Reason |
|---|---|---|---|
| `image.digest` | `ghcr.io/quant-rule/quantrule-platform` | `sha256:cdace4a996c06cd6142fb63eb23a52136c071d9450fe0e07f5556fc26b9edabe` | Labelled application fixture |
| `images.minio.digest` | `minio/minio` | `sha256:bdf0781acceb93983c15eec9e56bfbd59dcadc98ea57467022d61aeee074a2cc` | Anonymous registry metadata is denied here |
| `images.minio_client.digest` | `minio/mc` | `sha256:5c5b1f8f2a4e3d6c9b7a1e0f8d2c4b6a9e7f1d3c5b8a0e2f4d6c8b1a3e5f7d9c` | Anonymous registry metadata is denied here |

`qr-deploy validate --release` rejects every one of these three and every retired
infrastructure fixture at the exact field path. The check is a denylist; it does
not prove a fabricated digest exists. The registry is not queried at deploy time.

Compatibility notes from the image metadata (no layer was pulled): the Patroni
image entrypoint is `/usr/bin/patroni` with `PG_MAJOR=17`, `PGDATA=/var/lib/postgresql/data`,
and Patroni 4.0.7, matching the rendered config; the keepalived image is
Alpine-based with BusyBox `nc`, entrypoint `/usr/sbin/container`, and keepalived
2.4.3, matching the mounted `/etc/keepalived/keepalived.conf` and the 443
`nc -z` health check.

## 12. Ownership and residual risks

The operator owns the registry package, the offline private key, the public key
rotation, the allowlist approval, and the manual signature. Release Engineering
owns the workflow and the pinned tools. The human authorizes the tag and any
allowlist exception.

Residual risks: GHCR availability and retention are outside the operator's
control; a compromised signing key can sign a malicious digest; a vulnerability
disclosed after the release is not covered; 14 infrastructure pins are resolved
but `minio`, `minio_client`, and the app image remain exact blockers; the live
push, signature, SBOM, scan, and lab are `Unverified` here; `Real HA` is HA-014.
