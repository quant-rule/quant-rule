# PLAT-002 — Traefik ingress, quantrule.dev DNS and TLS (phases 1–2)

- Date: 2026-11-23 (UTC)
- Status: In Review (phase 1 and phase 2 implemented; review gate requested; no pull request is open yet)
- Issue: [#1115](https://github.com/quant-rule/quant-rule/issues/1115)
- Claim type: Behaviour Claim
- AI log: [`docs/ai-transparency-log/2026-11-23/1115-traefik-ingress.md`](../../ai-transparency-log/2026-11-23/1115-traefik-ingress.md)
- Workflow: [Execution Plan](../../workflows/execution-plan.md), [Release & Operations](../../workflows/release-operations.md)
- Phase: 1–2

## 1. Intent

Make the one public endpoint real. `https://quantrule.dev` resolves to one VIP
held by keepalived on the two edge hosts, Traefik terminates TLS at 443 only,
and the gateway is reached through `<stack>_gateway:8080` on the encrypted
overlay. The v3 descriptor gains the VIP and the interface, the renderer writes
the static and dynamic Traefik configuration plus the per-edge keepalived
configuration, and `bootstrap.sh` creates both Traefik configs with the existing
`quantrule.source_sha256` drift label.

Phase 2 adds the ingress check rows, shell/browser private-CA trust, and the
ingress-loss lab harness. A three-VM lab is `Simulated HA`; `Real HA` remains
HA-014 only. If no lab is available, the ingress-loss evidence is `Unverified`
and the issue cannot close.

## 2. Current state

- `quantrule.deploy/v3` validates, renders, and checks the 11-host stack; the
  ingress is one host-mode 443 service with two replicas on the edge pair.
- There is no `ingress.vip`, no `ingress.interface`, no DNS/TLS implementation,
  and no keepalived unit.
- `bootstrap.sh` checks that `quantrule_traefik` exists; it does not create it.
- The ingress service mounts `quantrule_traefik` at `/etc/traefik/traefik.yml`
  and, in cert mode, the certificate and key at the old paths.
- The descriptor requires a unique `public_address` per ingress host. The
  gateway token is a file secret; `/health`, `/ready`, and `/version` are open.
- `render/swarm.rs` is 777 lines; `check/remote.rs` stays 866 and the phase-2
  ingress rows live in `check/ingress.rs` (checked: no growth of the remote
  inspector). `deploy-v3.md` is 330/330.

## 3. Scope

Included (phase 1):

- `ingress.vip` (required IPv4) and `ingress.interface` (default `eth0`), with
  stable `deploy.ingress.vip` and `deploy.ingress.interface` codes.
- The checked-in `deploy/v3/traefik.yml` static configuration: `websecure` on
  443 only, file provider, dashboard off, `api.insecure: false`, default
  certificate, TLS 1.2 minimum, JSON access log with `Authorization` dropped,
  no metrics, 900 s responding timeouts.
- Rendered `out/traefik-dynamic.yml` with the `Host(quantrule.dev)` router on
  `websecure`, `tls: {}`, the stack-aware gateway URL, HSTS one year with
  `includeSubDomains` and no preload, `X-Content-Type-Options: nosniff`,
  `Referrer-Policy: no-referrer`, `Server` stripped, and a 512 MiB body limit.
- The static config copied to `out/quantrule_traefik.yml`; both files enter
  `render.json` and the ingress mounts.
- Cert-mode mounts: `quantrule_traefik_dynamic` at
  `/etc/traefik/dynamic/dynamic.yml`, `quantrule_ingress_cert` at
  `/etc/traefik/certs/ingress.crt`, `quantrule_ingress_key` at
  `/etc/traefik/certs/ingress.key`. No writable ACME state.
- Rendered per-edge `out/keepalived-<host>.conf` and a keepalived unit on each
  edge host with `network_mode: host`, `NET_ADMIN`/`NET_BROADCAST`, and no ports
  or deploy block. `images.keepalived` is required.
- `bootstrap.sh` creates `quantrule_traefik` and `quantrule_traefik_dynamic`
  from the rendered files before the stack, with drift refusal.
- Cert/key fixture, validator/render/bootstrap/golden tests, contracts, manual,
  architecture pointers, and changelog.

Included (phase 2):

- `check/ingress.rs`: static/dynamic/keepalived/certificate rows, the extended
  probe with response headers, the plain-HTTP port-80 row, the remote ingress
  mounts row, and the `limited` VIP/firewall row. A `CertificateQuery` seam runs
  `openssl x509 -checkend 2592000 -ext subjectAltName` in production; a missing
  `openssl` is a failed `limited` row, never a silent pass.
- Private-CA client trust: `QUANTRULE_CA_CERT` for the shell, the browser OS
  trust-store install, `--cacert`/`QUANTRULE_CA_CERT`/`CURL_CA_BUNDLE` in
  `tools/verify.sh`, and `QR_SWARM_CACERT` plus `--cacert` in the lab harness.
- The VIP-holder drain phase in `deploy/v3/test.sh`: baseline, a bounded 1 Hz
  HTTPS `/health` loop during the move, digest/stateful re-checks, the
  port-80/direct-edge/VIP/app-port negative controls, and the restore.
- Tests, contracts, manual, architecture pointers, changelog, plan, and AI log.

Excluded:

- ACME issuance and renewal. Cert mode is the v1.5.10 path; the `acme` enum
  variant is reserved for a future path and is refused by validation until it
  lands. There is no per-replica `acme.json`.
- A new command, a new external dependency, or a change to the 61-command
  surface.
- `Real HA`: HA-014 only. A three-VM lab is `Simulated HA`; without a lab the
  ingress-loss row is `Unverified`.
- HAProxy, Patroni, RabbitMQ, MinIO, and observability configuration
  (PLAT-003..006), and reference topologies or a remote client (PLAT-008).

## 4. Assumptions and uncertainty

- The two edge hosts share a public L2/L3 segment or an operator network where
  VRRP unicast and a floating IPv4 address are valid. The VIP is in the same
  subnet as the ingress `public_address` values.
- The pinned keepalived image provides `keepalived` and a BusyBox-style
  `nc -z 127.0.0.1 443`; the image pin is part of the descriptor and the
  requirement is documented.
- The private CA that signs `quantrule.dev` is the same CA named by
  `endpoints.database.tls_ca`; v3 has no separate ingress-CA field.
- Traefik runs as root in its pinned image, so a mode-0400 Swarm secret key is
  readable.
- The 512 MiB request-body limit matches `QR_UPLOAD_LIMIT_BYTES`; the gateway
  keeps its own limit.
- DNS is operator-managed: one A record, TTL 60, pointing at the VIP. There is
  no DNS provider integration in this phase.

## 5. Decisions

| Decision | Choice | Status |
|---|---|---|
| VIP | keepalived VRRP on the two edge hosts; 443 health check | Approved |
| DNS | one A record, TTL 60, no port 80 | Approved |
| TLS | private-CA `cert` mode for v1.5.10; `acme` reserved and refused; no `acme.json` | Approved |
| Static config | checked-in `deploy/v3/traefik.yml`, stack-name agnostic | Approved |
| Dynamic config | renderer emits `traefik-dynamic.yml` with the stack name | Approved |
| Config ownership | bootstrap creates both configs with drift labels | Approved |
| Cert storage | `out/pki/ingress.*`; rotation by new object names | Approved |
| Keepalived | host network, caps, `nc -z` check, 101/100 priorities | Approved |
| Headers | HSTS 1 year `includeSubDomains`, no preload; nosniff; no-referrer | Approved |
| Limits | 900 s responding timeouts, 512 MiB body, TLS 1.2+ | Approved |
| Exposure | dashboard off, `api.insecure: false`, no metrics, no ingress auth | Approved |
| Evidence | three-VM lab is `Simulated HA`; `Real HA` is HA-014; no lab is `Unverified` | Approved |

## 6. Acceptance criteria

- The sample validates with `ingress.vip` and `ingress.interface`; a missing or
  invalid VIP, a VIP equal to an ingress `public_address`, and an invalid
  interface are refused with the stable codes. An `acme` descriptor is refused
  with `deploy.ingress.tls`; the enum remains reserved for the future path.
- The checked-in static config has one `websecure` 443 entrypoint and no port 80,
  dashboard or insecure API, public metrics, or ACME state; the dynamic config
  names the DNS host, the stack gateway, the security headers, and the body
  limit.
- `stack.yaml` mounts the static, dynamic, certificate, and key objects at the
  documented paths, keeps exactly one host-mode 443 port, and publishes no app
  port.
- Each edge host has a keepalived unit with host networking, the two caps, a
  read-only config mount, no ports, and no deploy block; each rendered
  keepalived file has the descriptor interface, VIP/32, the other ingress
  `public_address` as a unicast peer, 101/100 priorities, and a 443 check.
- `bootstrap.sh` creates both Traefik configs with the drift label before the
  stack, stops on a changed file, and still creates the cert/key in cert mode.
- The v3 golden fixture covers the stack, both edge infra units, both keepalived
  files, the static and dynamic files, and the cert-mode example; v2 output is
  byte-identical.
- Phase 2: `check/ingress.rs` proves the static/dynamic/keepalived/certificate
  rows, the header and port-80 rules, and the four remote mounts with fake
  seams; a missing `openssl` is a failed `limited` row.
- Phase 2: the shell trusts `QUANTRULE_CA_CERT` without a new flag, dependency,
  or command-contract change; `tools/verify.sh` and the lab harness pass the CA
  to every curl; the browser install step is documented.
- Phase 2: `deploy/v3/test.sh` drains the VIP holder, probes 1 Hz, runs the
  negative controls, restores, and prints `Simulated HA` or `UNVERIFIED`; it
  contains no destructive command.
- Docs, plan, AI log, and changelog agree; every doc stays within its cap.

## 7. Work breakdown

| Step | Work | Agent role | Paths |
| ---: | --- | --- | --- |
| 1 | Plan, AI log, phase boundary | Orchestrator / Architect | `docs/` |
| 2 | VIP/interface schema and validation | Implementation | `descriptor/v3.rs`, `validate/network.rs`, `validate/v3/tests.rs` |
| 3 | Example, fixtures, image pin | Implementation | `deploy/v3/deploy.example.json`, `deploy/v3/pki/` |
| 4 | Static config, dynamic render, keepalived render | Implementation | `deploy/v3/traefik.yml`, `render/ingress.rs` |
| 5 | Stack mounts and edge units | Implementation | `render/swarm.rs`, `render/infra.rs` |
| 6 | Bootstrap config creation and drift | Implementation | `render/bootstrap.rs` |
| 7 | Golden, CLI, bootstrap, infra tests | Test | `src/**/tests`, `tests/` |
| 8 | Contracts, manual, architecture, README, changelog | Documentation | `docs/`, `README.md`, `deploy/v3/README.md`, `CHANGELOG.md` |
| 9 | Phase-2 ingress check rows | Implementation / Test | `check/ingress.rs`, `check/v3.rs` |
| 10 | Phase-2 shell/browser CA trust | Implementation / Test | `crates/quantrule-shell`, `apps/web` |
| 11 | Phase-2 ingress-loss lab harness | Test | `deploy/v3/test.sh`, `tools/test_tiers.sh` |

## 8. Verification

```bash
cargo fmt --all -- --check
cargo clippy -p quantrule-deploy -p quantrule-shell --all-targets -- -D warnings
cargo test -p quantrule-deploy
cargo test -p quantrule-shell
cargo run -q -p quantrule-governance -- check --all
make compose-check
tools/test_tiers.sh ha --plan
./deploy/v3/test.sh
tools/gate.sh
cargo run -q -p quantrule-deploy -- validate deploy/v3/deploy.example.json
cargo run -q -p quantrule-deploy -- render <local-descriptor> --out <dir> --target all
docker compose -f <out>/infra/edge-1.yaml config -q
```

The fast tier stays Docker-free. The route inspection and `docker compose config`
parse are static; they do not start a stack. A live VIP failover is the phase-2
lab row; it is `Unverified` when no three-VM lab is available.

## Verification table

| ID | Plan item | Acceptance criterion | Evidence / command | Expected | Observed | Status | Reviewer | Date | Notes |
|---|---|---|---|---|---|---|---|---|---|
| V01 | Plan | Plan and AI log exist before code | `ls docs/execution-plans/2026-11-23/1115-*.md docs/ai-transparency-log/2026-11-23/1115-*.md` | Both files | both files created before code | Pass | agent | 2026-11-23 |  |
| V02 | Schema | VIP/interface parse; missing VIP fails | `cargo test -p quantrule-deploy the_ingress_vip` | Refused | 133 unit tests pass | Pass | agent | 2026-11-23 |  |
| V03 | Validation | Invalid/colliding VIP and invalid interface use stable codes | `cargo test -p quantrule-deploy ingress_interface` | Codes | both tests pass; empty, IPv6, loopback, multicast, broadcast, collision covered | Pass | agent | 2026-11-23 |  |
| V04 | Sample | Cert-mode sample with VIP validates as 11 hosts | `qr-deploy validate deploy/v3/deploy.example.json` | `ok: v3 swarm-ha, 11 hosts` | `ok: v3 swarm-ha, 11 hosts, 7 services … ingress quantrule.dev:443` | Pass | agent | 2026-11-23 | structural; files checked separately |
| V05 | Static | One 443 entrypoint, no 80, dashboard/insecure off, no metrics, TLS 1.2, Authorization dropped | `cargo test -p quantrule-deploy traefik_static` | Pass | test passed | Pass | agent | 2026-11-23 |  |
| V06 | Dynamic | Host rule, stack gateway, HSTS/nosniff/no-referrer, Server stripped, body limit | `cargo test -p quantrule-deploy traefik_dynamic` | Pass | test passed; `536870912` body cap | Pass | agent | 2026-11-23 |  |
| V07 | Stack | Config/secret mounts; one host-mode 443; no app port | `cargo test -p quantrule-deploy the_stack_has_one_ingress` | Pass | static/dynamic/cert/key mounts; one `published: 443` | Pass | agent | 2026-11-23 |  |
| V08 | Keepalived | Per-edge config and unit; host network/caps; priorities; VIP/32; nc check | `cargo test -p quantrule-deploy keepalived` | Pass | both edge configs and units pass; 101/100 | Pass | agent | 2026-11-23 |  |
| V09 | Bootstrap | Two configs created with drift label before the stack; changed file stops; cert/key still created | `cargo test -p quantrule-deploy --test swarm_bootstrap` | Pass | 9 tests; 6 configs, 13 secrets; changed dynamic file stops | Pass | agent | 2026-11-23 |  |
| V10 | Golden | All new files and cert-mode example match; v2 byte-identical | `cargo test -p quantrule-deploy --test render_golden`; `two_renders_are_byte_identical` | Pass | golden compares static/dynamic/keepalived/edge/stack/pki cert; v2 output unchanged | Pass | agent | 2026-11-23 | key mode asserted at 600; `.key` excluded from golden by the private-key checker |
| V11 | Compose | Rendered edge unit parses with the pinned image | `docker compose -f <out>/infra/edge-1.yaml config -q` | Exit 0 | `edge-1 compose config ok` | Pass | agent | 2026-11-23 | static parse, no stack started |
| V12 | Docs | Contracts and pointers within caps | `qr-governance check --all`; `wc -l` | Zero findings | `governance check ok`; deploy-v3 328, ingress 178, manual 200, architecture 169–170 | Pass | agent | 2026-11-23 |  |
| V13 | Quality | fmt, clippy, targeted tests, compose, full gate | verification block | Pass | fmt clean; clippy clean; 134 unit + 18 CLI + 6 entrypoint + 7 ha + 6 remote_cli + 1 golden + 9 bootstrap + 8 wrapper; `compose config ok`; `gate passed: full` | Pass | agent | 2026-11-23 |  |
| V14 | Render | Inspect the rendered files and `render.json` | `qr-deploy render … --target all`; `qr-deploy check` with fixture | New files listed | new files rendered and listed; fixture check all `ok` | Pass | agent | 2026-11-23 |  |
| V15 | Correction | `acme` is reserved and refused; cert only in v1.5.10 | `cargo test -p quantrule-deploy acme_is_reserved`; `qr-deploy validate` on an `acme` copy | `deploy.ingress.tls` | test passed; validate exits 1 with `deploy.ingress.tls … v1.5.10 requires tls: cert; acme is reserved`; docs updated | Pass | agent | 2026-11-23 | Phase-1 review finding |
| P01 | Phase 2 | Local/remote/probe/certificate check rows | `cargo test -p quantrule-deploy`; `cargo test -p quantrule-deploy --test remote_cli` | Rows and fake seams | 149 unit + 19 CLI + 7 remote tests pass; static/dynamic/keepalived/certificate/header/port-80/mount rows covered; `check/ingress.rs` 759, `check/ingress/tests.rs` 488, `check/remote.rs` unchanged at 866 | Pass | agent | 2026-11-23 |  |
| P02 | Phase 2 | Shell and browser trust the private CA | `cargo test -p quantrule-shell`; `bash tools/test_verify.sh` | Client tests and CA preflight | 55 shell unit tests pass including missing/malformed CA; `--cacert` reaches fake curl; browser OS-store install documented, no SPA change; `QUANTRULE_CA_CERT` added without a flag or dependency | Pass | agent | 2026-11-23 |  |
| P03 | Phase 2 | Ingress-loss lab: VIP moves, 443 survives | `tools/test_tiers.sh ha --plan`; `deploy/v3/test.sh` | `Simulated HA` or `UNVERIFIED` | plan exits 0 and contacts no Docker; real harness exits 1 with `UNVERIFIED: Swarm is inactive; run this on an active Swarm manager`; fake tests cover the missing-VIP refusal and destructive-command absence | Blocked | agent | 2026-11-23 | No three-VM lab available; `Unverified` evidence, not a pass; criterion not claimed |
| P04 | Correction | Direct-edge control tests the public address, not the private SSH/Swarm address | `cargo test -p quantrule-deploy --test ha_harness`; `bash -n deploy/v3/test.sh` | `public_address` used; missing value is `UNVERIFIED` | `negative_controls` reads `hosts.<host>.public_address`; a regression test asserts it and refuses a missing value; plan/AI log/docs updated | Pass | agent | 2026-11-23 | Phase-2 review finding |

## 9. Risks and recovery

| Risk | Control |
|---|---|
| VIP and `public_address` collide | Validator refuses equality; sample uses distinct values |
| VRRP unicast cannot traverse the edge network | Document the same-subnet requirement and allow a recorded topology exception |
| Pinned keepalived image lacks `nc` | Document the image requirement; the check fails closed and the VIP moves only when 443 is unhealthy |
| Traefik starts without the certificate | Cert mode mounts both objects; drift refusal stops a changed cert; ACME state is absent |
| A changed config is silently reused | `quantrule.source_sha256` label; rotation uses a new object name |
| Port 80 or dashboard is exposed | Static config has one 443 entrypoint and no dashboard/metrics; tests assert it |
| Body limit mismatch | 512 MiB at Traefik and the gateway's own upload limit; document both |
| Doc caps exceeded | Move the probe paragraph to `ingress.md`; measure before completion |
| No three-VM lab | Phase 2 records `Unverified`, not `Simulated HA`; the issue cannot close |
| Phase 2 overclaims | The plan keeps phase 1 and phase 2 rows separate |

Recovery: keep the previous descriptor, rendered files, image digest, and cert
objects. A failed ingress update rolls back the Traefik service; keepalived
keeps the last healthy node. A certificate rotation creates new object names,
starts a rolling ingress update, and removes the old objects only after it
converges. DNS returns to the old VIP if the ingress is rolled back.

## 10. Stop conditions

Stop if a schema change alters v2 output; if the VIP rule cannot be enforced
without a new dependency; if the static config needs a writable ACME store; if
the fake-Docker tests cannot prove drift, ordering, or the cert/key path; if the
rendered edge unit does not parse; if a doc cap is exceeded; or if the full gate
fails. A live ingress-loss result is not a Pass: record `Unverified` with the
blocker and do not claim the done criterion.
