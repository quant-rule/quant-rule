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

```bash
export DESCRIPTOR=/etc/quantrule/quantrule.deploy.json

cp secrets.example.env secrets.env
chmod 600 secrets.env                 # fill every value first
qr-deploy validate "$DESCRIPTOR"      # refuses an unsafe placement or bind
qr-deploy render "$DESCRIPTOR"        # reads secrets.env, writes env/*.env
```

`env/` is ignored by Git. The renderer writes one file per service and
`env/infra.env`; each carries at least `QR_IMAGE`, `QR_IMAGE_DIGEST`,
`QR_BIND_IP`, `QR_BIND_PORT`, `QR_BIND`, `QR_PROFILE=server`, the backend URLs
and credentials, `QR_MIGRATE`, and `QR_INTERNAL_ONLY` on the internal HTTP
services. Keep the rendered files at mode 600: they contain the resolved
secrets.

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

Record `$digest` in the descriptor before rendering. `qr-deploy check` fails
when a service reports another digest, and `/version` and `/health` report the
value the service was started with.

## 6. Start order

Infrastructure, then core, then compute, then the gateway. The role files have
no `depends_on` because the dependency is on another machine; the order exists
so the first start does not spend its retries on a backend that is not up yet.

```bash
# infrastructure host
docker compose --env-file env/infra.env -f compose.infra.yaml up -d --wait

# core host
docker compose --env-file env/data.env -f compose.core.yaml up -d --wait

# compute host
docker compose --env-file env/model.env -f compose.compute.yaml up -d --wait

# gateway host
docker compose --env-file env/gateway.env -f compose.gateway.yaml up -d --wait
```

The `--env-file` supplies the shared `QR_IMAGE` to the project; each service
reads its own `env_file`. The infrastructure role needs `env/infra.env` for the
bind address, credentials, and bucket because Postgres, MinIO, and RabbitMQ use
their own variable names.

## 7. Verification

```bash
qr-deploy check "$DESCRIPTOR"
```

It probes every service endpoint and asserts one image digest across the hosts.
Then, by hand:

```bash
curl -fsS "http://<gateway-overlay-ip>:8080/ready"
curl -fsS "http://<gateway-overlay-ip>:8080/version"
```

From a host outside the overlay, `curl --max-time 3` against any core, compute,
or infrastructure port must time out. From a client, submit the verification
fixture through the gateway and compare its canonical digest with the
single-host run; the multi-host tier (#964) automates that comparison.

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
2. Stop the gateway, then compute, then core. Leave infrastructure running.
   Stopping the gateway first stops new submissions; stopping workers then lets
   the queue hold or redeliver their messages.
3. Update the descriptor's image tag and digest, then `qr-deploy validate` and
   `qr-deploy render`.
4. Start core, then compute, then the gateway (§6). Migrations run forward at
   startup and are idempotent.

Rollback is the same sequence with the previous tag and the previous rendered
files; keep the previous image loaded on every host. Migrations are
forward-only, so a rollback never reverts the schema and the previous version
must tolerate it. If it cannot, restore from backup.

## 10. Logs and stop

```bash
docker compose --env-file env/data.env -f compose.core.yaml logs -f --tail 200
```

Stop in reverse order: gateway, compute, core, infrastructure. `docker compose
... down` removes containers and keeps the named volumes; removing a volume is
a separate, destructive decision.

## 11. Backup and restore

Back up the Postgres database, the MinIO bucket, the descriptor, the rendered
`env/` directory, and `secrets.env` (separately, encrypted). The multi-host
backup and restore procedure is tracked in
[#963](https://github.com/quant-rule/quant-rule/issues/963); the pieces and the
archive format are the ones in [`tools/backup.sh`](../../tools/backup.sh) and
[`tools/restore.sh`](../../tools/restore.sh). A restore must reproduce the
canonical digest, and copying a live database or object-store volume is not a
backup.

## 12. What this profile does not provide

No high availability, autoscaling, scheduler, registry, TLS termination, or
service-to-service mTLS; one model endpoint in v1. The private overlay is the
trust boundary, exactly as the container network is on a single host. The
decision, its alternatives, and its non-goals are recorded in ADR
`958-multi-host-deployment.md` and in
[#958](https://github.com/quant-rule/quant-rule/issues/958).
