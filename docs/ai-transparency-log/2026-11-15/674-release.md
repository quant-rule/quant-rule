# AI transparency log — DIST-090 v1.0.0 release preparation

- Date: 2026-11-15 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: Release & Operations (DIST-000, DIST-090); issue #674, claim type
  Outcome Claim.
- Intent: prepare the v1.0.0 release without creating the tag: bump the product
  version to `1.0.0` everywhere it is declared, close the `[1.0.0]` changelog
  section, write the release note, and make the release workflow self-sufficient
  on GHCR with a pre-tag snapshot proof.
- Boundary: `Cargo.toml`/`Cargo.lock`, the desktop shell version fields and its
  one version test, `CHANGELOG.md`, `README.md`, `docs/README.md`,
  `docs/releases/**`, `docs/workflows/release-operations.md`,
  `.github/workflows/release.yml`, the DIST-000 verification rows this release
  evidences, and this log. No model, strategy, execution, portfolio, service,
  shell, browser, or binary-name behavior changed. No dependency was added.

## Implementation

- The workspace version moves from `0.3.0` to `1.0.0`; the 16 `quantrule-*`
  entries in `Cargo.lock` follow. The desktop shell moves in `Cargo.toml`,
  `Cargo.lock`, `package.json`, `tauri.conf.json`, `ui/shell-lib.js`, and the one
  test that asserts the four declarations agree. `apps/web/package.json` already
  declared `1.0.0`.
- The desktop README example artifact names and the Windows procedure now name
  `1.0.0`, which is what the bundler produces.
- `CHANGELOG.md`: `[Unreleased]` becomes an empty section above
  `## [1.0.0] - 2026-11-15`, with a short release summary. Every grouped entry is
  preserved; no released section was edited.
- `docs/releases/2026-11-15/v1.0.0.md`: created from the release-note template.
  It records the scope, the observed verification, the risks, the rollback, the
  tag, the changelog link, the image digest, and both desktop artifact names. Its
  status is `Prepared`; the tag and the release are not created here.
- `.github/workflows/release.yml`: the tag gate no longer requires four
  repository values. The default is `ghcr.io/<owner>/quantrule-server` with
  `GITHUB_TOKEN` and job-level `packages: write`; `QR_REGISTRY`,
  `QR_IMAGE_NAME`, `QR_REGISTRY_USERNAME`, and `QR_REGISTRY_TOKEN` stay as
  optional overrides, and a custom registry without its token fails with a clear
  message. `workflow_dispatch` gains `publish_snapshot` (default false), which
  pushes `snapshot-<sha>` and records the manifest digest without a release. The
  tag run still requires the annotated tag, `main` ancestry, and the release
  note. The Compose smoke now expects nine containers, checks `/version` and
  `/health` against the built image, and the release verification uses `jq`
  instead of inline Python.
- `docs/releases/README.md` and `docs/workflows/release-operations.md` now
  describe the two distribution profiles (server image, two unsigned GitHub
  Release assets), the GHCR default, and the snapshot proof. `README.md` and
  `docs/README.md` state `v1.0.0` as the current release instead of a target.
- `docs/execution-plans/2026-11-12/dist-000-distributed-platform.md`: rows V13,
  V17, V19, V20, and V21-V26 move to `Pass` with the observed evidence. No row
  was marked pass on a plan or a promise.

### The one golden this changed

`crates/quantrule-model-service/tests/model.rs` compares the stored model object
against a SHA-256 captured from the coupled gateway route. The object carries the
compiled product version, so `0.3.0` to `1.0.0` changes it. The oracle was
regenerated deliberately, and the delta was proven before the constant changed:
both objects are 66,022 bytes, `cmp -l` reports two differing bytes, and a
semantic diff reports one path, `.model_version` (`0.3.0` → `1.0.0`). The test
comment records the regeneration and its reason. No other golden moved: the
worker's coupled-baseline digest is unchanged (`sha256:c23925d4…`), and
`make verify` prints the same canonical digest as the 0.3.0 tree.

## Verification — 2026-11-15 (UTC)

