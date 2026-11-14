# Changelog

All notable changes to QuantRule are documented here. The changelog is grouped
by release and records meaningful user-facing or developer-facing changes. It is
the release history, not the execution tracker: issues, sub-issues, pull
requests, and their status live on the GitHub Project.

The changelog starts at the 2026-11-07 workflow documentation reset. Earlier
release notes are in [`docs/releases/`](docs/releases/), and the detailed change
history is in Git and the pull requests. Historical tags, including
`v0.4.0-exp-baseline` and `audit-closure-2026-10-30`, are retained as markers and
are not treated as releases.

## [Unreleased]

### Security

- The native command boundary now rejects filesystem paths outside the opened
  workspace and memory directory. The desktop Settings, Memory, and Run panels
  use the native dialog picker, and every selected path is scope-validated
  before I/O; typed paths remain validated inline.

### Added

- The shell is a `quantrule.api/v1` client: `health`, `dataset`, `config`,
  `run`, `inspect`, `verify`, `profile`, `simulation`, and `help` commands, a
  global `--profile`, and a `--json` machine view. Every command makes one
  gateway operation; printed output is stable and covered by fixture-server
  tests.
- The desktop shell (`apps/desktop-shell`) is a Tauri 2 window over the gateway
  URL. It remembers the gateway, leaves the active profile to the web client,
  checks GitHub Releases for a newer shell, and shows the bundled client only as
  the disconnected fallback. It owns no IPC command, reads no data file, and
  embeds no engine. The Linux AppImage and Windows NSIS artifacts are unsigned;
  publishing them waits on the release workflow.
- Testing is recorded as three tiers: the fast unit/integration gate, the
  opt-in Compose tier required before a release, and port parity backed by
  golden fixtures from the retired implementation. `tools/check_port_parity.py`
  runs in the gate and fails on a missing registry entry, fixture, marker, or
  comparison test.
- Reviewer verification is four commands and one line: `tools/verify.sh` runs
  the mini fixture twice, compares the canonical digest, and `--expected`
  compares a second host's line. `make verify` is the shortcut. The full
  example remains the release run.
