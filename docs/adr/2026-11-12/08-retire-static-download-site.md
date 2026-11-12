# ADR — Retire the Static Download Site

- Date: 2026-11-12 (UTC)
- Status: Accepted
- Owner: Human
- Claim type: ADR (Architecture Decision)
- Related: [`05`](05-svelte-only-web-client.md)

## Context

`site/` builds Linux installers and publishes them from a static page. That
existed because the product was a native desktop application. The product is now
a server stack plus a browser client, deployed by container image on an internal
network. A public download page has nothing to distribute.

## Decision

- **Delete `site/`**: the page, the styles, the download directory, and the
  installer build script.
- **Distribute by container image**, built from the repository, started with one
  command.
- **Remove the kata requirement** that promised a static download page.
- **No replacement page.** The client is served by the gateway at the same
  origin as the API.
- **GitHub Releases carries artifacts.** The container image digest and, from
  `DIST-114`, the desktop bundle are published as release assets. A release
  asset is distribution, not a maintained site.

## Alternatives considered

| Alternative | Why rejected |
|---|---|
| Keep the page for internal installers | There are no installers |
| Keep `check.py` and checksums only | Checksums belong to release notes and image digests |
| Publish images to a registry now | Useful later; the course environment builds locally |

## Consequences

- One fewer build pipeline.
- The release note names an image digest instead of a `.deb` checksum.
- `README.md` and the architecture index lose their packaging section.

## Revisit trigger

Revisit if images must be pulled by a machine that cannot build them. That is a
registry decision, not a page decision.

## Verification

- `site/` does not exist.
- No current document references an installer, `.deb`, `AppImage`, or download
  page.
- The retired-term gate fails if they return.
