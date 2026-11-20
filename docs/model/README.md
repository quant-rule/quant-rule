# Model Index

- Status: Current
- Registry: [`docs/registry/README.md`](../registry/README.md)
- Families: [`docs/model/2026-11-05/model-families.md`](2026-11-05/model-families.md)

This folder holds the analytical model contract and the evidence that the
implementation follows it. The registry is the source of truth for indicator
and event counts; the verification matrices are generated from it.

| Contract | Status | Path |
|---|---|---|
| M1.0 flat model | Historical baseline; path gate no edge; compression baselines accepted | `docs/model/2026-11-05/model-m1.0.md` |
| M1.1 opening-range sweep | Historical model; adds the opening-range sweep and control events; MR7 no edge | `docs/model/2026-11-08/model-m1.1.md` |
| M1.2 structure extensions | Historical model; pivot sequence, Fibonacci retracements, and horizontal bands; corrected by M1.3 | `docs/model/2026-11-10/model-m1.2.md` |
| M1.3 audit corrections | Current model; corrected sequence occurrence, Fibonacci tests, and fixed tolerances; predictive evidence pending | `docs/model/2026-11-10/model-m1.3.md` |
| Indicator coverage | Implemented 2026-11-05; updated 2026-11-10: 49 indicators, 150 active events, stable event codes; 199 validation claims; predictive evidence pending | `docs/model/2026-11-05/indicator-coverage.md` |
| Model families | Approved 2026-11-05; amended 2026-11-10; `M1.3` current, `M1.0`–`M1.2` historical, `M2.x`–`M4.x` historical, `MR5.x`–`MR7.x` research, and the future `S3.x` map | `docs/model/2026-11-05/model-families.md` |
| Model spec v1 | Implemented; condition-only; includes the `rvol_regime`, `vol_regime`, and `structure_state` states | `docs/model/2026-11-05/model-spec-v1.md` |
| `rvol_regime` context state | Current; low below 1.4, normal 1.4–2.0, high at or above 2.0; no event changes | `docs/model/2026-11-08/rvol-regime.md` |
| `vol_regime` context state | Current; compressed at or below 0.7, normal below 1.3, expanded at or above 1.3; no event changes | `docs/model/2026-11-09/vol-regime.md` |
| `structure_state` context state | Current; confirmed major HH/HL, LH/LL, or mixed; no event changes | `docs/model/2026-11-09/structure-state.md` |
| `signal.breakout_retest_hold` | Current; compression breakout plus a 4-bar retest within 0.2 ATR that closes back on the breakout side | `docs/model/2026-11-09/breakout-retest-hold.md` |
| Model spec v2 | Implemented; reading overrides, active events, causal vote, panel/session | `docs/model/2026-11-05/model-spec-v2.md` |
| Indicator and parameter registry | Current; CI check | `docs/registry/README.md` |
| Current-architecture history index | Current; retired paths and their recovery commit | `docs/registry/history.md` |
| M1.0 model lab tutorial | Historical; the v0.3.0 local stage commands | `docs/tutorials/2026-11-06/m1.0-model-lab.md` |
| Workflow lab tutorial | Historical; v0.3.0 workflow selection | `docs/tutorials/2026-11-07/workflow-lab.md` |
| Strategy lab tutorial | Historical; v0.3.0 strategy stage | `docs/tutorials/2026-11-06/strategy-lab.md` |
| Execution lab tutorial | Historical; v0.3.0 execution stage | `docs/tutorials/2026-11-06/execution-lab.md` |
| Portfolio lab tutorial | Historical; v0.3.0 portfolio stage | `docs/tutorials/2026-11-06/portfolio-lab.md` |
| Backtest lab tutorial | Historical; the v0.3.0 coupled run | `docs/tutorials/2026-11-06/backtest-lab.md` |
| Qualification lab tutorial | Historical; v0.3.0 qualification | `docs/tutorials/2026-11-06/qualification-lab.md` |
| Analytics lab tutorial | Historical; v0.3.0 analytics | `docs/tutorials/2026-11-06/analytics-lab.md` |
| Pipeline lab tutorial | Historical; the v0.3.0 stage surface | `docs/tutorials/2026-11-06/pipeline-lab.md` |
| Shell HTTP tutorial | Current; the 46-command gateway client, profiles, runs, model, strategy, portfolio, research, and simulation verbs | `docs/tutorials/2026-11-14/shell-http.md` |
| Run a backtest tutorial | Current; the distributed stack through the browser and `make` | `docs/tutorials/2026-11-12/01-run-a-backtest.md` |
| Verify determinism tutorial | Current; the canonical digest and a repeat run | `docs/tutorials/2026-11-12/02-verify-determinism.md` |
| Remote compute tutorial | Current; a second host and the gateway URL | `docs/tutorials/2026-11-12/03-remote-compute.md` |
| Reviewer verification tutorial | Current; reproduce the data plane and compare the digest | `docs/tutorials/2026-11-15/reviewer-verification.md` |
| End-to-end verification tutorial | Current; `make up`, `make verify`, the expected digest, and the composed tier | `docs/tutorials/2026-11-17/end-to-end-verification.md` |
| Stage to backtest tutorial | Current; model, strategy, and portfolio stages to a promoted coupled run | `docs/tutorials/2026-11-17/stage-to-backtest.md` |
| Indicator verification matrix | Current; 49 rows, generated from the evidence map | `docs/model/2026-11-06/indicator-verification.md` |
| Event verification matrix | Current; 150 rows, generated from the evidence map | `docs/model/2026-11-06/event-verification.md` |
| Verification evidence map | Current; row/family/tool/n-a scope per class | `docs/registry/verification.json` |
