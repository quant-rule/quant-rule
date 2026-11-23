# Fixture Secrets

These files satisfy the `quantrule.deploy/v3` `validate --files` contract
without containing real credentials. Each value is fake and every role uses a
different password so the duplicate-credential refusal is exercised.

The directory is a fixture, not a deployment secret store. Copy the files to a
mode-700 directory and make each file mode 600 before running `qr-deploy
validate --files`. Git does not preserve mode 600 on checkout.

| File | Contract |
|---|---|
| `gateway_token` | At least 32 characters, not a template or repeated character. |
| `<service>_database_url` | `postgres://<app_role>:<password>@postgres:5432/<database>?sslmode=verify-full` |
| `<service>_owner_database_url` | The owner role for the same database; a distinct password. |
| `queue_url` | `amqps://qr_queue:<password>@rabbitmq:5671/%2f`; the shared alias, the `qr_queue` user, and the default vhost are required. |
| `object_store_access_key` | At least 8 characters, not a documented default. |
| `object_store_secret_key` | At least 16 characters, distinct from every role password. |
| `postgres_superuser_password` | At least 16 characters, URL-safe, not a default; used by Patroni and the cluster bootstrap. |
| `patroni_replication_password` | At least 16 characters, URL-safe, distinct; the streaming-replication role. |
| `patroni_rewind_password` | At least 16 characters, URL-safe, distinct; the pg_rewind role. |
| `patroni_rest_password` | At least 16 characters, URL-safe, distinct; the Patroni REST basic-auth password and HAProxy check header. |
| `rabbitmq_erlang_cookie` | At least 32 URL-safe characters, distinct; installed as `.erlang.cookie` on the three brokers only. |

The five stateful secrets are rendered with `scope: stateful` in
`secrets/manifest.json` and copied to the hosts by `tools/stateful.sh`; they are
not app Swarm secrets. Their values are fake and deliberately URL-safe so they
can be interpolated into a Patroni, HAProxy, or RabbitMQ runtime configuration
without percent-encoding. The manifest carries a per-entry `hosts` selector:
the Postgres secrets go to the Patroni hosts (the REST password also to the
HAProxy pair), and the Erlang cookie goes only to the broker hosts. The queue
definitions export is a separate credential-free file (`users`/`permissions`
stripped, mode 600) and never contains these values.

The required names and the full file contract are in
[`docs/config/2026-11-22/deploy-v3.md`](../../../docs/config/2026-11-22/deploy-v3.md).
Never commit a real secret here.
