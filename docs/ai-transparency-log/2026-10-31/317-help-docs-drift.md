# AI Transparency Log — Help, Documentation, and Stale Text

- Date: 2026-11-01 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash), sequential role passes
- Work item: #317, branch `docs/317-help-docs-drift`
- Plan: `docs/execution-plans/2026-10-31/full-system-audit.md` (W10)
- Audit: `docs/audits/2026-10-31/full-system-audit.md` (AUD-61, -62, -69)

## Intent and boundary

The human approved the full-system audit plan on 2026-10-31. W10 corrects the
two wrong help descriptions, sweeps the eight documented drifts and the stale
module/contract text, and adds a relative-link check to CI. It does not change
command behavior, model values, strategy rules, or schema. The report summary
field decision follows D15=A: the docs are updated to the code because the
human did not declare the doc authoritative.

## Implemented

- **AUD-61.** `lab diagnose --out` now says the default is
  `<lab-dir>/diagnosis`; `fetch --png` is optional. `help-all.txt` is updated.
- **AUD-62.** Updated the research root default, the store query order and
  widths, the strategy summary field placement, the artifact-set guarantee,
  the target-contract status, the four v0.2.0 release links, the research
  export scope, and the MFE/MAE exit-bar rule.
- **AUD-69.** Updated the renderer's five-series order, the bounded snapshot
  doc, the `MarketReport::compute` signature, the `ReportLeg` definition,
  the dashboard-only reason 5 note, the JSONP row shape, and the default
  database path. The pipeline module says "selected ladder" rather than "four
  series".
- Added `tools/check_links.py` and a CI step. It skips external links, anchors,
  fenced code, and inline code, and checks every relative Markdown target.

## Verification and evidence

| Command | Observed |
|---|---|
| `python3 tools/check_links.py` | 309 Markdown files checked; no broken relative links |
| `cargo test -p quantrule-shell --test cli help` | 15 passed, 0 failed |
| `cargo test --workspace` | 1,580 passed, 0 failed, 7 ignored; 47 suites |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | exit 0 |
| `git diff --check` | clean |

Fail-before: the `help all` fixture test failed before the fixture update
because the old descriptions did not match the corrected option text. For the
link check, restoring one old `../execution-plans/...` release path made the
script report exactly one broken link and exit 1; restoring the corrected path
returned zero. The audit recorded four such broken links at the pre-fix
baseline.

## Learning and rejected suggestions

- The store order sentence "primary-key order" was false for `pivot` and
  `event` because their query order is semantic, not primary-key order. The
  README and help now state the per-table query order rather than a blanket
  claim.
- The report artifact test only checks that required files exist. The doc now
  says exactly that; claiming that an extra file fails the build was a
  stronger guarantee than the test provided. W11 may strengthen the test if
  the human wants the stricter contract.
- Reason 5 is produced by the dashboard's progression assembly, not by the
  model axis or the report. The report contract now says it is dashboard-only
  and that the report can leave `reason_code` null when a stable-but-null
  stage has no axis reason.
- The link check excludes `target/` and `data/` because they are generated or
  local artifacts. Historical documents are checked too; no historical link
  was exempted to make the check pass.

## Limitations

- The link check validates relative paths, not anchors inside a target file.
- It does not validate external URLs.
- The docs sweep is textual; no runtime behavior changed, so no Quant Validator
  record is required.
