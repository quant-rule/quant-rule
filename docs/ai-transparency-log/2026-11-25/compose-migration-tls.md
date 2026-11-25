# AI Transparency Log — Migration-Only TLS Validation Fix

- Date: 2026-11-25 (UTC)
- Work item: live composed-tier verification; no issue number
- Tool: OpenCode, DeepSeek V4.1 Flash 1017

## Intent and boundary

The human asked for a live run on the laptop. `make up` reached the four
migration jobs and failed in `evidence-migrate` with
`QR_DATABASE_URL_FILE must use sslmode=verify-full when a TLS CA is configured`.
The migration service receives only `QR_DATABASE_OWNER_URL_FILE`; the runtime
validated the default application URL, which was absent, against the CA.

The fix validates the owner URL with the CA for a migration-only process and
keeps the application-URL validation for a long-running service. It does not
change the TLS policy, any signal computation, or a migration.

## Work performed

- Added `validate_database_named` and used it for the owner URL.
- Validated the application URL only when it came from a secret.
- Added a regression test for an owner-only migration configuration.
- Added the changelog bullet.

## Verification and evidence

- `cargo test -p quantrule-runtime`: 141 unit tests passed, plus the runtime
  integration suites.
- `cargo clippy -p quantrule-runtime --all-targets -- -D warnings`: pass.
- `cargo doc -p quantrule-runtime --no-deps` with `-D warnings`: pass.
- `make up` before the fix: `evidence-migrate` exit 1 with the TLS error; the
  four databases and roles were created successfully.
- The live stack rerun after the fix is the remaining evidence; it is not
  claimed here.

## Learning

- The migration job correctly has no application URL; validating the default
  application URL is the defect.
- A live `make up` is a stronger contract check than the fast gate for the
  Compose secret/TLS wiring.

## Residual risk

- The fix is small and covered by a regression test, but the full live
  composed-tier digest still has to be rerun on the rebuilt image.
