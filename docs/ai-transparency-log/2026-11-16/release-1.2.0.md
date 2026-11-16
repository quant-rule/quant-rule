# AI transparency log — release 1.2.0

- Date: 2026-11-16 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash 1017.
- Workflow: Release & Operations; release record for the 2026-11-16 version and
  tag alignment.
- Intent: normalize the historical release notes and changelog to the current
  tags, cut `v1.1.0` on the current 1.1.0 tree, bump every version surface to
  `1.2.0`, and cut `v1.2.0`.
- Boundary: release records, version manifests, the one model-object oracle,
  current status documentation, the course sources and PDFs, and the release
  plan/log. No model, strategy, execution, portfolio, service, migration, or
  binary-name behavior changed. No dependency was added. Historical tags are
  not moved.

## Implementation

- The historical note files were renamed to the current tags: `v0.2.0` to
  `v0.1.0`, `v0.5.0` to `v0.4.0`, `v0.6.0` to `v0.5.0`, `v0.7.0` to `v0.6.0`,
  `v0.8.0` to `v0.7.0`, `v0.9.0` to `v0.8.0`, the distributed `v1.0.0` to
  `v0.9.0`, and the desktop `v1.1.0` to `v1.0.0`. Each note keeps its historical
  heading and gains a tag note naming the prepared version.
- The changelog headings were renumbered to the same current tags; the
  `[Unreleased]` entries became `[1.1.0] - 2026-11-16`.
- The v1.1.0 note records the current 1.1.0 tree, the two-run digest evidence
  inherited from the desktop release, and the fact that the tag run records the
  new image digest and assets.
- The version bump and the v1.2.0 note are the second commit, so the v1.1.0 tag
  can point at a consistent 1.1.0 tree.

## Verification and evidence

- `python3 tools/check_releases.py`: `releases ok: 10 release notes, 11 current
  tags, 7 changelog versions, baseline 2026-11-07`.
- `tools/gate.sh`: `workspace gate passed`; 2,118 passed, 0 failed, 14 ignored;
  24 checkers.
- The 1.1.0 model object was dumped before the bump: 66,022 bytes,
  `sha256:d3ed28f8a944de50bd9df986424f14bc6c5a89d8fd346f7895cd5f3914b16c6f`,
  matching the recorded oracle.
- `check_links.py`, `check_doc_length.py`, and `check_terms.py` passed after the
  note renames.
- The 1.2.0 oracle delta, the full gate, the PDF build, and both tag runs are
  recorded after they run; this log does not pre-claim them.

## Learning

- Renaming a release note breaks every historical link to it, not only the
  release index. The link checker found the cross-references in audits, plans,
  reports, and AI logs.
- The release workflow checks out the tag commit, so a normalized note on
  `main` does not change what an old tag's recovery run reads. The historical
  tag commits keep their original map and note files.
- The desktop release's image and installer names are build identities. Reusing
  the `v1.1.0` name for the new tag reassigns the registry tag, so the desktop
  note now records its digest as the stable reference.

## Rejected or corrected suggestions

- A single tag at `v1.2.0` would avoid the historical rename but would not
  satisfy the requested `v0.1.0` through `v1.2.0` sequence or put both profiles
  under `v1.1.0`.
- Renaming only the desktop note would collide with the existing `v1.0.0`
  distributed note, so the full cascade was required.
- Claiming the tag-run digest before the tag run was rejected; the note states
  that the run records it.
- No Quant Validator record is required because no model or strategy behavior
  changed; the oracle delta will prove only the version field changed.
