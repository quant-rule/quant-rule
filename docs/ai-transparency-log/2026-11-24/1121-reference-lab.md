# AI Transparency Log — PLAT-008 reference topologies, lab, and remote client

- Date: 2026-11-24 (UTC)
- Work item: [#1121](https://github.com/quant-rule/quant-rule/issues/1121)
- Plan: [`docs/execution-plans/2026-11-24/1121-reference-lab.md`](../../execution-plans/2026-11-24/1121-reference-lab.md)
- Tool: OpenCode agent session, DeepSeek V4.1 Flash 1017

## Intent and boundary

The human approved the phase-1 scope before this session: two reference
topologies on the existing `quantrule.deploy/v3` schema, a sizing/support matrix,
the real lab addresses, 14 registry-resolved infrastructure pins, the fixture
denylist, docs, and local tests. The agent implemented that scope in the
dedicated worktree `quant-rule--1121-reference-lab` on `feat/1121-reference-lab`.

The agent did not commit, push, open a PR, merge, tag, deploy, change the board,
create a real secret, certificate, or key, or pull or start a host image. No live
lab was run. The agent read the repository rules, the PLAT-001..007 plans and
ADRs, the v3 descriptor/validator/renderer/checker, the deployment, HA,
operations, security, control-plane, observability, and client architecture
documents, the v3 contracts, manuals, tutorial, deploy scripts, and tools before
writing the records and descriptors.

## Work performed

| Area | Agent contribution |
|---|---|
| Records | Drafted this plan, ADR-1121, the reference contract, and this log from the approved decisions |
| 3-AZ descriptor | Derived `reference/three-az.json` from the sample with 11 hosts, four `az*-rack-*` domains, edge pair on one L2/AZ, managers and stateful members across three AZs, MinIO four racks at EC:2, and observability spread over four infra hosts |
| Single-AZ descriptor | Derived `reference/single-az.json` with the same 11 host names and four `rack-*` domains in one AZ, profile `single-az` |
| Sizing | Wrote `reference/sizing.json` for edge, core, compute, infra, observability, lab, and client, plus platform, network, ports, and the unsupported list |
| Lab | Updated `deploy.lab.json` to VIP `10.10.0.20` and public addresses `10.10.0.11/12`; kept `10.10.0.11..13`, one `lab` domain, and three managers; documented `/etc/hosts`, CA, certificate, and firewall preconditions |
| Pins | Resolved 14 images from registry metadata; recorded tag, index digest, and linux/amd64 digest; kept the app, `minio`, and `minio_client` fixtures exact |
| Release gate | Replaced the one-digest check with a 17-entry fixture denylist covering the app, both MinIO blockers, and all 16 retired infra pins; errors name `image.digest` or `images.<key>.digest` |
| Tests | Added `tests/reference.rs` (sizing, topology, validate, release blockers, render/check); expanded `tests/release_image.rs`; updated the bootstrap fake traefik digest |
| Goldens | Regenerated the checked-in render fixtures with the ignored regeneration test; reviewed the diff as pin-only |
| Docs | Updated the plan, ADR, reference contract, config/ADR indexes, manuals 14/15, architecture client/ha/operations/deployment, deploy README, registry contract, and changelog |

## Phase 2 work

| Area | Agent contribution |
|---|---|
| Lab harness | Added `deploy/v3/lab.sh` with `--reference`, `--plan`, `--confirm`, the five skip flags, the release preflight, the provisioning order, the failure call, and the remote-client mode. Split the value-free evidence and the client phase into `deploy/v3/lab/evidence.sh` and `deploy/v3/lab/remote.sh` after the single file exceeded the 450-line target. |
| Fail closed | The preflight runs `qr-deploy validate <descriptor> --release`, extracts the `deploy.image.fixture` field, prints it with `Unverified`, and exits before Docker, SSH, jq, or a secret file is read. The reference descriptors are never provisioned; only the three-VM lab has lab hosts. |
| Provisioning | The order is stateful `network`/`copy`/`start`/`wait`/`queue-bootstrap`/`object-bootstrap`, `tools/swarm.sh --allow-unverified bootstrap`, observability `copy`/`start`/`wait`/`verify`, and `qr-deploy check --remote docker --overlay --runtime`. The failure phase calls the existing `deploy/v3/test.sh --confirm --skip-deploy`; its logic is not duplicated. |
| Remote CLI | Added `crates/quantrule-shell/tests/remote.rs` against the in-process fixture: URL and token environment, `--token`/`--profile`, a trailing slash, missing/malformed CA fail-closed, no token or CA body in output, and the absence of shell `--remote`/`--cacert`/`--url` flags. |
| Remote browser | Added `apps/web/playwright.remote.config.ts`, `apps/web/e2e-remote/gate.ts`, `e2e-remote/remote.e2e.ts`, and `e2e-remote/config.test.ts`; `bun run test:remote` runs the gate then Playwright. The config has the lab `baseURL`, no route mocks, no certificate bypass; the test asserts load, token, connected state, `/version`, and profiles/datasets. |
| Tier routing | `tools/test_tiers.sh ha` now routes to `deploy/v3/lab.sh`, so `ha --plan` lists the reference, failure, remote-client, and browser phases. |
| Tests | Added seven lab-harness tests (plan Docker-free, fixture fail-closed, remote gate, bash syntax, order, destructive-command refusal, tier plan) and eight shell remote tests. |
| Docs | Updated the reference contract, manuals 08/14/15, deploy README, tools README, architecture client/ha/operations, this plan, this log, and the changelog. |

## Registry resolution

Each row was resolved on 2026-11-24 with `skopeo inspect --raw` and
`skopeo inspect --format '{{.Digest}}'` (registry metadata only; no layer pull).
The index digest was re-hashed from the raw index body and the linux/amd64
manifest was confirmed in the index for all 14. The descriptor pins the index
digest.

| Key | Reference | Tag | Index digest | linux/amd64 digest |
|---|---|---|---|---|
| `traefik` | `docker.io/library/traefik` | `v3.7.20` | `sha256:8301d6f60957a422f98918aacf2b01482abde3e74ead3719aabdb7c64045fe28` | `sha256:cb035e329ea90292290e0199a062aa9bbc78e427356a63a0f8d31e40b058b3bb` |
| `postgres` | `docker.io/library/postgres` | `17.12` | `sha256:156d40cfa063d6babb15d329a8e15b26a8d0690ebb6084c956084f3b025c3b30` | `sha256:e711be2c7ff64c9eb8883fc87571621be6caa2f943d825a49e5d20ac5b0221d7` |
| `patroni` | `docker.io/batonogov/patroni-docker` | `main-17.10-4.0.7-trixie` | `sha256:0de7dc8bc5919c76721bee1475ed30155b1848c0a6bac7298605bf39eef1a7eb` | `sha256:f7f34a52db6218941315d40cc1008156c4694fe93b486efe99f33fd15499534f` |
| `etcd` | `quay.io/coreos/etcd` | `v3.6.17` | `sha256:316f5d367455f75a327f9dc72f669cbf645651859fa2e272f692a1f2dee6f5c9` | `sha256:c521f860433f4f1696e95ff2124c90e8a979766c8443f5f487bdf1b02d238de2` |
| `haproxy` | `docker.io/library/haproxy` | `3.4.8` | `sha256:8ab12982db911f3f13e53c8e4c11312f65b8367301787acbab86459cddf288ea` | `sha256:6ad3bce68cc37443c16f0a1be909d82af126e4ebf419aaa9b31db8ee9d6e8d94` |
| `keepalived` | `docker.io/osixia/keepalived` | `2.4.3` | `sha256:4bf02d569ed240e6da272242b368ce5a25cf0f7d92114536367ee2c963c318a2` | `sha256:3cf650f7570f5e3f1b0a7fe33a900af09cda47569845cdd12947fa8e4553a941` |
| `rabbitmq` | `docker.io/library/rabbitmq` | `4.4.1` | `sha256:183272aa2ff82718cabeef343de8d9a900931ea65fda4e563fed2a6d9efde578` | `sha256:cf6a49ed990149e548f13fa4e02e9446aab2c6e5d51980f0d9a88fe4160fece2` |
| `prometheus` | `docker.io/prom/prometheus` | `v3.9.1` | `sha256:1f0f50f06acaceb0f5670d2c8a658a599affe7b0d8e78b898c1035653849a702` | `sha256:2024d7bdfc559f6254de22d65e62d799eaa8eb818fe5dbeca212d3f200b70890` |
| `grafana` | `docker.io/grafana/grafana` | `13.3.1` | `sha256:0615ec4fbd4072004b7d86b46ded63013a74b7d6eb1119273dd89e8fa25c13cf` | `sha256:1c88285f1dd391a62232c010e6c224846e6a73bdc583b3365be2eba614b7919e` |
| `loki` | `docker.io/grafana/loki` | `3.7.10` | `sha256:b43c257ccfc925d4a50b8717cac61a3b51df8c48d01847936057e5c4075a24a7` | `sha256:6d789ab9a603ee4429d0fdc4d11512bf2423e8a8c5fc898a9e5aa2d8471ce459` |
| `alertmanager` | `docker.io/prom/alertmanager` | `v0.35.0` | `sha256:e638797febd034838eca8b98cdbd46f951e9bf7f178777aa8669167406a9946c` | `sha256:0f6727a39ff00b6cf95e8a5ebaf7442c55af855b79c689fb9a060e2bada1ecbf` |
| `node_exporter` | `docker.io/prom/node-exporter` | `v1.13.1` | `sha256:7296504d4ada44b7d7ab4431e27e9f9900d8488f8d55842f4b12ae2704e16dfe` | `sha256:f99f6e8dcad00fb95ff9fe81ceb1e6442fd9b4180f177da02e8437c845d570eb` |
| `promtail` | `docker.io/grafana/promtail` | `3.6.11` | `sha256:a761cb834cfaeee29745440d4884d6748f0a08d8f68928db1d707018c1dcfbe9` | `sha256:90a93ba8969799344f197d408334b57d4b86b8b11f32506f840606fc9588366d` |
| `blackbox_exporter` | `docker.io/prom/blackbox-exporter` | `v0.29.1` | `sha256:ea1c7aa3ef3f307cd03e85f34abb604478ccb90b567fc690da0cd9776da76b13` | `sha256:66eae352a79e6f677aeddd8c25a9bdb017586dc5fbd230f22329213d891f94bd` |

The `patroni/patroni` repository was probed and refused anonymous access; the
project has no official image. `batonogov/patroni-docker` is a maintained public
Patroni + PostgreSQL 17 image and provides the `patroni` entrypoint the renderer
calls. The old `patroni` fixture digest remains in the denylist.

The three blockers were probed and left unchanged: the application fixture
`sha256:cdace4…`, `minio/minio` `sha256:bdf0781a…`, and `minio/mc`
`sha256:5c5b1f8f…`. No digest was guessed.

## Verification and evidence

| Command | Observed |
|---|---|
| `skopeo inspect --raw` + `--format '{{.Digest}}'` for the 14 images | 14/14 index digests match the raw body hash; linux/amd64 is present in every index |
| `qr-deploy validate deploy/v3/reference/three-az.json` | `ok: v3 swarm-ha, 11 hosts, 7 services ... (files not checked; pass --files)` |
| `qr-deploy validate deploy/v3/reference/single-az.json` | `ok: v3 single-az, 11 hosts, 7 services ...` |
| `qr-deploy validate deploy/v3/deploy.lab.json` | `ok: v3 lab-single-host, 3 hosts, 7 services ...` |
| `cargo test -p quantrule-deploy --test reference` | 7 tests pass: sizing consistency, host-role coverage, doc table, 3-AZ shape, single-AZ shape, validate/release blockers, render/check |
| `cargo test -p quantrule-deploy --test release_image` | 10 tests pass: app fixture, MinIO and MinIO-client blockers, all 16 retired pins at the field, fully pinned accept, bare-reference refusal |
| `cargo test -p quantrule-deploy --test render_golden -- --ignored` | Regenerated both golden trees; review showed only image pin lines changed |
| `cargo fmt --all -- --check` | Clean |
| `make compose-check` | `compose config ok` |
| `cargo clippy -p quantrule-deploy --all-targets -- -D warnings` | Clean |
| `cargo test -p quantrule-deploy` | 265 unit tests plus every integration suite pass; 7 reference, 10 release-image, 15 release-tool, 2 render-golden (+1 ignored), 22 CLI, 23 HA, and the remaining suites |
| `qr-governance check --all` | `governance check ok` |
| `qr-deploy validate` on sample, lab, 3-AZ, single-AZ | All four print `ok` with 11 or 3 hosts and 7 services |
| `qr-deploy validate <descriptor> --release` | All four exit 1 at `deploy.image.fixture`/`image.digest`; a temp 3-AZ with a real app digest names `images.minio.digest`, then `images.minio_client.digest` |
| Staged `render --target all` for sample, lab, 3-AZ, single-AZ | All render; `docker compose config -q` passes on each `stack.yaml` and 3/11/11/11 infra units |
| `tools/release.sh --plan`; `bash -n tools/release.sh` | Plan prints the new three-blocker line; script parses |
| `tools/test_tiers.sh ha --plan` | Docker-free plan prints, including the lab digest and observability phases |
| `tools/gate.sh` | `gate passed: full with CARGO_TARGET_DIR=.../target/gate` |

### Phase 2 verification

| Command | Observed |
|---|---|
| `bash -n deploy/v3/lab.sh`; `bash -n deploy/v3/lab/evidence.sh`; `bash -n deploy/v3/lab/remote.sh` | All parse |
| `deploy/v3/lab.sh --plan` | Prints the selected reference, descriptor, provisioning order, remote commands, browser command, preconditions, evidence path, and `Simulated HA`/HA-014 boundary; no Docker/SSH/jq/qr-deploy call |
| `deploy/v3/lab.sh --reference lab` | `lab: blocker: image.digest`; `UNVERIFIED: deploy.image.fixture at image.digest ... no host is contacted`; exit 1 |
| `deploy/v3/lab.sh --remote-client --confirm` | `QR_LAB_PROVISIONED=1` gate, `Unverified`, exit 1; no network |
| `tools/test_tiers.sh ha --plan` | Lists the lab reference/lab descriptor, failure phases from `test.sh --plan`, remote client, and browser; no `deploy.example.json` |
| `cargo test -p quantrule-deploy --test ha_harness` | 30 tests pass, including the lab plan, fixture and non-lab-profile fail-closed, remote gate, bash, order, and tier-plan tests |
| `cargo test -p quantrule-shell --test remote` | 8 tests pass |
| `bun test apps/web/e2e-remote/config.test.ts` | 4 tests pass |
| `bun run e2e-remote/gate.ts`; `bun run test:remote` | `Unverified: set QR_REMOTE_BROWSER=1 ...`; exit 1 |
| `bun install --frozen-lockfile`; `bun run check`; `bun test`; `bun run e2e` | Install ok; 0 svelte-check errors; 186 Bun tests pass; 64 local chromium Playwright tests pass |
| `bunx playwright test --config playwright.remote.config.ts --list` | `Unverified: the remote browser suite is lab-gated ...`; exit 1 |
| `tools/gate.sh` | Passed (`gate passed: full`) |

A live lab, remote CLI/browser TLS connection, real certificate, VIP failover,
firewall, image pull, and `Real HA` were not observed. Those rows are
`Unverified`; the lab can claim `Simulated HA` at most, and `Real HA` is HA-014.

## Learning and corrections

- The sample's `az1..az4` is a four-domain example, not four AZs. Reusing it as a
  3-AZ topology would have overclaimed. The reference now uses four rack domains
  and two racks in AZ1, with the edge/VIP pair on one L2/AZ.
- The validator's `swarm-ha` ingress rule already requires two failure domains
  but not two AZs, so the approved edge pair is expressible without a schema
  change. The test now asserts the edge pair is both same-AZ and different-rack.
- `patroni/patroni` is not a resolvable public image; pinning a plausible but
  non-existent digest would have been worse than the maintained
  `batonogov/patroni-docker` image. The choice and the probe result are recorded.
- The first reference check failed on the fake certificate SAN because
  PostgreSQL/etcd/RabbitMQ now use `infra-4` as well as `infra-1..3`. The test
  fixture was corrected to include `patroni-infra-4`, `etcd-infra-4`, and
  `rabbitmq-infra-4`; the production descriptor was not weakened.
- The lab's public address equals its LAN address on the edge VMs. That is
  intentional for the flat lab LAN and is documented; it is not a second public
  interface.
- A guessed MinIO or application digest was rejected. The three blockers stay
  exact and fail with their field paths.
- The first single-file lab harness reached 537 lines, above the 450-line target.
  The evidence and remote phases were split into `deploy/v3/lab/` scripts; the
  entry point is 341 lines and the test asserts the provisioning order.
- The first preflight called `qr-deploy validate --release <path>`, but the CLI
  takes the descriptor first. The harness now runs `validate <path> --release`
  and the fake-CLI test covers the exact `deploy.image.fixture` field.
- The three-AZ and single-AZ descriptors use documentation addresses
  (`198.51.100.x`), so the harness validates them and refuses to provision them.
  Only `deploy.lab.json` has lab hosts; the contract already said only the lab
  can reach `Simulated HA`.
- The untrusted-CA negative uses `curl --cacert /dev/null` rather than a bare
  `curl`: the same client must trust the private CA in the OS/NSS store for the
  browser, so a bare curl could pass for the wrong reason.
- `tools/verify.sh` already had `--self-test`, `--api-url`, `--cacert`, and
  `--profile`; no public contract change was needed. The remote phase calls the
  existing script and prints only the digest.
- The web dependencies and Playwright were installed with `bun install
  --frozen-lockfile`. The local `bun run check`, `bun test`, and `bun run e2e`
  pass; the remote suite still prints `Unverified` without the lab. The remote
  config remains outside `tsconfig.json` (`include: src/**`), so `bun run check`
  does not typecheck it; the Bun config test is the local guard.

## Limitations

The pin metadata is a snapshot; a tag or digest can be withdrawn by the registry.
Release mode is an offline denylist, not a signature or vulnerability proof. The
reference descriptors are designs validated locally, not a live deployment. The
managed-stateful profile is not implemented. The lab harness and remote tests are
implemented, but no live lab, TLS connection, browser run, real certificate,
token, or host was exercised. The three fixtures still block provisioning;
`Simulated HA` is the maximum and `Real HA` is HA-014.
