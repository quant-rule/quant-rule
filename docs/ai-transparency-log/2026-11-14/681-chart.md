# AI transparency log — DIST-103 SVG chart and mark capture

- Date: 2026-11-14 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Intent: render the simulation window as deterministic client-side SVG and
  capture human marks with the keyboard, without a charting library or a server
  image.
- Boundary: presentation only. The work adds `apps/web/src/lib/chart/`, one
  simulation transport seam, one chart component, a fixture host, browser and
  unit tests, the changelog entry, and this log. No Rust, model, strategy,
  execution, portfolio, or shell behavior changed. The `simulation` service
  (#680) is not merged, so the series and mark calls follow
  `docs/architecture/http-api.md` section 6 and are exercised with a network
  mock; the exact series envelope remains a local view type until DIST-102
  fixes it.
- Verification: `bun install --frozen-lockfile` installed 54 packages;
  `bun test` passed 76 tests across 12 files; `bun run check` reported 0 errors
  and 0 warnings; `bun run build` produced the static assets; `bun run e2e`
  passed all 22 Playwright tests, including five chart tests; `tools/gate.sh`
  passed, ending with
  `workspace gate passed with CARGO_TARGET_DIR=/home/zantgo/Proyectos/quant-rule--681-chart/target/gate`.
  One earlier browser run hit five unrelated 30-second timeouts in the configs,
  profiles, and run-detail specs; a clean rerun passed 22/22, and none of those
  specs was modified.
- Learning: Svelte's static accessibility check does not treat
  `role="application"` as interactive, so the chart surface needed a narrow
  `svelte-ignore` with the reason stated at the element. The pointer mapping
  reads the rendered SVG rectangle and converts to viewBox coordinates; using a
  bordered wrapper instead would shift the left edge by the border width and
  break a half-pixel edge test.
- Rejected: a charting library. The issue forbids it, and the path builder is
  small enough to test directly. Rejected: a production fallback that renders a
  built-in fixture when the series request fails; that would present invented
  evidence. The fixture host is reachable only through
  `?fixture=simulation-chart` and still calls the real transport seam.
- Rejected: optimistic mark rendering before the server acknowledges it. The
  server is the record; a failed post keeps the selected class, bar, and note
  and shows the error instead of drawing an unaccepted mark.
