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
| `queue_url` | `amqps://<user>:<password>@rabbitmq:5671/<vhost>` |
| `object_store_access_key` | At least 8 characters, not a documented default. |
| `object_store_secret_key` | At least 16 characters, distinct from every role password. |

The required names and the full file contract are in
[`docs/config/2026-11-22/deploy-v3.md`](../../../docs/config/2026-11-22/deploy-v3.md).
Never commit a real secret here.
