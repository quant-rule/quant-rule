# PLAT-007 — Image registry, signing, SBOM and CVE scan (phases 1–2)

- Date: 2026-11-24 (UTC)
- Status: In Progress
- Issue: [#1120](https://github.com/quant-rule/quant-rule/issues/1120)
- Claim type: Behaviour Claim
- AI log: [`docs/ai-transparency-log/2026-11-24/1120-registry.md`](../../ai-transparency-log/2026-11-24/1120-registry.md)
- Workflow: [Execution Plan](../../workflows/execution-plan.md), [Release & Operations](../../workflows/release-operations.md)
- ADR: [`docs/adr/2026-11-24/1120-image-distribution.md`](../../adr/2026-11-24/1120-image-distribution.md)
- Contract: [`docs/config/2026-11-24/registry.md`](../../config/2026-11-24/registry.md)
- Phase: phases 1–2 implemented below, including the review corrections in section 13. The PR review gate and the live lab remain the handoff.

## 1. Intent

Make the released application image distributable, verifiable, and attributable:
pull by digest from a public GHCR package, sign the pushed digest with an
offline cosign key, publish an SPDX SBOM, and fail the release when a Critical or
fixable High CVE is open. Phase 1 owns the release contract, the operator tool,
the release workflow, evidence, descriptor pinning, the release-note schema, and
the documentation. It does not claim a live registry push, a live signature, a
live SBOM, a live scan, `Real HA`, or bit-for-bit reproducible rebuilds.

## 2. Current state

- `.github/workflows/release.yml` builds, checks, composes, pushes, and records a
  digest, but generates no SBOM, runs no CVE gate, and signs nothing.
- `deploy/v3/deploy.example.json` and `deploy.lab.json` point at
  `registry.quantrule.dev/quantrule/server` with a fixture digest; the reference
  is not a pullable package.
- The release note template records one `- Image:` line with the digest embedded
  and has no signature, SBOM, or scan field.
- `tools/release.sh` does not exist; `tools/swarm.sh bootstrap/upgrade` deploy
  without verifying a signature.
- The 16 infrastructure image pins are labelled fixtures and are not refreshed by
  PLAT-007; a production descriptor still needs real infrastructure digests.

## 3. Scope

Included:

- The plan, AI log, ADR, and registry contract.
- `tools/release.sh` with `--plan`, `sbom`, `scan`, `sign`, `verify`, `bundle`,
  `pin`, and a pinned `install-tools` helper.
- Release workflow SBOM, CVE gate, staging digest, final tag push after the gate,
  release assets, and release body fields.
- `deploy/release/cve-allowlist.json` with the strict schema and an empty list.
- `deploy/v3/cosign.pub.example`; the real public key is operator-generated and
  `verify` fails closed when it is absent.
- GHCR sample/lab reference, fixture labelling, regenerated render goldens, and
  release-mode fixture-digest refusal.
- Release-note fields, baseline, governance checker, and tests.
- Docs, manual, Makefile, tools README, and changelog.
- Phase 1 verification and the full gate.
- Phase 2: real Sigstore bundles and a `cosign save` signed image in the offline
  bundle, `qr-deploy` running-task image verification, the all-seven-service
  runtime digest probe, and the HA harness all-service digest row.

Excluded:

- A real registry push, signature, SBOM, scan, key, or secret.
- A live lab run of the phase-2 probes.
- Refreshing the 16 infrastructure image pins.
- A self-hosted HA registry, a new backup unit, a new Rust dependency, a new
  public shell command, or Python.
- A commit, push, PR, merge, tag, deployment, or board change in this phase.

## 4. Assumptions and uncertainty

- The GHCR repository and package are public, so a host pulls by digest without a
  registry credential. A private package remains a configurable override.
- CI has network access to download the pinned syft, Grype, and cosign release
  artifacts and to run Grype's vulnerability database update; the fast tier never
  does either.
- The offline cosign private key is held by the operator and is never in CI, a
  workflow secret, the repository, or an agent environment.
- Syft's SPDX JSON subject is the exact pushed manifest digest; the SBOM is
  evidence, not a reproducible build artifact.
- The allowlist is a human-approved, expiring exception record; PLAT-007 ships it
  empty.
- The sample/lab fixture digest is not a real image digest and is not deployable;
  release mode rejects it.

## 5. Decisions

| Decision | Choice | Status |
|---|---|---|
| Distribution | GHCR pull-by-digest is primary; offline `skopeo` OCI bundle plus a transient local `registry:2` is the fallback | Approved |
| Package | `ghcr.io/quant-rule/quantrule-platform`, public; no host credential | Approved |
| Signing | cosign key-based; private key offline; CI does not sign | Approved |
| Deploy gate | `tools/swarm.sh bootstrap/upgrade` verifies before deploy; `swarm-ha`/`single-az` fail closed; the lab may print `Unverified` only with `--allow-unverified` | Approved |
| SBOM | Syft SPDX JSON from the exact digest; attach to the release; hash in the note; cosign attest when signing | Approved |
| CVE | Grype on the SBOM plus `cargo audit`; Critical always fails; High fails with a fix; unfixed High needs a valid allowlist entry; Medium report-only | Approved |
| Tooling | Pinned cosign, syft, Grype, skopeo image digest, and cargo-audit; no new Rust dependency | Approved |
| Descriptor pin | `tools/release.sh pin` rewrites only `.image`, validates, and prints `reference@digest` | Approved |
| Release note | Add `Image`, `Image digest`, `Image signature`, `SBOM`, and `CVE scan` fields for releases at or after `v1.5.10` | Approved |
| Fixture pins | The 16 infrastructure pins remain fixtures; a production descriptor needs real digests | Approved with recorded boundary |
| Evidence | Live push/sign/SBOM/scan and the lab remain `Unverified` here; `Real HA` is HA-014 | Approved |

## 6. Acceptance criteria

- `tools/release.sh --plan` is Docker/network/key/secret-free and names the
  sequence, tool pins, evidence names, manual signing step, and fixture blocker.
- `sbom` refuses an unpinned reference and prints the subject digest and file
  SHA-256; `scan` writes Grype JSON/text and cargo-audit JSON, applies the
  allowlist, and exits non-zero on Critical, fixable High, an unfixed High without
  an entry, or an expired/malformed entry.
- `sign` never echoes the key or password, writes `cosign.bundle`,
  `attestation.bundle`, and `signed-image/`, and records their hashes plus the
  public-key fingerprint in `signature.json`; `verify` fails closed when
  `deploy/v3/cosign.pub` is absent and verifies `--local-image` offline.
- `bundle` writes an OCI layout and `bundle.json`, copies the real Sigstore
  bundles and saved image when evidence is passed, hashes every file, starts no
  daemon, and deletes nothing.
- `qr-deploy check --remote docker` compares every running task image, and
  `--runtime` requires all seven service digests to equal the descriptor; the HA
  tier runs the probe after deploy and after restore and marks it `Measured` only
  when all seven match.
- `pin` rewrites only the app image, validates with `qr-deploy`, and leaves all 16
  infrastructure pins byte-identical.
- The release workflow builds once for `linux/amd64`, pushes a staging digest,
  generates the SBOM and runs the CVE gate before the version image tag is
  pushed, requires the staging and final digest to match, attaches the SBOM and
  scan report, and never signs.
- The sample/lab validate as `ghcr.io/quant-rule/quantrule-platform@sha256:<fixture>`;
  all seven rendered services use the same reference@digest and `QR_IMAGE_DIGEST`;
  release mode rejects the fixture digest.
- A release note at or after `v1.5.10` needs the five fields; `pending` is allowed
  only before `Status: Released`; historical notes remain valid.
- `tools/swarm.sh bootstrap/upgrade` verifies before deploy; the lab exception is
  explicit and prints `Unverified`.
- Docs, plan, AI log, ADR, contract, and changelog agree; every doc cap is kept.
- No real secret/key, commit, push, PR, merge, tag, deployment, or board change.

## 7. Work breakdown

| Step | Work | Role | Paths |
| ---: | --- | --- | --- |
| 1 | Plan, AI log, ADR, contract before code | Orchestrator / Architect | `docs/` |
| 2 | Release tool, pins, allowlist, public-key example | Implementation | `tools/release.sh`, `deploy/release/`, `deploy/v3/` |
| 3 | GHCR reference, goldens, release-mode check | Implementation | `deploy/v3/*.json`, `validate/v3.rs`, `check/`, `main.rs` |
| 4 | Release workflow SBOM/CVE/evidence/body | Implementation | `.github/workflows/release.yml` |
| 5 | Release-note fields and checker | Implementation / Test | `checks/releases.rs`, `docs/releases/README.md` |
| 6 | Swarm pre-deploy verification | Implementation | `tools/swarm.sh`, `tools/swarm_wrapper.rs` |
| 7 | Tests: tool, pin, descriptor, workflow order, release note | Test | `crates/*/tests`, `checks/releases.rs` |
| 8 | Docs, manuals, Makefile, tools README, changelog | Documentation | `docs/`, `Makefile`, `tools/README.md`, `CHANGELOG.md` |
| 9 | Format, clippy, tests, governance, compose, render, gate | Verification | repository |
| 10 | Phase 2: `qr-deploy` task-image verification and all-seven runtime digest probe | Implementation / Test | `check/`, `deploy/v3/test.sh`, gate |

## 8. Verification

```bash
cargo fmt --all -- --check
cargo clippy -p quantrule-governance -p quantrule-deploy --all-targets -- -D warnings
cargo test -p quantrule-governance
cargo test -p quantrule-deploy
cargo test -p quantrule-deploy --test release_tool
cargo test -p quantrule-deploy --test remote_cli
cargo test -p quantrule-deploy --test ha_harness
cargo run -q -p quantrule-governance -- check --all
make compose-check
qr-deploy validate deploy/v3/deploy.example.json
qr-deploy validate deploy/v3/deploy.lab.json
qr-deploy validate deploy/v3/deploy.example.json --release   # expected failure
qr-deploy render <staged sample> --out <tmp> --target all
docker compose -f <tmp>/infra/<host>.yaml config -q
qr-deploy check <deploy.json> --remote docker --runtime   # lab, fake Docker in tests
tools/release.sh --plan
bash -n tools/release.sh tools/swarm.sh
tools/swarm.sh --help
tools/stateful.sh --plan
tools/observability.sh --plan
tools/test_tiers.sh ha --plan
tools/gate.sh
```

The fast tier never starts Docker, pushes, signs, or scans. A live registry,
signature, SBOM, CVE scan, and lab are `Unverified` here.

## Verification table

| ID | Plan item | Acceptance criterion | Evidence / command | Expected | Observed | Status | Reviewer | Date | Notes |
|---|---|---|---|---|---|---|---|---|---|
| V01 | Records | Plan, AI log, ADR, and contract exist before code | `ls docs/execution-plans/2026-11-24/1120-registry.md docs/ai-transparency-log/2026-11-24/1120-registry.md docs/adr/2026-11-24/1120-image-distribution.md docs/config/2026-11-24/registry.md` | Four files | Created before code | Pass | agent | 2026-11-24 |  |
| V02 | Release tool | Plan is offline; `bash -n`; args validated | `cargo test -p quantrule-deploy --test release_tool`; `tools/release.sh --plan` | 12 tests; pins and signing step | 12 tests pass; plan prints cosign/syft/Grype/skopeo/cargo-audit pins and the manual sign/verify step; `bash -n` clean | Pass | agent | 2026-11-24 | Fake tools only; no live scan |
| V03 | Pin | Only `.image` changes; validate passes; infra pins unchanged | `cargo test -p quantrule-deploy --test release_tool pin_rewrites` | Pass | `image` rewritten to GHCR@digest; `.images` deep-equal; `qr-deploy validate` accepted | Pass | agent | 2026-11-24 |  |
| V04 | CVE | Critical always fails; fixable High fails; unfixed High allowlisted; expired/malformed fails | `cargo test -p quantrule-deploy --test release_tool` | Stable exit | Clean passes; Critical/fixable High/unfixed High fail; valid unexpired entry passes; expired and malformed fail; cargo-audit fixed High fails | Pass | agent | 2026-11-24 | Fake Grype/cargo fixtures |
| V05 | Descriptor | GHCR reference; seven services share ref@digest and `QR_IMAGE_DIGEST`; fixture labelled | `cargo test -p quantrule-deploy --test release_image --test render_golden` | Pass | 7 release-image tests and 2 golden renders pass; 7 image lines and 7 `QR_IMAGE_DIGEST` lines per stack | Pass | agent | 2026-11-24 |  |
| V06 | Release mode | Fixture digest rejected only with `--release`/env; sample/lab default pass | `qr-deploy validate deploy/v3/deploy.example.json --release`; `deploy.lab.json` | `deploy.image.fixture` | Both default validates pass; both release validates exit 1 with `deploy.image.fixture`; `QR_DEPLOY_RELEASE=1` and `check` also reject | Pass | agent | 2026-11-24 |  |
| V07 | Workflow | SBOM -> scan -> version tag; staging equal final; evidence attached; no signing | `cargo test -p quantrule-governance --test release_workflow`; Bun YAML parse | Pass | 16 tests pass; YAML parses; step order is composed -> staging -> digest -> SBOM -> scan -> upload -> version push; publish attaches SBOM/CVE and names `pending` + PLAT-008; no `cosign sign` | Pass | agent | 2026-11-24 | Text/YAML checks only |
| V08 | Release note | New fields required after `v1.5.10`; `pending` only pre-final; historical passes | `cargo test -p quantrule-governance --lib checks::releases` | Pass | 14 tests pass; final note passes; each missing signature/SBOM/scan, malformed digest, and final `pending` fail; v1.5.2 historical passes | Pass | agent | 2026-11-24 |  |
| V09 | Swarm gate | Verify before deploy; fail closed; lab exception prints `Unverified` | `cargo test -p quantrule-deploy --test swarm_wrapper` | Pass | 10 tests pass; bootstrap calls verify; missing key exits 1; lab `--allow-unverified` prints `Unverified`; production flag refused | Pass | agent | 2026-11-24 | Fake cosign |
| V10 | Docs | Contract, ADR, releases, workflow, manuals, architecture within caps | `qr-governance check --all`; `wc -l` | Zero findings; caps | `governance check ok`; ADR 106; registry 240; releases 330; workflow 217; manual 15 200; manual 08 330; architecture 170/170/170/166/170; deploy-v3 330 | Pass | agent | 2026-11-24 |  |
| V11 | Quality | fmt, clippy, tests, governance, compose, validate, render, gate | verification block | Pass | fmt/clippy clean; deploy 257 unit + 22 CLI + 6 entrypoint + 22 HA + 2 MinIO CLI + 10 MinIO harness + 8 observability + 4 queue CLI + 13 definitions + 7 release-image + 12 release-tool + 7 remote + 2 golden + 14 stateful + 10 bootstrap + 10 wrapper; governance 125 unit + 4 composed + 6 gate + 5 history + 16 release-workflow; governance/compose ok; descriptors validate; staged 101-file render parses edge/core/infra; `tools/gate.sh` full passed | Pass | agent | 2026-11-24 |  |
| V12 | Evidence | Live push/sign/SBOM/scan/lab not run | n/a | Unverified | No live registry, key, SBOM, scan, or lab in this environment | Blocked | agent | 2026-11-24 | Lab stays `Unverified`; `Real HA` is HA-014 |
| P01 | Phase 2 | Running-task image verification | `qr-deploy check --remote docker`; `cargo test -p quantrule-deploy --test remote_cli` | Seven services plus ingress | `service ps` `Image` added to `TaskInspect`; 8 image rows; a stopped task is ignored; a foreign or missing running image fails; fake fixtures cover both | Pass | agent | 2026-11-24 | Docker-free tests |
| P02 | Phase 2 | All-seven runtime digest probe | `qr-deploy check --remote docker --runtime`; `cargo test -p quantrule-deploy --test remote_cli` | Seven digests, fail closed | Docker `run --network <overlay> --entrypoint curl`; `/version` for six, backtest `/health`; all seven compared; wrong/missing/unavailable rows fail | Pass | agent | 2026-11-24 | No live image/overlay |
| P03 | Phase 2 | HA harness digest row | `cargo test -p quantrule-deploy --test ha_harness`; `bash -n deploy/v3/test.sh` | Measured only on 7/7 | Plan names the phase; `digest_all` runs after deploy and restore, prints only service + digest, and is `Unverified` otherwise | Pass | agent | 2026-11-24 | Live lab `Unverified` |
| P04 | Correction 1 | Real Sigstore bundle and signed image | `cargo test -p quantrule-deploy --test release_tool` | Bundle hashes and offline verify | `sign` uses `--bundle` twice and `cosign save`; `bundle` copies and hashes `cosign.bundle`/`attestation.bundle`/`signed-image/`; `verify --local-image`; fake tools only | Pass | agent | 2026-11-24 | No real key/cosign |
| P05 | Correction 1 | Pinned transient registry | `tools/release.sh --plan`; `release_tool` test | Pinned digest only | `registry:2@sha256:a3d8…` printed in the plan and bundle output; no unpinned `registry:2` | Pass | agent | 2026-11-24 | Fixture pin |
| P06 | Evidence | Live push/sign/SBOM/scan/lab still not run | n/a | Unverified | No live registry, key, SBOM, scan, or lab in this environment | Blocked | agent | 2026-11-24 | `Real HA` is HA-014; infra pins await PLAT-008 |
| P07 | Quality | fmt, clippy, tests, governance, compose, validate, render, gate after phase 2 | verification block | Pass | fmt/clippy clean; deploy 265 unit + 22 CLI + 6 entrypoint + 23 HA + 15 release-tool + 12 remote-cli + 7 release-image + render/stateful/bootstrap/wrapper suites; governance 125 unit + 4 composed + 6 gate + 5 history + 16 release-workflow; `governance check ok`; `compose config ok`; both defaults pass and both `--release` fail `deploy.image.fixture`; staged 101-file sample and 84-file lab renders parse every infra unit; plans and `bash -n` clean; `tools/gate.sh` passed | Pass | agent | 2026-11-24 | Live lab `Unverified` |

## 9. Risks and recovery

| Risk | Control |
|---|---|
| A host pulls a mutable tag | Descriptor uses `reference@sha256:`; release mode rejects the fixture; `pin` writes the digest |
| A malicious or wrong image is deployed | cosign verify before bootstrap/upgrade; fail closed outside the explicit lab exception |
| A signature key leaks | Private key stays offline; CI has no signing step and no key secret; public key only |
| A vulnerable image is released | CVE gate before the version image tag; Critical always fails; fixable High fails; allowlist expires |
| The allowlist hides a vulnerability | Strict schema, owner, reason, ticket, expiry; expired/malformed fails; Medium stays report-only |
| The staging digest differs from the final tag | Final digest is inspected and compared to the staging digest |
| CI holds a registry token unnecessarily | `packages: write` stays on the image job only; the release job keeps `contents: write` |
| A historical release note breaks | The supply-chain fields apply only at or after `v1.5.10` |
| Infrastructure fixture pins look real | The contract, plan, README, release body, and tests label them as fixtures and record the PLAT-008 blocker |
| The lab looks verified | `--allow-unverified` prints `Unverified`; no pass is printed |

Recovery: keep the previous descriptor, digest, signature, SBOM, and scan report.
A failed scan blocks the version tag and the GitHub release; the staging tag is
not a release. A failed signature verification blocks deploy. Rollback is the
previous verified digest; no tag, signature, SBOM, or release asset is deleted.

## 10. Stop conditions

Stop if a real key or secret would be needed; if a scan would be skipped or a
failure downgraded; if the version image tag would be pushed before the gate
passes; if the sample/lab default check would fail; if the 16 infrastructure pins
would change; if a doc cap would be exceeded; if a new dependency or Python would
be needed; or if the full gate fails. A live registry, signature, SBOM, scan, or
lab result that was not observed is `Unverified`, never a pass.

## 11. Phase 1 result

- `tools/release.sh` is a 450-line shell tool with `--plan`, `sbom`, `scan`,
  `sign`, `verify`, `bundle`, `pin`, and `install-tools`. Its 12 tests cover the
  offline plan, argument validation, descriptor pinning, the allowlist gate,
  SBOM hashing, the secret-free sign path with SBOM attestation, and fail-closed
  verification. The tool pins cosign `v3.1.4`, syft `1.57.2`, Grype `0.122.1`,
  skopeo `v1.25.1` by image digest, and cargo-audit `0.23.0`.
- The application image is pinned to the public
  `ghcr.io/quant-rule/quantrule-platform` package with the labelled fixture
  digest. All seven rendered services use the same `reference@digest` and
  `QR_IMAGE_DIGEST`; default `qr-deploy validate` passes, while `--release` or
  `QR_DEPLOY_RELEASE=1` returns `deploy.image.fixture`.
- The release workflow installs the pinned tools, builds once for `linux/amd64`,
  pushes a staging tag, generates the SBOM from the staging digest, runs Grype
  plus `cargo audit` before the version image tag, requires the final digest to
  equal the staging digest, and attaches `sbom-<tag>.spdx.json` and
  `cve-report-<tag>.json`. It does not sign and does not create or move a tag.
- `tools/swarm.sh bootstrap/upgrade` verifies with the offline public key before
  deploy. `swarm-ha` and `single-az` fail closed; the lab can continue only with
  `--allow-unverified`, which prints `Unverified`. The HA lab harness passes that
  flag to its bootstrap so a lab without a key remains explicitly unverified.
  The real public key is absent;
  `deploy/v3/cosign.pub.example` documents the operator step.
- Release notes at or after `v1.5.10` require `Image`, `Image digest`,
  `Image signature`, `SBOM`, and `CVE scan`; `pending` is allowed only before
  `Status: Released`. Historical notes are unchanged.
- `cargo fmt`, clippy, `cargo test -p quantrule-governance`, `cargo test -p
  quantrule-deploy`, `qr-governance check --all`, `make compose-check`, both
  descriptor validates, the staged 101-file render and its edge/core/infra
  Compose parses, all tool plans, the HA plan, the YAML parse, and
  `tools/gate.sh` passed.
- A live push, signature, SBOM, CVE scan, and three-VM lab are `Unverified` here.
  The 16 infrastructure pins remain fixtures; PLAT-008 is the blocker. `Real HA`
  is HA-014. No commit, push, PR, merge, tag, deployment, board change, or real
  secret/key was made.

## 12. Phase 2 result

- `qr-deploy` now reads the Docker `service ps` `Image` field into `TaskInspect`.
  `--remote docker` adds eight rows, one for each of the seven services and the
  ingress: every running task must equal the descriptor `reference@digest`; a
  stopped or failed task is ignored, and a foreign or missing running image is a
  failure. The aggregate task-state row is unchanged.
- `qr-deploy check <deploy.json> --remote docker --runtime` runs only the
  all-service digest probe: `docker run --rm --network <descriptor overlay>
  --entrypoint curl <descriptor image@digest> -fsS http://<service>:8080/version`
  for gateway/data/model/strategy/evidence/simulation and
  `http://backtest:9090/health` for backtest. It parses `.image_digest`, compares
  all seven with the descriptor, and fails closed; Docker, image, or overlay
  unavailability is an `unavailable` failed row, never a pass.
- `deploy/v3/test.sh` calls the probe after deploy and after restore, prints only
  `digest <service> <digest>`, and sets the all-service `digest_status=Measured`
  only when all seven match. The final summary prints the all-service digest and
  the end-to-end canonical digest separately. The HA `--plan` names the phase.
- `tools/release.sh sign` now runs `cosign sign --key <key> --bundle
  cosign.bundle --yes <target>`, `cosign attest --type spdxjson --bundle
  attestation.bundle` when an SBOM is supplied, and `cosign save --dir
  signed-image <target>`. `signature.json` records
  `signature_bundle_sha256`, `attestation_bundle_sha256`, `signed_image`, and
  the public-key fingerprint.
- `tools/release.sh verify <target> [bundle-dir]` runs
  `cosign verify --key <pub> --local-image <bundle>/signed-image <target>` when
  the saved image exists, otherwise verifies from the registry, and checks the
  metadata digest and recorded bundle hashes. It fails closed without the public
  key.
- `tools/release.sh bundle <target> <out> [evidence]` copies the real
  `cosign.bundle`, `attestation.bundle`, `signature.json`, and `signed-image/`
  from the evidence directory, hashes every file in `bundle.json`, and prints
  the exact offline verify command. Without evidence it labels the signed-image
  proof absent. The transient registry is pinned as
  `registry:2@sha256:a3d8aaa63ed8681a604f1dea0aa03f100d5895b6a58ace528858a7b332415373`.
- Release notes record the signature-bundle SHA-256 (not the public-key
  fingerprint) in `Image signature`.

## 13. Review corrections

| Correction | Change | Evidence |
|---|---|---|
| 1. Offline proof | Real Sigstore bundles and a `cosign save` signed image; metadata is not the proof; offline `--local-image` verify; pinned transient registry | `release_tool` 15 tests; `--plan`; `bash -n` |
| 2. Running task image | `TaskInspect.Image`; eight rows; stopped ignored; foreign/missing fails | 12 `remote_cli` tests; 5 new unit tests |
| 3. All-service digest | `--runtime` probe; HA `digest_all` after deploy/restore; `digest_status=Measured` only 7/7 | `remote_cli` runtime tests; HA plan/harness test |

## 14. Remaining evidence

- A live registry push, real cosign signature, live syft SBOM, live Grype scan,
  and a three-VM lab are `Unverified`; no key or secret was created.
- The phase-2 probes are proven with fake Docker and fake cosign. The live HA lab
  is `Unverified` and is at most `Simulated HA`; `Real HA` is HA-014.
- The 16 infrastructure image pins remain fixtures until PLAT-008; the sample and
  lab application digest remains a labelled fixture.
