# AI Transparency Log — Linux Distribution and Download Page

- Date: 2026-11-11 (UTC)
- Work item: `609-linux-distribution`
- Issue: [#609](https://github.com/quant-rule/quant-rule/issues/609)
- Plan: [`docs/execution-plans/2026-11-11/609-linux-distribution.md`](../../execution-plans/2026-11-11/609-linux-distribution.md)
- Tool: OpenCode agent (DeepSeek V4.1 Flash)
- Branch: `feat/609-linux-distribution`
- Claim type: Outcome Claim

## Intent and boundary

Package the desktop app for Linux and publish a static download page with
checksums. The page is distribution only; it has no account, backend, or cloud
runtime. No model, strategy, execution, portfolio, or analytics behavior changes.

## Work performed

- Set the desktop identity to `0.9.0` in the npm package, Tauri crate, and Tauri
  config.
- Built `QuantRule_0.9.0_amd64.deb` and `QuantRule_0.9.0_amd64.AppImage` with
  Tauri 2.
- Added `site/index.html` and `site/styles.css`: Linux x86_64 downloads, install
  steps, runtime requirements, checksum instructions, included/CLI-only scope,
  and the no-account/no-cloud boundary.
- Added `site/build.sh` to build, stage, checksum, and verify the artifacts.
- Added `site/check.py` to verify links, version, artifact names, and SHA-256.
- Added the packaging and site check to the desktop CI job.
- Added the site README, root README section, architecture references, changelog,
  and this log.

## Verification

- `npm run tauri build`: built the `.deb` (4.33 MiB) and `.AppImage` (111.40 MiB).
- `./site/build.sh`: staged both and printed
  `site ok: version 0.9.0, 2 artifacts, 2 checksums`.
- `sha256sum -c site/checksums.txt`: both hashes match.
- AppImage under Xvfb: stayed open until the 10-second timeout (`124`).
- Deb data extracted with `ar` and `tar`; `usr/bin/quantrule-desktop` stayed open
  under Xvfb until the timeout (`124`).
- Local static server: page, `.deb` (200, 4,543,654 bytes), and checksums served.
- Desktop unit, backend, and browser E2E tests remain green.
- Root Rust and repository gates: 1,476 passed, 0 failed, 8 ignored; command surface, links, terms, legacy, structure, workflows, changelog, releases, partitions, registry, validation, verification, file size, claims, execution plans, and the site check pass.

## Learning

- Tauri's Debian bundler writes the archive in Rust with `ar`/`tar`; no
  `dpkg-deb` or `fakeroot` is needed.
- Tauri's AppImage bundler downloads `linuxdeploy` and runs it. The AppImage
  launched under Xvfb without a separate FUSE package on this machine.
- The page has duplicate download links (hero and cards), so the checker
  deduplicates links before comparing them with the artifact list.
- The `.deb` declares `libwebkit2gtk-4.1-0` and `libgtk-3-0`. A clean Ubuntu
  install still needs a real `apt install` in Phase 3.6; this phase extracts the
  package and launches the binary.
- The artifacts are generated and gitignored. Phase 4 publishes them as release
  assets and creates the tag after human acceptance.
- No Quant Validator record is required: no model, strategy, execution, or
  portfolio behavior changed.
