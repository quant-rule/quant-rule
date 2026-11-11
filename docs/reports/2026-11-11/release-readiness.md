# Release Readiness — QuantRule Desktop 0.9.0

- Date: 2026-11-11 (UTC)
- Release: `v0.9.0` at `57f5663`
- Status: Released
- Release note: [`docs/releases/2026-11-11/v0.9.0.md`](../../releases/2026-11-11/v0.9.0.md)
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

The bundles were rebuilt from the tagged commit, staged by `site/build.sh
--skip-build`, and uploaded to the GitHub release.

| Artifact | Size | SHA-256 | Status |
|---|---:|---|---|
| `QuantRule_0.9.0_amd64.deb` | 4,543,434 bytes | `b55a5ad900d745a45911d86dcd1eb1adeba198d514bc8f76af82dc4e5bd5d3ec` | Uploaded |
| `QuantRule_0.9.0_amd64.AppImage` | 116,812,280 bytes | `c7d9ad651db98775793205348ab1a7ddc4bf6911b28a0dd36249944700a29dbd` | Uploaded |
| `checksums.txt` | 209 bytes | — | Uploaded; matches the staged file |
| `site/index.html` | — | — | Static page with install, requirements, and verify sections |

The artifacts are generated and not committed. The static page links the staged
bundles; the GitHub release carries the same files and checksums.

## Publication

| Check | Observed |
|---|---|
| Annotated tag | `v0.9.0` at `57f5663f1f014237c006cd7815ed189f69f6aebc`; pushed |
| GitHub release | [Published](https://github.com/quant-rule/quant-rule/releases/tag/v0.9.0); not draft or prerelease |
| Assets | `.deb`, `.AppImage`, and `checksums.txt` attached |
| Checksum check | Downloaded `checksums.txt` matches `site/checksums.txt` byte for byte |
| Release note | `docs/releases/2026-11-11/v0.9.0.md` is the release body |
| Changelog | `## [0.9.0] - 2026-11-11` |

## Tag preconditions

- [x] Full gate passes on the release commit.
- [x] `CHANGELOG.md` has `## [0.9.0] - 2026-11-11`.
- [x] `docs/releases/2026-11-11/v0.9.0.md` exists with the tag, commit, scope,
      verification, risks, rollback, and changelog link.
- [x] No Critical audit finding is open on the changed path.
- [x] The human authorizes the annotated tag.
- [x] The release assets and checksums are uploaded.
- [x] The static page links the staged assets.

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

`v0.9.0` is released. The tag, release note, assets, and checksums are
published; the residual risks above remain recorded and accepted for this
release. A future release supersedes this one; the tag is never moved.
