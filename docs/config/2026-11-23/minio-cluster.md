# MinIO Distributed Erasure and Object Bootstrap

- Date: 2026-11-23 (UTC)
- Status: Current for the v3 MinIO render, TLS, HAProxy front door, boot order,
  bucket/versioning/policy/account bootstrap, the Docker-free check rows, and the
  node-loss lab. The lab evidence label is `Simulated HA`; no live run is claimed
  here, `Real HA` is HA-014, and the end-to-end digest is `Measured` only after
  `tools/verify.sh` succeeds
- Issue: [#1118](https://github.com/quant-rule/quant-rule/issues/1118)
- Claim type: Behaviour Claim
- Related: [ADR-HA](../../adr/2026-11-22/1082-ha-deployment-profile.md),
  [deploy/v3](../2026-11-22/deploy-v3.md),
  [internal TLS](internal-tls.md),
  [secret files](secret-files.md),
  [RabbitMQ cluster](rabbitmq-cluster.md),
  [HA architecture](../../architecture/ha.md),
  [manual 15](../../manual/15-ha-deployment.md)

The object store is one distributed MinIO deployment. The application connects
to `https://minio:9000` through the HAProxy edge pair; every MinIO node answers
on its own `minio-<host>` name. One node loss keeps the write quorum, the bucket
and every committed object; no output claims off-host replication.

## 1. Topology and quorum

```text
application (gateway, data, model, strategy, backtest, evidence, simulation)
        │ https://minio:9000 (TLS, CA verified)
        ▼
HAProxy edge-1 / edge-2          TCP passthrough, roundrobin, health-checked
        ▼
MinIO infra-1 … infra-4          one drive each, EC:2 (reference)
MinIO lab-1 … lab-3              two drives each, EC:3 (three-VM lab)
```

| Unit | Descriptor | Rendered file | Alias |
|---|---|---|---|
| MinIO | `stateful.minio.nodes` / `.drives` / `.parity` | `minio-<host>.conf` | `minio-<host>` only |
| HAProxy | `stateful.haproxy.nodes` | `haproxy-<host>.cfg` | `haproxy-<host>`, `postgres`, `rabbitmq`, `minio` |

The reference topology is four nodes × one drive with `parity: "EC:2"`; the lab
is three nodes × two drives with `parity: "EC:3"`. `auto` derives `total / 2`,
which gives the same values. An explicit value is refused unless:

- `parity > the largest per-node drive count`, so one node loss leaves the
  write quorum (`remaining >= data + 1`); and
- `2 * parity <= total drives`, so data blocks are not fewer than parity blocks.

The lab never uses `[2,1,1]`: losing `lab-1` removes two of four drives and the
cluster becomes read-only. `[2,2,2]` EC:3 leaves four drives after one node loss
and keeps writes. The shared `minio` alias lives only on the HAProxy pair;
putting it on a node would make a node loss a client address loss.

## 2. Configuration ownership

| File | Owner | Renderer |
|---|---|---|
| `minio-<host>.conf` | PLAT-005 | `render/minio.rs::minio_conf` |
| `minio-policy.json` | PLAT-005 | `render/minio.rs::bucket_policy` |
| `pki/minio/{public.crt,private.key,CAs/ca.crt}` | PLAT-005 + HA-010 | `render/minio.rs::render` |
| `infra/<host>.yaml` | PLAT-003/004/005 | `render/infra.rs` |
| `stateful-manifest.json` | PLAT-003/004/005 | `render/stateful.rs::stateful_manifest` |
| `object-bootstrap` | PLAT-005 | `tools/stateful.sh` |

The descriptor names hosts, drives, parity, image pins, and secret files. It
never contains a configuration body, a bucket policy, or a secret value. The
rendered config is mode 600; the policy, certificate, and CA are 644; the key is
600. Each MinIO Compose service key is `minio-<host>`, so Compose's implicit
service-name alias cannot recreate the shared `minio` name on a node. The
container is `quantrule-minio-<host>` and the hostname is `minio-<host>`.

## 3. TLS and listeners

The server starts through a small `sh -c` wrapper that exports the root user and
password from the mounted files, sources `/etc/minio/minio.conf`, and execs
`minio server --certs-dir /certs`. `/certs` contains `public.crt`,
`private.key`, and `CAs/ca.crt`. There is no plaintext listener, no console, no
published 9000 or 9001, and no `minio` alias on a node.

The leaf covers `minio` and every `minio-<host>`:

```bash
PKI_EXTRA_SANS="minio-infra-1,minio-infra-2,minio-infra-3,minio-infra-4" \
  tools/pki.sh --out /etc/quantrule/pki
```

The lab uses `minio-lab-1..3`. The shared `minio` name is already the CN and
`DNS.1`; `PKI_EXTRA_SANS` adds the unique node names. The manifest records the
same set, and the renderer test keeps the SANs and aliases aligned. Do not add a
plaintext fallback or disable hostname verification. The application uses
`QR_OBJECT_STORE_TLS_CA_PATH` with an explicit trust store; the platform roots
are not used for the private endpoint.

## 4. Erasure configuration

`minio-<host>.conf` contains no secret:

```text
MINIO_VOLUMES="https://minio-infra-1:9000/data1 https://minio-infra-2:9000/data1
               https://minio-infra-3:9000/data1 https://minio-infra-4:9000/data1"
MINIO_SERVER_URL="https://minio:9000"
MINIO_BROWSER="off"
MINIO_STORAGE_CLASS_STANDARD="EC:2"
MINIO_ROOT_USER_FILE="/run/secrets/minio_root_user"
MINIO_ROOT_PASSWORD_FILE="/run/secrets/minio_root_password"
MINIO_CONFIG_ENV_FILE="/etc/minio/minio.conf"
MINIO_REGION="us-east-1"
```

The volumes list is explicit per node and drive, in descriptor order. Every node
receives the same list; each server resolves its own `minio-<host>` and local
`/dataN` path. The `_FILE` names are references; the values are mode-600 host
files mounted read-only. The unit starts as the image's default user; if a later
image pin runs non-root, set `QR_STATEFUL_DATA_OWNER` and install the leaf for
that user.

## 5. Bucket, versioning, policy, and account

PLAT-005 owns the local object contract:

| Item | Value |
|---|---|
| Bucket | `quantrule` |
| Versioning | enabled |
| App user | `qr_app` (the `object_store_access_key` file value) |
| App secret | the `object_store_secret_key` file value |
| Policy | `quantrule-app`, bucket-scoped |
| Root credentials | `minio_root_user` / `minio_root_password` host files, `minio` scope |

The policy allows `GetBucketLocation`, `GetBucketVersioning`, `ListBucket`, and
`ListBucketMultipartUploads` on the bucket, and `GetObject`, `PutObject`,
`DeleteObject`, `AbortMultipartUpload`, and `ListMultipartUploadParts` on its
objects. It has no admin action, no bucket creation/deletion, no policy change,
and no wildcard resource. The app bucket is content-addressed and put-if-absent;
versioning keeps a superseded object recoverable without changing the key.

Root credentials never enter a rendered config, an app Swarm secret, or an app
container. The app `object_store_access_key` and `object_store_secret_key` stay
app Swarm secrets and are also copied to the MinIO hosts only for the one-shot
bootstrap. `tools/stateful.sh object-bootstrap` reads them from the mounted files
on the first MinIO host and never passes a value on the operator's command line.

## 6. Boot order and operator commands

The overlay is created by `bootstrap.sh --network-only`; the stateful units
attach to it, so they cannot start before it exists.

| Step | Command | Wait |
|---:|---|---|
| 1 | `tools/stateful.sh network` | overlay exists, attachable, encrypted |
| 2 | `tools/stateful.sh copy` | manifest, configs, PKI, secrets, drive dirs, units |
| 3 | `tools/stateful.sh start` | MinIO -> etcd -> Patroni -> HAProxy -> seed broker -> other two |
| 4 | `tools/stateful.sh wait` | MinIO nodes/drives/no healing, etcd, Patroni, brokers, psql |
| 5 | `tools/stateful.sh queue-bootstrap` | app queue user, vhost, permissions |
| 6 | `tools/stateful.sh object-bootstrap` | bucket, versioning, `qr_app`, policy, probe |
| 7 | `tools/swarm.sh bootstrap` | app objects, labels, jobs, stack |

`copy` reads the manifest `hosts` selector: MinIO config, leaf, policy, and root
files go only to `stateful.minio.nodes`; the app S3 pair also goes only there for
the bootstrap; the broker cookie/key and Patroni secrets stay on their own hosts.
It creates `/srv/quantrule/minio/<host>/drive1..N` with mode 700 and the
image's owner (or `QR_STATEFUL_DATA_OWNER` when the image is non-root).

`wait` writes a mode-600 `mc` config on the first MinIO node from the root files
and runs the pinned `images.minio_client` with
`/etc/quantrule/pki/minio/CAs/ca.crt` mounted at `/certs/ca.crt` and
`SSL_CERT_FILE=/certs/ca.crt`, so `mc` verifies the private MinIO leaf. It
requires every `minio-<host>:9000` in `mc admin info`, `0 drives offline`,
`0 drives healing`, and the expected online drive count. A missing node, a drive
count mismatch, or a healing drive is a wait failure, never a pass.

`object-bootstrap` uses a mode-700 directory and a mode-600 `config.json` for
both the root and app aliases. It creates the bucket with `mb --ignore-existing`,
enables versioning, recreates the `quantrule-app` policy, attaches it to `qr_app`,
adds the user with the pre-provisioned secret, and then probes with the app
credentials: list, put, get, delete (all versions of the one probe key), and
`version info`. An existing `qr_app` with a different secret makes the probe
fail; the operator rotates the secret file, updates the app Swarm secret, and
reruns. It never changes an existing user's secret silently.

`status` prints `mc admin info`, which includes the node, drive, and healing
state, plus the Patroni/etcd/RabbitMQ state. No tool removes a stack, volume,
secret, config, node, service, or bucket object outside the one-key bootstrap
probe.

## 7. HA-013 boundary

PLAT-005 owns local erasure coding, versioning, the bucket policy, and the app
account. HA-013 owns off-host/archive copies, restore drills, and any second-site
replication. `backup.object_replication` must be `false`; a true value is refused
as a replication claim without a second site. There is no object lock in
v1.5.10. A node or drive loss inside the cluster preserves the write quorum and
every committed object; a site loss is not protected by PLAT-005 and is not
claimed.

## 8. Failure behaviour and evidence

| Failure | Behaviour |
|---|---|
| One MinIO node loss (sample) | Three of four drives remain; EC:2 data 2 + write quorum 3 is satisfied; writes continue |
| One MinIO node loss (lab) | Four of six drives remain; EC:3 data 3 + write quorum 4 is satisfied; writes continue |
| Two node loss (sample) | Two of four drives remain; reads may continue, writes block; no committed object is lost |
| A drive is healing | `mc admin info` reports a non-zero healing count; `wait` and the lab treat it as not ready |
| A leaf lacks an alias | TLS verification fails closed; re-issue with `PKI_EXTRA_SANS` |
| `qr_app` has a different secret | The app probe fails; rotate the secret file and the app Swarm secret |
| A node returns after loss | MinIO heals the missing drive; the lab waits for zero healing before evidence |

A three-VM lab on one physical host is `Simulated HA`. The `ha` tier runs the
object phase after the queue phase and before the final full check. It requires
`lab-single-host`, three MinIO nodes, the `[2,2,2]` layout with EC:3,
`mc admin info` with every node and drive online, zero healing, the `quantrule`
bucket with versioning, and the app `/health`/`/ready`. It writes
`lab/minio-node-loss-probe` through `https://minio:9000` with the `qr_app`
credentials and records the SHA-256; it stops one node whose loss leaves the
write quorum with `docker stop`; it records the reduced admin info and reads the
same probe through the front door; it starts the node, waits for all drives,
runs a bounded `mc admin heal -r qr/quantrule`, waits for zero healing, and
reads the probe again. The value-free JSON under `QR_SWARM_EVIDENCE` records the
layout, parity, quorum, stopped host, probe hash before/after, the heal, and the
digest status. `--skip-object` leaves the object row `Unverified`. The
end-to-end canonical digest is `Measured` only when `tools/verify.sh` succeeds;
otherwise it is `Unverified` or `Blocked`, and no full-stack `PASS` is printed.
`Real HA` is HA-014.

## 9. Check rows and verification

`qr-deploy check` adds the MinIO rows after the queue rows. The default check
renders in memory and stays Docker-free; `--out` or `QR_DEPLOY_OUT` reads the
rendered bytes:

| Row | Checks |
|---|---|
| `minio` / `conf` | exact `MINIO_VOLUMES`, `https://minio:9000`, browser off, `EC:N`, root `_FILE`, region |
| `minio` / `no-secret` | no root/app secret value and no console port 9001 |
| `minio` / `overlay-only` | no host underlay address |
| `minio` / `quorum` | drives, parity, write quorum after one node loss; `[2,1,1]` is refused |
| `minio` / `unit` | unique key/hostname/alias, certs dir, config/PKI/root/drive mounts, wrapper, no published port |
| `haproxy` / `frontend`, `backend`, `unit` | `minio` alias on the edge pair, `*:9000`, health-checked `minio-<host>:9000`, shutdown-sessions |
| `manifest` / `hosts`, `minio-modes`, `sans` | config/root/policy/PKI only on MinIO hosts; modes; `minio` + every `minio-<host>` |
| `minio certificate` / `san` | leaf SANs and 30-day expiry; a missing `openssl` is `limited` |
| `app` / `object-store`, `credentials`, `replication` | URL, `_FILE` pair, CA, `qr_app`, `object_replication: false` |

```bash
cargo test -p quantrule-deploy minio
cargo test -p quantrule-deploy --test render_golden
cargo test -p quantrule-deploy --test stateful_tool
qr-deploy validate deploy/v3/deploy.example.json
qr-deploy validate deploy/v3/deploy.lab.json
docker compose -f <render>/infra/infra-1.yaml config -q
tools/stateful.sh --plan
tools/test_tiers.sh ha --plan
bash -n tools/stateful.sh deploy/v3/test.sh deploy/v3/minio-failover.sh
```

The fast tier proves the descriptor, parity/quorum rule, rendered config, policy,
unit, HAProxy front door, manifest hosts, certificate row, and shell contract. It
does not prove a live MinIO start, TLS handshake, node loss, or healing; those
are lab evidence. The `ha` tier defaults to the lab descriptor; a missing lab is
`UNVERIFIED`.

## 10. Recovery

| Symptom | First action |
|---|---|
| MinIO does not start | Check the root file mounts, `minio.conf`, the leaf under `/certs`, and the drive directory owner |
| `mc admin info` shows a node missing | Check `minio-<host>` DNS on the overlay and the node log; do not remove a drive |
| A drive is healing | Wait for zero healing; do not restart the node during the heal |
| The app cannot verify the endpoint | Check the leaf SAN for `minio` and every `minio-<host>`, the CA bundle, and `QR_OBJECT_STORE_TLS_CA_PATH` |
| The app gets access denied | Check the `qr_app` user, the `quantrule-app` policy, and the bucket name |
| Versioning is disabled | Rerun `object-bootstrap`; `version enable` is idempotent |
| A policy edit is rejected | Change the renderer, re-render, and rerun; never patch a running host file |

The previous image, descriptor, render directory, drive data, and root files stay
in place until the cluster passes. Re-render and recopy rather than editing a
host file. No tool in this contract deletes a bucket, object, volume, stack,
secret, config, node, or service.
