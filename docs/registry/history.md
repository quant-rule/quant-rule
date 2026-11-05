# Current-Architecture History Index

- Date: 2026-11-05 (UTC)
- Status: Current boundary; P0.1–P2 merged; P3 four-axis removal in progress
- Freeze commit: `32e6fb2`
- Manifest commit: `8acf857`
- Work item: [#468](https://github.com/quant-rule/quant-rule/issues/468)
- Deletion manifest: [`deletion-manifest.md`](deletion-manifest.md)
- Purpose: the single index of artifacts removed from the current surface and
  the history that remains

## 1. Boundary

The working product keeps one current architecture: the flat per-timeframe
model, the model-spec/config surface, the registry, and the surviving shell
commands. Historical evidence is kept in ADRs, audits, improvement records,
model/strategy/portfolio/policy run records, AI transparency logs, execution
plans, releases, and reports.

This file records what is removed from the current surface and where its
evidence remains. It is not a current architecture contract.

## 2. Keep as the current surface

| Path | Why |
|---|---|
| `crates/quantrule-data/**` | Data engine |
| `crates/quantrule-model/src/readings/**` | Indicator readings |
| `crates/quantrule-model/src/events/**` | Level, divergence, and signal events |
| `crates/quantrule-model/src/panel.rs`, `sessions.rs` | Panel and session inputs |
| `crates/quantrule-model/src/spec/**` | Model-spec evaluation |
| `crates/quantrule-model/src/timeframe.rs`, `run.rs`, `input.rs`, `kernel.rs` | Flat per-timeframe model and run |
| `crates/quantrule-backtest/src/{execution,portfolio}/**` | Execution and portfolio engines |
| `crates/quantrule-shell/src/{config,command,source}` | Current shell surface |
| `docs/registry/**` | Registry, validation, history |
| `docs/model/2026-11-05/**` | Current model family and spec contracts |
| `docs/architecture/**` | Current architecture |
| `docs/workflows/**`, `docs/governance/**` | Current operating contracts |

`docs/model/2026-11-05/model-m1.0.md` is the `M1.0` contract after the P2 rename.

## 3. Delete from the current surface

### 3.1 Old model modules

| Path | Why |
|---|---|
| `crates/quantrule-model/src/axes/direction.rs` | Retired Direction axis |
| `crates/quantrule-model/src/axes/momentum.rs` | Retired Momentum axis |
| `crates/quantrule-model/src/axes/structure/**` | Retired Structure score |
| `crates/quantrule-model/src/axes/mod.rs` | Axis wrapper |
| `crates/quantrule-model/src/dashboard/**` | Retired four-axis dashboard |
| `crates/quantrule-model/src/report/assemble.rs`, `report/chart.rs` | Retired axis report |
| `crates/quantrule-model/src/state/**` | Retired MarketState view |
| `crates/quantrule-model/src/consensus.rs` | Retired cross-axis consensus |
| `crates/quantrule-model/src/aggregate/**` | Retired cross-timeframe reporting aggregates |

Keep `axes/progression/**` only if it is moved to a flat `progression` module and
keeps the registered progression indicator. Keep `context.rs` as a flat reading.
Keep `report/flat.rs` only as the flat projection, not the old axis report.

### 3.2 Old shell commands

Delete: `market-state`, `market-dashboard`, `market-report`, `axis`, `legs`,
`model-series`, old store commands, old research-export commands, old market
chart command, old strategy/backtest commands that depend on the retired
strategy schemas.

Keep/rewrite: `report model`, `events`, data/fetch commands, `policy check`,
`help`.

### 3.3 Old config parsers and configs

The `quantrule.model/v1`–`/v3` parsers and the strategy parser surface were
removed. `quantrule.model/v4` is the current flat model schema.

`config/defaults/**` and `config/examples/**` are the current config set. The
example strategy/analytics files and `config/v5-swing/**` were deleted in P4.

These directories are kept as evidence, not current run configs:

| Path | Why |
|---|---|
| `config/model-v4/**` | Accepted M1.0 baseline |
| `config/compression/**` | Compression-breakout evidence |
| `config/compression-range/**` | Accepted compression-range baseline |
| `config/compression-range-5m/**` | Accepted 5m compression-range evidence |
| `config/compression-range-onset/**` | Accepted fresh-onset evidence |
| `config/compression-range-onset-5m/**` | Accepted 5m onset evidence |
| `config/research/**` | Historical M2.x programme evidence |

The evidence directories may reference retired strategy/analytics paths. They are
not expected to load through the current `run.json` loader. See
`config/README.md`.

### 3.4 Old tests and fixtures

The axis, dashboard, market-report, consensus, and aggregate goldens and tests
were removed with their modules. The flat readings, events, levels, panel,
session, spec, prediction, and edge-case tests remain; the exact list is in the
execution plan.

### 3.5 Old current-architecture docs

Delete old model contracts, old dashboard/market-report contracts, old model
config contracts, old tutorials, and old strategy config contracts:

- `docs/model/2026-10-21/**`
- `docs/model/2026-10-25/**`
- `docs/model/2026-10-27/**`
- `docs/model/2026-10-30/**`
- `docs/model/2026-11-01/**`
- `docs/model/2026-11-03/model-v3.md`
- `docs/config/2026-10-28/**`
- `docs/config/2026-10-30/model-v2.md`
- `docs/config/2026-10-30/migration-v2-to-v3.md`
- `docs/config/2026-10-30/per-engine-config.md`
- `docs/tutorials/**`

Keep model/strategy evidence documents under `docs/model/2026-11-03/` and
`docs/model/2026-11-04/` as historical evidence unless they are current
contracts. The current `M1.0` contract is the moved model-v4 path.

## 4. Keep as history

| Path | Why |
|---|---|
| `docs/adr/**` | Decision history |
| `docs/audits/**` | Audit evidence |
| `docs/improvement/**` | Improvement and programme records |
| `docs/ai-transparency-log/**` | AI usage history |
| `docs/execution-plans/**` | Plans of record |
| `docs/releases/**`, `docs/reports/**` | Release and report history |
| `docs/model/2026-11-03/{adx-verification,claim-validation,path-validation}.md` | Model evidence |
| `docs/model/2026-11-04/compression-*.md`, `barrier-path-5m-onset.md`, `path-validation-v4.md` | Model evidence |
| `data/improvement/**`, `data/research/**` | Model/strategy/portfolio/policy run evidence |

Old names and old architecture references are allowed in these paths because
they are history.

## 5. Naming

| Kind | Convention | Current / next |
|---|---|---|
| Model | `M<major>.<minor>` | `M1.0` baseline; `M2.1–M2.5` historical no-edge; `M3.x` next |
| Strategy | `S<major>.<minor>` | `S1.1–S1.5` historical; `S2.x` closed; `S3.x` after an M3 baseline |
| Portfolio | `P<major>.<minor>` | `P3.x` after an accepted S3 strategy |
| Policy | `POL<major>.<minor>` | `POL3.x` after an accepted P3 portfolio |
| Execution | `E<major>.<minor>` | `E3.x` with the accepted stack |

Technical schema versions (`quantrule.data/v3`, `quantrule.execution/v2`,
`quantrule.model-spec/v2`) are file-format versions, not model or strategy
identities. Current configs gain an `id` field with the M/S/P/POL identity.

## 6. Legacy-token allowlist

The P2 legacy check may report old names only in:

- `docs/adr/**`
- `docs/audits/**`
- `docs/improvement/**`
- `docs/ai-transparency-log/**`
- `docs/execution-plans/**`
- `docs/releases/**`
- `docs/reports/**`
- `docs/model/2026-11-03/{adx-verification,claim-validation,path-validation}.md`
- `docs/model/2026-11-04/{barrier-path-5m-onset,compression-*,path-validation-v4}.md`
- `data/improvement/**`
- `data/research/**`

Current `crates/**`, `config/**`, `docs/architecture/**`, `docs/model/2026-11-05/**`,
`docs/config/**`, `docs/tutorials/**`, and `README.md` must not contain old model
or strategy version labels after P2–P6.

## 7. Deletion order

1. P2 records aliases and renames the current contract.
2. P3 deletes old model modules and their tests.
3. P4 deletes old commands, parsers, and configs.
4. P5 replaces panel/session and formula contracts.
5. P6 deletes old docs and tutorials.
6. P7 runs the legacy scan and closeout.

No deletion starts before the P0 inventory is approved.
