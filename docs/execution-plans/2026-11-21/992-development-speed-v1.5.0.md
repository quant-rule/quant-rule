# Development Speed and v1.5.0

- Date: 2026-11-21 (UTC)
- Status: Approved
- Workflow: [Execution Plan](../../workflows/execution-plan.md)
- Epic: #992
- Issues: #993–#1031
- Release: v1.5.0
- Claim type: Behaviour Claim

## 1. Intent

Remove the 11/18-minute desktop release path, make the browser the only GUI, keep
the headless CLI, accelerate CI, remove Python, and delete only retired or
duplicated tests. Ship the result as v1.5.0.

The product behavior is unchanged. The desktop deployment profile, Tauri shell,
installers, and macOS support are retired.

## 2. Scope

### Included

- Delete `apps/desktop-shell/**` and every installer, AppImage, and NSIS path.
- Remove `QR_PROFILE=desktop`, SQLite, `rusqlite`, the filesystem CAS, and the
  in-process queue.
- Keep the browser GUI and `crates/quantrule-shell`; no browser console.
- Windows and Linux browser clients; Linux and Windows CLI.
- Build release binaries once on `main`; assemble a thin image at release.
- Port live Python governance to `qr-governance`; delete 66 historical files.
- Audit tests; remove only retired or duplicated coverage.
- Release v1.5.0 with an image-only release.

### Excluded

- macOS support, packages, runners, and CLI builds.
- Desktop GUI applications, installers, and Tauri source.
- Browser terminal or in-browser CLI.
- Model, strategy, execution, portfolio, or server behavior changes.
- Deleting live tests to hit a time target.

## 3. Decisions

| ID | Decision |
|---|---|
| D1 | Browser GUI + headless CLI only; no desktop app or browser console |
| D2 | macOS unsupported in v1.5.0 |
| D3 | CLI Linux on every PR; Windows at the release gate |
| D4 | Build six service binaries once on `main`; thin image at release |
| D5 | PR jobs timeout at 10 minutes; release jobs at 15; release target 6–8 |
| D6 | 26 Rust checkers after desktop retirement; no Python in the gate |
| D7 | Delete historical Python; no archive; Git history is recovery |
| D8 | Test deletion requires per-batch human approval |

## 4. Work breakdown

| Phase | Issues | Work |
|---|---|---|
| 0A | #993 | Issue tree, board fields, dependencies |
| 0B | #994–#999 | Baseline, four ADRs, guardrails, AI log |
| 1 | #1000–#1005 | Delete Tauri, image-only release, retire embedded profile, coverage migration, browser/CLI docs, Firefox smoke |
| 2 | #1006–#1014 | Caches, one gate, release dedupe, thin image, sccache, split/shard, timeouts, release-binaries |
| 3 | #1015–#1022 | Zero-Python inventory, harness, checkers, operations, research, historical deletion, policy |
| 4 | #1023–#1025 | Timing, classification, deletion batches |
| 5 | #1026 | Final gate and release dry run |
| 6 | #1027–#1031 | Version, changelog, artifact/thin image/composed/CLI gate, tag, cleanup |

## 5. Client and release contract

| Surface | v1.5.0 |
|---|---|
| Browser GUI | Windows and Linux |
| CLI | Linux and Windows, source-built |
| macOS | Unsupported |
| Desktop app | None |
| Installers | None |
| Browser console | None |
| Server | Linux image and Compose |
| Release assets | Image digest and release note |

The CLI is a client, not a server component. It holds no engine, database, queue,
or model. The release gate runs the Linux and Windows CLI matrix before publish.

## 6. Release build

1. `main` builds the six `qr-*` binaries in the pinned toolchain.
2. It uploads `release-binaries-<sha>` with a SHA-256 manifest and provenance.
3. The release gate downloads the artifact for the exact tag SHA.
4. `image-assemble` copies the binaries into the thin runtime image.
5. `image-verify` runs the composed suite and records the digest.
6. `publish` runs only after the composed suite passes.

A missing or mismatched artifact fails the release. There is no silent rebuild.

## 7. Acceptance criteria

- No Tauri, installer, desktop profile, SQLite, `rusqlite`, or macOS claim.
- Browser and CLI remain on the 49-command contract.
- Zero Python files and zero `python3` in the gate, CI, release, and current docs.
- 26 Rust checkers replace the Python checker gate.
- Test deletions are only retired or duplicated coverage.
- PR `verify` target is 2–3 minutes; release critical path 6–8 minutes.
- v1.5.0 is tagged from `main`, image-only, `/version` reports 1.5.0.
- Historical releases and assets are untouched.

## Verification table

