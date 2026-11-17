# AI transparency log — Data-v3 admission and merged event retention (P2.11, AUD-52/53)

- Date: 2026-11-17 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: [Audit Workflow](../../workflows/audit-workflow.md) stage 3
  (Execution) for `AUD-2026-11-17-52` and `AUD-2026-11-17-53`.
- Claim type: Behaviour Claim.
- Intent: make the data-v3 document describe the loader it actually has, and
  make the model's full event batch and bounded history carry the extended
  events the retention contract promises.
- Boundary: branch `fix/audit-data-events`, based on current `main` `7ae3a984`.
  No push, PR, merge, or board change. The Quant Validator pass is sequential by
  the same agent, not independent.

## What changed

- `docs/config/2026-10-30/data-v3.md` no longer requires the instrument or the
  execution fill timeframe to be in the retired policy universe. It states the
  loader's actual checks: non-empty instrument, ordered/1m-derived ladder, and
  the run loader's fill-timeframe check.
- `TimeframeModel` builds the `EventBatch` and `EventHistory` from the merged
  core and extended event stream. The `all_events()` view and the full batch
  now share one ordered `EventBatch`; the base `events()` accessor still names
  the v4 core batch for the shell's separate event CSV.
- `event_retention.rs` asserts the full batch length equals the core plus
  extended counts, that the fixture produces an extended event, and that the
  bounded history is the newest records of that full batch.

## Verification observed

| Command | Result |
|---|---|
| `cargo test -p quantrule-model --test event_retention` | 1 passed, 0 failed |
| `cargo test -p quantrule-model --lib` | 468 passed, 0 failed |
| `cargo fmt --all -- --check` | pass |
| `tools/gate.sh` | pass: 2,321 passed, 0 failed, 16 ignored; 25 checkers |

## Quant validation (sequential, not independent)

- **Expected invariant:** the retention contract says the batch contains every
  event and the history is its newest records (`retention.rs:1-8`); the data-v3
  document describes the loader's admission checks (`data-v3.md:43,57-59`).
- **Observed before:** the batch/history were built from the core `events` batch
  only; the extended events existed only in `merged_events`/`all_events()`. The
  data-v3 document required a policy universe the loader never consulted.
- **Observed after:** the full batch is the merged stream; the history is the
  newest 1,024 of it; the retention test proves the extended source contributes.
  The data document names the actual checks.
- **Model values:** no reading or event definition changed. The full batch and
  history expose records that were already published through `all_events()` and
  the shell's separate event CSV; the base `events()` accessor is unchanged.
- **Residual risk:** a consumer of `event_batch()` that assumed the core-only
  batch now sees extended records; the method's contract says "full event
  batch", so this is the correction. The shell's combined CSV count still uses
  the base and extended batches separately and does not double-count.
- **Independence:** the Quant Researcher/Developer/Validator roles were
  performed sequentially by the same agent. This is not an independent
  validation and is not claimed as one.

## Learning

- The retention unit tests proved the container but not the model's feed; the
  integration test now checks the actual source composition.
- A document that describes a retired policy is worse than a missing sentence:
  it makes a valid config look invalid. The loader's real checks are now the
  documented ones.
