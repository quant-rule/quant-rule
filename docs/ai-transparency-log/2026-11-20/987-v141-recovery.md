# AI transparency log — v1.4.1 release recovery

- Date: 2026-11-20 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: Release & Operations recovery for `v1.4.0`; issue
  [#987](https://github.com/quant-rule/quant-rule/issues/987).
- Claim type: Behaviour Claim.
- Intent: publish a corrected patch release after the `v1.4.0` release workflow
  failed, without touching the published `v1.4.0` tag.
- Boundary: branch `fix/987-v141-recovery`, based on `main` `2b1f8abf`; no HA
  work, no command or screen contract change, and no release tag created in this
  work item.

## What changed

- `crates/quantrule-simulation-service/src/postgres.rs`: the four summary
  projections format `archived_at` as RFC3339 text, and `list` filters
  `archived_at IS NULL` in both branches, matching the SQLite store.
- `apps/desktop-shell/ui/shell-lib.js`: `SHELL_VERSION` is `1.4.1`.
- `apps/desktop-shell/tests/shell.test.js`: the boundary test checks the version
  shape and compares the manifests, so the next bump does not need a literal.
- Version `1.4.1` across the workspace, web, desktop, and supervisor manifests;
  the model-object oracle was regenerated because the object carries the package
  version.
- `CHANGELOG.md`, the `v1.4.1` release note, the withdrawn `v1.4.0` note, the
  release map, README, and the current-version references.

## Verification observed

| Command | Result |
|---|---|
| `cargo test -p quantrule-simulation-service` | all suites passed |
| `cargo test -p quantrule-model-service --test model` | 7 passed |
| `apps/desktop-shell` `bun test` | 35 passed, 0 failed |
| `tools/test_tiers.sh composed` | 8 targets passed; simulation conformance passed; digest `sha256:a0372f02…` |
| `tools/gate.sh` | `workspace gate passed` |
| `make up` | `QuantRule v1.4.1 ready → http://localhost:8080` |
| Live archive smoke over the gateway | `archived_at` RFC3339; active list hides the run; `include_archived` shows it; erase removes it |

## Learning

- The SQLite conformance suite is the right guard for store parity: it failed
  on the unfiltered list before the fix and passed after it.
- The desktop boundary test now derives its expectation from the manifests; a
  hardcoded version literal turns every release bump into a false failure.
- The `v1.4.0` tag stays published. The correction is a new patch version, not a
  re-run of the failed workflow.
