# QuantRule `quantrule.deploy/v3`

This directory holds the checked-in v3 descriptor, the checked-in Traefik static
configuration, the fake fixture secrets and ingress certificate, and the local
contract examples. The contract is
[`docs/config/2026-11-22/deploy-v3.md`](../../docs/config/2026-11-22/deploy-v3.md);
the ingress contract is
[`docs/config/2026-11-23/ingress.md`](../../docs/config/2026-11-23/ingress.md),
and the stateful database contract is
[`docs/config/2026-11-23/patroni-cluster.md`](../../docs/config/2026-11-23/patroni-cluster.md).

`qr-deploy` implements the closed schema, version-first dispatch, `validate`,
`validate --files`, `render`, and `check`. The renderer writes the seven-service
Swarm stack, the static and dynamic Traefik files, the per-edge keepalived files,
the per-host Patroni/etcd/HAProxy configuration, the per-broker RabbitMQ config
and plugin list, the RabbitMQ/MinIO/observability units, the stateful manifest,
the ordered operator jobs, the secret/PKI copies, and `render.json`. The checker
validates the descriptor, placement, pins, secrets, PKI, and local files and
probes the one HTTPS ingress. `tools/swarm.sh` wraps render, bootstrap, status,
upgrade, rollback, and verify; the control-plane contract is
[`docs/config/2026-11-23/swarm-control.md`](../../docs/config/2026-11-23/swarm-control.md).
`qr-deploy check` adds the stateful rows (Patroni sync/rewind/archive/TLS/
pg_hba/watchdog, etcd mTLS, HAProxy `/primary`, the manifest, and the Postgres/
etcd certificate SANs) and the queue rows (TLS-only broker, three-node cluster,
alarms, HAProxy `rabbitmq`/5671, the broker unit, manifest hosts/modes, and the
RabbitMQ leaf SANs). `--out <render-dir>` or `QR_DEPLOY_OUT` reads a rendered
tree; without it the configs render in memory and the certificates come from the
descriptor CA. `--remote docker --overlay` adds the read-only Swarm, node,
overlay, service, port, task, and ingress-mount rows; the default check stays
Docker-free. `tools/test_tiers.sh ha` defaults to `deploy.lab.json` and runs the
opt-in lab in `test.sh`; `--plan` is Docker-free. The harness drains the VIP
holder, pauses the Patroni primary and one HAProxy, stops the RabbitMQ quorum
leader, measures RTO and the queue gap, writes and re-reads markers through
`postgres`, proves a confirmed publish survives the queue loss, compares the
digest/LSN, runs the 8404/5432/8080/9000/5671/15692/15672 negative controls,
records the rejoin, and prints `UNVERIFIED` when a precondition is not met.
`--skip-database`/`--skip-queue` leave that row `Unverified`; the queue result
is `Simulated HA` at the queue level while the end-to-end digest stays `Blocked`
until PLAT-005 MinIO. The shell trusts the private CA with `QUANTRULE_CA_CERT`;
the browser uses the OS trust store. v2 output is unchanged.

## Validate, render, check

The sample names `/etc/quantrule/secrets/...` and `/etc/quantrule/pki/ca.crt`,
so a structural check needs no local copies:

```bash
qr-deploy validate deploy/v3/deploy.example.json
```

For the file contract, copy the fixture secrets to a temporary mode-700
directory, make each file mode 600, point a copy of the descriptor at them, and
run `--files`. The test suite does this; the fixture values are not deployable
credentials.

```bash
install -d -m 700 /tmp/qr-v3-secrets
find deploy/v3/secrets -type f ! -name README.md \
  -exec install -m 600 {} /tmp/qr-v3-secrets/ \;
install -d -m 700 /tmp/qr-v3-pki
install -m 644 deploy/v3/pki/ingress.crt /tmp/qr-v3-pki/ingress.crt
install -m 600 deploy/v3/pki/ingress-key.pem /tmp/qr-v3-pki/ingress.key
# point secrets.files, endpoints.*.tls_ca, and ingress.files at the local copies, then:
qr-deploy validate /tmp/deploy.json --files
qr-deploy render /tmp/deploy.json --out /tmp/qr-v3-out --target all
qr-deploy check /tmp/deploy.json --out /tmp/qr-v3-out
```

`--target swarm` writes `stack.yaml`, `quantrule_traefik.yml`,
`traefik-dynamic.yml`, `bootstrap.sh`, `jobs/`, and the shared `secrets/`,
`pki/`, and `render.json`. `--target infra` writes `infra/<host>.yaml`,
`keepalived-<host>.conf`, `patroni-<host>.yml`, `etcd-<host>.conf.yml`,
`haproxy-<host>.cfg`, `rabbitmq-<host>.conf`, `rabbitmq-enabled_plugins`, and the
shared material. `--target all` writes both. A v3 render without `--target` is a
usage error; v2 `render` is unchanged and rejects `--target`.

## Fixture secrets

`secrets/` contains one fake value per required secret: the gateway token, the
four database app/owner URLs, the queue URL, the S3 access/secret pair, and the
five stateful Patroni/cluster/RabbitMQ values. Every role has a distinct
password, so the duplicate-credential rule can be tested. The stateful values
are URL-safe; `rabbitmq_erlang_cookie` is at least 32 characters. The values are
deliberately non-default but are not real secrets; never deploy them.

After the stateful units are up, `tools/stateful.sh queue-bootstrap` creates the
`qr_queue` user, the default vhost, and its permissions from `queue_url`. The
password travels on standard input and is hashed before it reaches
`rabbitmqctl`; it is never an argument or a log line. Run it before
`tools/swarm.sh bootstrap`. `tools/queue-definitions.sh export` stores the
stripped topology (users, permissions, and topic permissions removed) with mode
600 on every broker; `verify`, `import`, and 30-day `prune` are in the queue
contract.

`pki/ingress.crt` and `pki/ingress-key.pem` are fake cert-mode fixtures. The
`.pem` name and the absent PEM key block keep the repository private-key checker
honest; the file is not a key and is never used to serve TLS. The sample points
at `/etc/quantrule/pki/...`; point a test copy at the fixtures before running
`--files`.

Git preserves the executable bit, not mode 600. After copying, run the
`install -m 600` command above.

The full shape, field rules, stable `deploy.*` codes, render layout, check
output, profiles, and evidence boundary are in the contract document. The sample
declares eleven hosts, three odd Swarm managers, three compute hosts across
`az1`..`az3`, four infrastructure hosts across `az1`..`az4`, and four MinIO
nodes with one drive each. It has etcd, HAProxy, and keepalived blocks and a
non-empty `secrets.files` map, a VIP, and a cert-mode ingress. The stack uses
`/health` checks, stop-first when no spare slot exists, and a 180 s update
monitor; the bootstrap creates both Traefik configs before `docker stack deploy`
and refuses secret/config drift.
