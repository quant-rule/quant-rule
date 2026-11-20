# 10. GUI Reference

- Status: Current
- Audience: operator or reviewer finding the browser control for a command

The browser and the shell reach the same gateway contract. Test ids are stable:
a Playwright test and the client-doc checker use them. The evidence badge column
names what the screen must display; a screen that hides a diagnostic boundary is
a defect.

| Screen / tab | Component | Shows | Controls and test ids | Evidence badge | Commands mirrored |
|---|---|---|---|---|---|
| Connection | `ConnectionPanel.svelte` | Gateway state, product, API, image digest, service health, token entry | Refresh; token input; Use token; Forget token; `health-report` | Connection badge: `connected`, `degraded`, `version mismatch`, `unreachable`, or `token required`; no evidence badge | `health` |
| Profiles | `ProfilesPanel.svelte` | Profile list, counts, create | Refresh; `profile-table`; `profile-create` | `active` badge on the selected profile; no evidence badge | `profile list`, `profile create` |
| Workbench | `WorkbenchPanel.svelte` | Data, Model, Strategy, Portfolio tabs | Tab buttons (role `tab`); no command test id | None; the container shows no evidence claim | — |
| Workbench — Data | `DatasetsPanel.svelte` | Dataset table, upload, record detail, quality | Upload; Skip bad rows; `dataset-upload`; `dataset-table`; `dataset-detail-open`; `dataset-detail` | None; upload is diagnostic preparation | `dataset list`, `dataset show`, `dataset upload` |
| Workbench — Model | `ModelStagePanel.svelte` | Model stage form, stage table, detail, events, coverage, downloads | `stage-submit`; `stage-table`; `stage-detail`; `stage-events`; `stage-fetch`; `stage-cancel`; `stage-delete` | `Evidence: false` in the detail; the stage is diagnostic and never evidence | `stage run model`, `stage list`, `stage show`, `stage events`, `stage fetch`, `stage cancel`, `stage delete` |
| Workbench — Strategy | `StrategyStagePanel.svelte` | Strategy stage form, decisions, explain, downloads | `strategy-submit`; `strategy-table`; `strategy-detail`; `strategy-decisions`; `strategy-explain`; `strategy-fetch`; `strategy-cancel`; `strategy-delete` | `diagnostic · not evidence`; `Evidence: false`; no P&L | `stage run strategy`, `stage list`, `stage show`, `stage explain`, `stage fetch`, `stage cancel`, `stage delete` |
| Workbench — Portfolio | `PortfolioStagePanel.svelte` | Portfolio form, metrics, execution and portfolio tables | `portfolio-submit`; `portfolio-table`; `portfolio-detail`; `portfolio-metrics`; `portfolio-summary`; `portfolio-execution-summary`; `portfolio-orders`; `portfolio-fills`; `portfolio-rejections`; `portfolio-fetch`; `portfolio-cancel`; `portfolio-delete` | `diagnostic · not evidence`; `Evidence: false`; P&L is diagnostic accounting | `stage run portfolio`, `stage list`, `stage show`, `stage fetch`, `stage cancel`, `stage delete` |
| Datasets | `DatasetsPanel.svelte` | Dataset table, upload, record detail | `dataset-table`; `dataset-detail`; `dataset-upload` | None; data identity and quality only | `dataset list`, `dataset show`, `dataset upload` |
| Configs | `ConfigsPanel.svelte` | Config table, validity, parser message, submit, detail | `config-table`; `config-detail-open`; `config-detail`; `run-submit` | `valid` or `invalid` badge; a frozen model artifact is marked `diagnostic input` on the run detail | `config list`, `config show`, `run submit` |
| Runs | `RunsPanel.svelte` | Run rows, status, cancel | `run-table`; `run-cancel`; open a run by its id | Run status badge: `queued`, `running`, `succeeded`, `failed`, `cancelled` | `run list`, `run cancel` |
| Run detail | `RunDetail.svelte` | Stages, provenance, frozen model artifact, digest, verified downloads | `run-status`; `run-verify`; `run-fetch` | Run status badge; `diagnostic input` badge on a frozen model artifact | `run status`, `run fetch`, `verify` |
| Research | `ResearchPanel.svelte` | Create form, improvement table, detail, submit, comparison, decision, baselines, invalidations | `research-create`; `research-table`; `research-detail`; `research-submit`; `research-compare`; `research-decide`; `baseline-table`; `baseline-detail`; `invalidation-list` | State and decision labels; the record is governed evidence; the human decision is explicit | `research create`, `research submit`, `research list`, `research show`, `research compare`, `research decide`, `baseline list`, `baseline show`, `invalidation list` |
| Operations | `OperationsPanel.svelte` | Health, product, API, image digest, profile, token presence, retention, run verification | `operations-health`; `operations-profile`; `operations-retention`; `operations-verify`; `operations-verify-digest` | None; it states that only diagnostic stages expire and runs, research records, and evidence are permanent | `health`, `verify`; active profile shown read-only |
| Inspect | `InspectPanel.svelte` | Candles, timeframe ladder, event batch | `inspect-candles`; `inspect-timeframes`; `inspect-events` | `read-only` badge; never evidence | `inspect candles`, `inspect timeframes`, `inspect events` |
| Simulation | `SimulationRunsPanel.svelte` | Simulation rows, create form, status, retry | `simulation-table`; `simulation-create`; `simulation-refresh`; `simulation-retry`; `simulation-empty` | Simulation status badge; research plane, never evidence | `simulation list`, `simulation create` |
| Simulation run | `SimulationRunDetail.svelte` | Meta, windows, chart, marks, oracle, check, decision | `simulation-run`; `sim-back`; `sim-status`; `sim-run-meta`; `sim-windows`; `sim-window-<n>`; `sim-mark`; `sim-marks`; `sim-oracle`; `sim-review`; `sim-check`; `sim-decision`; `sim-lock` | Status badge; locking notice after a decision; never evidence | `simulation windows`, `simulation mark`, `simulation marks`, `simulation oracle`, `simulation check`, `simulation decide` |
| History | `HistoryPanel.svelte` | Five kind tabs, archived filter, archive, typed-confirmation erase | `history-table`; `history-tab-<kind>`; `history-include-archived`; `history-archive`; `history-erase`; `history-confirm-input`; `history-confirm-erase` | Active or archived state; a protected record refuses erase | `history list`, `history archive`, `history erase` |
| Help | `HelpPanel.svelte` | Every command and the screen that reaches it | `help-table` | `49 commands` badge; no evidence badge | `help` |

