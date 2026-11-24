# AI Transparency Log — PLAT-007 image registry, signing, SBOM, and CVE scan

- Date: 2026-11-24 (UTC)
- Work item: [#1120](https://github.com/quant-rule/quant-rule/issues/1120)
- Plan: [`docs/execution-plans/2026-11-24/1120-registry.md`](../../execution-plans/2026-11-24/1120-registry.md)
- Tool: OpenCode agent session, DeepSeek V4.1 Flash 1017

## Intent and boundary

The human approved the phase-1 design before this session: GHCR pull-by-digest,
offline cosign, syft SBOM, Grype plus cargo-audit gate, pinned tools, descriptor
pinning, release-note fields, and the 16-infrastructure-pin boundary. The agent
implemented that scope in the dedicated worktree `quant-rule--1120-registry` on
`feat/1120-registry`.

The agent did not commit, push, open a PR, merge, tag, deploy, change the board,
create a real key or secret, or run a real registry push, signature, SBOM, or
CVE scan. The live lab was not run. The agent read the repository rules,
execution plans, ADR, architecture, contracts, workflows, release notes,
workflow files, descriptor code, governance checker, runtime health code, and
tool scripts before writing the plan and design records.

## Work performed

| Area | Agent contribution |
|---|---|
| Records | Drafted the execution plan, ADR-1120, registry contract, and this log from the approved decisions |
| Release tool | Designed and implemented the shell commands, pinned tool checksums, checksum verification, allowlist gate, and descriptor pin |
| Release workflow | Reworked the image job to build once, push a staging digest, generate the SBOM, run the CVE gate, and only then push the version tag; added release assets and body fields |
| Descriptor | Changed the sample/lab app reference to GHCR, kept the fixture digest, regenerated goldens, added release-mode fixture refusal |
| Release note | Added the five fields and the `v1.5.10` baseline to the Rust checker and its tests |
| Deploy gate | Added pre-deploy cosign verification to `tools/swarm.sh`, with the explicit lab-only `--allow-unverified` path |
| Docs | Updated the release index, release workflow, manuals, architecture, config index, deploy README, tools README, Makefile, and changelog |
| Phase 2 release tool | Replaced the metadata-only signature claim with real `cosign sign --bundle`, `cosign attest --bundle`, and `cosign save --dir` outputs; added offline `verify --local-image`, hashed bundle metadata, and the pinned transient registry image |
| Phase 2 deploy check | Added the `service ps` `Image` field, eight running-task image rows, and the `--runtime` all-service digest probe using the descriptor image and overlay |
| Phase 2 HA harness | Added `digest_all` after deploy and after restore, the `Measured` rule for seven matches, the plan phase, and the split all-service/end-to-end summary |

## Verification and evidence

| Command | Observed |
|---|---|
| `cargo fmt --all -- --check` | Clean after formatting the new tests and checker |
| `cargo clippy -p quantrule-governance -p quantrule-deploy --all-targets -- -D warnings` | Clean |
| `cargo test -p quantrule-governance` | 125 unit tests plus 4 composed, 6 gate, 5 history, and 16 release-workflow tests pass |
| `cargo test -p quantrule-deploy` | 257 unit tests plus every integration suite pass, including 7 release-image and 12 release-tool tests |
| `cargo run -q -p quantrule-governance -- check --all` | `governance check ok` |
| `make compose-check` | `compose config ok` |
| `qr-deploy validate deploy/v3/deploy.example.json` / `deploy.lab.json` | Both pass |
| `qr-deploy validate deploy/v3/deploy.example.json --release` | Exits 1 with `deploy.image.fixture` |
| Staged `render --target all` | 101 files; `docker compose config -q` passes for `edge-1`, `core-1`, and `infra-1` |
| `tools/release.sh --plan`; `bash -n` on the changed scripts | Plan prints the pins and manual signing step; all scripts parse |
| `tools/swarm.sh --help`; `tools/stateful.sh --plan`; `tools/observability.sh --plan`; `tools/test_tiers.sh ha --plan` | All print their documented plans; HA plan is Docker-free |
| Bun YAML parse of `.github/workflows/release.yml` | Parses; step order is composed -> staging -> digest -> SBOM -> scan -> upload -> version push |
| `tools/gate.sh` | `gate passed: full` |

The live registry push, real cosign signature, live syft SBOM, live Grype scan,
and three-VM lab were not run; they are `Unverified`. The fast tests use fake
syft, Grype, cargo-audit, cosign, skopeo, and Docker binaries. No real key or
secret was created.

### Phase 2 verification

| Command | Observed |
|---|---|
| `cargo test -p quantrule-deploy --test release_tool` | 15 tests pass, including the real-bundle sign, offline bundle/verify, absent-proof label, and tampered-bundle rejection |
| `cargo test -p quantrule-deploy --test remote_cli` | 12 tests pass, including the wrong task image, missing running task, seven-digest pass, wrong runtime digest, and `--runtime` usage error |
| `cargo test -p quantrule-deploy --test ha_harness` | 23 tests pass, including the plan phase and the seven-service `digest_all` contract |
| `qr-deploy check --remote docker` / `--runtime` | Exercised with the fake Docker fixture; a live Swarm is `Unverified` |
| `bash -n deploy/v3/test.sh tools/release.sh` | Both parse |
| `cargo run -q -p quantrule-governance -- check --all` | `governance check ok` |
| `make compose-check` | `compose config ok` |
| Staged `render --target all` for sample and lab; `docker compose config -q` on each infra unit | 101 sample files and 84 lab files; every sample/lab infra unit parses |
| `tools/release.sh --plan`; `tools/swarm.sh --help`; `tools/stateful.sh --plan`; `tools/observability.sh --plan`; `tools/test_tiers.sh ha --plan` | Plans print; HA plan includes the runtime digest phase; no Docker contacted |
| `tools/gate.sh` | `gate passed: full` |

## Review corrections

- The phase-1 `sign` wrote only `signature.json`, and `verify` asked the registry.
  That is not an offline proof; the review corrected it to real Sigstore bundles
  plus `cosign save` and a `--local-image` verify. The metadata is now explicitly
  not the proof.
- The release-note signature value was the public-key fingerprint. The review
  corrected it to the signature-bundle SHA-256.
- The transient registry was named as an unpinned `registry:2`. The review pinned
  the Docker Hub digest and made the plan print it.
- The phase-1 task check compared the service spec image only. The review added
  the running task `Image` and the eight rows, with a stopped task ignored.
- The `/version` ingress digest was the only runtime proof. The review added the
  all-seven probe with the descriptor image and overlay, and made the HA harness
  set `Measured` only on seven matches.
- No live Docker, registry, key, signing, SBOM, scan, or lab was used. The new
  evidence is fake-tool/fake-Docker only and is not a live claim.

## Learning, rejected or corrected suggestions

- The first `releases.rs` proposal would have required the new fields on every
  note after the 2026-11-07 changelog baseline. It was rejected because
  `v1.5.2` and earlier published notes have no signature or SBOM field; the
  baseline is the `v1.5.10` release.
- A `cosign.pub` placeholder PEM was rejected. A fake key can be mistaken for a
  real trust root; `deploy/v3/cosign.pub.example` documents the file, and
  `verify` fails closed until the operator generates the real key.
- A signing step in CI was rejected: the approved decision keeps the private key
  offline and makes signing a manual operator step.
- The first workflow draft pushed the version tag before the scan. It was
  corrected to push a staging tag, scan its digest, and require the version tag's
  digest to equal the staging digest before the release is published.
- Refreshing the 16 infrastructure pins was rejected as out of scope; they stay
  labeled fixtures, and the release note/body records the PLAT-008 blocker.
