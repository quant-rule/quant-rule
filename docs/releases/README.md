# Releases and Tags

- Date: 2026-11-07 (UTC)
- Status: Current
- Scope: release notes, version tags, and the changelog alignment
- Contract: [`docs/workflows/release-operations.md`](../workflows/release-operations.md)
- Changelog: [`CHANGELOG.md`](../../CHANGELOG.md)
- Claim type: release and tag process; no model, strategy, or edge claim.

## 1. Purpose

A release has three linked records:

| Record | Answers |
|---|---|
| Git tag | Which commit is the release? |
| Release note | What is in it, how was it verified, and how is it recovered? |
| Changelog | What meaningful changes does a user or developer see? |

They are not duplicates. The tag is the Git identity; the release note is the
detailed record; the changelog is the grouped release history. The GitHub
Project tracks execution and is not a release record.

## 2. Location

```text
docs/releases/<YYYY-MM-DD>/<tag>.md
```

The date is the UTC release date. The tag is `vMAJOR.MINOR.PATCH`.

## 3. Tag rule

- A release tag uses `vMAJOR.MINOR.PATCH` and is annotated.
- Pre-1.0: `v0.MINOR.0` for a feature/release, `v0.MINOR.PATCH` for a fix.
  `v1.0.0` only when the human declares the product stable.
- The tag is created from `main` after the release PR merges.
- No date-based tags, no `latest`, and no tag per pull request.
- The tag message names the release note and the changelog section.

### Preconditions

- The full gate passes on the release commit.
- `CHANGELOG.md` has a `## [<version>] - YYYY-MM-DD` section.
- The release note exists and records the scope, verification, risks, rollback,
  tag, and changelog link.
- No Critical audit finding is open on the changed path.
- The human authorizes the tag.

### Immutability

- A published tag is never moved, deleted, or reused.
- A bad release gets a new tag and a release note that supersedes the old one.
- A local tag that was never pushed may be deleted before publication.
- Protect `v*` tags in GitHub if the repository settings allow it.

## 4. Historical tags

The following tags predate the 2026-11-07 changelog baseline. They are retained
as history and are not treated as new releases. Do not delete or move them.

| Tag | Class | Purpose |
|---|---|---|
| `v0.2.0` | Release | MVP v1 |
| `v0.3.0` | Release | Lab Summary and Help |
| `v0.4.0-exp-baseline` | Historical marker | Experiment baseline; not a released version |
| `v0.5.0` | Release | Workflow reset baseline |
| `audit-closure-2026-10-30` | Historical marker | Audit closure evidence |

The two historical markers are not release tags. New tags use the release
format unless the human approves a named marker.

The first post-baseline release is [`v0.6.0`](2026-11-07/v0.6.0.md) on
2026-11-07. It is not in the historical table. The latest release is
[`v0.9.0`](2026-11-11/v0.9.0.md) on 2026-11-11, the first Tauri Platform release.
The previous release is [`v0.8.0`](2026-11-11/v0.8.0.md), the last pre-Tauri
release; before it is [`v0.7.0`](2026-11-09/v0.7.0.md).

## 5. Changelog alignment

- `## [Unreleased]` accumulates entries until a release.
- At release time, `## [Unreleased]` becomes `## [<version>] - YYYY-MM-DD`.
- Every release tag after the 2026-11-07 baseline has a matching changelog
  section and a release note.
- Historical releases before the baseline are linked here and in
  [`CHANGELOG.md`](../../CHANGELOG.md); their changelog entries are not
  backfilled.
- A correction to a released section is a new release or a dated addendum in the
  next release.

## 6. Release note template

```markdown
# <Tag> — <Title>

- Date: <YYYY-MM-DD> (UTC)
- Tag: `<Tag>`
- Commit: `<commit>`
- Status: Released
- Previous release: [`<previous tag>`](<relative path>)
- Image digest: `sha256:<64 hex characters>` (a local image ID on a dry run)
- Desktop artifacts: `<AppImage name>`, `<NSIS installer name>` (unsigned)
- Changelog: [`CHANGELOG.md`](../../CHANGELOG.md#<version>)
- Claim type: release record; no model, strategy, or edge claim.

## Scope

- Included:
- Deliberately excluded:

## Verification

| Check | Observed |
|---|---|
|  |  |

## Risks

## Rollback

## Changelog

- Section:
- Entries:
```

## 7. Release pipeline

[`.github/workflows/release.yml`](../../.github/workflows/release.yml) builds the
release artifacts. It creates no tag; the human creates and pushes the annotated
tag after the release PR merges.

| Trigger | Result |
|---|---|
| Annotated `vMAJOR.MINOR.PATCH` tag on `main` | Published image, Linux AppImage, Windows NSIS installer, GitHub release |
| `workflow_dispatch` | Build only, never pushes; digest in the job summary; no release, no asset |

The workflow fails a lightweight tag, a tag off `main`, a tag whose release note
is missing, and a tag without the registry configuration. Before it publishes it
runs `tools/gate.sh`, the image contract checks, the shell tests and boundary
check, and a six-container Compose smoke that reads the digest back from
`/health`.

A tag run requires all four repository values, so a published release always
names a pullable image. A `workflow_dispatch` run never pushes: it is the
build-only dry run, records the local image ID, and creates no release.

| Name | Kind | Purpose |
|---|---|---|
| `QR_REGISTRY` | variable | Registry host, for example `ghcr.io` |
| `QR_IMAGE_NAME` | variable | Repository path, for example `quant-rule/quantrule-server` |
| `QR_REGISTRY_USERNAME` | secret | Registry user |
| `QR_REGISTRY_TOKEN` | secret | Registry token with push scope |

The desktop jobs use `GITHUB_TOKEN` only. The release job gets `contents: write`
and `actions: read` to create or update the release and read the uploaded
assets; every other job is read-only.

The workflow appends the image reference, the digest, and both artifact names to
the GitHub release body. Copy the two fields into the release note's
`- Image digest:` and `- Desktop artifacts:` lines in the follow-up record
commit. A published tag is never moved.

A dry run is `gh workflow run release.yml --ref main`, or **Release** from the
Actions tab. The run summary names the digest and both artifacts, and the run's
artifacts hold the installers.

## 8. Checker

`python3 tools/check_releases.py` checks:

- release note file names and dates;
- the `- Tag:` line;
- changelog alignment for releases at or after the 2026-11-07 baseline;
- duplicate release notes and duplicate changelog versions.

Historical releases before the baseline are exempt from the changelog
requirement. The checker does not create or move a Git tag; tag creation remains
a human-authorized Git operation.

## 9. Related documents

- [`docs/workflows/release-operations.md`](../workflows/release-operations.md)
- [`docs/workflows/recordkeeping.md`](../workflows/recordkeeping.md)
- [`CHANGELOG.md`](../../CHANGELOG.md)
- [`AGENTS.md`](../../AGENTS.md)
