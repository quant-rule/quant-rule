# Web Client

- Status: Current; product 1.4.1, current tag `v1.4.1`; approved 2026-11-12
- Part of: [`README.md`](README.md)
- Decision: [`ADR-2026-11-12-05`](../adr/2026-11-12/05-svelte-only-web-client.md)
- Claim type: Behaviour Claim

The client is a Svelte 5 single-page application served by the gateway at the same origin as the API; it holds no domain logic and no source of truth.

The gateway serves the built assets from `QR_STATIC_DIR`. A path outside that
root is refused, an unknown page falls back to `index.html`, and an `/api/*`
path never falls back to the client.

## 1. Screens

| Screen | Component | Shows | Empty state |
|---|---|---|---|
| Connection | `ConnectionPanel.svelte` | Gateway state, version, image digest, service health, bearer token | `unreachable` or `token required` with a retry |
| Profiles | `ProfilesPanel.svelte` | Profile list, counts, create; the top-bar `ProfileSwitcher.svelte` selects the active profile | "No profiles. Create one." |
| Workbench | `WorkbenchPanel.svelte` | Data, Model, Strategy, and Portfolio tabs; the tabs render `DatasetsPanel.svelte`, `ModelStagePanel.svelte`, `StrategyStagePanel.svelte`, and `PortfolioStagePanel.svelte` | "No stage runs. Submit a model stage above." |
| Datasets | `DatasetsPanel.svelte` | `data_version`, candles, first and last bar, upload, record detail with object key, hash, quality, and created time | "No datasets. Upload a CSV." |
| Configs | `ConfigsPanel.svelte` | Name, origin (repo or uploaded), validity, parser message, optional frozen model artifact, submit, record detail with config id and hash | "No configs on the server." |
| Runs | `RunsPanel.svelte` | Status, id, config, data version, submitted, duration, cancel | "No runs. Submit one from Configs." |
| Run detail | `RunDetail.svelte` | Stages, provenance, the frozen model artifact input, canonical digest with Verify, artifacts with verified download | Only for a known run |
| Research | `ResearchPanel.svelte` | Improvement-run table, v3 create form, detail, submit, comparison, explicit decision, baseline table/detail, and invalidation list | "No improvement runs." |
| Operations | `OperationsPanel.svelte` | Connection, product, API, health, image digest, active profile, token presence, retention days, last cleanup counts and error, and the run verification form | "Retention status is unavailable." |
| Inspect | `InspectPanel.svelte` | Candles, timeframe ladder, and event batch for one dataset; read-only | "No datasets to inspect." |
| Simulation | `SimulationRunsPanel.svelte` | `sim_id`, premise, bundle, windows, marked/unmarked, status | "No simulation runs." |
| Simulation run | `SimulationRunDetail.svelte` | Run state and requests from `simulationRun.svelte.ts`; window navigator, chart, review, and the marks, check, and decision panels | Only for a known run |
| History | `HistoryPanel.svelte` | Kind tabs, archived filter, archive, and typed-confirmation erase | "No run records." |
| Help | `HelpPanel.svelte` | The 49 registered commands and the screen that reaches each use case | — |

Every screen is owned by one component. The browser and the command shell reach one HTTP
contract; the shared fixture and both parity tests enforce one browser control per command.
## 2. Stores

| Store | Holds | Polling |
|---|---|---|
| `connection.svelte.ts` | gateway state, version, health, token presence | on demand |
| `profiles.svelte.ts` | profile list and active id | on demand |
| `datasets.svelte.ts` | profile datasets | on demand |
| `configs.svelte.ts` | profile configs and the last submission | on demand |
| `runs.svelte.ts` | profile run list, selected run, and detail | 2 s active, 30 s idle |
| `simulation.svelte.ts` | profile simulation summaries | on demand |
| `simulationRun.svelte.ts` | one run's windows, series, marks, and action state | on demand |
| `stages.svelte.ts` | profile stage list, selected stage detail, the event/coverage view, the strategy decision explain view, and the published portfolio bundles | on demand |
| `research.svelte.ts` | profile improvement runs, selected detail, comparison, baselines, and invalidations | on demand |
| `operations.svelte.ts` | retention days, last cleanup, stage-store presence, and load state | on demand |
| `profileScope.svelte.ts` | the switch, clear, and reload boundary for the profile stores | — |
| `labels.generated.ts` | numeric code → word | static, generated |
Each store has one server resource. No store caches a result for offline use.
## 3. Transport

