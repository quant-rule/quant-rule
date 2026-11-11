# Tauri Platform — Implementation Verification

- Date: 2026-11-11 (UTC)
- Issue: [#611](https://github.com/quant-rule/quant-rule/issues/611)
- Plan: [`docs/execution-plans/2026-11-11/611-implementation-verification.md`](../../execution-plans/2026-11-11/611-implementation-verification.md)
- Parent plan: [`docs/execution-plans/2026-11-11/588-tauri-platform.md`](../../execution-plans/2026-11-11/588-tauri-platform.md)
- Claim type: Behaviour Claim

## Verdict

Phase 3 passes its implementation gate. The Rust workspace, desktop frontend and
backend, Linux bundles, static page, clean Debian install, offline AppImage
launch, determinism, and research-memory round-trips all pass. The parent plan's
P0–P3E rows are `Pass`. P4 (course PDFs) and P5 (final closure) remain open.

## Environment

| Item | Value |
|---|---|
| Host | Fedora Linux 44, x86_64 |
| Rust | 1.99.1 |
| Node / npm | 22.23.1 / 10.9.8 |
| Native UI | GTK 3.24, WebKitGTK 4.1, libsoup 3 |
| Clean install | `ubuntu:24.04` podman container |
| Offline launch | `bwrap --unshare-net` |
| Artifacts | `QuantRule_0.9.0_amd64.deb`, `QuantRule_0.9.0_amd64.AppImage` |

## Gate results

| Check | Command | Observed |
|---|---|---|
| Format | `cargo fmt --all -- --check` | pass |
| Lints | `cargo clippy --workspace --all-targets -- -D warnings` | pass |
| Tests | `cargo test --workspace` | 1,476 passed, 0 failed, 8 ignored |
| Docs | `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps` | pass |
| Frontend check | `npm run check` | 0 errors, 0 warnings |
| Frontend unit | `npm test` | 7 passed |
| Browser E2E | `npm run test:e2e` | 1 passed |
| Desktop backend | `cargo test --manifest-path src-tauri/Cargo.toml` | 4 passed |
| Desktop lints | `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | pass |
| Bundles | `npm run tauri build` | `.deb` 4.33 MiB, `.AppImage` 111.40 MiB |
| Site | `./site/build.sh --skip-build` | 2 artifacts, 2 checksums |
| Repository checks | command surface, links, terms, legacy, structure, workflows, changelog, releases, partitions, registry, validation, verification, file size, claims, plans | pass |

## Clean Debian install

The host has no `dpkg`, so the install ran in an Ubuntu 24.04 container:

```bash
podman run --rm -v "$PWD/site/downloads:/artifacts:ro,Z" ubuntu:24.04 \
  bash -lc 'apt-get update -qq; apt-get install -y -qq xvfb; \
  dpkg -i /artifacts/QuantRule_0.9.0_amd64.deb; \
  apt-get install -f -y -qq; xvfb-run -a timeout 10 quantrule-desktop'
```

`dpkg -i` reported the expected missing dependencies, `apt-get install -f`
resolved them, `/usr/bin/quantrule-desktop` was installed, and the app stayed
open under Xvfb until the timeout (`124`). The EGL DRI3 warnings are the expected
software-rendering messages.

## Offline launch

```bash
xvfb-run -a bwrap --unshare-net --dev-bind / / \
  --setenv APPIMAGE_EXTRACT_AND_RUN 1 --setenv DISPLAY "$DISPLAY" \
  -- timeout 10 ./site/downloads/QuantRule_0.9.0_amd64.AppImage
```

The AppImage stayed open until the timeout (`124`) with the network namespace
removed.

## Determinism

| Check | Command | Observed |
|---|---|---|
| Model bundles | `report model` twice, then `diff -r` | byte-identical |
| Run check | `run check` twice, then `cmp` | byte-identical |
| Service model run | `cargo test -p quantrule-app service::tests` | repeated runs equal |

## Research memory

`cargo test -p quantrule-app memory::` passes 22 tests, including schema,
migrations, CRUD, null/integer metrics, foreign keys, ID reuse, deterministic
export, export/import, and backup/restore.

## Parent plan rows

P0–P3E are `Pass` in
[`588-tauri-platform.md`](../../execution-plans/2026-11-11/588-tauri-platform.md).
P4 and P5 remain `Pending`.

## Limitations and residual risks

- The browser E2E drives the Vite build with a fixture backend, not the native
  window. The native launch smoke proves the Tauri shell starts, but it does not
  drive the UI. A WebDriver E2E remains a Phase 3.6 follow-up if the human wants
  native interaction coverage before release.
- The clean install runs in a container, not a bare-metal Ubuntu machine. The
  package install and launch are real; hardware GPU rendering is not exercised.
- No tag or GitHub release exists. Phase 4 publishes the artifacts after human
  acceptance.
- The desktop UI does not run backtests or analytics; those remain CLI
  operations until the application service owns them.

## Evidence locations

- Plan: `docs/execution-plans/2026-11-11/611-implementation-verification.md`
- Parent plan: `docs/execution-plans/2026-11-11/588-tauri-platform.md`
- AI log: `docs/ai-transparency-log/2026-11-11/611-implementation-verification.md`
- Site check: `site/check.py`
- Desktop tests: `apps/desktop/src/**/*.test.ts`, `apps/desktop/e2e/app.spec.ts`
- Memory tests: `crates/quantrule-app/src/memory/tests.rs`