## Screen rules

| Rule | Meaning |
|---|---|
| One owner | Each screen is owned by one component |
| Same contract | No screen computes an indicator, a strategy decision, a fill, or a metric |
| Stable ids | A control that mirrors a command carries the test id in this table |
| No stale rows | A profile switch or disconnection clears profile-scoped rows |
| Numbers, then labels | The gateway returns codes; the client maps them in one generated table |
| Evidence boundary | A diagnostic stage never renders as evidence, a baseline, or a canonical digest |

## Workbench tab behavior

| Tab | Evidence badge | What it never shows | Primary commands |
|---|---|---|---|
| Data | None; upload is preparation | A model reading, a decision, or a fill | `dataset list`, `dataset show`, `dataset upload` |
| Model | `Evidence: false`; diagnostic | A strategy decision, a fill, or P&L | `stage run model`, `stage events`, `stage fetch` |
| Strategy | `diagnostic · not evidence` | Execution feedback, cash, P&L, drawdown | `stage run strategy`, `stage explain`, `stage fetch` |
| Portfolio | `diagnostic · not evidence` | An accepted result or a canonical digest | `stage run portfolio`, `stage show`, `stage fetch` |

A tab switch clears the selected stage so a model detail cannot appear under the
Strategy tab, and a portfolio detail cannot appear under Model.

## Profile scope and disconnection

| Event | Browser behavior |
|---|---|
| Profile switch | Clears datasets, configs, runs, simulations, stages, research, and history rows before the next read |
| Gateway unreachable | Shows `disconnected` or `unreachable`; does not show the previous profile's rows |
| Token required | Shows `token required`; submission is disabled |
| Version mismatch | Shows the expected and observed API versions; does not use the connection |
| Slow response | A generation counter drops a read that started under the previous scope |

The browser holds UI state only. The server is the record for a run, a stage, a
research decision, and a simulation decision.

## Test id rules

| Rule | Meaning |
|---|---|
| One stable id per mirrored control | The capability table, the component, and the Playwright test use the same string |
| Dynamic rows use a stable prefix | A window row is `sim-window-<n>`; the prefix is stable, the index is the row identity |
| An id is not a label | The visible text may change; the id is the contract for the test |
| A missing id is a defect | The capability table and the browser test that drives the control must use the same string; a drift fails that test |

## What not to expect

| Expectation | Reality |
|---|---|
| The browser computes a result | It renders the gateway response |
| A chart library draws the simulation | The client renders deterministic SVG |
| A diagnostic stage shows a digest | It has no canonical digest |
| A profile is a security boundary | It is organisational only; the token is the network gate |
| A hidden future bar is available | The chart is hindsight by design, but no model or run reads it |

