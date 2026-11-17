# AI Transparency Log — v1.3.0 Release Preparation

- Date: 2026-11-17 (UTC)
- Tool: DeepSeek V4.1 Flash 1017 in OpenCode
- Work item: Issue #860, `860-release`
- Plan: [`docs/execution-plans/2026-11-16/842-stage-execution-surface.md`](../../execution-plans/2026-11-16/842-stage-execution-surface.md)
- Depends on: #844–#859, all merged into `main` at `baeca6f6`

## Intent and boundary

Prepare the v1.3.0 stage execution surface release: bump every current version
surface, regenerate the model-object oracle and stage fixtures, move the
changelog to a dated release section, add the release note and current-tag map
entry, update the status indexes and desktop documentation, and run the full,
composed, parity, manual, release, docs, and browser gates.

The AI did not create, push, or move a tag; did not merge the pull request; did
not change model, strategy, execution, portfolio, command, API, schema, or
migration behavior; and did not rewrite historical release notes, changelog
sections, AI logs, or course PDFs. The tag remains a separate human
authorization gate.

## Verification and evidence

- `tools/gate.sh` — `workspace gate passed`; 2,321 passed, 0 failed, 16
  ignored; 25 checkers.
- `tools/test_tiers.sh parity` — `port parity ok: 1 registered port(s)`.
- `make up` — `QuantRule v1.3.0 ready → http://localhost:8080`.
- `make verify` — `canonical digest sha256:453c90b4442dd049074e126bb46b400b6f61baa986443c97fa27c3edd41c1daa`;
  two runs verified 12 canonical artifacts and 3,559,171 bytes each.
- `tools/test_tiers.sh composed` — `composed: gateway healthy, product 1.3.0, run parity verified`.
- `make down` — stack stopped.
- `python3 tools/check_releases.py` — 12 release notes, 13 current tags, 9
  changelog versions, baseline 2026-11-07.
- `python3 tools/check_releases.py --note v1.3.0` — resolves
  `docs/releases/2026-11-17/v1.3.0.md`.
- `python3 tools/check_manual.py` — 12 chapters, 46 commands, 13 screens, 4
  tabs; `tools/test_check_manual.py` — 9 pass.
- `python3 tools/check_command_surface.py` — 46 implemented, 0 planned; 17
  tutorial files.
- `python3 tools/check_client_docs.py` — pass.
- `python3 tools/check_links.py` — 1,025 Markdown files, no broken links.
- `python3 tools/check_terms.py` — 626 files, no banned terms.
- `python3 tools/check_doc_length.py` — 39 documents within cap.
- `python3 tools/check_changelog.py` — 10 releases, 161 entries.
- `python3 tools/check_structure.py` — pass.
- `python3 tools/check_file_size.py` — pass.
- `python3 tools/check_schema_prefixes.py` — 8 ledger rows, 19 tables, 7
  baselines.
- `bun run check` — 0 errors, 0 warnings; `bun test` — 146 pass; `bun run
  build` — pass; `bunx playwright test` — 53 pass.
- `apps/desktop-shell`: `bun test` — 35 pass; supervisor `cargo test` — 9 pass.

## Learning and corrections

- The model oracle was regenerated from the 1.3.0 object at 66,022 bytes and
  `sha256:90473762…`. Replacing only `"model_version":"1.3.0"` with `1.2.0`
  reconstructs the recorded `sha256:73e2fe4f…` object at the same size, proving
  the only delta is `.model_version`; the model-registry key component
  `2026-11-10.2` is unchanged. The old note's "one byte differs" claim was not
  copied without evidence.
- `tools/check_changelog.py` requires an `[Unreleased]` section, while the
  release brief asked not to leave an empty one. The compromise is a
  non-empty one-line note under `[Unreleased]`; no fake release entry was
  added and the checker was not weakened.
- The workspace `Cargo.lock` updated all 16 workspace package entries to
  `1.3.0`; third-party `1.2.0` entries in the root and desktop lockfiles were
  left untouched.
- The stage `report.json` body changes only the version string; it stays 308
  bytes and the new SHA-256 is `sha256:9a9cf9a7…`. The `events.csv` hash and
  synthetic artifact ids were not changed.
- The canonical digest stayed
  `sha256:453c90b4442dd049074e126bb46b400b6f61baa986443c97fa27c3edd41c1daa`;
  the version string is not in the canonical artifact set, and a different
  digest was treated as a blocker rather than a new expected value.
- The 2026-11-17 end-to-end tutorial and the first-run manual were changed only
  after `make up` and `make verify` observed `1.3.0`.
- Review corrected the architecture status index: a document that changed in
  this release says "current in v1.3.0", not "unchanged through v1.3.0". The
  stage-surface status now attributes the manual to #859 as its own deliverable.

## Retarget

The human authorized `v1.3.0` before any tag was created. The prepared
`v2.0.0` records were renamed and retargeted to `v1.3.0`; no `v2.0.0` tag or
release exists, and the historical `1.2.0` evidence is unchanged.

## Residual risk

- The release note is prepared, not released; the image digest and desktop
  asset names remain unrecorded until the human-authorized tag run.
- Cross-machine determinism is reviewer-executed; this record claims the local
  gate and the canonical digest only.
- The opt-in Postgres research tests ran only through the Compose service path,
  not the ignored unit-test configuration.
- The desktop artifacts are unsigned; the legacy MinIO stopgap and
  startup-only retention remain as previously recorded.
- The course sources and PDFs remain the v1.2.0 snapshot by design.
