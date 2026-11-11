# P3.5 — Linux Distribution and Download Page

- Date: 2026-11-11 (UTC)
- Issue: [#609](https://github.com/quant-rule/quant-rule/issues/609)
- Parent plan: [`588-tauri-platform.md`](588-tauri-platform.md)
- Status: In Review
- Claim type: Outcome Claim
- Owner: Human
- Workflow: Release & Operations

## Intent

Ship the Linux desktop app as an AppImage and a `.deb`, with a static download
page that names the requirements, install steps, checksums, and release
boundary. The page is distribution only: no account, backend, or cloud runtime.

## Current state

`apps/desktop` builds a Tauri 2 release binary. Tauri is configured for `deb` and
`appimage` bundles. No `site/` page or packaging script exists. The desktop app
version is `0.1.0`; this phase aligns it with the next product release, `0.9.0`.

## Scope

Included:

- Set the desktop app version to `0.9.0`.
- Build `QuantRule_0.9.0_amd64.deb` and `QuantRule_0.9.0_amd64.AppImage` with
  Tauri.
- Add `site/` with a static `index.html`, `styles.css`, and install instructions.
- Add `site/build.sh` to build bundles, copy them to `site/downloads/`, write
  `site/checksums.txt`, and verify the page links and hashes.
- Add `site/check.py` for the link/checksum verification.
- Add the site check to the desktop CI job.
- Document the distribution boundary and update the changelog/AI log.

Excluded:

- Publishing a GitHub release or tag. That is Phase 4 and needs human
  authorization.
- Windows and macOS packages.
- Auto-update, telemetry, accounts, or a backend.
- Signing keys or a hosted CDN.

## Assumptions and uncertainty

- Tauri's Debian bundler writes the archive in Rust and does not need
  `dpkg-deb`; the local machine has `ar`, `tar`, and `gzip`.
- Tauri's AppImage bundler downloads its own `linuxdeploy` tool; the first build
  needs network access.
- The AppImage and deb are x86_64 only. ARM is future work.
- The page is served from a static host or opened locally; links are relative to
  `site/`.
- The `0.9.0` version is the intended next product release; the tag is created
  only in Phase 4 after acceptance.

## Decisions

| Decision | Choice | Why |
|---|---|---|
| App version | `0.9.0` | The next product release after the pre-Tauri `v0.8.0`. |
| Artifact names | Tauri's `QuantRule_0.9.0_amd64.*` | Keeps the upstream bundle names and hashes unambiguous. |
| Page | Hand-written static HTML/CSS | No framework, no network, no runtime. |
| Checksums | SHA-256 in `site/checksums.txt` | Standard `sha256sum -c` verification. |
| Artifact storage | `site/downloads/` is generated and gitignored | Binaries are release assets, not source. |
| CI | Build bundles and run `site/check.py --require-artifacts` | Proves the page and hashes match the built artifacts. |

## Acceptance criteria

- `npm run tauri build` produces a `.deb` and an `.AppImage`.
- `site/build.sh` copies both, writes `checksums.txt`, and passes its check.
- Every download link on the page resolves to a file with a matching SHA-256.
- The page names Linux x86_64, the system libraries, both install paths, and the
  no-account/no-cloud boundary.
- The `.deb` installs in a clean disposable root and the binary launches.
- The AppImage launches under Xvfb.
- The desktop CI job builds the bundles and runs the site check.
- The full repository gate passes.

## Work breakdown

| Step | Work | Role | Paths |
|---:|---|---|---|
| 1 | Bump the desktop version to 0.9.0 | Implementation | `apps/desktop/package.json`, `src-tauri/Cargo.toml`, `tauri.conf.json` |
| 2 | Build the deb and AppImage | Release | `apps/desktop/src-tauri/target/release/bundle/` |
| 3 | Add the static page and packaging script | Implementation | `site/` |
| 4 | Add the link/checksum checker and CI step | Test | `site/check.py`, `.github/workflows/ci.yml` |
| 5 | Verify deb install and AppImage launch | Release | disposable root; Xvfb |
| 6 | Docs, changelog, AI log | Implementation | `README.md`, `CHANGELOG.md`, `docs/` |

## Verification table

Commands are run from the repository root unless noted.

| ID | Plan item | Acceptance criterion | Evidence / command | Expected | Observed | Status | Reviewer | Date | Notes |
|---|---|---|---|---|---|---|---|---|---|
| V1 | Version | Desktop identity is 0.9.0 | npm package, Tauri crate, Tauri config | 0.9.0 | 0.9.0 | Pass | Agent | 2026-11-11 | Human review pending |
| V2 | Deb build | Tauri writes a deb | `npm run tauri build` | file exists | 4.33 MiB | Pass | Agent | 2026-11-11 | Human review pending |
| V3 | AppImage build | Tauri writes an AppImage | `npm run tauri build` | file exists | 111.40 MiB | Pass | Agent | 2026-11-11 | Human review pending |
| V4 | Page | Links and requirements present | `python3 site/check.py` | pass | pass | Pass | Agent | 2026-11-11 | Human review pending |
| V5 | Checksums | Hashes match artifacts | `./site/build.sh`; `sha256sum -c` | pass | 2 artifacts, 2 checksums | Pass | Agent | 2026-11-11 | Human review pending |
| V6 | Deb install | Package extracts and launches | `ar`/`tar` extract; Xvfb | launch | exit 124 after 10s | Pass | Agent | 2026-11-11 | Full apt install in P3.6 |
| V7 | AppImage | AppImage launches | Xvfb | launch | exit 124 after 10s | Pass | Agent | 2026-11-11 | Human review pending |
| V8 | CI | Desktop job packages and checks | `.github/workflows/ci.yml` | pass | Pending | Pending | — | — | |
| V9 | Gate | Full Rust and repository gate | root `cargo test`; checkers | pass | 1,476 passed, 8 ignored; checkers pass | Pass | Agent | 2026-11-11 | Human review pending |

## Risks and recovery

| Risk | Control | Recovery |
|---|---|---|
| AppImage bundler needs FUSE | Build and launch under Xvfb; use extract-and-run if needed | Document the requirement or add `APPIMAGE_EXTRACT_AND_RUN=1`. |
| Deb dependencies are incomplete | Declare the runtime libraries in `tauri.conf.json` | Add the missing `depends` and rebuild. |
| Page links drift from artifact names | `site/check.py --require-artifacts` | Fail the build; fix the link or copy name. |
| Large binaries enter Git | `site/downloads/` and `*.AppImage`/`*.deb` are ignored | Keep them as release assets only. |
| Version mismatch | One version in three files; check script reads them | Bump all three together. |

## Stop conditions

Stop and ask if:

- the deb or AppImage cannot be built on this machine;
- a new packaging dependency or signing key is required;
- the page would need a backend or account;
- publishing a release or tag is requested before Phase 4;
- the version decision conflicts with the release plan.
