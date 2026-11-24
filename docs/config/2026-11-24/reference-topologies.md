# Reference Topologies, Sizing, and Lab Preconditions

- Date: 2026-11-24 (UTC)
- Status: Current; PLAT-008 phases 1–2
- Issue: [#1121](https://github.com/quant-rule/quant-rule/issues/1121)
- Claim type: Behaviour Claim
- Related: [ADR-1121](../../adr/2026-11-24/1121-reference-topologies.md), [deploy/v3](../2026-11-22/deploy-v3.md), [registry](registry.md), [multi-host chapter](../../manual/14-multi-host.md), [HA chapter](../../manual/15-ha-deployment.md), [remote compute](../../tutorials/2026-11-12/03-remote-compute.md)

This contract defines the two reference descriptors, the machine-readable sizing
table, the lab template, and the evidence boundary. The descriptors are designs
that a local render and check exercise; no live lab is claimed here.

## 1. Topologies and labels

| Topology | Descriptor | Profile | Failure claim |
|---|---|---|---|
| 3-AZ | `deploy/v3/reference/three-az.json` | `swarm-ha` | Four rack domains across three AZs; ingress is a single-AZ dependency |
| Single-AZ | `deploy/v3/reference/single-az.json` | `single-az` | Four racks in one AZ; no AZ failure tolerance |
| Managed-stateful | none in PLAT-008 | deferred | Operator-managed external endpoints; no public-cloud claim |
| Lab | `deploy/v3/deploy.lab.json` | `lab-single-host` | Three VMs on one host; `Simulated HA` only |

Evidence labels are exact:

- **Designed** — the descriptor and the sizing table exist and pass local validation.
- **Validated locally** — `qr-deploy validate`, `render --target all`, and `check --out` pass in the fast tier.
- **Simulated HA** — the opt-in lab tier ran; only `deploy.lab.json` can reach it.
- **Unverified** — no run exists here.
- **Real HA** — HA-014 only; no PLAT-008 output says `Real HA`.

The reference descriptors do not add an `availability_zone` field and do not
create a v4 schema. The existing `failure_domain` label carries the rack or zone.

## 2. 3-AZ reference

Eleven hosts, four rack-level failure domains, three AZs. The edge pair shares
one L2 segment and one AZ, in two racks, because keepalived needs the VIP and the
gratuitous ARP on one segment. If that AZ is lost, the data plane keeps its
quorum but public ingress is lost until the AZ returns. That is a stated
single-AZ edge dependency, not an AZ failure claim.

| Host | Role | Failure domain | Private address | Public address |
|---|---|---|---|---|
| `edge-1` | ingress, gateway, keepalived, HAProxy | `az1-rack-1` | `10.0.1.11` | `198.51.100.10` |
| `edge-2` | ingress, gateway, keepalived, HAProxy | `az1-rack-2` | `10.0.1.12` | `198.51.100.11` |
| `core-1` | data, evidence, simulation | `az2-rack-1` | `10.0.2.21` | — |
| `core-2` | data, evidence, simulation | `az3-rack-1` | `10.0.3.21` | — |
| `compute-1` | model, strategy, backtest | `az1-rack-1` | `10.0.1.31` | — |
| `compute-2` | model, strategy, backtest | `az2-rack-1` | `10.0.2.31` | — |
| `compute-3` | model, strategy, backtest | `az3-rack-1` | `10.0.3.31` | — |
| `infra-1` | PostgreSQL, etcd, RabbitMQ, MinIO, Prometheus | `az1-rack-1` | `10.0.1.41` | — |
| `infra-2` | MinIO, Grafana | `az1-rack-2` | `10.0.1.42` | — |
| `infra-3` | PostgreSQL, etcd, RabbitMQ, MinIO, Loki | `az2-rack-1` | `10.0.2.41` | — |
| `infra-4` | PostgreSQL, etcd, RabbitMQ, MinIO, Alertmanager | `az3-rack-1` | `10.0.3.41` | — |

`198.51.100.20/24` is the VIP on the edge hosts' public `eth0`; the public and
VIP addresses share that /24. The private addresses are the overlay/LAN side.
`quantrule.dev` is a placeholder DNS name; the operator's A record TTL 60 points
at the VIP.

Placement facts:

- Swarm managers: `edge-1` (az1), `core-1` (az2), `compute-3` (az3); three odd.
- PostgreSQL, etcd, and RabbitMQ: `infra-1` (az1), `infra-3` (az2), `infra-4` (az3).
- HAProxy and keepalived: `edge-1`, `edge-2`; the same L2/AZ, distinct racks.
- MinIO: all four infra hosts, one drive each, across four racks, EC:2.
- Observability: Prometheus `infra-1`, Grafana `infra-2`, Loki `infra-3`,
  Alertmanager `infra-4`; Prometheus and Alertmanager never share a host.

A single non-edge AZ loss keeps data, queue, object, and compute quorum. A rack
loss inside AZ1 removes one of the two ingress hosts and one MinIO drive; if the
rack also hosts `compute-1` or `infra-1`, one stateless replica or one
PostgreSQL/etcd/RabbitMQ member goes with it, and the remaining quorum continues.
The VIP survivor keeps the ingress. A full AZ1 loss removes public ingress;
PostgreSQL, etcd, and RabbitMQ each keep two of three members in AZ2/AZ3, but
clients cannot reach the data plane without a second ingress site.

## 3. Single-AZ reference

Eleven hosts and four racks in **one** AZ. It is an honest “single-AZ, four
racks” topology: one rack loss is survivable at the data tier, an AZ loss is
not. Do not describe it as zone-redundant and do not reuse the `swarm-ha`
profile.

| Host | Role | Failure domain | Private address | Public address |
|---|---|---|---|---|
| `edge-1` | ingress, gateway, keepalived, HAProxy | `rack-1` | `10.20.0.11` | `198.51.100.10` |
| `edge-2` | ingress, gateway, keepalived, HAProxy | `rack-2` | `10.20.0.12` | `198.51.100.11` |
| `core-1` | data, evidence, simulation | `rack-3` | `10.20.0.21` | — |
| `core-2` | data, evidence, simulation | `rack-4` | `10.20.0.22` | — |
| `compute-1` | model, strategy, backtest | `rack-1` | `10.20.0.31` | — |
| `compute-2` | model, strategy, backtest | `rack-2` | `10.20.0.32` | — |
| `compute-3` | model, strategy, backtest | `rack-3` | `10.20.0.33` | — |
| `infra-1` | PostgreSQL, etcd, RabbitMQ, MinIO, Prometheus | `rack-1` | `10.20.0.41` | — |
| `infra-2` | MinIO, Grafana | `rack-2` | `10.20.0.42` | — |
| `infra-3` | PostgreSQL, etcd, RabbitMQ, MinIO, Loki | `rack-3` | `10.20.0.43` | — |
| `infra-4` | PostgreSQL, etcd, RabbitMQ, MinIO, Alertmanager | `rack-4` | `10.20.0.44` | — |

Managers are `edge-1` (rack-1), `core-2` (rack-4), and `compute-3` (rack-3), so a
single rack loss does not remove manager quorum. The VIP, DNS, and client path
are the same as the 3-AZ reference.

## 4. Managed-stateful boundary (deferred)

PLAT-008 does not add a managed-stateful profile or schema. The supported
descriptor still renders and pins the self-hosted PostgreSQL/Patroni, etcd,
RabbitMQ, and MinIO units on the operator's hosts. A future managed profile may
point `endpoints.database`, `endpoints.queue`, and `endpoints.object_store` at
external operator-controlled endpoints, but that profile, its validation, its
sizing, and its evidence labels are a follow-up.

The external endpoints are operator-managed hosts, not a public cloud. A
public-cloud managed database, queue, object store, or load balancer needs a new
ADR and a qualified self-hosted claim; the existing reliability, retention,
restore, and `Real HA` language does not transfer to it. No PLAT-008 descriptor
names a cloud endpoint.

## 5. Sizing and support matrix

`deploy/v3/reference/sizing.json` is the machine-readable source; this table is
checked against it by `crates/quantrule-deploy/tests/reference.rs`. A host may
use the recommended value or higher. The minimum is the point below which the
role is not supported, not a target.

| Role | vCPU min / rec | RAM GiB min / rec | Disk GiB min / rec | Placement |
|---|---|---|---|---|
| `edge` | 2 / 4 | 4 / 8 | 40 / 80 | ingress, gateway, keepalived, HAProxy |
| `core` | 4 / 8 | 8 / 16 | 100 / 200 | data, evidence, simulation |
| `compute` | 8 / 16 | 16 / 32 | 100 / 200 | model, strategy, backtest |
| `infra` | 4 / 8 | 16 / 32 | 200 / 500 | PostgreSQL, etcd, RabbitMQ, MinIO, observability |
| `observability` | 2 / 4 | 4 / 8 | 50 / 100 | dedicated monitoring hosts |
| `lab` | 8 / 16 | 16 / 32 | 150 / 300 | three VMs on one physical host |
| `client` | 2 / 2 | 4 / 4 | 20 / 20 | browser or command shell |

The `infra` disk is local NVMe. Network storage does not satisfy the fsync or
erasure assumptions. The reference uses the four `infra` hosts for the four
observability units, so the dedicated `observability` role is optional until the
deployment separates monitoring.

### Platform and network

| Item | Requirement |
|---|---|
| OS | Ubuntu 24.04 LTS or Debian 13; Linux `x86_64` only |
| Engine | Docker Engine 27.5 or later with Swarm mode |
| Compose | Docker Compose v2.30 or later for the local tier |
| Managers | Three odd managers; one manager loss keeps quorum |
| Overlay | 2377/tcp managers, 7946/tcp+udp all nodes, 4789/udp all nodes; no NAT for VXLAN |
| Edge | VRRP protocol 112 on the same L2 segment; 443/tcp public; 22/tcp restricted |
| Link | 1 GbE minimum, 10 GbE recommended; MTU 1500 |
| Time | NTP on every host; UTC timestamps and logs |
| Stateful ports | 5432/5671/9000 through HAProxy on the overlay only; no public bind |

### Unsupported

- macOS server
- Windows server
- ARM64 server
- a single Swarm manager
- one stateful node
- one MinIO drive
- all MinIO drives on one physical disk
- public 8080/tcp
- public 5432/tcp
- public 9000/tcp
- public 5671/tcp
- public 15672/tcp
- ACME certificate issuance
- a public-cloud load balancer
- a public-cloud managed stateful service without a separate ADR

A broken row is a validation failure, not a tuning opportunity. Do not weaken
the profile or the placement to fit an unsupported host.

## 6. Lab template and preconditions

`deploy/v3/deploy.lab.json` is a template, not a live deployment. It uses three
VMs on one physical host, one failure domain `lab`, and three managers.

| Item | Lab value |
|---|---|
| Hosts | `lab-1` `10.10.0.11`, `lab-2` `10.10.0.12`, `lab-3` `10.10.0.13` |
| Failure domain | `lab` on all three; co-location is the recorded exception |
| Managers | All three VMs; one manager loss keeps quorum |
| VIP | `10.10.0.20/24` on the lab LAN |
| Public addresses | `10.10.0.11` and `10.10.0.12`; the same LAN addresses |
| Ingress DNS | `quantrule.dev` placeholder; replace it or map it in `/etc/hosts` |
| TLS | Operator certificate and key at `/etc/quantrule/pki/ingress.{crt,key}`; cert mode |
| CA | Private CA at `/etc/quantrule/pki/ca.crt`; the clients trust it |
| MinIO | Three VMs, two drives each, EC:3; one VM loss keeps the write quorum |

Before a lab run:

1. Add `10.10.0.20 quantrule.dev` to every client and lab host that resolves the
   name, or point a DNS A record at the VIP. Do not rely on mDNS or a public
   record.
2. Put the private CA and the ingress certificate/key on the edge VMs. The
   repository ships no lab certificate; `deploy/v3/pki/ingress.crt` and
   `ingress-key.pem` are non-serving fixtures and must not be copied to a host.
3. Open 443 and VRRP protocol 112 between `lab-1` and `lab-2`; keep 22 restricted,
   and do not publish 8080, 5432, 9000, 5671, or 15672. VXLAN 4789/udp must not be
   NATed.
4. Trust the private CA in the client OS store for the browser, or set
   `QUANTRULE_CA_CERT` for the shell. The certificate SAN must include the DNS
   name and every internal service name the check expects.
5. Keep the fixture secrets off the host. Generate real values and copy them at
   mode 600; the lab is not evidence until `deploy/v3/lab.sh --confirm` prints
   its measured rows.

### 6.1 Lab provisioning order

`deploy/v3/lab.sh` is the harness; `deploy/v3/test.sh` remains the failure phase.
`--plan` reads no Docker, SSH, jq, or secret file. A real run first executes
`qr-deploy validate <descriptor> --release`; the app, `minio`, and
`minio_client` fixtures stop it at the exact field with `Unverified` before any
host is contacted, and there is no bypass.

| # | Phase | Command |
|---:|---|---|
| 1 | Preflight | `qr-deploy validate <descriptor> --release` |
| 2 | Overlay | `tools/stateful.sh network` |
| 3 | Material | `tools/stateful.sh copy` |
| 4 | Stateful | `tools/stateful.sh start`, `wait`, `queue-bootstrap`, `object-bootstrap` |
| 5 | Control plane | `tools/swarm.sh --allow-unverified bootstrap` (lab only) |
| 6 | Observability | `tools/observability.sh copy`, `start`, `wait`, `verify` |
| 7 | Check | `qr-deploy check <descriptor> --remote docker --overlay --runtime` |
| 8 | Failure | `deploy/v3/test.sh --confirm --skip-deploy` (lab only) |
| 9 | Remote client | `deploy/v3/lab.sh --remote-client --confirm` on the client |
| 10 | Browser | `cd apps/web && QR_REMOTE_BROWSER=1 QR_REMOTE_URL=https://quantrule.dev bun run test:remote` |

`--reference three-az|single-az` validates the selected design and stops: the
reference descriptors use documentation addresses and are not lab hosts. Only
`deploy.lab.json` can reach `Simulated HA`. `--skip-database`, `--skip-queue`,
`--skip-object`, `--skip-observability`, and `--skip-failure` leave that row
`Unverified`, never a pass.

## 7. Image pins and blockers

The sample, lab, and both reference descriptors carry the same 16
infrastructure keys. Fourteen are resolved from registry metadata (tag, index
digest, and linux/amd64 digest) and are listed in the [registry contract](registry.md#11-infrastructure-pins).
The three blockers are exact and deliberate:

| Field | Fixture | Why it is blocked | `--release` result |
|---|---|---|---|
| `image.digest` | `sha256:cdace4…` | Labelled application example; not a published image | `deploy.image.fixture` at `image.digest` |
| `images.minio.digest` | `sha256:bdf0781a…` | Anonymous registry metadata is denied here | `deploy.image.fixture` at `images.minio.digest` |
| `images.minio_client.digest` | `sha256:5c5b1f8f…` | Anonymous registry metadata is denied here | `deploy.image.fixture` at `images.minio_client.digest` |
| any retired infra pin | the 16 old values | A revert must not pass silently | `deploy.image.fixture` at `images.<key>.digest` |

Release mode is a denylist, not a signature, a vulnerability scan, or proof that
a new digest exists. A production descriptor still runs `tools/release.sh pin`,
`cosign verify`, and the CVE gate.

## 8. Lab and remote client path

The shell uses the environment path, not a `--remote`, `--cacert`, or `--url`
flag; `--remote` is a `qr-deploy check` option:

```bash
export QUANTRULE_API_URL='https://quantrule.dev/api/v1'
export QUANTRULE_CA_CERT='/path/to/ca.crt'
export QUANTRULE_TOKEN='<gateway token>'
quantrule health
tools/verify.sh --api-url "$QUANTRULE_API_URL" --cacert "$QUANTRULE_CA_CERT" --profile <id>
openssl s_client -connect quantrule.dev:443 -servername quantrule.dev -CAfile "$QUANTRULE_CA_CERT" -verify_return_error
curl --cacert "$QUANTRULE_CA_CERT" https://quantrule.dev/health
curl --cacert /dev/null https://quantrule.dev/health   # must fail
```

The shell sends the bearer token on every request and trusts only the named CA
while `QUANTRULE_CA_CERT` is set. The browser uses the OS/NSS trust store and the
Connection screen; it never reads a server path. Only 443 is published.

`deploy/v3/lab.sh --remote-client --confirm` runs the CLI phase from the client
outside the Swarm and prints only state/digest/TLS/curl status. The browser suite
is `apps/web/playwright.remote.config.ts` (`bun run test:remote`): base URL
`https://quantrule.dev`, no route mocks, no certificate-error bypass, token from
`QR_REMOTE_TOKEN`, and it asserts load, token entry, connected state,
`/version` image digest, and the profiles/datasets routes. Without
`QR_REMOTE_BROWSER=1`, `QR_REMOTE_URL`, and a token, the gate prints `Unverified`,
not a silent skip.

Evidence is value-free JSON under `QR_LAB_EVIDENCE` (default
`deploy/v3/evidence/lab-<UTC>{-provision,-failure,-remote}.json`): descriptor
hash, image digest, tool versions, phase rows, remote/TLS rows, canonical digest,
and the evidence label. It never carries a token, CA body, key, or inspect body.
The lab's remote path is `Unverified` here: no live host, certificate, token, or
TLS connection was exercised in this work item. Phase 2 implements the harness
and tests; a live lab remains `Unverified`, `Simulated HA` is the maximum, and
`Real HA` is HA-014.

## 9. Verification and evidence

| Row | Command | Evidence label |
|---|---|---|
| Descriptor schema and placement | `qr-deploy validate <descriptor>` | Validated locally |
| Render both targets | `qr-deploy render <descriptor> --out <dir> --target all` | Validated locally |
| Rendered-tree check | `qr-deploy check <descriptor> --out <dir>` | Validated locally |
| Sizing/doc agreement | `cargo test -p quantrule-deploy --test reference` | Validated locally |
| Lab harness and remote tests | `deploy/v3/lab.sh --plan`; `cargo test -p quantrule-deploy --test ha_harness`; `cargo test -p quantrule-shell --test remote`; `bun test apps/web/e2e-remote/config.test.ts` | Validated locally; no host contacted |
| 14 registry pins | `skopeo inspect --raw` plus `--format '{{.Digest}}'` | Measured from registry metadata |
| Live lab, VIP, remote TLS | `deploy/v3/lab.sh --confirm`; `deploy/v3/lab.sh --remote-client --confirm` | Unverified; fixtures block provisioning |
| Physical host/AZ/power failure | HA-014 | `Real HA` only after HA-014 |

A passing local check proves the descriptor, placement, pins, secret file names,
rendered units, harness order, and the recorded probe. It does not prove a live
host, a real certificate, a firewall, a VIP failover, or an AZ failure.