| Check | Command | Observed |
|---|---|---|
| Full gate | `tools/gate.sh` | exit 0; 1,995 tests passed, 0 failed, 10 ignored; fmt, clippy `-D warnings`, rustdoc, 22 checkers |
| Model oracle before the bump | `cargo test -p quantrule-model-service --test model` | 7 passed on the regenerated constant |
| Oracle delta | temporary dump of the stored object at `0.3.0` and `1.0.0` | 66,022 bytes each; 2 bytes differ; only `.model_version` |
| Desktop helpers | `bun test` in `apps/desktop-shell` | 24 passed, 0 failed |
| Desktop boundary | `bash tests/boundary.sh` | `no IPC command, no Rust file read, no engine, no plugin` |
| One-command start | `make up` | exit 0; `QuantRule v1.0.0 ready → http://localhost:8080` |
| Containers | `docker compose ps --status running --services \| wc -l` | 9 of 9 running; only `gateway` publishes `8080` |
| Version contract | `curl /api/v1/version` | `product: 1.0.0`, `api: quantrule.api/v1` |
| Health digest | `QR_IMAGE_DIGEST=<id> docker compose up -d --no-deps gateway`; `curl /health` | `image_digest: sha256:2e48f120…`, equal to `/version` |
| Image probe | `docker run --rm quantrule-server:local --version` | `quantrule-server 1.0.0` |
| Six binaries | `docker run --rm … ls -1 /usr/local/bin` | six `qr-*` plus `quantrule-entry` and `check-no-python` |
| No Python | `docker run --rm … check-no-python`; `command -v python3`; `command -v bun` | `image python gate ok: no interpreter and no .py file`; both lookups empty |
| `make verify` | `make verify` | `canonical digest sha256:453c90b4442dd049074e126bb46b400b6f61baa986443c97fa27c3edd41c1daa` |
| `make verify` again | same, after the image digest was set | same line; 12 artifacts and 3,559,171 bytes per run |
| Wrong expected | `make verify EXPECTED="canonical digest sha256:0000…"` | exit 2; expected and observed lines printed |
| Profiles | `GET /api/v1/runs` and `/catalog/datasets` as `alpha` and `default` | `alpha` 0/0; `default` 3 runs and 2 datasets; unknown profile 404 |
| Repository configs | `GET /api/v1/catalog/configs` | 581 listed, 5 valid examples, 576 historical invalids with their message |
| Compose parse | `docker compose config -q` | exit 0; nine services |
| Linux desktop | `bunx @tauri-apps/cli@2 build --bundles appimage` | `QuantRule_1.0.0_amd64.AppImage`, 81 MiB |
| Linux smoke | `bash tests/smoke-linux.sh target/release/quantrule-desktop-shell` | `smoke passed: the shell probed /health and navigated to the gateway` |
| Workflow YAML | PyYAML parse of `.github/workflows/*.yml` | 3 files parse |
| Reference resolver | the workflow's resolver script run locally with simulated env | local, snapshot, tag, and custom-registry paths; custom registry without a token fails |
| Docs checkers | the 22 checkers | pass, including `check_releases` (8 notes) and `check_changelog` (6 releases) |

## Not verified here, and why

- **Tag run.** The tag is published (object `ced9bc14…`, commit `748355f7`) and
  the push run failed at the tag check; see the recovery follow-up below. The
  recovery dispatch's own digest and release body remain the final tag evidence.
- **Cross-machine determinism.** Reviewer-executed; this log claims only the two
  local invocations.

## Follow-up — the GHCR snapshot proof (2026-11-15)

- `workflow_dispatch` with `publish_snapshot: true` ran on `main` `90e22047`:
  run [`43684761275`](https://github.com/quant-rule/quant-rule/actions/runs/43684761275).
  Gate, Image and digest, and both desktop jobs succeeded; Publish was skipped.
- The first snapshot under the `quantrule-server` package was denied by the
  registry. The package is now `quantrule-platform`; `QR_IMAGE_NAME` carries it
  and the workflow default follows, so a tag run does not hit the denied package.
- The pushed manifest is
  `ghcr.io/quant-rule/quantrule-platform:snapshot-90e22047d5acb4f9459eacf69db37f162a550cdf`
  with digest `sha256:9667e23d8bc2607582a32bc5a0967e66f478206d2c21833950f3de6116bea255`.
  `/health` reported that digest and `/version` reported `1.0.0`; both lines were
  read from the run log, not from a summary.
- The desktop jobs produced `QuantRule_1.0.0_amd64.AppImage` and
  `QuantRule_1.0.0_x64-setup.exe`, and each version assertion passed in the run.
- The release note now names the release image and the registry digest, removes
  the local-digest caveat and the external-registry exclusion, and records both
  desktop artifacts as built by the dispatch. No changelog entry: the release
  section and the product behavior are unchanged.

## Follow-up — the tag and the manual recovery path (2026-11-15)

- The annotated tag `v1.0.0` is published: tag object `ced9bc14…` targets
  `748355f7` on `main`. It was not moved, deleted, or recreated.
- Tag run [`43686787967`](https://github.com/quant-rule/quant-rule/actions/runs/43686787967)
  failed in the gate's tag check, not in a build. Its log shows checkout fetching
  `+748355f7…:refs/tags/v1.0.0` with `--no-tags`, which writes a lightweight
  local ref, so `git for-each-ref` reported `commit`. No image and no release
  were published.
- The workflow now runs `git fetch --tags --force origin` before
  `git cat-file -t "refs/tags/<tag>"`, on both the tag push and the new
  `release_tag` dispatch. The step was exercised against the published tag in a
  scratch clone: the failing lightweight state reports `commit`, the fetch
  restores `tag`, and a lightweight tag, a malformed name, a tag off `main`, and
  a tag without a release note each exit 1.
- `release_tag` is the manual recovery input, mutually exclusive with
  `publish_snapshot`. Every job checks out that tag, the image job pushes the
  version tag, both desktop jobs rebuild, and the publish job creates the release
  with `gh release create --verify-tag` or updates the existing one in place.
- The command to run is
  `gh workflow run release.yml --ref main -f release_tag=v1.0.0`. It remains to
  run it, and the release note's status and digest are updated if the registry
  digest differs.

## Learning

- The first workflow draft kept the four-value tag gate and only defaulted the
  registry. That would still have required human secrets for a release. The gate
  was corrected to require the tag, `main`, and the note, and to fail only when a
  custom registry is configured without its own token.
- The version appears in the model object but not in the canonical digest of the
  mini fixture. Bumping it therefore regenerates one oracle and leaves the
  reviewer line intact; both facts were checked rather than assumed.
- Updating the `Status: Target v1.0.0` lines in the architecture set was
  considered and rejected as outside this issue's ownership. It is a
  documentation follow-up, not a release blocker; no claim depends on it.
