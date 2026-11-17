# AI transparency log — Model-input promotion identity (P2.8b, AUD-23)

- Date: 2026-11-17 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: [Audit Workflow](../../workflows/audit-workflow.md) stage 3
  (Execution) for `AUD-2026-11-17-23`.
- Claim type: Behaviour Claim.
- Intent: let `run submit --model-artifact` reuse a frozen model stage when only
  the strategy changes, while refusing a stage whose model input differs. The
  full config hash covers the strategy, so it cannot be the reuse identity.
- Boundary: branch `fix/audit-model-promotion`, based on `fix/audit-config-identity`
  (`4e21a87b`). No push, PR, merge, or board change. The Quant Validator pass is
  sequential by the same agent, not independent.

## What changed

- `RunConfig` carries a `model_input_hash`: the data version label, the model
  config bytes, and the effective model ladder. The full `config_hash` still
  covers every config file.
- `quantrule_api::model_input_hash` is the one canonical function; the app and
  `ConfigBundle::model_input_hash` use it, so the worker and the evidence
  submission compute the same value from the same files.
- `model_bundle_from` writes the model-input hash into the model bundle
  provenance. The model bundle's artifact bytes are therefore invariant under a
  strategy-only change; its manifest still carries the full config hash for the
  stage identity.
- `run submit --model-artifact` no longer compares the stage's full
  `config_hash`; it still checks `data_version` and `model_version`. The worker
  reads the frozen model bundle's `provenance.json`, compares its model-input
  hash to the submitted config, and refuses a changed model input before reuse.
- Tests: the service-API bundle test proves the hash ignores a strategy change
  and moves with a model change; the data-service scan test proves the same
  through `RunConfig`; the worker promotion suite adds a strategy-only reuse
  test and a changed-model refusal; the evidence promoted-run test records the
  relaxed submission check.

## Verification observed

| Command | Result |
|---|---|
| `cargo test -p quantrule-api --lib digest` | 12 passed, 0 failed |
| `cargo test -p quantrule-service-api --lib storage` | 6 passed, 0 failed |
| `cargo test -p quantrule-app --lib` | 165 passed, 0 failed |
| `cargo test -p quantrule-model-service --tests` | all suites pass |
| `cargo test -p quantrule-backtest-service --test worker promotion::` | 7 passed, 0 failed |
| `cargo test -p quantrule-backtest-service --test worker` | 40 passed, 0 failed |
| `cargo test -p quantrule-evidence-service --test promoted_runs` | 3 passed, 0 failed |
| `cargo test -p quantrule-shell --tests` | all suites pass |
| `cargo fmt --all -- --check` | pass |
| `tools/gate.sh` | pass: 2,328 passed, 0 failed, 16 ignored; 25 checkers |

## Quant validation (sequential, not independent)

- **Expected invariant:** promotion reuses a frozen model artifact byte-for-byte
  and recomputes strategy, execution, and portfolio
  (`docs/workflows/run-contract.md:111`). A strategy-only change keeps the model
  input; a model config, data label, or ladder change does not.
- **Observed before:** `load_promoted_model` required the stage and run full
  `config_hash` to be equal, and the model bundle provenance carried that full
  hash, so a strategy change both failed the check and changed the model bytes.
- **Observed after:** the model bundle provenance names the model-input hash;
  the strategy-only worker test finds the original artifact, calls the model
  service zero times, reuses every model artifact, and produces the coupled
  baseline of the changed strategy config. The changed-model test fails before
  compute with "another model input".
- **Submission boundary:** the evidence route accepts a model stage from another
  full config hash because it cannot see the frozen model's inputs; the worker
  is the enforcement point and refuses a changed model input. The route still
  rejects a changed dataset or model version.
- **Model values:** no reading, event, execution, or portfolio value changed.
  The model bundle provenance identity changed, and `provenance.json` is
  excluded from the canonical digest, so the coupled digest remains stable.
- **Residual risk:** a stage created before this change has a full config hash
  in its model provenance and will be refused for promotion until it is
  re-created; that is the safe failure. The submission route alone is now
  permissive about the full config hash; a bad artifact is refused by the
  worker, not at submission.
- **Independence:** the Quant Researcher/Developer/Validator roles were
  performed sequentially by the same agent. This is not an independent
  validation and is not claimed as one.

## Learning

- The full config hash inside the model bundle provenance was the hidden
  coupling: even a correct promotion check would have produced different model
  bytes for a strategy cell. Naming the model input in provenance fixes both
  the check and the bytes.
- The model-input identity belongs in the frozen artifact, not only in the
  stage row; the artifact is what the promotion reuses, and its provenance is
  already content-addressed.
- The evidence route cannot verify the model input without reading the model
  artifact; keeping that verification in the worker puts the check next to the
  bytes and the compute.
