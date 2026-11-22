# QuantRule multi-host deployment

This directory is the per-role half of the `quantrule.deploy/v2` profile. It
splits the six-service server stack across several Linux hosts that share a
private overlay network. The single-host stack in [`compose.yaml`](../../compose.yaml)
stays the default; nothing here changes a service binary or an engine.

**Only the gateway is public.** The five internal services have no application
authentication of their own, so the overlay network and the host firewall are
the boundary. The gateway's bearer token is the only application gate, and it is
optional on the overlay. Never publish an internal port on a LAN or public
interface.

## 1. Roles

| Role | Compose file | Services | Reachable ports |
|---|---|---|---|
| Gateway | `compose.gateway.yaml` | `gateway` | `QR_BIND_PORT` (behind a TLS proxy if the client network is untrusted) |
| Core | `compose.core.yaml` | `data`, `evidence`, `simulation` | one `QR_BIND_PORT` each, overlay only |
| Compute | `compose.compute.yaml` | `model`, `backtest` | `model` port, overlay only; `backtest` none |
| Infrastructure | `compose.infra.yaml` | `postgres`, `minio`, `rabbitmq` | `5432`, `9000`, `5672`, overlay only |

Each file declares its own Compose project name, so bringing one role down
never treats another role's containers as orphans. Every role except
infrastructure uses `network_mode: host`: the overlay address is a host address,
and the service binds it directly instead of publishing a mapped port.

## 2. Host prerequisites

- Linux x86_64 (the server profile; Docker Desktop host networking is not this
  profile). Docker Engine with the Compose v2 plugin, `curl`.
- One private overlay address per host. That address is the per-host bind
  address the descriptor records and `qr-deploy` renders.
- The same image loaded on every host. There is no registry in this profile.
- The repository `config/` directory on the core host, at `../../config`
  relative to this bundle, or a `QR_CONFIG_HOST_DIR` exported before Compose
  runs. It is mounted read-only into `data`.
- The operator secrets from [`secrets.example.env`](secrets.example.env).

## 3. Private overlay

WireGuard is the reference; Tailscale is an accepted alternative. It provides
both reachability and transport confidentiality between hosts, which is what
replaces mTLS in this profile.

- Assign every host a stable overlay address and record it in the descriptor.
- Keep the overlay up before Docker starts; a host that loses its address fails
  its healthchecks until the overlay returns.
- Bind every service, including the gateway, to the overlay address. If remote
  clients need public HTTPS, terminate TLS in a reverse proxy on the gateway
  host and forward to the gateway's overlay address; do not bind `0.0.0.0`.

## 4. Descriptor, secrets, and rendered environment

The descriptor (`quantrule.deploy/v2`, JSON) is operator-owned and lives outside
this repository. It names the hosts, service placement, endpoints, replicas,
network gate, image tag, and one image digest.

Run these from the repository root; the rendered directory is the one the
per-role Compose files read. From `deploy/multi-host`, the same flags are
`--secrets secrets.env` and `--out env`.

```bash
export DESCRIPTOR=/etc/quantrule/quantrule.deploy.json

cp deploy/multi-host/secrets.example.env deploy/multi-host/secrets.env
chmod 600 deploy/multi-host/secrets.env    # fill every value first
qr-deploy validate "$DESCRIPTOR" --secrets deploy/multi-host/secrets.env
qr-deploy render "$DESCRIPTOR" --out deploy/multi-host/env \
  --secrets deploy/multi-host/secrets.env
```

The shipped descriptor and secrets file are templates, not deployable values.
`qr-deploy validate` rejects the all-one-character image digest, and
`--secrets` rejects the `replace-with-...` passwords and token. Replace the
digest, every password, and `QR_BEARER_TOKEN` (use `tools/token.sh`) before the
first validation. The token must be at least 32 characters; the passwords at
least 16 and not a template marker. The example descriptor enables
`auth.gateway_token`, so the token is mandatory.