One function keeps the existing shape:

```ts
call<T>(path: string, init?: RequestInit): Promise<T>
```

`callVoid` covers a `204`, and `fetchBytes` returns an artifact body after the
same error-envelope check. The profile header is attached in one place.

- Production: `fetch` against `/api/v1`.
- Test: an HTTP fixture router keyed by path and method; no socket.
- The mock backend is the only test seam and must not diverge from the contract.

## 4. Disconnection and profile scope

Selecting a profile, or losing the gateway, clears every profile-scoped row
before the next read: datasets, configs, runs, simulations, stages, and research.

- The connection pill turns red on `unreachable`, `unauthorized`, or `mismatch`.
- The runs list shows `disconnected`, not stale rows.
- An open run detail, simulation, stage, or research selection is cleared.
- Submission is disabled.
- No result is read from local storage.

Each store carries a generation counter, so a read that started under the
previous profile is dropped when it returns. A tool whose value is "these bytes
are the truth" must not present stale evidence. Only connection settings persist
locally.

## 5. Numbers, not labels

The gateway returns codes. The client maps them using
`labels.generated.ts`, produced from the Rust registry and checked by the gate.
A code with no mapping renders as `unknown(<code>)`, never as a guess.

## 6. Chart and marking

The chart is a **client-side SVG renderer**, not a library and not a server PNG.

| Element | Source |
|---|---|
| Candles, volume | `series` |
| EMA 20/50/200 | `series` |
| One indicator panel | `series`, chosen from the run premise |
| Event markers, confirmed levels | `series` |
| Marks | `marks`, human and oracle |

- Pointer position maps to a bar through the rendered scale; no geometry manifest
  and no pixel detection.
- Keys `1`–`8` select the eight mark classes: entry/exit × good/bad × long/short.
- `Z` undoes the last mark in the session; the server is the record.
- A note is optional and capped at 500 characters.
- The chart is deterministic for a fixed series: the same window draws the same
  path data. Marks add to it; they never change it.

The server never renders an image. Export is an SVG download from the client.

## 7. Config origin

The list merges the read-only repository mount with uploaded configs. The origin
is shown as a label, never as a path. A repository config updates when the file
changes and the data service reloads; it is not editable in the client.

## 8. Upload

The client streams the CSV to `POST /catalog/datasets` and displays progress.
A "Skip bad rows" control uses `POST /catalog/datasets/tolerant` and shows the
quality report. The server owns the limit, the hash, and the catalog row. The
client never constructs a `data_version`.

## 9. Desktop shell

`apps/desktop-shell/` is a Tauri 2 window over the gateway URL. The package also
carries a dependency-free `qr-desktop` supervisor: the shell starts it once from
the resource directory, the supervisor starts the six services on loopback,
waits for health, restarts a crashed child, and stops them when its control
socket closes. The shell still has no IPC domain.

| It does | It does not |
|---|---|
| Load the gateway URL | Embed an engine |
| Remember the active profile | Read a local data file |
| Check for a new release | Own an IPC command |
| Start one packaged supervisor | Manage a service or a port |

It is a window onto the Svelte client, not a second client. When the gateway is
unreachable it shows the disconnected state, exactly like the browser, after
probing the supervisor's 8080–8089 range.

The shell persists one setting, the gateway URL. The active profile remains the
web client's `quantrule.profile` setting and its `X-QuantRule-Profile` header.
The embedded backends are #693/#694; build, supervisor tests, the package check,
and the unsigned-artifact warning are in
[`apps/desktop-shell/README.md`](../../apps/desktop-shell/README.md).

## 10. Accessibility and determinism

- Every action is reachable by keyboard and announced by role.
- Status is text plus colour, never colour alone.
- Rendered order is fixed; no timestamp is rendered from the client clock.
- The same fixture produces the same DOM in Playwright.

## 11. Toolchain

| Concern | Choice |
|---|---|
| Package manager | **Bun** — `bun install` |
| Build | `bun run build` (Vite) |
| Dev server | `bun run dev`, proxying `/api` to `localhost:8080` |
| Unit tests | the Vite test runner under Bun |
| End-to-end | Playwright |

Bun is the client toolchain only. The built assets are static and served by the gateway; no Bun runtime exists in the image.

## 12. Out of scope

No offline mode, no native shell, no local file access beyond upload, no charting library, and no client-side model value or check arithmetic.
