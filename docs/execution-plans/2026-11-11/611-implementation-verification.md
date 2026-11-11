# P3.6 — Implementation Verification

- Date: 2026-11-11 (UTC)
- Issue: [#611](https://github.com/quant-rule/quant-rule/issues/611)
- Parent plan: [`588-tauri-platform.md`](588-tauri-platform.md)
- Status: In Review
- Claim type: Behaviour Claim
- Owner: Human
- Workflow: Execution Plan

## Intent

Verify the Tauri Platform implementation against the approved plan before release.
The pass covers the Rust and frontend gates, the Linux bundles, a clean Debian
install, offline launch, determinism, research-memory backup/restore, and the
plan verification table. It records evidence and residual risks; it does not add
features.

## Current state

Phase 3.1–3.5 are merged at `6190a4c`. The desktop app builds a `.deb` and
`.AppImage`; the static page and checksums pass. Podman, `bwrap`, `WebKitWebDriver`,
and the Tauri native libraries are available. `dpkg-deb` is not installed on the
host, so the clean install runs in an Ubuntu 24.04 container.

## Scope

Included:

- Full Rust workspace gate: fmt, clippy, tests, docs.
- Desktop gate: frontend check/unit/E2E, backend tests/lints, native build.
- Rebuild the 0.9.0 bundles and run the site link/checksum check.
- Clean `.deb` install in an Ubuntu 24.04 container and Xvfb launch.
- AppImage launch with the network namespace removed.
- CLI and desktop model determinism checks.
- Research-memory export/import and backup/restore checks.
- Fill the parent plan's P3A–P3E verification rows.
- Write the verification report and AI log.

Excluded:

- A tag, GitHub release, or deployment. Those remain Phase 4.
- Windows, macOS, ARM, or a cloud runtime.
- New features or model/strategy changes.
- A WebDriver-driven native UI flow; the browser E2E and native launch smoke are
  the current desktop evidence.

## Assumptions and uncertainty

- The Ubuntu 24.04 container can install the `.deb` and its WebKitGTK/GTK
  dependencies from its configured repositories.
- The container can run Xvfb and the app with software rendering.
- `bwrap --unshare-net` can launch the AppImage with the host filesystem and the
  X11 socket, proving the app starts without a network namespace.
- The AppImage is large (111 MiB); the verification does not commit it.

## Decisions

| Decision | Choice | Why |
|---|---|---|
| Clean install | Ubuntu 24.04 podman container | The host is Fedora and has no `dpkg`; the deb targets Debian/Ubuntu. |
| Offline launch | `bwrap --unshare-net` | Proves the app starts without network access. |
| Determinism | Repeat CLI and desktop runs; compare bytes/JSON | Direct evidence for section 12. |
| Memory | Existing memory tests plus an export/import/backup script | Proves the Phase 3.3 contract. |
| Native UI E2E | Browser E2E + Xvfb smoke | WebDriver setup is deferred; the limitation is recorded. |

## Acceptance criteria

- The full Rust and desktop gates pass.
- The `.deb` installs in a clean Ubuntu 24.04 container and launches under Xvfb.
- The AppImage launches with the network namespace removed.
- Repeated model runs produce identical output.
- Memory export/import and backup/restore round-trip.
- The parent plan P3A–P3E rows carry observed results.
- The verification report records commands, results, and limitations.

## Work breakdown

| Step | Work | Role | Paths |
|---:|---|---|---|
| 1 | Run the Rust and desktop gates | Test | repository |
| 2 | Rebuild bundles and check the site | Release | `apps/desktop`, `site/` |
| 3 | Clean deb install in Ubuntu | Release | container |
| 4 | Offline AppImage launch | Test | `bwrap` |
| 5 | Determinism and memory round-trips | Test | CLI, app tests |
| 6 | Update the parent plan and write the report/AI log | Implementation | `docs/` |

## Verification table

Commands are run from the repository root unless noted.

| ID | Plan item | Acceptance criterion | Evidence / command | Expected | Observed | Status | Reviewer | Date | Notes |
|---|---|---|---|---|---|---|---|---|---|
| V1 | Rust gate | fmt, clippy, test, doc pass | cargo fmt/clippy/test/doc | pass | 1,476 passed, 8 ignored; all pass | Pass | Agent | 2026-11-11 | Human review pending |
| V2 | Desktop gate | check, unit, E2E, backend, build pass | npm/cargo commands | pass | 0 errors; 7 unit; 1 E2E; 4 backend; build pass | Pass | Agent | 2026-11-11 | Human review pending |
| V3 | Bundles | deb and AppImage build | `npm run tauri build` | pass | 4.33 MiB deb; 111.40 MiB AppImage | Pass | Agent | 2026-11-11 | Human review pending |
| V4 | Site | links and checksums match | `./site/build.sh --skip-build` | pass | 2 artifacts, 2 checksums | Pass | Agent | 2026-11-11 | Human review pending |
| V5 | Clean install | deb installs and launches | Ubuntu container + Xvfb | launch | installed; exit 124 | Pass | Agent | 2026-11-11 | Container install |
| V6 | Offline | AppImage starts without network | `bwrap --unshare-net` | launch | exit 124 | Pass | Agent | 2026-11-11 | Network namespace removed |
| V7 | Determinism | repeated runs are identical | CLI/desktop repeat | identical | model bundles and run check identical | Pass | Agent | 2026-11-11 | Human review pending |
| V8 | Memory | export/import and backup/restore | memory tests | round-trip | 22 tests pass | Pass | Agent | 2026-11-11 | Human review pending |
| V9 | Plan table | P3A–P3E observed | plan review | complete | P0–P3E Pass | Pass | Agent | 2026-11-11 | P4/P5 pending |

## Risks and recovery

| Risk | Control | Recovery |
|---|---|---|
| Container cannot install WebKitGTK | Install the declared dependencies explicitly | Record the blocker and stop. |
| Xvfb cannot render WebKit | Use software rendering and a longer timeout | Record the limitation; native smoke on the host remains. |
| AppImage needs FUSE | Use `APPIMAGE_EXTRACT_AND_RUN=1` | Document the requirement. |
| Determinism differs | Compare artifacts; stop and report | Treat as a defect, not a pass. |
| Plan rows incomplete | Fill every P3A–P3E row | Do not mark the plan Verified. |

## Stop conditions

Stop and ask if:

- the clean install or offline launch cannot be completed;
- a determinism or memory round-trip fails;
- a new dependency or feature is required to make the verification pass;
- the plan's acceptance criteria conflict with the evidence.