- Crash tests prove that a backtest stopped before the manifest leaves no run
  prefix and no artifact, and that an evidence process stopped before `complete`
  leaves the run `running`; each restarts from the durable object set and
  finishes. A real `SIGKILL` remains composed-tier evidence (#643).
- A scale-invariance harness proves one pass and four model shards share one
  object digest in any completion order, and a reversed requested ladder
  produces the same bytes; an unsorted merge changes the digest.
- A source guard over the signal path fails on transcendental, power, and
  fused-multiply-add calls; `sqrt`, rounding, and the four arithmetic operations
  remain allowed. The benchmark and test fixtures are outside the guard.
- The client label table is generated from `docs/registry/indicators.json` into
  `apps/web/src/lib/labels.generated.ts`; `check_labels` fails the gate when the
  checked-in table drifts, so every numeric code has one word mapping.
- The browser client distinguishes a gateway API mismatch from a healthy
  connection and shows the version it expected; a shared Playwright fixture
  router drives the connected, mismatch, degraded, and unreachable tests.
- The control-plane queue topology: `runs.backtest`, `runs.evidence`, and
  `runs.dead-letter` are named durable queues. A run records its attempt count;
  a retryable failure holds the delivery for 5 s, then 25 s; the third failure
  marks the run `failed` and publishes `run.dead`. A broker redelivery reclaims
  an uncomputed running run. Cancelling publishes `run.cancelled`, and every
  message carries the run id only.
- The backtest worker core: `POST`-submitted runs are claimed, their six-file
  config and catalogued dataset are materialized into a private workspace, the
  coupled backtest runs, artifacts are written to CAS, and the canonical digest
  is computed over `results.json`, `*.csv`, and `*.ndjson`. The run stays
  `running` for evidence. `qr-backtest` consumes `run.requested`; duplicate
  deliveries skip a terminal or running run, and a cancel before finalization
  leaves it cancelled.
- The evidence service: `qr-evidence` consumes `run.computed`, verifies every
  artifact against its recorded size and SHA-256, recomputes the canonical
  digest, rejects a missing, changed, or mismatched artifact, and marks the run
  `succeeded` with the evidence stage `Done`. A finalized run is never finalized
  again. The gateway and worker now publish the run id only.
- A `QueueConsumer` trait with a RabbitMQ implementation and an in-memory fake;
  a successful handler is acked, a failed one is nacked.
- The model compute route: `POST /model/compute` loads a catalogued dataset and
  run config by reference, materializes the config bundle, computes the declared
  ladder per timeframe, merges the reports sorted, and stores one JSON object in
  CAS. A single timeframe and the full ladder produce identical bytes.
- Config rows are profile-scoped while the file bundle is shared by content id:
  a repository scan writes a row for the requesting profile, an upload writes
  one for its profile, and a run submission scans the mount on a miss. Switching
  the client profile reloads the config list.
- A catalogued config stores its whole bundle, not just `run.json`. A repository
  scan reads the five referenced files; an uploaded config keeps the validated
  path-to-body map. A run can reproduce the configuration after the staging
  directory is gone.
- The Datasets screen exposes tolerant upload: a "Skip bad rows" control uses
  the tolerant route and renders the quality report, including the first skipped
  line and its reason.
- Tolerant ingest: `POST /catalog/datasets/tolerant` skips a bad row instead of
  failing the upload and records the accepted candles plus a quality report
  (rows read, accepted, rejected, duplicate, out of order, invalid, and each
  skipped line with its reason). The accepted rows keep the identity a strict
  upload of the same clean rows would produce, and the report is stored on the
  dataset record.
- Config uploads: `POST /catalog/configs` accepts a run-config bundle as
  relative paths and bodies, stages it privately, validates it with the same
  loader as the mount, and catalogues it as `upload`. A path that is absolute
  or contains `..` is refused; an invalid bundle is still catalogued with its
  parser message.
- Run detail: the browser opens a run from the inbox and shows the stage
  timeline, provenance including the image digest, the artifact list with
  hashes, and the canonical digest with a copy action. A failed run shows its
  error and hides the artifact section.
- The configs screen: the browser lists the config catalog with origin and
  validity, renders the parser message verbatim, disables submit on an invalid
  config or with no dataset, and submits a run from a valid config without a
  path. A queued run can be opened in the runs inbox.
- The runs inbox: the browser lists runs from the server with status, config,
  data version, submission time, and duration; it polls every 2 s while a run is
  active and every 30 s otherwise, cancels only an active run, drops the rows on
  a failed read, and shows a disconnected state instead of stale evidence.
- Run submission: `POST /runs` validates the dataset and the run config, derives
  `run_id` from `data_version`, `config_hash`, `model_version`, and
  `image_digest`, stores a queued run, and publishes `run.requested`. A duplicate
  returns `200` with the same run. `GET /runs`, `GET /runs/{id}`, and
  `POST /runs/{id}/cancel` complete the control-plane surface. Runs are shared
  across profiles through submission rows, so identical inputs produce one run.
- The config catalog: `GET /catalog/configs` scans the read-only run-config
  mount, validates each `run.json` with its referenced files, and lists a
  content-derived id, validity, the parser message, and a SHA-256 over the five
  files. A second read refreshes the rows without duplicating them. Config
  uploads remain planned.
- The gateway serves the built client from `QR_STATIC_DIR`: assets are cached
  immutably, an unknown page falls back to `index.html`, a write method is not
  served the SPA, and an `/api/*` path never falls back.
- A Playwright harness for the client: create a profile, switch it, survive a
  reload, and surface an unknown stored profile. The API is mocked at the
  network boundary, so the tests need no backend.
- The profile switcher and Profiles screen: the client lists profiles, creates
  one, remembers the active id locally, and sends it on every call. Switching
  reloads the dataset list for that profile.
- The browser client's Datasets screen: it lists `data_version`, candles, and
  UTC coverage, uploads a CSV with the file as the request body (no path field),
  shows an indeterminate upload bar, and renders the stable error code. A
  duplicate upload returns the existing record and does not add a row.
- The gateway now serves the full API under `/api/v1`, the base the client
  already uses, while the same routes remain at the root for probes.
- The stored ladder: `GET /datasets/{version}/ladder/{timeframe}` folds the
  canonical M1 bytes into one timeframe's CSV and writes it under
  `ladder/{data_version}/full/{timeframe}`. A repeat request reuses the object.
  `LadderWriter` streams the same buckets as `TimeframeSet`, so an emitted bar
  and a published bar cannot disagree.
- Hypotheses are profile-scoped: migration 0008 adds `profile_id`, the store keys
  by `(profile_id, fingerprint)`, and the hypothesis routes require a known
  profile. The same hypothesis can be sealed in two profiles without one
  becoming visible to the other.
- The candle stream: `GET /datasets/{version}/candles` returns the stored
  canonical CSV after re-hashing it against `data_version` and sets an ETag, so
  a run consumes exactly the bytes the catalog identifies.
- Read-only inspection over stored bytes: `GET /inspect/candles` returns counts
  and coverage, and `GET /inspect/timeframes` returns complete bars and partial
  buckets per timeframe. The object is re-hashed before it is parsed, so a
  catalog row and its bytes cannot disagree silently.
- The ladder's streaming count (`LadderSummary`) folds the same buckets as
  `TimeframeSet` without retaining the series, so a large dataset can be
  inspected in bounded memory.
- Ingest now refuses an out-of-order, duplicate, misaligned, or coarser-than-M1
  source instead of cataloguing a series the ladder would reject.
- Profiles are real records: `GET /profiles` lists them with dataset counts,
  `POST /profiles` creates one idempotently, and migration 0007 seeds the
  `default` profile. The dataset routes now refuse an unknown profile instead of
  accepting any syntactically valid id.
- The dataset catalog: `POST /catalog/datasets` turns CSV bytes into a
  content-addressed, profile-scoped record, `GET /catalog/datasets` lists the
  profile's summaries, and `GET /catalog/datasets/{version}` returns one record.
  A repeat upload is idempotent and returns the existing record.
- The code module map at `docs/architecture/module-map.md`: crate and module
  ownership, reading order, and the where-to-add guide for a reading, event,
  command, configuration field, use case, or desktop command.
- Native file and folder pickers in the desktop Settings, Memory, and Run
  panels. A selected path still passes the backend workspace/memory scope check
  before any I/O, and typed paths remain available.
- The shell `memory` command family and the desktop memory write forms:
  status, record with computed model counts, workspace/run lists, metric,
  claim, and note CRUD, logical export/import, and SQLite backup/restore. The
  shared application service owns the validation; the command surface now has
  16 implemented commands.
- The board permanence checker (`tools/check_board_permanence.py`) and the
  project-sync status test. CI rejects a delete or archive mutation in the board
  automation and proves that `Done` stays terminal.

### Changed

- The shell depends only on `quantrule-api`, `ureq`, and `serde_json`. Local
  source fetch and local stage execution are gone; a run is submitted to the
  gateway and read back through the run and inspection routes.
- Run artifacts publish atomically: bytes live at `artifacts/{sha256}` and one
  `runs/{run_id}/manifest.json` is the publish point. A worker stopped mid-write
  leaves blobs but no manifest, no recorded artifact, and no succeeded run.
- The v4 strategy example uses a 1m execution fill to match the strategy. The
  worker exposed that the app requires the strategy execution timeframe and the
  execution fill timeframe to agree; the v4 contract now states it.
- The model engine's extended signal detector now sits under
  `events/extended/`, the extended submodules use explicit imports, and source
  codes are defined once in `events/source_codes.rs`.
- The stage-provenance reader is shared by the model, strategy, execution, and
  portfolio bundles; the duplicate reader error names are gone.
- The desktop command boundary is split into `views`, `scope`, `workspace`, and
  `memory` modules.
- The model report and data-inspection use cases now live behind
  `quantrule-app`; the model bundle, report renderer, labels, and model-spec
  parser moved out of the shell. The shell keeps parsing and terminal output.
- The strategy, execution, portfolio, backtest, qualification, and analytics
  use cases now live behind `quantrule-app`; the shell is a parse/render/fetch
  adapter and the desktop has one Tauri command per use case.
- The desktop UI has a Pipeline section for inspecting data and running the
  model, strategy, execution, portfolio, backtest, qualification, and analytics
  stages. Source fetch remains shell-only.
- The desktop UI is a first-class surface of the same program as the shell. Both
  call the shared application service and are required to have feature parity;
  the shell is primarily for agents. The
  [one-program ADR](docs/adr/2026-11-11/one-program-first-class-desktop.md) and
  the [audit remediation plan](docs/execution-plans/2026-11-11/audit-app-service-native-remediation.md)
  record the target and the remaining parity extraction.
- The project board is append-only history. A `Done` item is terminal and is
  never deleted, archived, removed, bulk-cleaned, or moved back to an earlier
  status; worktree and branch cleanup never touches the board.
- `project-sync.sh` now keeps a `Done` leaf or container `Done` instead of
  resetting an open leaf to `Todo` or recomputing a container when a child
  reopens.
- Execution plans now carry a delivery sequence and a verification table with a
  row for every work-breakdown phase, acceptance criterion, and closure step. The
  template names roles, dependencies, and affected paths, and the plan checker
  rejects duplicate IDs, empty core cells, and unresolved rows on a `Verified`
  plan.

### Removed

- The local stage commands (`candles`, `candles-tolerant`, `timeframes`,
  `events`, `report model`, `run check`, `strategy explain`, `strategy run`,
  `execution run`, `portfolio replay`, `backtest run`, `qualify`, and
  `analytics evaluate`), source `fetch`, and the local `memory` command are
  removed from the shell in v1.0.0. Their capability moves into the run and
  read-only inspection.

### Fixed

- The in-memory run store now rejects a `complete` or `fail` transition from a
  queued or terminal run, matching the Postgres `status = 'running'` predicate;
  a test that accepted a transition production rejects can no longer pass.
- An invalid uploaded config now keeps the `upload` origin; the shared record
  loader had hard-coded `repository` on the invalid branch.
- The client's startup load no longer loops: the two initial requests run from
  `onMount` instead of an effect that read the state it updated.
- Stale module docs that described the shell as the only surface, the backtest
  runner as future work, the app as the only configuration parser, the desktop
  as doing no path resolution, the model as six scalar readings, and the flat
  report as the M1.0 view. The shell glossary now names M1.3 as the current
  model.

### Added

- The v1.0.0 distributed architecture record: eight decision records under
  `docs/adr/2026-11-12/`, the service catalog, and the data-plane,
  control-plane, HTTP API, deployment, client, user-journey, security, and
  operations contracts under `docs/architecture/`.
- The distributed execution plan
  `docs/execution-plans/2026-11-12/dist-000-distributed-platform.md` and its
  issue tree `DIST-000` … `DIST-090`.
- Verification tutorials for the distributed target: run a backtest, verify the
  canonical digest, use remote compute, and scale workers.

- The `simulation` service and the research plane: chart-first, hindsight-labeled
  research, isolated from the signal path, with an inputs digest and no output
  digest (`ADR-2026-11-12-09`).
- The Python boundary: the product image carries no Python interpreter; Python is
  confined to repository governance (`ADR-2026-11-12-10`).

- Profiles: organisational scoping over every record, with shared content-addressed
  bytes and no security promise (`ADR-2026-11-12-11`).
- A Tauri desktop shell over the gateway URL, and backend adapter traits so the
  embedded desktop profile is additive rather than a fork (`ADR-2026-11-12-12`).

### Changed

- The board is aligned to the kata: six issues carry the six requirements
  (hypothesis record, exploration, costs, statistical significance,
  out-of-sample validation, comparison). Profiles and the desktop shell stay in
  scope because they serve the same single user; multi-user is not built.
- The platform grows from five services to six; the deployment from eight
  containers to nine.
- The browser client renders charts as SVG and captures marks in the page; the
  server no longer renders PNGs.
- The client toolchain is Bun; the image carries static assets, not a Bun runtime.
- `AGENTS.md` now describes a distributed single-operator platform: a browser
  client and a command shell on one HTTP contract, two planes, two test tiers,
  and a bounded determinism claim.
- `README.md` now states the v1.0.0 target and keeps the implemented command
  table labelled as the v0.3.0 tree.
- `docs/architecture/` now separates current decisions from target contracts;
  every target document carries `Status: Target v1.0.0`.

## [0.9.0] - 2026-11-11

### Added

- The four-stage development workflow: `Todo`, `In Progress`, `In Review`, and
  `Done`, with a required claim type, start and close dates, a human review gate,
  and an execution-plan verification table that the reviewer checks row by row.
  The new `check_claims.py` and `check_execution_plans.py` gates cover the
  templates and current plans.
- The Tauri Platform definition: local-first ADR, course folders, kata,
  architecture characteristics, architecture-description outline, methodology
  checkpoints, and a Dockerized PDF build. The current product is solo and local;
  the team cloud is future work.
- The `quantrule-app` application service: workspace discovery, dataset summaries,
  run validation, model-run summaries, and deterministic run-summary export. The
  command shell and the future desktop UI call the same service; `run check` now
  goes through it.
- The local research memory: `quantrule.research-memory/v1` over bundled SQLite,
  with workspace, dataset, config, run, metric, claim, and note records, ordered
  queries, deterministic JSON export/import, and consistent backup/restore.
  `AppService::record_run` writes a validated run and its workspace, dataset, and
  configuration in one call.
- The Tauri 2 + Svelte 5 desktop UI in `apps/desktop`: workspace, data, configs,
  model run, research memory, export/backup, and settings over the application
  service. It has no account, network path, or domain computation. Frontend unit
  tests, Playwright browser E2E, and a native Xvfb launch smoke test cover it.
- The Linux distribution path: desktop version `0.9.0`, Tauri `.deb` and
  `.AppImage` bundles, a static download page at `site/`, SHA-256 checksums, and
  `site/check.py` verification. The page is distribution only; no account,
  backend, or cloud runtime.

### Changed

- The current run schema is `quantrule.run/v3`; the policy layer is removed and
  `run check` replaces `policy check`. Portfolio capital, sizing, costs,
  drawdown, leverage, and position limits remain first-class constraints. The
  qualification dimension `prop_firm_limits` is removed.
- The course deliverables are final: the architecture description now cites the
  implemented desktop, application service, memory, and distribution code, the
  characteristics and methodology checkpoints match the Phase 3 evidence, and
  all nine PDFs are rebuilt. The 0.9.0 release-readiness record is added; the tag
  remains a human gate.

### Removed

- The current `policy.json`, policy loader, `policy check` command, and policy
  cross-config checks. Historical `run/v2` evidence remains read-only.

## [0.8.0] - 2026-11-11

### Added

- The Binance BTCUSDT public-archive importer and coverage audit, with 96
  checksum-verified monthly files covering 2018–2025. The source is admitted for
  cross-venue robustness outside the formal partitions; 2023 remains excluded.
- `analytics evaluate` now publishes buy-and-hold and matched random-timing
  benchmarks in `analytics/benchmarks.csv` and `analytics/metrics.json`, with an
  explicit seed, path count, cost assumptions, and null reasons.
- A deterministic 15m trade-chart tool that renders one 1,000-candle chart per
  trade, with OHLC, volume, model EMA 20/50/200, entry/exit markers, and a manifest.
- A local data-cleanup record: large historical evidence is archived outside the
  working tree and reachable through the original `data/` symlinks.
- The `tools/trades/` context exporter, annotation mapper, and exit-quality
  table. Each trade gets a 250-bar entry/exit window with all readings, events,
  decisions, orders, and fills; the post-fill window is separate.
- A local click-to-annotate chart page. It maps a click to the candle from the
  chart manifest and writes `annotations.csv` directly, with no hand-edited CSV
  and no pixel detection.
- The `M1.2` model structure extensions: confirmed pivot-sequence counts and
  lower-high/lower-low events, causal Fibonacci retracements and level tests,
  and multi-touch horizontal support/resistance bands and level tests. The
  readings CSV grows to 121 columns and every annotation context carries them.
  The registry is `2026-11-10.1` with 49 indicators and 150 events.
- Strategy position excursions: entry price/ATR, unrealized ATR, MFE, MAE, and a
  configurable giveback exit with an arming threshold. The strategy still sees
  no cash, equity, P&L, fill, order, or quantity.
- The `tools/trades/annotation_zones.py` zone extractor and the 2018–2019
  annotation zone report. It publishes EMA-tap, Fibonacci, band, pivot-sequence,
  and forward-return features around each human mark.
- A deterministic raw-edge diagnostic (`tools/research/raw_edge.py`) that measures
  forward returns, MFE/MAE, and barrier outcomes for a published entry signal,
  with event-only, state-only, and unconditional controls. It writes only an
  evaluation report and never a strategy artifact.
- A configurable event-edge screen (`tools/research/event_edge.py`) that applies
  the same forward and barrier measurements to a declared event identity and
  window, with an all-bars control. It rejects event subtypes the strategy
  engine cannot read.
- A deterministic SBE-01 scale screen (`tools/research/scale_screen.py`) that
  tests a range-fraction stop/target geometry for cost feasibility against a
  published compression range and ATR. It writes only an evaluation report and
  no strategy or portfolio artifact.
- A fresh-onset 5m breakout screen (`tools/research/breakout_edge.py`) with nine
  fixture tests and the SR13 development record. The 6-bar pooled result is
  negative; no candidate and 2024 remains closed.
- Undo for the click annotator: `Ctrl+Z`, `Cmd+Z`, or the **Undo** button
  reverses the last save or delete in the current server session.
- The simulation-run harness (`tools/simulation/`) and the
  `simulation-research` workflow. A run selects deterministic M1.3 windows,
  renders clean charts, writes hindsight good/bad marks in the annotator schema,
  reviews the candle-shape context, and checks the extracted rule on 2020–2022
  with an unfiltered control and a 6 bps proxy. It is exploration only; a
  survivor goes to Strategy Research. The same harness includes the SIM-01
  human-mark meta-label and the SIM-03 five-exit comparison.
- A persistent local data root outside the repository and every worktree
  (`$QUANTRULE_DATA_ROOT` or `$HOME/quant-rule-data`), and the
  `tools/worktree_cleanup.py` guard. The guard classifies local data, archives
  it with a SHA-256 manifest, and refuses to remove an unmerged or unclassified
  worktree. The merged-task cleanup now checks and archives before removal.

### Changed

- Merged-task cleanup is mandatory: `AGENTS.md` now requires removing the task
  worktree and deleting its local and remote branches before a task is reported
  complete. The completion report says `Clean up` while cleanup remains;
  `No action` is reserved for after cleanup.
- The BTC 15m break-only long control is not promoted. On the dense Binance
  BTCUSDT series, 2018–2022 is 65 trades, −$389.64, PF 0.871, and 2023 is 8
  trades, −$202.50, PF 0.229; the Dukascopy lead is not venue-robust.
- The model identity is `M1.3` with registry `2026-11-10.2`. It corrects the
  M1.2 sequence-event occurrence bar, makes `level.fib_test` a bar-range
  intersection with one event per tested level, and fixes the Fibonacci/band
  tolerances as model constants; M1.2 remains historical.
- The annotation-zone close-based features are renamed `fib_close_proximity`
  and `band_close_inside` and are documented as distinct from the model's
  `level.fib_test` and `level.band_test` events.

### Fixed

- Pivot-sequence events now record the confirmed pivot's occurrence bar
  separately from its confirmation availability bar.
- `level.fib_test` now detects a level the bar trades through, not only a level
  near the candle high or low, and emits one event per tested level.
- Spec v2 now rejects the fixed `fib_test_atr` and `band_atr` overrides, and the
  model validates both tolerances for programmatic callers.
- The strategy contract now describes the registered-subtype rule the
  event-composite validator actually enforces.
- The `band` reading no longer uses `expect` in library code; an empty cluster
  or band publishes null instead of panicking.
- Trade charts now render the model's `ema_fast`/`ema_mid`/`ema_slow`
  (20/50/200) instead of a chart-side EMA 10/50/200 calculation.
- Trade charts now draw the exit marker and label the exit action (`SELL` for a
  long, `BUY` for a short); previously only the entry marker was visible.

## [0.7.0] - 2026-11-09

### Added

- An optional `session` calendar in `data.json` and session-aware coverage
  fields in the model report. Weekends and declared holidays no longer count as
  missing bars for session-bound instruments; the existing span coverage is
  retained.
- The `signal.breakout_retest_hold` event: a compression breakout followed by a
  4-bar retest within 0.2 ATR that closes back on the breakout side. It is a
  signal event with code 146, subtype 24, source 5; the break-only event is the
  control.
- The `vol_regime` and `structure_state` context states. `vol_regime` is the
  persistent 0/1/2 volatility regime; `structure_state` is the confirmed major
  +1/0/-1 swing structure. Both are strategy-readable and leave existing event
  values unchanged.
- `help workflows` in the shell: an intention-to-workflow selection guide with
  the first command or tool for each workflow.
- The workflow lab now shows the command-level selection table and the model
  research/swarm quick path.
- The model-prediction condition gate accepts the published `level_touches`
  reading and integer `min`/`max` bounds, enabling a location-conditioned model
  evaluation without a Rust model change.
- The `MR`/`SR`/`PR` research-family prefix and the `-Cnn` configuration suffix;
  historical `M2.x`–`M4.x` aliases remain in `docs/registry/history.md`.
- Per-cell model configuration in the research runner, with the model config
  hash recorded in provenance.
- Sequence, confluence, freshness, and descriptive path evaluators for model
  research; the MR5 inherited-coverage checker, grid builder, and reference
  grid.
- Event-family sequence conditions (`first_events` / `second_events`), a strict
  ordering gap, and a confirmed-level filter on the confirming bar.
- Per-hypothesis timeframes, a configurable family/run prefix, and synthesized
  run configs for an extra instrument in the research grid builder.
- The MR6 ordered-confirmation grid (15 cells, five primary, reverse-order
  controls) and its `no_edge` development result.
- The `M1.1` opening-range sweep and no-reclaim control events, with the
  `session.sweep_atr` spec field; the registry moves to 145 events and 191
  validation claims.
- `COPPERCMDUSD` and `TSLAUSUSD` as price-only instruments, with aliases
  `COPPER` and `TSLA`; volume, participation, and flow events exclude them.
- The MR7 opening-range sweep grid (15 cells) and its `no_edge` result.
- The research report applies a declared mean-net-R path gate before naming a
  candidate.
- An optional `contrarian` boolean on an event-composite event entry: the
  entry takes the opposite side of the matched event while the event identity
  and evidence stay unchanged. The default is `false`; the parser rejects the
  field on a state condition and on the compression-bracket engine.
- The SR1 compression-breakout fade strategy config, 15-cell development
  matrix, 9 bps sensitivity configs, and per-cell qualification criteria.
- A deterministic strategy feasibility preflight
  (`tools/improvement/feasibility.py`) and its Strategy Research gate: a
  hypothesis whose target is below its round-trip cost, whose breakeven win
  rate exceeds 50% after costs, or whose timeframe exceeds 1 hour is rejected
  before any backtest cell. The SR1 development run stopped early as
  infeasible after 10 of 15 cells; the completed cells all halted, and the 4:1
  payoff failed the preflight.
- The SR2-A BTCUSD robustness audit and its read-only analysis tool: the
  positive BTC lead survives 9 bps and the 5% drawdown run in sign, but is
  concentrated in the top five trades, is not statistically significant, and is
  about 0.1%/month at the 5% ceiling. The audit generates condition hypotheses
  but no candidate.
- The SR2 dual-design fade configs and its 30-cell development run: 1h with a
  1 ATR stop / 3 ATR target and 15m with a 1 ATR stop / 4 ATR target, both on
  BTCUSD/EURUSD/XAUUSD. Both designs returned `no_edge`; the 9 bps sensitivity
  and the 2024 validation were not opened.
- The `rvol_regime` model context state: `-1` below 1.4, `0` from 1.4 to 2.0,
  and `+1` at or above 2.0. It is published in `readings.csv` and `report.json`
  and is readable as a strategy state; no event value changes.
- The SR2-BTC-FEATURES top-trade attribution tool and record: 139 decision-bar
  feature rows, five 1,000-candle charts, and the stack-aligned low-RVOL fade
  hypothesis. The 2019 loss persists in every filter subset.
- The SR3-FEATURES feature-filtered BTCUSD fade programme: eight 2018–2022
  variants, 40 base cells, and 30 9 bps sensitivity cells. The 1h sides are
  positive at 6 and 9 bps but have 9–21 trades per side; the 15m entry is
  negative; H4 long reaches 5.04%. Verdict: no candidate; 2024 not opened.
- The SR4-STACK-FADE multi-instrument programme: three 1h rules × BTCUSD,
  EURUSD, and XAUUSD × two sides × 2018–2022, 90 base cells. The pooled S1 sides
  are positive but BTC-only; EURUSD and XAUUSD are negative on every variant and
  side, and the BTC cells breach 5% drawdown. Verdict: no candidate; 9 bps and
  2024 not opened.
- The SR5 absorption-rejection programme and its count gate: 15m
  `signal.absorption_rejection` with a 1h EMA-stack context on BTCUSD. A1 has 5
  long and 8 short events over 2018–2022, below the 30-per-side gate; the
  event-only control is 15/17. Verdict: underpowered; no backtest, 9 bps, or
  2024 access.
- The SR6-VWAP programme: 15m `signal.vwap_rejection` with a 1h EMA-stack
  context on BTCUSD, 30 base cells. V1 long and short are negative with PF
  0.966/0.853 and every cell breaches 5% drawdown. Verdict: no candidate; 9 bps
  and 2024 not opened.
- The SR7-H3-CRYPTO programme: BTCUSD+ETHUSD H3 1h fade, 20 base cells. H3
  long is +$2,289 with PF 2.098 and 5/5 positive years but 27 trades and 5.62%
  drawdown; H3 short is PF 1.067 with 1/5 years and a negative ETH side.
  Verdict: no candidate; 9 bps and 2024 not opened.
- The SR8-H3-BUFFER programme: one-shot 2023 BTC buffer for the frozen H3 rule.
  2023 1h coverage is 61.3%, below the 95% gate; H3 long has 0 trades and H3
  short 4. Verdict: buffer ineligible; no no-edge conclusion; 2024 not opened.
- The SR8-A-H3-ARCHIVE check: the Dukascopy 2023 archive has 525,600 rows but
  38,948 flat zero-volume placeholders; only 61.4% of 1h hours are genuine. The
  archive is rejected, no H3 run is made, and 2024 remains closed.
- The SR9-15M-STRUCTURE campaign: 15m structure-transition on BTCUSD, XAUUSD,
  and EURUSD. All six primary cells are below 30 trades and five have PF below
  1.0; the break-only BTC long control is positive at 6 and 9 bps (30 trades,
  PF 1.746, max drawdown 2.87%) but is not promoted. Verdict: no candidate,
  underpowered; 2024 not opened.

### Changed

- `run.json` accepts an optional `model_timeframes` ladder. The model computes
  only the selected timeframes while the data engine still builds the full data
  ladder; the selected timeframe's model output is unchanged. The SR2 cells
  dropped from minutes to seconds.

### Fixed

- The SR9 data configs resolve the source path from `config/sr9/data` with three
  parent levels; the previous path skipped one too many and could not rerun.
- The event-composite strategy accepts registered signal subtypes above 7; it
  previously rejected `signal.vwap_rejection` (subtype 14) even though the
  registry marks it a signal. Diagnostics remain rejected.
- The SR5 absorption count gate now filters on the unique event code 52; the
  first gate also matched `signal.absorption_break` (code 51). The corrected A1
  count is 5 long / 8 short; the underpowered verdict is unchanged.
- The model condition evaluator accepts float `min`/`max` bounds for continuous
  readings such as `rvol_ratio` and `atr`; integer `values` is unchanged.
- The strategy event snapshot selects its event window by binary search over
  the availability-sorted event batch instead of rescanning the full batch on
  every execution bar. The selected events are unchanged; a one-year 1-minute
  cell no longer spends its runtime in a quadratic scan.

## [0.6.0] - 2026-11-07

### Added

- Workflow index, tier runbooks for model research, model change, strategy
  research, and portfolio research, and engineering workflows for requirements,
  release/operations, deprecation, and recordkeeping.
- Workflow and record templates, including the workflow skeleton, policy change,
  data change, release, deprecation, and requirements records.
- Workflow, changelog, and partition checkers, with CI coverage.
- Current improvement run adapter (`tools/improvement/run_current.py`) and the
  `quantrule.improvement-run/v2` manifest.
- Issue templates for task, audit, refactor, research programme, data, policy,
  release, deprecation, and remediation work.
- Workflow lab tutorial.

### Changed

- `AGENTS.md`, `README.md`, and `docs/README.md` now describe the current S3
  command surface and point to the workflow index.
- The improvement run contract is `quantrule.improvement-run/v2`; v1 is
  historical.
- `data/partitions.json` uses canonical instrument IDs with aliases for the
  historical research configs.
- The improvement evidence extractor reads the current analytics, portfolio,
  qualification, and model bundles.

### Fixed

- Removed stale pre-S3 status claims from the workflow, architecture, policy, and
  model documentation.
- The improvement run adapter invokes `backtest run`, `qualify`, and
  `analytics evaluate` instead of the retired `backtest` command.

### Removed

- No runtime surface was removed in this change.

## Release rules

- `## [Unreleased]` accumulates entries until a release.
- A released section is `## [<version>] - YYYY-MM-DD`.
- Entries are grouped under `Added`, `Changed`, `Deprecated`, `Removed`,
  `Fixed`, or `Security`.
- One entry per meaningful user-facing or developer-facing change, not one per
  pull request. Internal refactors, test-only changes, CI-only changes, and typo
  fixes do not need an entry.
- A pull request that does not change user-facing or developer-facing behavior
  records "no changelog entry needed" and the reason in its description.
- Released sections are immutable. A correction is a new release or a dated
  addendum in the next release.
- A release tag `vX.Y.Z` has a matching `## [X.Y.Z] - YYYY-MM-DD` section and a
  release note at `docs/releases/<YYYY-MM-DD>/vX.Y.Z.md`.
- Published tags are immutable. A bad release gets a new tag; a published tag is
  never moved, deleted, or reused.
- The changelog is not a duplicate of the GitHub Project, the AI transparency
  log, or the audit/refactor/improvement records. Each layer answers a different
  question.
