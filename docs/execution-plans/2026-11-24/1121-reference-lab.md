# PLAT-008 — Reference Topologies, Single-Host Lab, and Remote Client

- Date: 2026-11-24 (UTC)
- Status: In Progress
- Issue: [#1121](https://github.com/quant-rule/quant-rule/issues/1121)
- Claim type: Behaviour Claim
- AI log: [`docs/ai-transparency-log/2026-11-24/1121-reference-lab.md`](../../ai-transparency-log/2026-11-24/1121-reference-lab.md)
- Workflow: [Execution Plan](../../workflows/execution-plan.md), [Requirements & Design](../../workflows/requirements-design.md)
- ADR: [`docs/adr/2026-11-24/1121-reference-topologies.md`](../../adr/2026-11-24/1121-reference-topologies.md)
- Contract: [`docs/config/2026-11-24/reference-topologies.md`](../../config/2026-11-24/reference-topologies.md)
- Phase: phase 2 of 2. Phase 1 owns the records, both reference descriptors,
  sizing, lab addresses, 14 real pins, the fixture denylist, docs, and local
  tests. Phase 2 owns the lab provisioning harness, remote CLI/browser TLS tests,
  and the final gate rerun. The harness and tests are implemented; the live lab,
  TLS connection, and browser run remain `Unverified` at the three fixtures.

## 1. Intent

Give the operator two reviewable reference topologies and one lab template
without inventing a new schema: a 3-AZ design and an honest single-AZ design,
both using the existing `failure_domain` label, plus a machine-readable sizing
and support matrix. Resolve the infrastructure image pins that can be resolved
from real registry metadata, keep the three blocked digests exact, and make
release mode reject every fixture or reverted pin at its field. Phase 1 improves
the operator's ability to choose and validate a topology; it does not claim a
live lab, a real HA result, or a public-cloud managed service.

## 2. Current state

- `deploy/v3/deploy.example.json` is the only eleven-host HA sample. It has no
  checked-in 3-AZ or single-AZ reference and no sizing matrix.
- `deploy/v3/deploy.lab.json` already names `lab-1..3` at `10.10.0.11..13`, but
  its VIP and public addresses are documentation values; the lab preconditions
  are not written down.
- The 16 infrastructure pins are labelled fixtures. The app, `minio`, and
  `minio_client` fixtures are the three that cannot be resolved anonymously.
- `validate_release` rejects only the app fixture. A reverted infrastructure pin
  would pass release mode.
- `docs/config/2026-11-24/registry.md` records the fixture boundary but has no
  pin table.
- The command, render, and check surface is current; its goldens contain the old
  fixture pins.

## 3. Scope

Included:

- The plan, AI log, ADR, and reference contract.
- `deploy/v3/reference/three-az.json` (`swarm-ha`, 11 hosts, four rack domains
  across three AZs, edge/VIP pair on one L2/AZ) and
  `deploy/v3/reference/single-az.json` (`single-az`, 11 hosts, four racks in one
  AZ).
- `deploy/v3/reference/sizing.json` with the seven roles, platform, network, and
  unsupported list.
- `deploy/v3/deploy.lab.json` addresses, VIP, public addresses, one `lab`
  failure domain, three managers, and documented preconditions.
- 14 real infrastructure pins with index and linux/amd64 digests; exact
  blockers for the app, `minio`, and `minio_client`.
- The expanded fixture denylist in `validate/v3.rs` and its release-image tests.
- A render/validate/check test in `crates/quantrule-deploy/tests/reference.rs`.
- Regenerated render goldens.
- Docs, index updates, README, changelog, and phase-1 verification.

Excluded:

- A new `availability_zone` field or a v4 descriptor.
- A managed-stateful profile or schema; only the boundary ADR and deferred
  section.
- A public-cloud managed service, cloud load balancer, or public-cloud claim.
- A live lab run, a real certificate/key/secret, a registry push, a signature,
  a tag, a deployment, or a board change.
- Phase 2: the lab provisioning harness, remote CLI/browser TLS tests, and the
  final gate rerun.
- A commit, push, PR, merge, or release in this phase.

## 4. Assumptions and uncertainty

- The 14 chosen tags are compatible with the renderer: Traefik 3.x, PostgreSQL
  17, Patroni 4.x, etcd 3.6, HAProxy 3.x, RabbitMQ 4.x, Prometheus 3.x, Grafana
  13.x, Loki/promtail 3.x, Alertmanager 0.35, node_exporter 1.13, blackbox 0.29,
  and osixia/keepalived 2.4.3 (Alpine, BusyBox `nc`).
- `batonogov/patroni-docker` is the maintained public Patroni + PostgreSQL image.
  The `patroni/patroni` repository is not publicly resolvable and has no
  official image; the change is recorded in the registry contract.
- A multi-arch index digest is the descriptor pin; the linux/amd64 manifest
  digest is review metadata. A host pulls the index and resolves its platform.
- The edge pair can share one L2/AZ with two distinct rack domains. The
  descriptor has no AZ field, so the reference documents the ingress
  single-AZ dependency rather than hiding it.
- The single-AZ file is literally one AZ; four racks do not survive an AZ loss.
- The lab remains a template. `/etc/hosts`, a private CA, an operator
  certificate, and firewall rules are preconditions; no fake certificate is
  added.
- `qr-deploy validate --release` is an offline denylist. It cannot prove a
  fabricated digest exists; the registry, signature, and CVE gate do that.

## 5. Decisions

| Decision | Choice | Status |
|---|---|---|
| 3-AZ reference | `reference/three-az.json`, `swarm-ha`, 11 hosts, four rack domains across three AZs, edge/VIP pair on one L2/AZ | Approved |
| Single-AZ reference | `reference/single-az.json`, `single-az`, 11 hosts, four racks in one AZ; no AZ claim | Approved |
| Failure-domain field | Keep `failure_domain`; no `availability_zone` and no v4 schema | Approved |
| Managed stateful | Operator-managed external endpoints on owned hosts; profile/schema deferred; public cloud needs a new ADR | Approved |
| Image pins | Resolve 14 from registry metadata; keep app, `minio`, and `minio_client` as exact blockers | Approved |
| Lab | `10.10.0.11..13`, VIP `10.10.0.20`, public `10.10.0.11/12`, one `lab` domain, three managers; template only | Approved |
| Sizing | `reference/sizing.json` plus a ≤330-line contract; a Rust test validates roles and min <= recommended | Approved |
| Evidence | No live lab; remote/TLS rows `Unverified`; `Simulated HA` at most; `Real HA` is HA-014 | Approved |

## 6. Acceptance criteria

- The 3-AZ descriptor validates as `swarm-ha` with 11 hosts and four rack
  domains across `az1`..`az3`; the edge pair shares one AZ and two racks; the
  three managers and the PostgreSQL/etcd/RabbitMQ members span three AZs; MinIO
  uses four racks, one drive each, EC:2.
- The single-AZ descriptor validates as `single-az` with 11 hosts and four
  `rack-*` domains in one AZ; it makes no AZ-failure claim.
- `sizing.json` has `edge`, `core`, `compute`, `infra`, `observability`, `lab`,
  and `client`; every min <= recommended; every reference host role is covered;
  the contract table matches the JSON values.
- The lab template names the real addresses, VIP, public addresses, one `lab`
  domain, and three managers; the preconditions are documented and no fake cert
  is added.
- 14 infrastructure pins are real registry digests with tag/index/amd64 recorded;
  the app, `minio`, and `minio_client` remain exact blockers.
- Release mode rejects the app fixture, both MinIO blockers, and every retired
  infrastructure fixture at the exact field path; the default sample/lab/reference
  validate stays green.
- Both reference descriptors render `--target all` and check with a recorded
  probe; the generated units parse with `docker compose config -q`.
- Render goldens are regenerated, not hand-edited.
- Docs, ADR, plan, AI log, registry, changelog, and indexes agree; every doc cap
  is kept.
- No live pull, start, lab, secret, certificate, key, commit, push, PR, merge,
  tag, deployment, or board change in phase 1.

## 7. Work breakdown

| Step | Work | Role | Paths |
| ---: | --- | --- | --- |
| 1 | Plan, AI log, ADR, contract | Orchestrator / Architect | `docs/` |
| 2 | Two reference descriptors and the lab addresses | Implementation | `deploy/v3/reference/`, `deploy/v3/deploy.lab.json` |
| 3 | Sizing JSON and the reference test | Implementation / Test | `deploy/v3/reference/sizing.json`, `crates/quantrule-deploy/tests/reference.rs` |
| 4 | Registry resolution of 14 pins | Data/Release Engineering | `deploy/v3/*.json`, `docs/config/2026-11-24/registry.md` |
| 5 | Fixture denylist and release tests | Implementation / Test | `crates/quantrule-deploy/src/validate/v3.rs`, `tests/release_image.rs` |
| 6 | Render goldens and staged compose parse | Test | `crates/quantrule-deploy/tests/fixtures/` |
| 7 | Docs, indexes, README, changelog | Documentation | `docs/`, `deploy/v3/README.md`, `CHANGELOG.md` |
| 8 | Phase-1 verification and full gate | Verification | repository |
| 9 | Phase 2: lab provisioning harness, remote CLI/browser TLS tests, final gate | Implementation / Test | `tools/`, `deploy/v3/`, `apps/web/` |

## 8. Verification

```bash
cargo fmt --all -- --check
cargo clippy -p quantrule-deploy --all-targets -- -D warnings
cargo test -p quantrule-deploy
qr-governance check --all
make compose-check
qr-deploy validate deploy/v3/deploy.example.json
qr-deploy validate deploy/v3/deploy.lab.json
qr-deploy validate deploy/v3/reference/three-az.json
qr-deploy validate deploy/v3/reference/single-az.json
qr-deploy validate deploy/v3/deploy.example.json --release        # expected failure
qr-deploy render <staged descriptor> --out <tmp> --target all
docker compose -f <tmp>/infra/<host>.yaml config -q
tools/release.sh --plan
bash -n tools/release.sh
tools/test_tiers.sh ha --plan
tools/gate.sh
```

A staged descriptor means a copy whose `secrets.files`, `endpoints.*.tls_ca`,
and `ingress.files` point at fixture files in a mode-700 temporary directory.
The fixture values are not deployable credentials. No live pull or start is part
of this verification.

## Verification table

| ID | Plan item | Acceptance criterion | Evidence / command | Expected | Observed | Status | Reviewer | Date | Notes |
|---|---|---|---|---|---|---|---|---|---|
| V01 | Records | Plan, AI log, ADR, and contract exist before code | `ls docs/execution-plans/2026-11-24/1121-reference-lab.md docs/ai-transparency-log/2026-11-24/1121-reference-lab.md docs/adr/2026-11-24/1121-reference-topologies.md docs/config/2026-11-24/reference-topologies.md` | Four files | Four files present | Pass | agent | 2026-11-24 |  |
| V02 | 3-AZ descriptor | 11 hosts, four rack domains, three AZs, edge pair one L2/AZ | `qr-deploy validate deploy/v3/reference/three-az.json`; `cargo test -p quantrule-deploy --test reference three_az` | `ok: v3 swarm-ha, 11 hosts, 7 services` | `ok: v3 swarm-ha, 11 hosts, 7 services`; shape test passes | Pass | agent | 2026-11-24 |  |
| V03 | Single-AZ descriptor | 11 hosts, four racks, one AZ, no AZ claim | `qr-deploy validate deploy/v3/reference/single-az.json`; `cargo test -p quantrule-deploy --test reference single_az` | `ok: v3 single-az, 11 hosts, 7 services` | `ok: v3 single-az, 11 hosts, 7 services`; four `rack-*` domains | Pass | agent | 2026-11-24 |  |
| V04 | Sizing | Seven roles, min <= recommended, host roles covered, doc table matches | `cargo test -p quantrule-deploy --test reference` | 7 tests pass | 7 tests pass | Pass | agent | 2026-11-24 |  |
| V05 | Lab | Addresses, VIP, public addresses, one `lab` domain, three managers | `jq` over `deploy/v3/deploy.lab.json`; `qr-deploy validate deploy/v3/deploy.lab.json` | Values and `ok` | `10.10.0.11..13`, VIP `10.10.0.20`, public `10.10.0.11/12`, one `lab` domain, three managers; `ok: v3 lab-single-host, 3 hosts, 7 services` | Pass | agent | 2026-11-24 |  |
| V06 | Pins | 14 real pins; three blockers exact | `skopeo inspect --raw`; `jq` over the descriptors | 14 match; three fixture digests | 14/14 index bodies hash to the recorded digest and contain linux/amd64; app, `minio`, and `minio_client` keep their exact fixtures | Pass | agent | 2026-11-24 | No layer pull |
| V07 | Release denylist | App, MinIO, MinIO client, and every old pin rejected at the field | `cargo test -p quantrule-deploy --test release_image`; CLI on a temp descriptor | 10 tests pass; named fields | 10 tests pass; CLI prints `images.minio.digest` then `images.minio_client.digest` | Pass | agent | 2026-11-24 |  |
| V08 | Render/check | Both references render and check; units parse | `cargo test -p quantrule-deploy --test reference`; `docker compose config -q` | Pass | 2 render/check cases pass; staged sample 11, lab 3, 3-AZ 11, single-AZ 11 infra units plus `stack.yaml` parse | Pass | agent | 2026-11-24 | No Docker start |
| V09 | Goldens | Goldens regenerated deliberately; only image/VIP lines changed | `cargo test -p quantrule-deploy --test render_golden`; `git diff` | 2 golden tests pass; pin/VIP only | 2 pass (1 ignored); diff is image pins plus the lab VIP/public addresses | Pass | agent | 2026-11-24 |  |
| V10 | Docs | Caps and index agreement | `qr-governance check --all`; `wc -l` | Zero findings; caps | `governance check ok`; ADR 101, reference 259, registry 329, manual 15 200, deploy-v3 330, architecture 170/170/169/170 | Pass | agent | 2026-11-24 |  |
| V11 | Quality | fmt, clippy, tests, governance, compose, plans, gate | verification block | Pass | fmt clean; clippy clean; 265 unit + 22 CLI + 6 entrypoint + 23 HA + 2 MinIO CLI + 10 MinIO harness + 8 observability + 4 queue CLI + 13 definitions + 7 reference + 10 release-image + 15 release-tool + 12 remote + 2 golden + 14 stateful + 10 bootstrap + 10 wrapper; governance ok; compose ok; plans and `bash -n` clean; `tools/gate.sh` passed | Pass | agent | 2026-11-24 |  |
| V12 | Evidence | No live lab, pull, or start | n/a | `Unverified` | No live lab, pull, start, certificate, or connection was run | Blocked | agent | 2026-11-24 | `Real HA` is HA-014 |
| P01 | Phase 2 | Lab provisioning harness | `deploy/v3/lab.sh --plan`; `cargo test -p quantrule-deploy --test ha_harness` | Ordered, fail-closed harness | `--plan` lists stateful -> swarm -> observability -> check -> failure -> remote -> browser; 30 tests pass; preflight stops at `image.digest` with `Unverified` and contacts no host | Pass | agent | 2026-11-24 | Live provisioning blocked by fixtures |
| P02 | Phase 2 | Remote CLI test | `cargo test -p quantrule-shell --test remote` | Env precedence, no flags, CA fail-closed | 8 tests pass: URL/token env, `--token`, `--profile`, trailing slash, missing/malformed CA, no token/CA body, `--remote`/`--cacert`/`--url` rejected | Pass | agent | 2026-11-24 | No live gateway |
| P03 | Phase 2 | Remote browser suite | `bun test apps/web/e2e-remote/config.test.ts`; `bun run test:remote`; `bun run check`; `bun test`; `bun run e2e` | Lab-gated config and Unverified off-lab | 4 config tests pass; `bun run check` 0 errors; `bun test` 186 pass; local Playwright 64 pass; `bun run test:remote` prints `Unverified` and exits 1 without the lab | Pass | agent | 2026-11-24 | Live browser `Unverified` |
| P04 | Phase 2 | Live lab, VIP, TLS, browser | `deploy/v3/lab.sh --confirm`; `--remote-client --confirm`; `bun run test:remote` | Measured lab rows | Not run; the app/`minio`/`minio_client` fixtures block provisioning and no real certificate/token/host exists | Blocked | agent | 2026-11-24 | `Simulated HA` at most; `Real HA` is HA-014 |
| P05 | Phase 2 | Final gate rerun | `tools/gate.sh` | Pass | `gate passed: full` | Pass | agent | 2026-11-24 |  |

## 9. Risks and recovery

| Risk | Control |
|---|---|
| A guessed pin ships | Only registry-resolved digests are written; the three blockers stay exact and release mode fails closed |
| A reverted pin passes | The denylist carries every retired fixture and a test names the field |
| The 3-AZ design overclaims | The edge single-AZ dependency is explicit; `Real HA` stays HA-014 |
| The single-AZ design looks zone-redundant | The profile is `single-az`, the domains are `rack-*`, and the contract says four racks in one AZ |
| The lab template looks live | It stays a template; `/etc/hosts`, CA, certificate, and firewall are preconditions; no live run is claimed |
| A doc cap is exceeded | The capped docs replace lines or link out; `doc-length` runs in governance |
| A marker changes the failure domain count | Anti-affinity is tested on the final assignment; no AZ field is added |
| The render goldens are hand-edited | The ignored regeneration test rewrites them, and the diff is reviewed for pin-only changes |

Recovery: revert the descriptor, pin, or doc to the previous reviewed value. A
failed pin is a new reviewed change, never a guessed digest. A failed render is a
validation defect, not a reason to weaken placement.

## 10. Stop conditions

Stop if a pin cannot be resolved without a guessed digest; if an anti-affinity
rule rejects the approved topology and the only fix is a new schema field; if a
real secret, certificate, key, registry push, or lab host would be needed; if a
doc cap would be exceeded; if `--release` would accept a fixture; or if the full
gate fails. An unobserved live result is `Unverified`, never a pass.

## 11. Phase 1 result

Phase 1 implements the reference descriptors, sizing, lab addresses, 14 pins,
the fixture denylist, the reference test, regenerated goldens, and the docs. The
3-AZ file validates as `swarm-ha` with 11 hosts, four rack domains over three
AZs, the edge pair on one L2/AZ, three managers across three AZs, stateful
members across three AZs, and MinIO on four racks at EC:2. The single-AZ file
validates as `single-az` with the same 11 host names and four `rack-*` domains.
The lab validates as `lab-single-host` with `10.10.0.11..13`, VIP `10.10.0.20`,
public `10.10.0.11/12`, one `lab` domain, and three managers.

The 14 infrastructure pins were resolved from registry metadata; the index body
hashes and the linux/amd64 membership were checked for all 14. `minio`,
`minio_client`, and the app remain exact fixture blockers. Release mode names
`image.digest`, `images.minio.digest`, `images.minio_client.digest`, or
`images.<key>.digest` for a retired pin. The default sample/lab/reference
validate stays green.

All local checks pass: 7 reference tests, 10 release-image tests, 2 render-golden
tests, 265 deploy unit tests, every deploy integration suite, `cargo fmt`,
clippy, `qr-governance check --all`, `make compose-check`, the four validates,
the four staged `render --target all` outputs, `docker compose config -q` on
every unit, `tools/release.sh --plan`, `bash -n`, `tools/test_tiers.sh ha --plan`,
and `tools/gate.sh`. The live lab, remote CLI/browser TLS, real certificate, VIP
failover, and `Real HA` remain `Unverified`; `Simulated HA` is the most a lab run
can claim. Phase 2 owns the lab provisioning harness, the remote TLS tests, and
the final gate rerun.

## 12. Phase 2 handoff

- Add the lab provisioning harness that uses the updated `deploy.lab.json`
  addresses, `/etc/hosts`, the private CA, and the firewall preconditions.
- Add the remote CLI and browser TLS tests against the lab (`QUANTRULE_CA_CERT`,
  browser OS trust).
- Rerun the full gate, update the verification table, and record the lab result
  as `Simulated HA` at most; `Real HA` stays HA-014.

## 13. Phase 2 result

`deploy/v3/lab.sh` is the lab harness, with `deploy/v3/lab/{evidence,remote}.sh`
for the value-free evidence and the client phase. It selects `three-az`,
`single-az`, or `lab`; `--plan` reads no Docker, SSH, jq, qr-deploy, or secret
file; `--confirm` gates the failure phase; and every `--skip-*` leaves its row
`Unverified`. The release preflight stops at `deploy.image.fixture` and prints the
exact field (`image.digest`, then `images.minio.digest`, then
`images.minio_client.digest`) with no host contacted; there is no bypass. The
provisioning order is stateful `network`/`copy`/`start`/`wait`/`queue-bootstrap`/
`object-bootstrap`, `tools/swarm.sh --allow-unverified bootstrap`, observability
`copy`/`start`/`wait`/`verify`, and `qr-deploy check --remote docker --overlay
--runtime`. Only the lab descriptor calls `deploy/v3/test.sh --confirm
--skip-deploy`, which remains the failure harness.

The remote CLI path is tested without a live gateway in
`crates/quantrule-shell/tests/remote.rs`: `QUANTRULE_API_URL`, `QUANTRULE_TOKEN`,
`--token`, `--profile`, a trailing slash, missing/malformed CA fail-closed, no
token or CA body in output, and rejection of the non-existent `--remote`,
`--cacert`, and `--url` flags. The lab-gated browser suite is
`apps/web/playwright.remote.config.ts` plus `e2e-remote/`: base URL
`https://quantrule.dev`, no route mocks, no certificate bypass, token from
`QR_REMOTE_TOKEN`, assertions for load, token entry, connected state, `/version`
image digest, and profiles/datasets, and a wrong-token negative. Without
`QR_REMOTE_BROWSER=1`, `QR_REMOTE_URL`, and a token, the gate prints
`Unverified`.

`tools/test_tiers.sh ha` routes to the harness, so `ha --plan` lists the
reference, failure, remote-client, and browser phases. `bun install
--frozen-lockfile`, `bun run check`, `bun test` (186), `bun run e2e` (64), and
the remote config test pass locally. The live lab, TLS connection, and browser
run were not exercised: the three fixtures block provisioning, the remote suite
prints `Unverified` without the lab, and no real certificate, key, token, host,
or registry was used. Those rows are `Unverified`; `Simulated HA` is the maximum
and `Real HA` is HA-014.
