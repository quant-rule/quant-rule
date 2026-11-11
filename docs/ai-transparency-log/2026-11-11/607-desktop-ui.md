# AI Transparency Log — Tauri and Svelte Desktop UI

- Date: 2026-11-11 (UTC)
- Work item: `607-desktop-ui`
- Issue: [#607](https://github.com/quant-rule/quant-rule/issues/607)
- Plan: [`docs/execution-plans/2026-11-11/607-desktop-ui.md`](../../execution-plans/2026-11-11/607-desktop-ui.md)
- Tool: OpenCode agent (DeepSeek V4.1 Flash)
- Branch: `feat/607-desktop-ui`
- Claim type: Problem Claim

## Intent and boundary

Build the local desktop surface over `quantrule-app`. The app covers workspace,
data, configs, model run, research memory, export/backup, and settings. It has no
account, network path, or domain computation. Backtest, strategy, execution,
portfolio, and analytics remain CLI operations.

## Work performed

- Added `apps/desktop` with Tauri 2, Svelte 5, TypeScript, and Vite.
- Added one Tauri command per application-service operation: workspace, dataset,
  run check, model run, export, memory status/CRUD, record, export/import, and
  backup/restore.
- Added the Svelte shell and panels for Workspace, Data, Configs, Run, Memory,
  and Settings.
- Added a deterministic browser fixture backend for frontend and Playwright use.
- Added a standalone Tauri crate, excluded from the root Cargo workspace so the
  engine gate does not require GUI libraries.
- Added frontend unit tests, a Playwright browser E2E flow, Tauri backend tests,
  a native Xvfb smoke command, and a CI desktop job.
- Added the desktop README, root README section, architecture references,
  changelog, and this log.

## Verification

- `npm run check`: 0 errors, 0 warnings.
- `npm test`: 7 tests pass.
- `npm run test:e2e`: 1 Playwright flow passes.
- `cargo test --manifest-path src-tauri/Cargo.toml`: 4 tests pass.
- `npm run tauri build -- --no-bundle`: release binary built.
- `xvfb-run -a timeout 15 ./src-tauri/target/release/quantrule-desktop`: exit
  124 after the window stayed open until the timeout.
- The native dependencies were installed by the human after the preflight found
  GTK, WebKitGTK, libsoup, librsvg, appindicator, patchelf, and Xvfb missing.
- Root Rust and repository gates: 1,476 passed, 0 failed, 8 ignored; command surface, links, terms, legacy, structure, workflows, changelog, releases, partitions, registry, validation, verification, file size, claims, and execution plans pass.

## Learning

- Vite 8's `minify: "esbuild"` requires a separately installed esbuild; the
  default minifier avoids that extra dependency.
- TypeScript 7 conflicts with `svelte-check` 4's peer range; TypeScript 6 is the
  compatible choice.
- Svelte 5 component tests need the browser resolve condition in Vitest,
  otherwise `mount` resolves to the server build.
- Playwright's strict locators found real ambiguity in the UI text; the tests use
  exact roles and labels.
- The browser E2E uses a fixture backend, not the native window. The native smoke
  proves the Tauri shell launches, but it does not drive the UI. A WebDriver E2E
  remains a Phase 3.6 item.
- No Quant Validator record is required: no model, strategy, execution, or
  portfolio behavior changed.
