# AI transparency log — DIST-082 release pipeline

- Date: 2026-11-14 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: Release & Operations plus Execution Plan; issue #673, claim type
  Behaviour Claim.
- Intent: add `.github/workflows/release.yml` to build the server image and
  record its digest, build the Linux AppImage and the Windows NSIS installer,
  and append the digest and both artifact names to the GitHub release body.
- Boundary: the new workflow, the release pipeline and template fields in
  `docs/releases/README.md`, the `CHANGELOG.md` entry, and this log. No server,
  engine, client, desktop, `Dockerfile`, `compose.yaml`, `ci.yml`, `Makefile`, or
  release-note file changed. No tag was created and no release was published.

## Implemented

- `release.yml` triggers on a pushed `v*` tag and on `workflow_dispatch`. The
  tag path verifies `vMAJOR.MINOR.PATCH`, an annotated tag object, ancestry on
  `main`, the release note, and the four registry values before any build.
- The gate job runs `tools/gate.sh`. The image job builds `linux/amd64`,
  checks the image carries no Python or Bun interpreter, captures the digest,
  starts the six-container default Compose stack with `QR_IMAGE_DIGEST`, and
  reads the digest back from `/health`.
- The image is pushed only on a tag, and only when `QR_REGISTRY`,
  `QR_IMAGE_NAME`, `QR_REGISTRY_USERNAME`, and `QR_REGISTRY_TOKEN` are all
  present. A tag run without them fails before the gate, so a published release
  never records an unpullable digest; `workflow_dispatch` is always the
  build-only dry run. The names and the rule are documented in
  `docs/releases/README.md` section 7.
- The desktop matrix follows `apps/desktop-shell/README.md` section 7: Ubuntu
  AppImage, Windows NSIS, both smoke-tested, both uploaded. No desktop file was
  edited.
- The publish job runs only for a tag. It appends the image reference, digest,
  and both artifact names to the release body, creates or updates the release,
  uploads both assets, and verifies the body and the asset names. The release
  note template gains `- Image digest:` and `- Desktop artifacts:` fields.
- All actions are pinned by commit SHA. Top-level permissions are
  `contents: read`; only the publish job adds `contents: write` and
  `actions: read`.

## Verification

| Command | Observed |
|---|---|
| `python3` with `yaml.compose` over `release.yml` | parsed; triggers `push`, `workflow_dispatch`; jobs `gate`, `image`, `desktop`, `publish`; 8 action refs pinned by SHA |
| `bash -n` over every extracted Linux shell step | passed for 17 steps; the Windows `pwsh` step skipped |
| `docker build --platform linux/amd64 --tag quantrule-server:release-local-673 .` | passed in 5.1 s from the build cache |
| `docker image inspect ... --format '{{.Id}}'` | `sha256:4d25180dc332ba9f98ebfde6703c8ec1eee3c7d742222ab01750ee63e08ce4d7`; matches `^sha256:[0-9a-f]{64}$` |
| `docker run --rm quantrule-server:release-local-673 --version` | `quantrule-server 0.3.0` |
| `docker run ... sh -c 'command -v python3 \|\| command -v bun'` | no interpreter; no `.py` file under `/usr/share` |
| `QR_IMAGE=... QR_IMAGE_DIGEST=... docker compose up -d --wait --no-build` | six services running |
| `curl http://127.0.0.1:8080/health` | `"image_digest":"sha256:4d2518…"`; equal to the captured digest |
| `tools/gate.sh` | passed; `workspace gate passed with CARGO_TARGET_DIR=.../target/gate` |
| `python3 tools/check_doc_length.py` | 33 files within cap |
| `python3 tools/check_links.py` | 931 Markdown files; no broken relative links |
| `python3 tools/check_releases.py` | 7 release notes, 4 changelog versions, baseline 2026-11-07 |
| `find docs/releases -type f -name "v0.9.0.md"` | resolves the existing release note |
| `git for-each-ref --format='%(objecttype)' refs/tags/v0.9.0` | `tag` (annotated); the check accepts it |

Not verified here:

- GitHub Actions was not executed locally. The YAML parses, every Linux shell
  body passes `bash -n`, and the commands were exercised on this host, but the
  runner image, the Windows NSIS build, `tauri-action`, and the release API call
  are unobserved until the first real run.
- The Windows installer was not built: it requires `windows-latest`.
- The registry push path was not exercised: no registry credentials exist, so
  the local build exercised only the documented build-only dry-run path.
- Provenance was verified by reading the path, not by a new test. `submit_run`
  copies `state.version.image_digest` into `NewRun`, and the Postgres adapter
  stores it. No unit test asserts the configured digest reaches `/health`; the
  Compose smoke is the live evidence. A server-side test belongs to the B
  stream, because this issue's boundary excludes `crates/**`.

## Learning and rejected alternatives

- **Rejected — `tools/verify.sh` as a release step.** It runs `quantrule
  backtest run`, but the shell was rewritten as an HTTP gateway client (#646)
  and no longer has a `backtest` command. The script failed locally with
  `unknown command 'backtest'`. It is stale on `main`; the workflow uses the
  gate, the image contract checks, the desktop tests, and a live `/health`
  digest check instead. The stale script should be routed to an audit.
- **Rejected — new registry secrets invented for GHCR only.** The registry host
  is a human decision. The workflow names four documented values and publishes
  only when all four exist; the default is a build-only dry run. Any registry
  that accepts a username and token works.
- **Rejected — `tauri-action` creating the release itself.** The publish job
  owns the release body so the digest and both artifact names are generated in
  one place. The action only builds.
- **Rejected — rebuilding the image in the publish job to delay the push.**
  A second Rust build doubles the release time. The image is pushed after the
  gate and its live health check; a desktop failure still prevents the GitHub
  release, and the pushed digest is the record.
- **Corrected — a single `curl` after Compose `--wait`.** The first local run
  answered `/health` before the AMQP listener was ready. The digest was already
  correct, but the step now retries the request instead of assuming one answer.
- **Deferred — a composed-tier run in the release workflow.** `tools/test_tiers
  composed` still exits 2 (#643/#644); the six-container Compose smoke is the
  available integration check. Wire the composed tier when it exists.
