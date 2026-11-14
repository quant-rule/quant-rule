# AI transparency log — DIST-104 simulation screens

- Date: 2026-11-14 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Intent: give the simulation research loop a first-class browser surface: a
  runs list with creation, and a run detail with the window navigator, the
  existing SVG chart, the mark toolbar, a human/oracle marks table, review,
  check, and a decision that locks marking.
- Boundary: presentation only. The work adds
  `apps/web/src/lib/SimulationRunsPanel.svelte`,
  `SimulationRunDetail.svelte`, `simulation.svelte.ts`, `simulationReview.ts`
  and its test, extends `simulation.ts` and its test, extends the chart with a
  lock and a per-window mark filter, wires the section in `App.svelte`, adds a
  Playwright suite, the changelog entry, and this log. No Rust, model,
  strategy, execution, portfolio, shell, charting library, or new dependency.
  The `simulation` service (#680) is not merged, so the screens follow
  `docs/architecture/http-api.md` section 6 and are exercised against a
  network mock at the contract boundary.

## Contract boundary

- The create body is `premise`, `bundle`, `inputs`, `seed`, and `criteria`, as
  the issue and `http-api.md` section 6 state. `quantrule-api` currently pins
  four fields (`CreateSimulationRequest` has no `inputs`); the seam is one file
  so #680 can confirm or rename the field without touching a screen.
- The list, detail, window, marks, oracle, check, and decision responses are
  local view types in `simulation.ts`. The normalizers accept a bare array or
  the `{ windows }` / `{ marks }` envelope the shell already tolerates, and a
  direct or `{ check }` / `{ decision }` check and decision record.
- The mark `chart` is a string in `quantrule-api` but a number in the merged
  chart. The seam normalizes a numeric string to the chart's number. A
  non-numeric window fails loudly instead of drawing a mark in the wrong
  window.
- No `GET /review` or `GET /check` exists in the contract, so the review
  section counts the server's mark rows by origin and the check result is
  session state. The service owns pairing, agreement, and check arithmetic;
  the client never recomputes them.

## Implementation

- `SimulationRunsPanel` lists runs with status, windows, marked windows, marks,
  and bundle; creates one from the five recorded fields; and shows the
  `No simulation runs.` empty state. A disconnected gateway hides the table and
  the create form instead of showing stale rows.
- `SimulationRunDetail` opens the run, calls the idempotent window selection,
  loads the first window, and navigates the other windows. It renders the
  chart, the marks table with a textual and coloured `human`/`oracle` origin,
  review counts, the check table and verdict, and the decision form.
- The chart is read-only for a decided run: the mark classes, undo, note, and
  the key handler are disabled, and a defensive guard refuses the call if a
  handler is reached anyway. A stale client still sees the server's
  `simulation.locked` refusal and keeps its note.
- `SimulationChart` now filters the scene by `mark.chart === series.chart`. The
  run owns one mark list across every window, so without the filter a mark from
  window 1 would draw on window 0. This is the one chart integration fix.
- Error paths clear evidence rather than keeping it: a failed marks read clears
  the table, a new check attempt clears the previous result, and a disconnected
  list clears its rows.

## Verification

| Command | Observed |
|---|---|
| `bun install --frozen-lockfile` | 54 packages installed |
| `bun run check` | 0 errors, 0 warnings |
| `bun test` | 100 passed, 0 failed, 14 files, 1,165 assertions |
| `bun run build` | 152 modules transformed; `dist/index.html`, one CSS and one JS asset |
| `bun run e2e` | 27 passed, including five simulation-screen tests |
| `tools/gate.sh` | passed, ending with `workspace gate passed with CARGO_TARGET_DIR=/home/zantgo/Proyectos/quant-rule--682-sim-screens/target/gate` |

The five browser tests cover: create → mark → oracle → review → check →
decide; a decided run opens locked and sends no mark; a server-locked mark is
refused and keeps the note; the no-windows/no-marks/no-check empty states; and
a disconnected gateway clearing the rows. The existing chart suite still passes
unchanged.

## Learning

- The chart scene had no window filter. The merged chart assumed one window's
  marks; the run has one list across windows, so the filter belongs in the
  chart where the scene is built rather than in a second marks array in the
  screen. This keeps the table and the chart on one list.
- The decision lock has two layers for different failures: the prop closes the
  toolbar for a decided run, and the server's 409 closes a stale tab that
  learned about the decision late. A UI-only lock would be defeated by a tab
  that was already open.
- The review endpoint is absent from the eleven documented simulation
  endpoints, while the workflow and `module-map.md` name review. Counting the
  returned rows is presentation and lets a human compare the two sets; pairing
  and agreement stay in `quantrule-simulation::review`, where the contract
  already puts them.
- The chart's mark badge counted every window's marks. It now counts the marks
  the scene draws, which is the only count the chart can honestly show.

## Rejected

- A second transport for the screens. Every call stays in `simulation.ts`.
- Client-side review pairing or check arithmetic. Both are service
  measurements; duplicating them would let the two clients drift.
- A production fixture fallback for a failed simulation read. The fixture is
  only the Playwright network mock, and a failed read shows an error or an
  empty state, never invented evidence.
- Gating check or decision on a `GET /check` that does not exist. The check
  result is session state; a reload of a checked run offers the idempotent
  check again rather than pretending to know its last table.