`env/` is ignored by Git. The renderer writes one file per service, one
`<service>-migrate.env` for each database-owning service, and `infra.env`; each
service file carries at least `QR_IMAGE`, `QR_IMAGE_DIGEST`, `QR_BIND_IP`,
`QR_BIND_PORT`, `QR_BIND`, `QR_PROFILE=server`, the backend URLs and
credentials, `QR_MIGRATE`, and `QR_INTERNAL_ONLY` on the internal HTTP services.
Each database-owning service file gets only `QR_DATABASE_URL` for its
`<database>_app` role with `QR_MIGRATE=0`; the matching migration file gets only
`QR_DATABASE_OWNER_URL` for its `<database>_owner` role. `model` and `backtest`
receive no database URL. Keep the rendered files at mode 600: they contain the
resolved secrets.

The descriptor still names one Postgres endpoint. The renderer keeps its scheme
and host:port and uses `QR_POSTGRES_PASSWORD` for all eight roles; the four
databases (`qr_gateway`, `qr_catalog`, `qr_evidence`, `qr_simulation`) and the
roles are fixed by the bootstrap. Per-role password files belong to v3. The full
contract is
[`docs/config/2026-11-22/four-databases.md`](../../docs/config/2026-11-22/four-databases.md).

## 5. Image distribution — one digest

One image carries all six binaries. Build it once, then distribute the archive;
every host must report the same image ID, which is the descriptor's image
digest.

```bash
# release host, repository root
docker build -t quantrule-server:1.3.0 .
digest=$(docker image inspect --format '{{.Id}}' quantrule-server:1.3.0)
docker save quantrule-server:1.3.0 -o quantrule-server-1.3.0.tar
sha256sum quantrule-server-1.3.0.tar > quantrule-server-1.3.0.tar.sha256

# every service host
scp quantrule-server-1.3.0.tar* operator@host:/var/tmp/
ssh operator@host 'cd /var/tmp && sha256sum -c quantrule-server-1.3.0.tar.sha256'
ssh operator@host 'docker load -i /var/tmp/quantrule-server-1.3.0.tar'
ssh operator@host "docker image inspect --format '{{.Id}}' quantrule-server:1.3.0"
```

Record `$digest` in the descriptor before rendering, and confirm
`docker image inspect --format '{{.Id}}'` on every host matches it. The role
files use `pull_policy: never`, so Compose cannot pull a different tag.
`qr-deploy check` compares the digest each service *reports*; that is an
environment-consistency check, not proof of the running image. The operator
must run the `docker image inspect` check on each host.

## 6. Start order

Infrastructure, then core, then compute, then the gateway. The role files have
no cross-host `depends_on` because the dependency is on another machine; the
order exists so the first start does not spend its retries on a backend that is
not up yet. The infrastructure file also runs the one-shot `db-bootstrap` after
Postgres is healthy; it creates the four databases and the owner/app roles
before the core host starts. The core file runs `data-migrate`,
`evidence-migrate`, and `simulation-migrate` before their services; the gateway
file runs `gateway-migrate` before the gateway. Each migration job is local to
its host, so its service can use `depends_on: service_completed_successfully`.

```bash
# infrastructure host, from the repository root
docker compose --env-file deploy/multi-host/env/infra.env \
  -f deploy/multi-host/compose.infra.yaml up -d --wait

# core host
docker compose --env-file deploy/multi-host/env/data.env \
  -f deploy/multi-host/compose.core.yaml up -d --wait

# compute host; set the count from the descriptor's services.backtest.replicas
docker compose --env-file deploy/multi-host/env/model.env \
  -f deploy/multi-host/compose.compute.yaml up -d --wait --scale backtest=2

# gateway host
docker compose --env-file deploy/multi-host/env/gateway.env \
  -f deploy/multi-host/compose.gateway.yaml up -d --wait
```

The `--env-file` supplies the shared `QR_IMAGE` to the project; each service
reads its own `env_file`. The infrastructure role needs
`deploy/multi-host/env/infra.env` for the bind address, credentials, and bucket
because Postgres, MinIO, and RabbitMQ use their own variable names.

## 7. Verification

```bash
qr-deploy check "$DESCRIPTOR" --secrets deploy/multi-host/secrets.env
```

Run this on the gateway host or another host on the private overlay; it needs
the full secrets file and probes the internal services directly. It does not
send the bearer token on the open `/version` and `/health` paths. The backtest
worker has no HTTP surface, so the row is printed as `SKIP`; check it on the
compute host with `docker ps --filter name=backtest`. The tool asserts that
every probed service reports the descriptor's image digest, but it does not
inspect the running image on a remote host. Then, by hand:

