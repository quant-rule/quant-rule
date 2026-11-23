# File-Backed Secrets

- Date: 2026-11-23 (UTC)
- Status: Current for the local Compose stack, the v2 multi-host profile, and v3 render/check
- Issue: [#1107](https://github.com/quant-rule/quant-rule/issues/1107)
- Claim type: Behaviour Claim
- Related: [HA architecture](../../architecture/ha.md), [security](../../architecture/security.md), [four databases](../2026-11-22/four-databases.md)

The application services never receive a secret through an environment
variable. Each secret is a file under `/run/secrets`, named by a
`<NAME>_FILE` variable. `docker inspect`, `/proc/<pid>/environ` and a child
process therefore see the path, not the value.

## 1. File contract

`quantrule_runtime::config` is the only production reader. It applies the same
rules to every secret:

| Rule | Result |
|---|---|
| Path | Must be absolute. |
| Size | At most 8 KiB. |
| Encoding | UTF-8; a NUL byte is refused. |
| Trailing newline | Exactly one `\n` and an optional preceding `\r` are stripped. |
| Empty | A value empty after stripping is refused. |
| Whitespace | Leading, trailing or embedded newline whitespace is refused. |
| Both sources | `<NAME>` and `<NAME>_FILE` set together is an error. |
| Plain variable | `<NAME>` alone is a startup error; the operator's `QR_BEARER_TOKEN` input to `make remote` is not a server deployment source. |
| Optional | A secret with neither variable is unset; the consumer that needs it fails closed. |

The value is never printed, hashed or fingerprinted by the runtime. A
`ConfigError` names the variable and the rule, never the value. `Config`'s
`Debug` redacts every secret field.

## 2. Variables

| Secret | File variable | Read by |
|---|---|---|
| Service Postgres URL | `QR_DATABASE_URL_FILE` | gateway, data, evidence, simulation |
| Owner Postgres URL | `QR_DATABASE_OWNER_URL_FILE` | the one-shot `<service>-migrate` job |
| AMQP URL | `QR_QUEUE_URL_FILE` | evidence, backtest |
| S3 access key | `QR_OBJECT_STORE_ACCESS_KEY_FILE` | data, model, evidence, simulation, strategy, backtest |
| S3 secret key | `QR_OBJECT_STORE_SECRET_KEY_FILE` | same six services |
| Gateway bearer token | `QR_BEARER_TOKEN_FILE` | gateway, only when authentication is enabled |

`QR_OBJECT_STORE_ACCESS_KEY_FILE` and `QR_OBJECT_STORE_SECRET_KEY_FILE` are a
pair: setting one without the other is an error. The object-store URL
(`QR_OBJECT_STORE_URL`) must not carry userinfo; the credentials come from the
two files. The S3 client is built with those explicit credentials and does not
consult `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, the shared AWS config or
instance metadata.

Everything else remains a plain environment value: service URLs, bucket,
queue generation, bind addresses, limits, log and metrics settings, drain
deadlines, image digest and the leader/worker identifiers. The internal TLS CA
paths (`QR_DATABASE_TLS_CA_PATH`, `QR_QUEUE_TLS_CA_PATH`,
`QR_OBJECT_STORE_TLS_CA_PATH`) are also plain values: a CA bundle is public and
is mounted at `/run/pki`, not under `/run/secrets`. The contract is
[internal TLS](internal-tls.md).

## 3. Local Compose

`make up` depends on `make secrets` and `make pki`, which run `tools/secrets.sh`
and `tools/pki.sh`:

- `secrets/compose.env` (mode 600) holds the three infrastructure passwords
  used by Postgres, MinIO and RabbitMQ. Those containers are infrastructure and
  keep their own bootstrap variables.
- `secrets/<name>` (mode 600) holds the application URLs, the S3 pair and the
  token. Each application service mounts only the files it reads and sets only
  the matching `_FILE` variables.
- The directory is mode 700 and Git-ignored. A normal `make up` reuses existing
  values; `make secrets FORCE=1` rotates them deliberately.
- `pki/` is a separate mode-700, Git-ignored directory with the CA and server
  certificates; only `ca.crt` is mounted into an application container. A
  private key is never a secret file under `/run/secrets`.

`make remote` layers `compose.remote.yaml`, writes `secrets/bearer_token` and
mounts it at `/run/secrets/bearer_token`. The base stack leaves the token file
unset, so it keeps the documented unauthenticated local profile.

`make compose-check` writes disposable secrets to a temporary directory and
validates both Compose files, so CI needs no committed secret.

## 4. Multi-host v2 and Swarm v3

`qr-deploy render` writes `out/secrets/` at mode 700 with one mode-600 file per
secret, and the service env files name `/run/secrets/<name>`. The role Compose
files declare the mount. Copy `env/` and `env/secrets/` to the host that runs
the role; never put a value in an env file.

For `quantrule.deploy/v3`, the renderer reads each `secrets.files` source once
and writes the same value to `secrets/<name>`; `secrets/manifest.json` lists the
names, relative paths, modes, and Swarm names, never a value or a fingerprint.
`bootstrap.sh` creates one external Swarm secret per app file from
`secrets/<name>`; it hashes the rendered file locally for the
`quantrule.source_sha256` drift label and never prints, logs, or passes the
value. The app services mount only the secrets they consume; owner URLs go only
to the migration jobs. The five stateful secrets (four Patroni passwords and
`rabbitmq_erlang_cookie`) are excluded from app Swarm secrets and copied by
`tools/stateful.sh`; the cookie and broker key go only to
`stateful.rabbitmq.nodes`. The two MinIO root secrets (`minio_root_user`,
`minio_root_password`) have `minio` scope: they go only to
`stateful.minio.nodes`, never an app Swarm secret and never an app container.
`object_store_access_key` is exactly `qr_app`; that pair stays an app Swarm
secret and is copied to the MinIO hosts only for the one-shot object bootstrap.
`QR_INTERNAL_ONLY=1` and the entrypoint's private-address resolution are
unchanged; a task with no resolvable private address fails closed. The queue
definitions export is credential-free (`users`/`permissions` stripped) and mode
600. The contract is [deploy/v3](../2026-11-22/deploy-v3.md) and the MinIO
contract is [minio-cluster](minio-cluster.md). The MinIO check verifies the root
files only on MinIO hosts and the app `_FILE` path; the lab proves one node loss
keeps the probe hash.

## 5. Token rotation

`QR_BEARER_TOKEN_FILE` is the only gateway token source when authentication is
enabled. The gateway reads it at startup, re-reads it every
`QR_TOKEN_RELOAD_SECONDS` (default 5) and immediately on `SIGHUP`. A successful
read swaps one immutable snapshot; an in-flight request keeps the token it
started with and the next request uses the new one. A missing, empty, short or
malformed file keeps the last good token, increments
`quantrule_auth_reload_failures_total{reason}` and never disables
authentication. A file present but invalid at startup fails startup.

The image entrypoint starts as root only to stage the mounted files into a
mode-700 directory owned by UID 10001, rewrites the `_FILE` values to that copy,
and drops privileges before exec. The host files and Compose mounts stay mode
600; no service binary runs as root. Rotation therefore updates the staged file:
`make remote` does it, or copy the file with `docker compose cp` and set its
owner with the commands in the manual.

```bash
tools/token.sh --out secrets/bearer_token
docker compose -f compose.yaml -f compose.remote.yaml \
  cp secrets/bearer_token gateway:/run/quantrule-secrets/bearer_token
docker compose -f compose.yaml -f compose.remote.yaml exec -T -u 0 gateway \
  sh -c 'chown 10001:10001 /run/quantrule-secrets/bearer_token && chmod 400 /run/quantrule-secrets/bearer_token'
docker compose -f compose.yaml -f compose.remote.yaml kill -s HUP gateway
export QUANTRULE_TOKEN="$(cat secrets/bearer_token)"
```

Rotate by replacing the file and sending `SIGHUP`; no restart is required. Every
client re-exports the new value. The browser forgets and re-enters it on the
Connection screen. The token age is `quantrule_auth_token_age_seconds`; no token
fingerprint or generation appears in `/version` or a log record.

Under Swarm, a secret object is immutable, so the HA profile uses the recorded
exception: create a new secret name, update the gateway service to mount it,
wait for the rolling replacement to drain, then remove the old secret. The
mount target stays `gateway_token`, so `QR_BEARER_TOKEN_FILE` and the gateway
token code are unchanged. The exact operator sequence is in
[manual 15](../../manual/15-ha-deployment.md#token-rotation-under-swarm).

## 6. Verification

- `cargo test -p quantrule-runtime config` proves the contract and redaction.
- `cargo test -p quantrule-runtime --test secret_sources` proves one reader and
  no AWS default chain.
- `cargo test -p quantrule-runtime --test database_topology` proves application
  services name only `_FILE` variables.
- `cargo test -p quantrule-deploy` proves the rendered secret files and modes.
- `cargo test -p quantrule-gateway --test auth_rotation` proves the swap and the
  last-good-token rule.
- `make compose-check` proves both Compose files parse without committed secrets.
