# AI transparency log — Config reference confinement (AUD-10)

- Date: 2026-11-17 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: [Audit Workflow](../../workflows/audit-workflow.md) execution for
  `AUD-2026-11-17-10`.
- Claim type: Behaviour Claim.
- Intent: refuse a `run.json` reference that is absolute or contains `..`
  before the data service reads it, so an uploaded bundle cannot escape its
  staging root.
- Boundary: branch `fix/audit-config-path`, based on `main` `fb5b66b7`.

## What changed

- `RunConfig::load` resolves `data`, `model`, `execution`, `portfolio`, and
  `strategy` references through `referenced_path`, which accepts only normal
  components (and `.`). Absolute paths and `..` are a `ConfigError` at
  `$.data` and peers.
- The uploaded-bundle route now rejects the escape while cataloguing the
  config; the worker's existing key check remains as a second boundary.
- Tests: the app loader test mutates the run fixture's `data` to
  `../outside.json`; the data upload test proves the catalog record is invalid
  with a relative-path message.

## Verification observed

| Command | Result |
|---|---|
| `cargo test -p quantrule-app --lib config::tests` | 31 passed, 0 failed |
| `cargo test -p quantrule-data-service --lib configs::upload` | 7 passed, 0 failed |
| `cargo fmt --all -- --check` | pass |
| `tools/gate.sh` | CI `verify` on the PR |

## Quant validation

Not required: this is a path-confinement boundary, not a model or strategy
value change.

## Learning

- The upload validator checked the file-map keys but not the references inside
  `run.json`; the shared loader was the right place for the second check
  because both the upload and the repository mount use it.
- A trusted repository mount still uses the same loader; a `..` reference in a
  committed config is now a config error rather than a quiet read outside the
  directory.
