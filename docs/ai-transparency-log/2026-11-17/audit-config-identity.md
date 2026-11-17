# AI transparency log — Config identity and metrics digest (P2.8a, AUD-21/22)

- Date: 2026-11-17 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: [Audit Workflow](../../workflows/audit-workflow.md) stage 3
  (Execution) for `AUD-2026-11-17-21` and `AUD-2026-11-17-22`.
- Claim type: Behaviour Claim.
- Intent: unify the configuration identity on one SHA-256 and include the
  compared `metrics.json` in the canonical digest. This is the first half of
  P2.8; the model-input promotion hash is the next branch.
- Boundary: branch `fix/audit-config-identity`, based on current `main`
  `7ae3a984`. No push, PR, merge, or board change. The Quant Validator pass is
  sequential by the same agent, not independent.

## What changed

- `RunConfig::config_hash` is now the SHA-256 of the run, data, model,
  execution, portfolio, and optional strategy file bytes in that order, each
  followed by a NUL. This is the data service's canonicalization, so the app,
  catalog, run record, and stage provenance name the same identity.
- `quantrule-app` depends on `quantrule-api` for the shared `Digest` type; the
  shell architecture guard allows that contract edge.
- `canonical_digest` includes every `metrics.json` (for example
  `portfolio/metrics.json`) in addition to `results.json`, `*.csv`, and
  `*.ndjson`.
- Tests: the data-service scan compares the app's `RunConfig::config_hash` to
  the catalog hash for the same fixture and proves a strategy change changes
  it; the app checks the `sha256:` shape; the service API proves a metrics
  change changes the digest; the coupled worker oracle is updated.

## Verification observed

| Command | Result |
|---|---|
| `cargo test -p quantrule-app --lib config::tests` | 27 passed, 0 failed |
| `cargo test -p quantrule-data-service --lib configs::scan` | 5 passed, 0 failed |
| `cargo test -p quantrule-service-api --lib manifest` | 15 passed, 0 failed |
| `cargo test -p quantrule-backtest-service --test worker` | 38 passed, 0 failed; new coupled digest `sha256:2d98b8ea…` |
| `cargo test -p quantrule-shell --tests` | all suites pass after the contract-edge guard update |
| `cargo test -p quantrule-evidence-service --tests` | all suites pass |
| `cargo fmt --all -- --check` | pass |
| `tools/gate.sh` | pass: 2,324 passed, 0 failed, 16 ignored; 25 checkers |

## Quant validation (sequential, not independent)

- **Expected invariant:** the run record's `config_hash` and the artifact
  provenance name the same configuration (`docs/workflows/run-contract.md:95`);
  the canonical digest covers the values a research comparison reads
  (`docs/adr/2026-11-12/07-canonical-digest-and-claim-boundary.md:20`).
- **Observed before:** the app used a 64-bit FNV fingerprint, the catalog used
  SHA-256, and `metrics.json` was outside the canonical set.
- **Observed after:** the scan test asserts the app and catalog hashes are equal
  for the same files, and a strategy change moves the hash. The service API
  test asserts a changed `portfolio/metrics.json` changes the canonical digest
  while input order does not.
- **Digest impact:** the coupled worker baseline changes from
  `sha256:c23925d4…` to `sha256:2d98b8ea…`; the new value was stable across two
  runs. The change is expected: the artifact provenance now names the SHA-256
  identity and `portfolio/metrics.json` is covered.
- **Residual risk:** a canonical digest recorded before this change no longer
  matches a recomputation. The accepted baselines that compare canonical
  digests must be re-recorded under the new boundary; no baseline is silently
  treated as equal.
- **Finding 23:** promotion still compares the full config hash, so a
  strategy-only change cannot reuse a model until the next branch adds the
  model-input hash.
- **Independence:** the Quant Researcher/Developer/Validator roles were
  performed sequentially by the same agent. This is not an independent
  validation and is not claimed as one.

## Learning

- The data service already had the canonical hash; the app was the outlier. A
  cross-crate test that compares the two values is the guard that would have
  caught the drift.
- Adding `metrics.json` to the claim boundary is a behavior change for every
  stored digest, not just a reporting detail. The coupled oracle and the
  baseline records must be updated deliberately.
- Using the contract `Digest` type avoids a second SHA-256 implementation; the
  architecture guard now records the app's allowed contract edge.
