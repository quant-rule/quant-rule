# AI Transparency — Worker v5 Resolution

- Date: 2026-11-26 (UTC)
- Work item: `REM-RA-004` ([#1207](https://github.com/quant-rule/quant-rule/issues/1207))
- Tool: OpenCode coding agent; model identifier reported by the harness is
  `DeepSeek V4.1 Flash 1017`.
- Claim type: Behaviour Claim; no model, strategy, signal, or edge claim.

## 1. Intent and boundary

The worker must consume a v5 run without a legacy strategy file: resolve the
reference set once, verify the bytes, load the resolved `RunConfig`, and run the
fold. This change does not add the full v5 digest test (RA-005) or a composed
run.

## 2. Implemented

- `DataClient` gains `pack_bytes` and `signal_bytes`; HTTP and memory
  implementations carry them.
- The worker builds `MemoryReferences` from the data service bytes, verifies
  every hash, and synthesizes the model-bundle manifest from the materialized
  model files and their `provenance.json` ladder.
- A missing or mismatched reference fails closed; a transport failure retries.
- `Workspace` accepts a v5 bundle without a strategy file.
- `backtest_from_model` exposes `run_resolved`; `AppService` exposes the same
  seam. The legacy `run` path still loads v3/v4 files.
- The worker uses the resolved config for v5 and the file path for v3/v4.

## 3. Verification

Commands actually run in the RA-004 worktree:

- `cargo test -p quantrule-backtest-service` → 15 lib + 45 worker + 6 stage +
  2 identity tests passed.
- `cargo test -p quantrule-app -p quantrule-runtime` → 348 app and 141 runtime
  tests passed, plus integration targets.
- Resolver tests cover a valid pack, a missing pack and the model manifest
  artifact set.
- `qr-governance check --all` is run before the PR.

No composed or lab run was performed. The full v5 digest and no-partial-publish
claim are RA-005 evidence.

## 4. Learning

- The existing `DataClient` seam was the right place for pack and signal bytes;
  adding two methods avoided changing every `WorkerServices` construction.
- The v5 model reference needs a model-bundle manifest, but the stage artifact
  is a file set. The published `provenance.json` ladder plus the materialized
  files reconstruct the manifest deterministically.
- `Workspace` had an unconditional strategy-file check; v5 only needs it when
  `run.json` names a strategy file.
- The fold can take a resolved `RunConfig` without changing the per-bar stage
  loop; the legacy loader remains the v3/v4 path.

## 5. Next action

The full v5 end-to-end digest and no-partial-publish tests are RA-005. Push,
pull request, merge and release remain gated.
