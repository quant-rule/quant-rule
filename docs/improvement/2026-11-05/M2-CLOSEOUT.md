# M2.x Closeout

- Date: 2026-11-05 (UTC)
- Status: **Records complete**; `no_edge` recorded; checklist signed; legacy
  removal delivered in [#463](https://github.com/quant-rule/quant-rule/issues/463);
  P5-F1 remains open
- Plan: [`docs/execution-plans/2026-11-05/410-m2-model-programme.md`](../../execution-plans/2026-11-05/410-m2-model-programme.md)
- Charter: [`docs/governance/2026-11-05/M2-CHARTER.md`](../../governance/2026-11-05/M2-CHARTER.md)
- Development result: [`M2-DEV-SWARM.md`](M2-DEV-SWARM.md)
- Audit: [`INDICATOR-AUDIT.md`](INDICATOR-AUDIT.md)
- Checklist: [`INDICATOR-VERIFICATION.md`](INDICATOR-VERIFICATION.md)
- Legacy inventory: [`410-legacy-inventory.md`](../../execution-plans/2026-11-05/410-legacy-inventory.md)
- Claim type: programme closeout record; no edge claim

## 1. Outcome

The M2.x programme stopped at the development gate. `PROGRAM-2026-11-05-03`
reported all 15 cells; no primary cell passed, so there is no candidate, no
accepted M2.x baseline, and `model_validation` was not opened. `S2.x` remains
closed.

The development result, hashes, gate detail, and finding P5-F1 are in
[`M2-DEV-SWARM.md`](M2-DEV-SWARM.md).

## 2. Records updated

| Record | Change |
|---|---|
| [`docs/improvement/baselines.md`](../baselines.md) | Added `PROGRAM-2026-11-05-03` as **not accepted — no edge**; no baseline changed |
| [`docs/improvement/index.md`](../index.md) | Added the M2.x run row and program-status row |
| [`docs/model/2026-11-05/model-families.md`](../../model/2026-11-05/model-families.md) | `M2.1`–`M2.5` marked historical, `model_dev` no edge, not accepted; `S2.x` remains closed |
| [`docs/registry/indicators.json`](../../registry/indicators.json) | F-04: `volume_oscillator` removed from `rvol.outputs`; indicator 15 is the single owner |
| [`docs/improvement/2026-11-05/INDICATOR-AUDIT.md`](INDICATOR-AUDIT.md) | F-04, F-07, and F-09 moved to **Fixed** with evidence |
| [`docs/README.md`](../../README.md) | M2.x programme and charter rows updated; M2.x evidence row added |

## 3. Audit closure

| Finding | Status | Evidence |
|---|---|---|
| F-01 retrospective audit | Fixed | Audit document delivered |
| F-02 `Sign` column | Fixed | Human signed 37/37 rows on 2026-11-05 |
| F-03 v1 spec limits | Fixed | `model-spec-v2.md` |
| F-04 `volume_oscillator` double owner | Fixed | `rvol.outputs` no longer lists it; `check_registry.py` passes |
| F-05 absorption formula | Fixed | Coverage document matches the registry |
| F-06 panel/session wiring | Fixed | v2 panel/session tests |
| F-07 event-reference overcount | Fixed | `coverage.py` dedupes by event ID; 46 indicators, 143 events |
| F-08 context events as votes | Fixed | v2 role-rejection test |
| F-09 profile concentration sparse | Fixed | `EXPL-02` reproduction: 13,541 events; metrics match P5 |
| F-10 audit scope | Triaged | Ten of 46; the remaining 36 rest on the checklist and goldens |

The F-09 reproduction used the committed `EXPL-02` spec and yearly run configs
on `model_dev`; its pooled metrics matched the preserved P5 result exactly.

## 4. Checklist signature

Table B of [`INDICATOR-VERIFICATION.md`](INDICATOR-VERIFICATION.md) has all
`F/C/D/G/E/A/N/P/M` cells ticked for 37 rows. The human signed rows 1–37 on
2026-11-05; the agent recorded the signature and did not supply it. Every row is
now `verified`, including row 37 (`calendar_bucket`), which is reading-only and
has no event to activate in a spec. F-02 is **Fixed**.

## 5. Legacy removal

The legacy inventory is **Follow-up approved**. The removal is tracked by
[#463](https://github.com/quant-rule/quant-rule/issues/463) as a separate
`chore/legacy-removal` change after P7 closeout. This closeout does not delete
any legacy path.

Candidate list from the inventory:

| Class | Scope | Size |
|---|---|---:|
| `S1.1`–`S1.3` configs | `config/strategy-002/**`, `config/strategy-003/**`, `config/verification/**` | 89 files |
| `S1.4`–`S1.5` configs | `config/compression/strategy-v9/**`, `config/compression-bracket/**`, `config/compression-bracket-5m/**`, `config/compression-bracket-onset-5m/**` | 164 files |
| Pre-family pilots | `config/model-gate/**`, `config/path-pilot/**`, `config/prediction-pilot/**`, `config/macd-pilot/**`, `config/structure-pilot/**`, `config/session-spike/**`, `config/improvement/2026-11-01/**` | 173 files |
| Strategy contract docs | Eight files under `docs/config/2026-11-01`, `2026-11-03`, and `2026-11-04` | 8 files |
| Strategy-specific code | `strategy_v9.rs`, `strategy_v10.rs`, their test blocks, and the v9/v10 branches in the shell and backtest strategy dispatch | 10 Rust files |
| Strategy-specific tools | `build_strategy_v9_matrix.py`, `strategy_v9_evaluate.py`, `test_strategy_v9_evaluate.py`, and the pilot/matrix harnesses | 14 files |

The reference check found:

- History records under `docs/improvement/**`, `docs/ai-transparency-log/**`,
  `docs/execution-plans/**`, `docs/audits/**`, `docs/reports/**`, and
  `docs/releases/**` mention the old names. Those records stay; a link check
  must distinguish history links from live callers.
- `docs/README.md` currently links the `S1.4` and `S1.5` contract docs; those
  index rows must be updated when the docs are removed.
- The v9/v10 dispatch is reachable from `strategy.rs`, `strategy_explain.rs`,
  `policy.rs`, the backtest `StrategyMode`, the ladder, and the runner. Removing
  the parsers without removing those branches will not compile.
- The generic bracket engine is reused by `S2.x`; the v9/v10 constructors are
  not.

Route: [#463](https://github.com/quant-rule/quant-rule/issues/463) carries the
removal with its own plan and reference check. P7 closes the records, audit, and
checklist; #410 closes after #430. The removal is a separate follow-up, not a
condition of the `no_edge` closeout.

## 6. Follow-ups and closure

| Finding | Impact | Status |
|---|---|---|
| P5-F1 | Runtime emission is 101/106 codes and 138/143 event IDs; two compact-state events have no emitter and three compression events are disabled by default | Open; needs an audit/remediation route |
| Legacy inventory | Removed in [#463](https://github.com/quant-rule/quant-rule/issues/463); history records and shared engines kept | Closed |

## 7. Board and next gates

| Item | State |
|---|---|
| #428 P5 development swarm | Closed; board Done |
| #429 P6 validation | Closed as not applicable; no candidate |
| #430 P7 closeout | Merged in #464 |
| #410 M2.x programme | Closed and Done |
| #405 indicator platform | Closed and Done |
| #463 legacy removal | Delivered in #463 |

The next gate is the #463 pull request. P5-F1 remains the only open finding.
