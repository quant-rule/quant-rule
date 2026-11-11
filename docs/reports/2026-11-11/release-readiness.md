# Release Readiness — QuantRule Desktop 0.9.0

- Date: 2026-11-11 (UTC)
- Candidate: this Phase 4 PR on top of `a81ec0a`
- Status: Ready for human acceptance; tag pending
- Claim type: release readiness record; no edge or profitability claim.

## Scope

| Included | Excluded |
|---|---|
| Tauri 2 + Svelte 5 Linux desktop app | Windows and macOS |
| AppImage and `.deb` bundles | Auto-update |
| Static download page and SHA-256 checksums | Accounts, team cloud, sync |
| Local SQLite research memory | Cloud compute or cloud data |
| CLI backtest, strategy, execution, portfolio, analytics | UI backtest/analytics orchestration |

## Gate

| Check | Observed | Evidence |
|---|---|---|
| Rust gate | 1,476 passed, 0 failed, 8 ignored | `cargo test --workspace` |
| Frontend | 0 errors; 7 unit tests; 1 Playwright E2E | `npm run check`, `npm test`, `npm run test:e2e` |
| Desktop backend | 4 tests; clippy clean | `cargo test/clippy --manifest-path src-tauri/Cargo.toml` |
| Bundles | `.deb` 4.33 MiB; `.AppImage` 111.40 MiB | `npm run tauri build` |
| Clean install | Ubuntu 24.04 container; install and Xvfb launch | verification report |
| Offline | AppImage launch with `bwrap --unshare-net` | verification report |
| Determinism | Model bundles and `run check` byte-identical | verification report |
| Memory | 22 tests, including export/import and backup/restore | `cargo test -p quantrule-app memory::` |
| Site | 2 artifacts, 2 checksums | `site/check.py --require-artifacts` |
| Repository | command surface, links, terms, legacy, structure, workflows, changelog, releases, partitions, registry, validation, verification, file size, claims, plans | checkers |

The detailed evidence is in
[`docs/reports/2026-11-11/implementation-verification.md`](implementation-verification.md).

## Artifacts

| Artifact | Status |
|---|---|
| `QuantRule_0.9.0_amd64.deb` | Built; rebuilt at release and checksummed |
| `QuantRule_0.9.0_amd64.AppImage` | Built; rebuilt at release and checksummed |
| `site/checksums.txt` | Generated beside the staged bundles |
| `site/index.html` | Static page with install, requirements, and verify sections |

The artifacts are generated and not committed. The release upload attaches them
to the release; the page and checksums are staged from the same build.

## Tag preconditions

- [ ] Full gate passes on the release commit.
- [ ] `CHANGELOG.md` has `## [0.9.0] - 2026-11-11`.
- [ ] `docs/releases/2026-11-11/v0.9.0.md` exists with the tag, commit, scope,
      verification, risks, rollback, and changelog link.
- [ ] No Critical audit finding is open on the changed path.
- [ ] The human authorizes the annotated tag.
- [ ] The release assets and checksums are uploaded.
- [ ] The static page links the published assets.

The tag is not created by this phase.

## Residual risks

| Risk | Impact | Control |
|---|---|---|
| Native UI E2E gap | The browser E2E uses a fixture backend; the native smoke does not drive the UI. | Recorded; add WebDriver if release risk requires it. |
| Container-only clean install | No bare-metal Ubuntu install. | Recorded; the package install and launch are real in the container. |
| AppImage size | 111 MiB download. | Documented on the page. |
| System WebKitGTK | The AppImage relies on the system WebKitGTK/GTK. | Requirements on the page. |
| No auto-update | Users reinstall manually. | Explicit scope; add only if it does not delay the release. |
| Linux only | No Windows or macOS. | Future work. |

## Rollback

- Withdraw the release assets and page links.
- Point the page at the previous verified `v0.8.0` shell release.
- Keep the local research memory export/import path for recovery.
- A bad published tag is not moved; a new tag and note supersede it.

## Decision

The Phase 3 implementation is ready for human acceptance and a `v0.9.0` tag. The
tag, release note, and asset upload remain separate human-authorized operations.
