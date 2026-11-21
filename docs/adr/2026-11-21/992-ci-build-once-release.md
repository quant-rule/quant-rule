# ADR — Build Once, Release Fast, and Time Out Safely

- Date: 2026-11-21 (UTC)
- Status: Accepted
- Owner: Human
- Claim type: ADR (Architecture Decision)
- Plan: [`992-development-speed-v1.5.0.md`](../../execution-plans/2026-11-21/992-development-speed-v1.5.0.md)
- Epic: #992

## Context

The release gate duplicates the PR gate. The image job is already 10m15s, and
the desktop jobs add 11 and 18 minutes. Docker layer caching does not persist the
Rust `target` directory, and the Dockerfile compiles the workspace in one layer,
so a source change invalidates the expensive build.

A total release under 10 minutes cannot be guaranteed by caching alone. The
release also must not fail a correct run at 11 or 12 minutes.

## Decision

**Build the six service binaries once on `main`; promote them at release.**

1. A `main`-only job builds `qr-gateway`, `qr-backtest-service`,
   `qr-data-service`, `qr-model-service`, `qr-evidence-service`, and
   `qr-simulation-service` in the pinned toolchain.
2. It writes a manifest with commit SHA, toolchain, and SHA-256 for every binary.
3. It uploads `release-binaries-<sha>` with provenance/attestation.
4. The release gate downloads the exact-SHA artifact and verifies the manifest.
5. `image-assemble` copies the binaries into a thin runtime image; no Rust
   compile and no Bun build.
6. `image-verify` runs the composed suite on that image.
7. `publish` runs only after the composed suite passes.

A missing or mismatched artifact fails the release; there is no silent rebuild.

**Timeouts:**

- PR jobs: 10 minutes.
- Release jobs: 15 minutes.
- Intended release critical path: 6–8 minutes.

The 15-minute release timeout is a safety net, not a target. A correct 11–12
minute release passes.

## Alternatives considered

| Alternative | Why rejected |
|---|---|
| Rebuild Rust inside Docker at release | Source changes invalidate the layer; 10–12+ minutes |
| Cache the 74 GB target | GitHub cache limit is 10 GB |
| Persist BuildKit cache mounts | Not durable through GitHub Actions |
| Raise every timeout to 20 | Hides hangs and violates the release policy |
| Move composed tests after publish | Ships before verification |

## Consequences

- `main` carries the release binary build; PRs stay fast.
- The release is a promote-and-verify pipeline, not a build pipeline.
- Provenance is mandatory: exact SHA, manifest, hashes, attestation.
- A patch release is a new tag; published tags are never moved.
- The release wall clock no longer depends on the Rust compiler.

## Verification

- `release-binaries-<sha>` exists for the exact tag SHA.
- Every binary hash matches the manifest.
- `image-assemble` logs no `cargo build`.
- Composed suite passes before push and publish.
- Release critical path is 6–8 minutes; every job timeout is 15.
