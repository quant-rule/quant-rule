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
| `object_store_access_key` | Exactly `qr_app`; the dedicated least-privilege bucket user. |
| `object_store_secret_key` | At least 16 characters, distinct from every role password. |
| `minio_root_user` | At least 8 characters, not a documented default; `minio` scope, MinIO hosts only. |
| `minio_root_password` | At least 16 characters, distinct, not a default; `minio` scope, MinIO hosts only. |
| `postgres_superuser_password` | At least 16 characters, URL-safe, not a default; used by Patroni and the cluster bootstrap. |
| `patroni_replication_password` | At least 16 characters, URL-safe, distinct; the streaming-replication role. |
| `patroni_rewind_password` | At least 16 characters, URL-safe, distinct; the pg_rewind role. |
| `patroni_rest_password` | At least 16 characters, URL-safe, distinct; the Patroni REST basic-auth password and HAProxy check header. |
| `rabbitmq_erlang_cookie` | At least 32 URL-safe characters, distinct; installed as `.erlang.cookie` on the three brokers only. |
| `pgbackrest_cipher_pass` | At least 16 characters, distinct; read from a mode-600 file for the AES-256 repository. Never in `pgbackrest.conf`, an argument, or a log. |
| `pgbackrest_s3_access_key` | The off-host S3 repository account; required only for an S3 `backup.repo`. |
| `pgbackrest_s3_secret_key` | The paired secret; at least 16 characters and distinct. |
| `object_archive_reader_secret_key` | The read-only `qr_archive_reader` secret; required when `backup.object_archive.enabled`, `minio` scope, MinIO hosts only. |

The five stateful secrets are rendered with `scope: stateful`, the two MinIO
root files and the optional read-only archive reader with `scope: minio`, and
the pgBackRest passphrase and S3 pair with `scope: backup` in
`secrets/manifest.json`; they are copied to the hosts by
`tools/stateful.sh` and are not app Swarm secrets. The stateful values are fake
and URL-safe so they can enter a Patroni, HAProxy, or RabbitMQ runtime
configuration without percent-encoding. The manifest carries a per-entry `hosts`
selector: the Postgres secrets go to the Patroni hosts (the REST password also to
the HAProxy pair), the Erlang cookie only to the brokers, the MinIO root files
only to the MinIO hosts, and the pgBackRest files to every Patroni host and the
backup host. The app `qr_app` pair is an app Swarm secret and is also copied to
the MinIO hosts only for the one-shot object bootstrap. The queue definitions
export is a separate credential-free file (`users`/`permissions` stripped, mode
600) and never contains these values.

The required names and the full file contract are in
[`docs/config/2026-11-22/deploy-v3.md`](../../../docs/config/2026-11-22/deploy-v3.md).
Never commit a real secret here.
