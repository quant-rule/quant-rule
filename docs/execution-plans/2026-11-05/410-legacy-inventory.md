# Legacy Inventory — S1.x and Pre-Family Artifacts

- Date: 2026-11-05 (UTC)
- Status: **Completed** — removed in [#463](https://github.com/quant-rule/quant-rule/issues/463)
  (2026-11-05); history records and shared engines retained
- Scope: old strategy versions v6–v10 (`S1.1–S1.5`), pre-family strategy
  configs, and old research pilots
- Plan: [`410-m2-model-programme.md`](410-m2-model-programme.md)
- Naming registry: [`docs/model/2026-11-05/model-families.md`](../../model/2026-11-05/model-families.md)
- Claim type: repository inventory; no behavior change

## 1. Decision rule

The human’s instruction is: keep the **history records**, erase the **legacy
code and docs**. This inventory applies that rule as follows.

| Class | Decision | Reason |
|---|---|---|
| AI transparency logs, improvement run records and gates, audits, execution plans, reports, releases | **Keep** | They are the historical record of what was tried and decided |
| `M1.0` model engine, model contracts, registry, data/model/backtest/analytics/shell engines | **Keep** | Current baseline and shared substrate for `S2.x` |
| Old strategy contract docs, strategy-specific configs, strategy-specific parsers, strategy-specific evaluation tools | **Remove** | Operational artifacts for retired `S1.x`; history remains in the records |
| Old research pilots whose only purpose was a failed pre-M2 experiment | **Remove after reference check** | Their committed improvement records remain |
| Accepted model-baseline configs and contracts | **Keep** | They are the evidence behind accepted `M1.0` baselines |
| Historical file names | **Keep** | The naming registry maps them; renaming old paths adds risk with no evidence value |

The removal was executed as a separate `chore/legacy-removal` change in
[#463](https://github.com/quant-rule/quant-rule/issues/463), after P7 closeout.
The reference check, tests, links, and terms checks passed.

## 2. Keep — history records

| Path | What it holds | Why keep |
|---|---|---|
| `docs/ai-transparency-log/**` | Material AI contributions, including `S1.x` work | History |
| `docs/improvement/**` | Run records, gates, candidates, baselines, invalidations | History |
| `docs/audits/**` | Read-only findings and remediation records | History |
| `docs/execution-plans/**` | Approved plans, including the `S1.x` plans | History |
| `docs/reports/**` | Delivered report contracts and evidence | History |
| `docs/releases/**` | Release notes and acceptance records | History |
| `data/improvement/**` | Local run artifacts and the generated index | History; local workspace |
| `data/labs/**` | Lab outputs | History; local workspace |
| `data/reports/**` | Report bundles | History; local workspace |
| `docs/registry/**` | Current indicator/event/parameter registry | Current contract |
| `docs/model/2026-11-03/model-v3.md` | Prior model contract | History |
| `docs/model/2026-11-05/model-m1.0.md` | `M1.0` current model contract | Current baseline |
| `docs/model/2026-11-04/compression-*.md` | Accepted model-baseline protocols | Current evidence |
| `docs/model/2026-11-05/**` | Indicator coverage, spec v1, model families | Current contract |
| `docs/governance/**`, `docs/workflows/**`, `docs/roles/**`, `docs/architecture/**`, `docs/adr/**` | Living process and architecture | Current |
| `docs/tutorials/**` | End-to-end procedures | Current |

## 3. Keep — engines and current configs

| Path | What it holds | Why keep |
|---|---|---|
| `crates/quantrule-data/**` | Data engine | Pipeline stage |
| `crates/quantrule-model/**` | `M1.0` model engine, readings, events, spec v1 | Baseline and M2 substrate |
| `crates/quantrule-backtest/**` | Strategy, execution, portfolio engines and runner | Shared substrate for `S2.x` |
| `crates/quantrule-analytics/**` | Research engine | Shared substrate |
| `crates/quantrule-shell/**` | Shell, config loaders, report commands | Product surface |
| `config/defaults/**` | Default model, policy, portfolio | Current |
| `config/examples/**` | Example run bundles | Current |
| `config/model-v4/**` | `M1.0` baseline matrices and model variants | Baseline evidence |
| `config/research/**` | Current research programs and reference manifest | Current |
| `config/compression-range/**`, `config/compression-range-5m/**`, `config/compression-range-onset-5m/**` | Compression-range model baselines | Accepted `M1.0` evidence |
| `config/compression/**` except `strategy-v9/**` | Compression-breakout model matrices | Accepted `M1.0` evidence |
| `crates/quantrule-backtest/src/strategy/bracket.rs` | Generic two-sided bracket engine | Reusable by `S2.x`; not a version parser |
| `crates/quantrule-backtest/src/strategy/config.rs` | Generic strategy config and engine | Reusable; remove only the v9/v10-specific constructors if no caller remains |
| `crates/quantrule-shell/src/config/strategy.rs` | Generic strategy schema dispatch | Keep until `S2.x` replaces it; remove the v9/v10 branches |
| `crates/quantrule-shell/src/command/strategy_explain.rs` | Strategy explanation output | Keep; update strings when v9/v10 branches go |
| `crates/quantrule-shell/src/command/policy.rs` | Policy output | Keep; update v9/v10 references if any remain |

## 4. Remove — `S1.x` strategy artifacts

### 4.1 `S1.1` — confirmed-level breakout (strategy v6)

| Path | Kind |
|---|---|
| `config/strategy-002/*-v6-*` | Strategy configs |
| `config/strategy-002/*-v3-*` | Pre-family strategy configs in the same directory |
| `docs/config/2026-11-01/strategy-v6.md` | Strategy contract doc |
| `docs/config/2026-11-01/strategy-candidates.md` | Strategy candidate index |

The directory `config/strategy-002/` can be removed whole if no non-strategy
file remains. The history records are
`docs/improvement/2026-11-01/STRATEGY-002-comparison.md` and the AI log.

### 4.2 `S1.2` — failed-level-test fade (strategy v7)

| Path | Kind |
|---|---|
| `config/strategy-003/**` | Strategy configs |
| `docs/config/2026-11-01/strategy-v7.md` | Strategy contract doc |

The history records are `docs/improvement/2026-11-01/STRATEGY-003-comparison.md`
and `docs/execution-plans/2026-11-01/level-fade-v7.md`.

### 4.3 `S1.3` — event composite (strategy v8)

| Path | Kind |
|---|---|
| `config/verification/**` | v8 and v8-DI configs |
| `docs/config/2026-11-03/strategy-v8.md` | Strategy contract doc |

The history records are the AI transparency log and the v8 execution plan. The
generic event-composite parser is shared with the current engine; keep it.

### 4.4 `S1.4` — 1m compression breakout (strategy v9)

| Path | Kind |
|---|---|
| `config/compression/strategy-v9/**` | Strategy configs and matrices |
| `docs/config/2026-11-04/strategy-v9-compression.md` | Strategy contract doc |
| `crates/quantrule-shell/src/config/strategy_v9.rs` | Strategy-specific parser |
| `crates/quantrule-shell/src/config/tests.rs` v9 block | Parser tests |
| `tools/improvement/build_strategy_v9_matrix.py` | Matrix builder |
| `tools/improvement/strategy_v9_evaluate.py` | Strategy evaluator |
| `tools/improvement/test_strategy_v9_evaluate.py` | Evaluator tests |

The history records are `docs/improvement/2026-11-04/STRATEGY-V9-GATE.md` and
the run record. Keep them.

### 4.5 `S1.5` — compression bracket family (strategy v10)

| Path | Kind |
|---|---|
| `config/compression-bracket/**` | Strategy configs and matrices |
| `config/compression-bracket-5m/**` | Strategy configs and matrices |
| `config/compression-bracket-onset-5m/**` | Strategy configs and matrices |
| `docs/config/2026-11-04/strategy-v10-compression-bracket.md` | Strategy contract doc |
| `docs/config/2026-11-04/strategy-v10-compression-bracket-5m.md` | Strategy contract doc |
| `docs/config/2026-11-04/strategy-v10-compression-bracket-onset-5m.md` | Strategy contract doc |
| `crates/quantrule-shell/src/config/strategy_v10.rs` | Strategy-specific parser |
| `crates/quantrule-shell/src/config/tests.rs` v10 block | Parser tests |

The history records are `STRATEGY-BRACKET-GATE.md`,
`STRATEGY-ONSET-5M-GATE.md`, and the run records. Keep them.

## 5. Remove — pre-family research pilots

These are not `S1.x` strategies, but they are retired research surfaces whose
only durable output is the committed improvement record. Remove after the
reference check in section 7.

| Path | Kind | Record kept |
|---|---|---|
| `config/model-gate/**` | v3/v4 model-gate configs | `docs/improvement/2026-11-03/MODEL-2026-11-03-*.md` |
| `config/path-pilot/**` | Path pilot configs | `docs/improvement/2026-11-03/PATH-PILOT-COPPER.md` |
| `config/prediction-pilot/**` | Prediction pilot configs | `docs/improvement/2026-11-03/PREDICTION-PILOT-RSI.md` |
| `config/macd-pilot/**` | MACD pilot configs | `docs/improvement/2026-11-03/MACD-PILOT-XAG.md` |
| `config/structure-pilot/**` | Structure pilot configs | `docs/improvement/2026-11-03/STRUCTURE-PILOT-SPX.md` |
| `config/session-spike/**` | Session spike configs | `docs/improvement/2026-11-04/SESSION-LIQUIDITY-SPIKE.md` |
| `config/improvement/2026-11-01/**` | v6 improvement-run configs | The committed run record |
| `tools/improvement/path_pilot.py` | Pilot harness | The committed pilot record |
| `tools/improvement/prediction_pilot.py` | Pilot harness | The committed pilot record; the P2 evaluator reuses the forward-outcome logic, so keep this file until P2 has its own implementation |
| `tools/improvement/macd_pilot.py` | Pilot harness | The committed pilot record |
| `tools/improvement/structure_pilot.py` | Pilot harness | The committed pilot record |
| `tools/improvement/session_spike.py`, `eurusd_session_spike.py` | Spike harnesses | The committed spike records |
| `tools/improvement/v5_phase2_spike.py` | Pre-M2 spike | The committed spike record |
| `tools/improvement/strategy_compare.py` | v3–v6 comparison tool | The committed comparison records |
| `tools/improvement/build_range_matrix.py`, `build_onset_range_matrix.py`, `build_eurusd_model_matrix.py` | Old matrix builders | The committed model records |

`prediction_pilot.py` is explicitly deferred: the P2 evaluator may reuse its
forward-return and control machinery. It is not removed until P2 has an
equivalent tested implementation.

## 6. Ambiguous — decide at removal time

| Path | Question | Default |
|---|---|---|
| `crates/quantrule-backtest/src/strategy/bracket.rs` | Does `S2.x` need a two-sided bracket? | Keep the generic engine; remove only the v10-specific constructor if unused |
| `crates/quantrule-backtest/src/strategy/config.rs` | Are the v9/v10 constructors used by anything else? | Keep `EventComposite`; remove `CompressionBreakout` and `CompressionBracket` constructors if no caller remains |
| `crates/quantrule-shell/src/config/strategy.rs` | Does the generic parser still serve `S2.x`? | Keep the parser; remove the `Schema::V9`/`Schema::V10` branches |
| `config/compression/**` (non-strategy-v9) | Is it an `M1.0` baseline? | Keep; it is accepted model evidence |
| `config/model-v4/**` | Is it the `M1.0` baseline? | Keep |
| `docs/config/2026-11-03/model-macd.md` | Is it a current model contract? | Keep; it documents a model reading, not an `S1.x` strategy |
| `docs/improvement/2026-11-04/STRATEGY-*.md` | Is it a history record? | Keep |
| `docs/reports/**` | Does it reference a removed config? | Keep the report; update the link if the config is removed |

## 7. Reference check before deletion

Run these from the repository root. A hit is a caller or a document that must be
updated before the path is removed.

```bash
# Strategy names and schemas.
grep -rn 'strategy-v6\|strategy v6\|quantrule.strategy/v6' \
  --exclude-dir=target --exclude-dir=.git .
grep -rn 'strategy-v7\|strategy v7\|quantrule.strategy/v7' \
  --exclude-dir=target --exclude-dir=.git .
grep -rn 'strategy-v8\|strategy v8\|quantrule.strategy/v8' \
  --exclude-dir=target --exclude-dir=.git .
grep -rn 'strategy-v9\|strategy v9\|quantrule.strategy/v9' \
  --exclude-dir=target --exclude-dir=.git .
grep -rn 'strategy-v10\|strategy v10\|quantrule.strategy/v10' \
  --exclude-dir=target --exclude-dir=.git .

# Config paths.
grep -rn 'config/strategy-002\|config/strategy-003\|config/verification\|config/compression/strategy-v9\|config/compression-bracket' \
  --exclude-dir=target --exclude-dir=.git .

# Code modules.
grep -rn 'strategy_v9\|strategy_v10\|CompressionBreakout\|CompressionBracket' \
  --exclude-dir=target --exclude-dir=.git crates tools

# Docs index and links.
python3 tools/check_links.py
python3 tools/check_terms.py
```

Then:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

The removal is not complete until the reference check is clean, the tests pass,
and the links and terms checks pass.

## 8. Removal sequence

1. **P0 (now):** inventory and naming registry. No deletion.
2. **P7a:** delete the `S1.1–S1.3` configs and docs; update `docs/README.md`;
   run the reference check.
3. **P7b:** delete the `S1.4–S1.5` configs, parsers, tests, and tools; keep the
   history records; run `cargo test --workspace`.
4. **P7c:** delete the pre-family pilots listed in section 5, except
   `prediction_pilot.py` until P2 is complete.
5. **P7d:** re-run the naming check, links, terms, fmt, clippy, and tests.
6. **P7e:** update the board and the naming registry status.

Each step is a focused commit. The removal never rewrites Git history and never
deletes a history record.

## 8.1 Execution record

Executed on 2026-11-05 in [#463](https://github.com/quant-rule/quant-rule/issues/463):
the `S1.1–S1.5` configs, contract docs, parsers, parser tests, and strategy
tools were removed; the pre-family pilot configs and tools were removed; the
history records and shared engines were kept. `cargo test`, `cargo clippy`,
`cargo fmt --check`, links, terms, and the registry check passed.

## 9. What must not happen

- Do not delete `docs/improvement/**`, `docs/audits/**`,
  `docs/execution-plans/**`, `docs/ai-transparency-log/**`, or
  `docs/reports/**`.
- Do not delete the `M1.0` model engine or its tests.
- Do not delete the shared data, backtest, analytics, or shell engines.
- Do not delete an accepted model-baseline config or contract.
- Do not rename a historical file to its `S1.x` alias; map it in the registry.
- Do not remove `prediction_pilot.py` before P2 has a tested replacement.
- Do not claim the removal is complete while a reference check fails.

## 10. Counts and scope

The five strategy config trees and three bracket trees contain 346 files in the
current worktree. The inventory groups them by directory; it does not list every
JSON file. The count is a measure of the surface, not a deletion target: the
removal is by directory and by reference, not by file count.

The `S1.x` history records are not part of that count. They stay.