| ID | Plan item | Acceptance criterion | Evidence / command | Expected | Observed | Status | Reviewer | Date | Notes |
|---|---|---|---|---|---|---|---|---|---|
| V00 | P0A | Parent and all sub-issues exist on the board with claim types and dependencies | `gh issue list`; board fields; issue tree | Complete | verified locally | Pass |  |  |  |
| V01 | P0B | Baseline CI, release, desktop, cache, and test counts recorded | CI timings; test counts | Recorded |  | Pending |  |  |  |
| V02 | P1A | No Tauri shell, supervisor, or packaging in current tree | `test ! -e apps/desktop-shell`; `rg` for Tauri/AppImage/NSIS | No current reference | verified locally | Pass |  |  |  |
| V03 | P1B | Release has no desktop job or artifacts | Workflow diff; `rg` in `release.yml` | No match | verified locally | Pass |  |  |  |
| V04 | P1C | Embedded desktop profile and SQLite are gone | `rg` for `QR_PROFILE=desktop`, `rusqlite`, SQLite; `cargo tree` | No match | verified locally | Pass |  |  |  |
| V05 | P1E | Only browser GUI and headless shell remain | Client docs; `cargo test -p quantrule-shell` | Browser + shell only | verified locally | Pass |  |  |  |
| V05A | P6C | CLI builds and tests on Linux and Windows at the release gate | CLI matrix; `cargo test -p quantrule-shell`; `quantrule help` | Pass |  | Pending |  |  |  |
| V06 | P1E | Windows/Linux browser access documented; macOS unsupported | Platform matrix; workflow diff | Documented; no macOS job | verified locally | Pass |  |  |  |
| V07 | P1B | Historical releases untouched | Compare tags and assets | Unchanged | verified locally | Pass |  |  |  |
| V08 | P1 | Phase 1 full gate green | `./tools/gate.sh` | Pass | verified locally | Pass |  |  |  |
| V09 | P2A | Cargo cache warm and under 10 GB | Two runs; `gh cache list` | Verified |  | Pending |  |  |  |
| V10 | P2B | CI uses `tools/gate.sh` | CI run; gate diff | Verified |  | Pending |  |  |  |
| V11 | P2C | Release does not duplicate a green CI on the same SHA | Release run; exact-SHA check | Verified |  | Pending |  |  |  |
| V12 | P2D | Rustup, Playwright, Bun caches hit | CI timings | Verified |  | Pending |  |  |  |
| V13 | P1F | Firefox smoke passes; Chromium full suite passes | Cross-browser job | Pass | verified locally | Pass |  |  |  |
| V14 | P2E | Thin image assembly; release critical path 6–8 min | Release timing | Verified |  | Pending |  |  |  |
| V15 | P2F | sccache improves or is reverted | Two runs; hit rate | Verified/reverted |  | Pending |  |  |  |
| V16 | P2G | Split/shard beats one cached job or is reverted | A/B timing | Verified/reverted |  | Pending |  |  |  |
| V17 | P2H | PR jobs timeout at 10; release jobs timeout at 15; no unexpected failure at 11–12 | Workflow diff; run | Verified |  | Pending |  |  |  |
| V18 | P2 | Phase 2 full gate green | `./tools/gate.sh` | Pass |  | Pending |  |  |  |
| V19 | P3A | Post-retirement Python inventory approved | Inventory and deletion list | Approved |  | Pending |  |  |  |
| V20 | P3B–P3D | 26 checkers match the Python oracle | `check --all` parity | Pass |  | Pending |  |  |  |
| V21 | P3E | Operational tools match | Fixtures/tests | Pass |  | Pending |  |  |  |
| V22 | P3F | Research tools match to 1e-9 | Golden fixtures | Pass |  | Pending |  |  |  |
| V23 | P3G | 66 historical files deleted; no live reference | `find`; `rg`; `git log` | Pass |  | Pending |  |  |  |
| V24 | P3H | No `.py`; gate passes without Python | `find`; `PATH` test; image check | Pass |  | Pending |  |  |  |
| V25 | P4A | Per-test timing and classification recorded | nextest report; audit table | Pass |  | Pending |  |  |  |
| V26 | P4C–P4D | Only retired/duplicated tests removed; time measured | Deletion batches; approval | Pass |  | Pending |  |  |  |
| V27 | P5A | Final gate, web check/build/e2e, cross-browser | `gate.sh`; `bun`; cross-browser | Pass |  | Pending |  |  |  |
| V28 | P6E | Issues, board, worktrees, branches closed | `gh`; `git worktree list`; `git branch -a` | Pass |  | Pending |  |  |  |
| V29 | P1E | CLI source-build path preserved and documented; no installer | `cargo build --release -p quantrule-shell`; docs | Pass | verified locally | Pass |  |  |  |
| V30 | P1E | Client surface is browser GUI + terminal CLI; no browser console, no desktop app | Architecture docs; command surface; workflow diff | Matches GitHub model | verified locally | Pass |  |  |  |
| V31 | P6A–P6B | Version 1.5.0, changelog, and release note present | `Cargo.toml`; `Cargo.lock`; `apps/web/package.json`; `CHANGELOG.md`; release note | Consistent |  | Pending |  |  |  |
| V32 | P6C–P6D | Annotated v1.5.0 on main; Linux/Windows CLI matrix pass; image-only release; `/version` 1.5.0; `/health` digest; no desktop assets | `git cat-file`; `gh release view`; `/version`; `/health` | Pass |  | Pending |  |  |  |
| V33 | P6E | Board Done, worktree and local/remote branches removed | `gh`; `git worktree list`; `git branch -a` | Pass |  | Pending |  |  |  |
| V34 | P2I | Release binaries come from the exact tag SHA; manifest and SHA-256 checks pass; image assembly compiles no Rust | Artifact manifest; `sha256sum`; workflow log | Pass |  | Pending |  |  |  |
| V35 | P6C | Release critical path 6–8 min; every release job timeout 15; no failure at 11–12 | Release timings; workflow diff | Pass |  | Pending |  |  |  |
