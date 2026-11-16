# Release 1.2.0 — Version and tag alignment

- Status: In Review
- Date: 2026-11-16 (UTC)
- Workflow: [Release & Operations](../../workflows/release-operations.md)
- Claim type: Outcome Claim (release identity and reproducibility)
- Human decision: the 2026-11-16 instruction authorized the full second
  historical renumbering and the `v1.1.0` and `v1.2.0` tags.

## Intent

Make the current product version, the release tags, the release-note filenames,
the changelog headings, the README, the course deliverables, and the board name
one version: `1.2.0`, with current tag `v1.2.0`. The tree passes through
`v1.1.0` first so the server platform and the desktop package share one tag
before the final bump.

## Current state

- The current tag is `v1.0.0`; the product version is `1.1.0`.
- The historical desktop note was `docs/releases/2026-11-15/v1.1.0.md` and
  resolved from tag `v1.0.0`.
- The historical distributed note was `docs/releases/2026-11-15/v1.0.0.md` and
  resolved from tag `v0.9.0`.
- The `v1.1.0` name was occupied by both a note filename and a changelog
  section, so a new tag needed the historical normalization first.

## Scope

Included:

- Rename the historical note files and renumber the changelog headings to the
  current tags, keeping each note's historical heading and adding a tag note.
- Cut `v1.1.0` on the current 1.1.0 tree after the normalization.
- Bump every code and documentation version surface to `1.2.0`.
- Cut `v1.2.0` after the bump.
- Regenerate the one model-object oracle that carries the product version.
- Update the course methodology and architecture sources and rebuild the PDFs.
- Record the image digest and desktop artifacts after the tag runs.

Excluded:

- Moving, deleting, or reusing a published Git tag.
- Any model, strategy, execution, portfolio, or service behavior change.
- A profitability, edge, or cross-machine determinism claim.

## Decisions

| Decision | Consequence | Status |
|---|---|---|
| Cut both tags | `v1.1.0` is the current 1.1.0 tree; `v1.2.0` is the final version | Approved |
| Full second renumbering | Historical note files and changelog headings match current tags; old tag commits keep their original files | Approved |
| Keep build names | The desktop release keeps its `1.1.0` installer and image names; its digest is the stable reference | Approved |
| Follow-up record commit | The tag runs record the image digest and artifacts after the tags exist | Approved |

## Work breakdown

1. Normalize the historical notes and changelog; add the `v1.1.0` note and map.
2. Run the gate; commit the 1.1.0 tree; open the release PR.
3. Bump the workspace, desktop, web, and shell versions; regenerate the oracle;
   add the `v1.2.0` note, changelog, and map.
4. Update the README, status docs, course sources, and PDFs to `1.2.0`.
5. Merge, tag `v1.1.0` and `v1.2.0`, push, and verify both release runs.
6. Record the observed digests and artifacts; set the board item; clean up.

## Acceptance criteria

- Every version surface reports `1.2.0`.
- `v1.1.0` and `v1.2.0` are annotated tags on `main`; `v0.1.0` through `v1.0.0`
  are unchanged.
- `tools/check_releases.py` resolves every current tag to exactly one note.
- The changelog has `## [1.1.0] - 2026-11-16` and `## [1.2.0] - 2026-11-16`.
- The README, `docs/README.md`, release index, course sources, and PDFs name
  `1.2.0` / `v1.2.0`.
- The release workflow publishes both GitHub releases and records the image
  digest and desktop assets.

## Verification table

| ID | Plan item | Acceptance criterion | Evidence / command | Expected | Observed | Status | Reviewer | Date | Notes |
|---|---|---|---|---|---|---|---|---|---|
| V1 | Release alignment | Every current tag resolves | `python3 tools/check_releases.py` | passes | `releases ok: 11 release notes, 12 current tags, 8 changelog versions` | Pass | Agent self-check | 2026-11-16 | Human review pending |
| V2 | Workspace gate | Fast and full gate pass | `tools/gate.sh` | passes | `workspace gate passed`; 2,118 passed, 0 failed, 14 ignored; 24 checkers | Pass | Agent self-check | 2026-11-16 | Human review pending |
| V3 | Version surfaces | All report `1.2.0` | manifest and shell checks | all `1.2.0` | workspace, 16 lockfile entries, web, desktop, supervisor, Tauri, shell, and test all `1.2.0` | Pass | Agent self-check | 2026-11-16 |  |
| V4 | Oracle delta | Only `.model_version` changes | dumps and `cmp` | one semantic path | 66,022 bytes each; 1 byte differs; only `.model_version` (`1.1.0` to `1.2.0`) | Pass | Agent self-check | 2026-11-16 |  |
| V5 | Links and docs | No broken links or over-cap docs | `check_links`, `check_doc_length`, `check_terms` | pass | 980 files, 35 within cap, 602 terms, no banned terms | Pass | Agent self-check | 2026-11-16 |  |
| V6 | PDFs | Sources and PDFs agree | `tools/course/build_pdfs.sh` | builds and content matches | 11 PDFs built; six methodology PDFs and five architecture PDFs; v1.2.0 text and page counts verified | Pass | Agent self-check | 2026-11-16 |  |
| V7 | Tags | Both annotated and on `main` | `git cat-file`, workflow runs | both published | `v1.1.0` object `c13cfe22…` targets `a734e7fb`; `v1.2.0` object `00387d41…` targets `231c1395`; runs `43889152217` and `43889152059` succeeded | Pass | Agent self-check | 2026-11-16 | Human review pending |
| V8 | Release records | Digest and assets recorded | GitHub release pages | appended | `v1.1.0` digest `sha256:d5ce83da…`; `v1.2.0` digest `sha256:26350916…`; both releases carry the AppImage and NSIS assets | Pass | Agent self-check | 2026-11-16 |  |
| V9 | Closure | Worktree and branches removed; board items remain | `python3 tools/worktree_cleanup.py check <worktree>` | check passes; board items present | pending | Pending |  |  |  |

## Risks and recovery

- The registry tag `:v1.1.0` is reassigned to the new release; the historical
  desktop digest remains the stable reference.
- Historical tag commits keep their old note paths by design; a recovery run
  for an old tag uses that tag's tree.
- The desktop artifacts are unsigned.
- If a tag run fails, use the manual `release_tag` dispatch; never move a tag.

## Stop conditions

- A published tag would need to move.
- A gate failure cannot be fixed within the release records.
- The release workflow cannot resolve a current tag to one note.
