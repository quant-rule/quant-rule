# AI transparency log — release 1.2.0

- Date: 2026-11-16 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash 1017.
- Workflow: Release & Operations; release record for the 2026-11-16 version and
  tag alignment. The human accepted the release on 2026-11-16, and the plan moved
  to `Verified`.
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

- `python3 tools/check_releases.py`: `releases ok: 11 release notes, 12 current
  tags, 8 changelog versions, baseline 2026-11-07`.
- `tools/gate.sh` on the 1.1.0 tree: `workspace gate passed`; 2,118 passed, 0
  failed, 14 ignored; 24 checkers.
- `tools/gate.sh` on the 1.2.0 tree: `workspace gate passed`; 2,118 passed, 0
  failed, 14 ignored; 24 checkers.
- The 1.1.0 model object was dumped before the bump: 66,022 bytes,
  `sha256:d3ed28f8a944de50bd9df986424f14bc6c5a89d8fd346f7895cd5f3914b16c6f`,
  matching the recorded oracle.
- The 1.2.0 model object is 66,022 bytes,
  `sha256:73e2fe4f4f0bffe5ee8cc30f24e7773952fad4ffad300ec717352868d2b62de7`;
  `cmp -l` reports one differing byte and a semantic JSON diff reports one path,
  `.model_version` (`1.1.0` to `1.2.0`). No reading, event, report, or canonical
  artifact value changed.
- `check_links.py` (980 files), `check_doc_length.py` (35 within cap), and
  `check_terms.py` (602 files, no banned terms) passed.
- `tools/course/build_pdfs.sh` built 11 PDFs (six methodology, five
  architecture). The generated text contains `v1.2.0` in the overview, the
  checkpoint evidence, the deployment example, and the user-journey example;
  page counts are unchanged.
- `v1.1.0` is an annotated tag object `c13cfe2276474c7143ddaaca93624ed476edee7d`
  targeting `a734e7fb` on `main`; tag run `43889152217` succeeded and published
  registry digest `sha256:d5ce83da06ae8a5c8018c91582ed78e3c5905b5285ce447b677321ac60911751`
  with `QuantRule_1.1.0_amd64.AppImage` and `QuantRule_1.1.0_x64-setup.exe`.
- `v1.2.0` is an annotated tag object `00387d4143bb46a1be958fa16daf2be57641977d`
  targeting `231c1395` on `main`; tag run `43889152059` succeeded and published
  registry digest `sha256:26350916267cee482a8e4a847e347afb297a733a0f11a8104e80404b1d0eae68`
  with `QuantRule_1.2.0_amd64.AppImage` and `QuantRule_1.2.0_x64-setup.exe`.
- The GitHub releases at `v1.1.0` and `v1.2.0` carry both assets; the release
  bodies record the registry digests and artifact names.

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
