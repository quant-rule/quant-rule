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

### Renumbering (2026-11-15)

The release tags were renumbered on 2026-11-15 so the history reads `v0.1.0`
through `v1.0.0` without gaps. Commits, images, and published artifacts are
unchanged; release notes and changelog sections keep their original version
headings as historical records.

| Current tag | Commit | Was |
|---|---|---|
| `v0.1.0` | `42e00210` | `v0.2.0` |
| `v0.2.0` | `86bcb06a` | `audit-closure-2026-10-30` |
| `v0.3.0` | `2739a67f` | `v0.3.0` (unchanged) |
| `v0.4.0` | `42307b7e` | `v0.4.0-exp-baseline` |
| `v0.5.0` | `898881ee` | `v0.6.0` |
| `v0.6.0` | `b71c510f` | `v0.7.0` |
| `v0.7.0` | `29a647f3` | `v0.8.0` |
| `v0.8.0` | `57f5663f` | `v0.9.0` |
| `v0.9.0` | `748355f7` | `v1.0.0` |
| `v1.0.0` | `6373c293` | `v1.1.0` |

The GitHub release pages follow the current tag names; published asset and image
names keep their original build versions.

The current release tag is `v1.0.0`, the desktop release; its historical
release note is [`v1.1.0`](2026-11-15/v1.1.0.md). The previous tag is `v0.9.0`,
the distributed platform, whose historical note is
[`v1.0.0`](2026-11-15/v1.0.0.md).

The map below is authoritative: a current tag resolves to exactly one note, and
the release workflow and `tools/check_releases.py` both read it. Paths are
relative to this directory. The map must name every tag in the renumbering table
above; the checker fails when the two drift.

<!-- current-tags:start -->
```json
{
  "v0.1.0": "2026-10-28/v0.2.0.md",
  "v0.2.0": "2026-10-30/v0.3.0.md",
  "v0.3.0": "2026-10-30/v0.3.0.md",
  "v0.4.0": "2026-11-01/v0.5.0.md",
  "v0.5.0": "2026-11-07/v0.6.0.md",
  "v0.6.0": "2026-11-09/v0.7.0.md",
  "v0.7.0": "2026-11-11/v0.8.0.md",
  "v0.8.0": "2026-11-11/v0.9.0.md",
  "v0.9.0": "2026-11-15/v1.0.0.md",
  "v1.0.0": "2026-11-15/v1.1.0.md"
}
```
<!-- current-tags:end -->

`v0.2.0` is the 2026-10-30 audit-closure commit. No separate note was written
for it, so it resolves to the `v0.3.0` note that records the release of that
date. `v0.3.0` is the release commit itself and resolves to the same note.

## 4. Historical tags

The pre-renumbering markers `v0.4.0-exp-baseline`, the duplicate `v0.5.0`
commit marker, and `audit-closure-2026-10-30` were retired in the 2026-11-15
renumbering. Their evidence remains in Git history; the current tags are the
mapping in section 3.

The first post-baseline release note is
[`v0.6.0`](2026-11-07/v0.6.0.md) on 2026-11-07; after the renumbering its tag
is `v0.5.0`. Release notes keep their original filenames and headings, so use
the mapping to find the note for a current tag.

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
| `workflow_dispatch` with `release_tag: vX.Y.Z` | Manual recovery of an existing tag: image pushed under that tag, both artifacts built, release created or updated |
| `workflow_dispatch` with `publish_snapshot: true` | `snapshot-<sha>` image pushed, digest in the job summary, no release, no asset |
| `workflow_dispatch` | Local build only, never pushes; the recorded digest is a local image ID; no release, no asset |

The workflow fails a lightweight tag, a tag off `main`, and a tag whose release
note is missing. The tag check runs `git fetch --tags --force origin` before
`git cat-file -t "refs/tags/<tag>"`: `actions/checkout` fetches a tag push as
`+<commit>:refs/tags/<tag>` with `--no-tags`, which leaves a lightweight local
ref and made a valid annotated tag look lightweight. The explicit tag fetch
restores the tag object, and a genuinely lightweight tag still fails. Before it
publishes the workflow runs `tools/gate.sh`, the image contract checks
(`--version` against `Cargo.toml`, no Python, no Bun), the desktop helper tests
and boundary check, and a nine-container Compose smoke that reads the digest
back from `/health` and the product version back from `/version`.

The default registry is GHCR, so a tag run needs no repository value: it pushes
`ghcr.io/<owner>/quantrule-platform` and logs in with `GITHUB_TOKEN` under the
image job's `packages: write` permission. The four values below are optional
overrides for another registry, and a custom registry then needs its own token.

| Name | Kind | Purpose |
|---|---|---|
| `QR_REGISTRY` | variable | Registry host; defaults to `ghcr.io` |
| `QR_IMAGE_NAME` | variable | Repository path; defaults to `<owner>/quantrule-platform` |
| `QR_REGISTRY_USERNAME` | secret | Registry user; defaults to the workflow actor |
| `QR_REGISTRY_TOKEN` | secret | Registry token with push scope; defaults to `GITHUB_TOKEN` on GHCR |

The desktop jobs use `GITHUB_TOKEN` only. The release job gets `contents: write`
and `actions: read` to create or update the release and read the uploaded
assets; every other job is read-only.

The workflow appends the image reference, the digest, and both artifact names to
the GitHub release body. Copy the two fields into the release note's
`- Image digest:` and `- Desktop artifacts:` lines in the follow-up record
commit. A published tag is never moved.

The pre-tag proof is `gh workflow run release.yml --ref main -f
publish_snapshot=true`, or **Release** from the Actions tab with the input
checked. It pushes a `snapshot-<sha>` image and records its manifest digest
without creating a release; that digest is the GHCR evidence the release note
may cite before the tag exists. A run without the input records the local image
ID only, and its artifacts hold the installers.

### Manual recovery

A published tag is never moved, deleted, or recreated, so a failed tag run is
recovered from the existing tag:

```bash
gh workflow run release.yml --ref main -f release_tag=v1.0.0
```

Every job checks out that tag, the gate repeats the tag check, and the image job
pushes the same version tag to the registry. Both desktop artifacts are rebuilt,
and the release is created for the existing tag with `gh release create
--verify-tag`, so the step cannot create, move, or delete a tag. A rerun for a
tag whose release already exists updates that release and replaces its assets
instead of creating a second one. `release_tag` and `publish_snapshot` are
mutually exclusive.

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