```bash
curl -fsS "http://<gateway-overlay-ip>:8080/ready"
curl -fsS "http://<gateway-overlay-ip>:8080/version"
```

From a host outside the overlay, `curl --max-time 3` against any core, compute,
or infrastructure port must time out. A client VPN peer must be routed to the
gateway only; if it can reach the overlay subnet, it can call the internal
services without the gateway token. Submit the verification fixture through the
gateway and compare its canonical digest with the single-host run. The local
multi-host harness (#964) checks the fixture and two worker counts; the real
three-host LAN comparison is a separate verification row and is not automated.

## 8. Firewall

The bind address is the first control; the host firewall is the second.

| Host | Allow inbound | Source |
|---|---|---|
| Gateway | gateway port | the operator network or the TLS proxy only |
| Core | `data`, `evidence`, `simulation` ports | overlay subnet only |
| Compute | `model` port | overlay subnet only |
| Infrastructure | `5432`, `9000`, `5672` | overlay subnet only (core, compute, gateway hosts) |

```bash
# gateway host, WireGuard interface wg0
ufw allow in on wg0 to any port 8080 proto tcp
ufw default deny incoming
```

Ports published by Docker are DNATed before ufw's INPUT chain. The
`QR_BIND_IP` binding is what keeps them off other interfaces; add a
`DOCKER-USER`/nftables rule when the host firewall must be authoritative. Never
forward `5672`, `5432`, or `9000` beyond the overlay.

## 9. Upgrade and rollback

1. Load the new image on every host and confirm the image ID is identical (§5).
2. Stop the gateway, then compute, then core (all application roles). Leave
   infrastructure running.
   Stopping the gateway first stops new submissions; stopping workers then lets
   the queue hold or redeliver their messages.
3. Update the descriptor's image tag and digest, then repeat the `validate` and
   `render` commands in §4.
4. Start core, then compute, then the gateway (§6). Migrations run forward at
   startup and are idempotent.

Rollback is the same sequence with the previous tag and the previous rendered
files; keep the previous image loaded on every host. Migrations are
forward-only, so a rollback never reverts the schema and the previous version
must tolerate it. If it cannot, restore from backup.

## 10. Logs and stop

```bash
docker compose --env-file deploy/multi-host/env/data.env \
  -f deploy/multi-host/compose.core.yaml logs -f --tail 200
```

Stop in reverse order: gateway, compute, core, infrastructure. `docker compose
... down` removes containers and keeps the named volumes; removing a volume is
a separate, destructive decision.

## 11. Backup and restore

Stop the gateway, compute, and core hosts; leave infrastructure running. Run
the [`backup.sh`](backup.sh) and [`restore.sh`](restore.sh) wrappers on the infrastructure
host with `--application-stopped`; they wrap the tested archive tools and read
the custom database, bucket, and queue names from `env/infra.env`. The archive
holds a Postgres dump, an object mirror, and a manifest; the queue is not
archived. The current tools dump one database, so they do not yet archive all
four; treat that as a known limitation until the backup change lands. A restore
must reproduce the canonical digest, and copying a live database or object-store
volume is not a backup.

## 12. What this profile does not provide

No high availability, autoscaling, scheduler, registry, TLS termination, or
service-to-service mTLS; one model endpoint in v1. The private overlay is the
trust boundary, exactly as the container network is on a single host. The
decision, its alternatives, and its non-goals are recorded in ADR
`958-multi-host-deployment.md` and in
[#958](https://github.com/quant-rule/quant-rule/issues/958).

The profile records these residual risks:

- Database roles are per service, but this v2 profile still uses one Postgres
  password for all eight roles. MinIO root credentials and one queue user are
  also shared. A compromised service host can read every database with the
  shared password and the whole bucket. Per-role password files and bucket
  policies are not implemented.
- There is no service-to-service authentication. The private overlay is the
  boundary; a client VPN peer must be routed to the gateway only, and internal
  ports must be firewalled to the exact host addresses, not the whole subnet.
- `qr-deploy check` reads the digest each service reports. It does not inspect
  the running image on a remote host; the operator runs `docker image inspect`
  on each host.
- The gateway token and TLS are not exercised by `check`; use the Connection
  screen or a `curl` matrix against a gated route.
- Postgres, MinIO, and RabbitMQ are single instances. The profile is not high
  availability, and a core-host failure takes data, evidence, and simulation
  with it.
