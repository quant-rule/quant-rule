# AI Transparency Log: The Research Dataset Export (AN5)

- Work item: [#80](https://github.com/quant-rule/quant-rule/issues/80) — feat: export CSV with manifest and data dictionary
- Plan: [`docs/execution-plans/2026-10-24/80-export.md`](../../execution-plans/2026-10-24/80-export.md)
- Contract: [`docs/analytics/2026-10-24/research-output.md`](../../analytics/2026-10-24/research-output.md)
- Date: 2026-10-24 (UTC)

## Entry: the dataset export

- **Date and tool:** 2026-10-24 (UTC). opencode (DeepSeek V4.1 Flash 1017).
- **Intent and boundary:** Render the three layers and the raw M1 series into the research dataset, add the shell command, and document it. The record files are excluded because the issue's scope names the layer files and the records belong to the store epic.
- **Verification and evidence:**
  - Added `quantrule-analytics::research`: the column registry, the layer writers, the dictionary, the code table, the manifest, the README, and the raw writer.
  - Added `research export <csv-path> <dataset> --data-version <id> [--root <dir>]` to the parser, both dispatchers, the help text, and the README.
  - 44 analytics tests and 2 doctests pass; the 3 new CLI tests pass.
  - `cargo test --workspace`: 738 passed, 0 failed, 2 ignored.
  - `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` are clean.
  - The 260-bar fixture exports 260 raw, 333 Layer 1/2, and 260 Layer 3 rows in 9 files; a second export to the same root matches every file.
  - The Layer 3 header is 356 columns; the dictionary has 496 rows; the code table has 95 rows.
- **Learning:**
  - The first Layer 3 header builder wrote each segment with its own trailing newline, so the first line had 73 columns and the rest of the header leaked into the first data row. The width test caught it. The fix separates the column appending from the newline and adds the missing commas at the segment boundaries.
  - The dictionary needs the block columns with their `m5_`/`m15_`/`h1_` prefixes and prefixed reason columns. A flat list of the base specs would repeat `direction` three times; an explicit dictionary row owns the prefixed name and reason.
  - The raw file uses the source header and timestamp form, not the processed Unix-second form, so the dataset can be read back by the existing `candles`/`market-state` commands. The contract now states the distinction.
  - The manifest cannot list its own byte count, so its `files` list covers the raw and layer data files; the metadata files are self-describing. The contract is updated.
  - The record files are not written. The issue's scope names the layer files, and the event/leg/level/consensus records are store tables (#64). The contract keeps their schemas as the record contract and states that the layer export does not write them. This is a deliberate, recorded gap rather than an omission.
  - Clippy's `too_many_arguments` rejects the eight-field column constructor. It is allowed at the constructor only, with a comment: the registry has 161 rows, and a builder would add a line per field to every row without encoding a new invariant.
  - The Layer 3 file is about 1.2 KB per row on the 260-bar fixture because of the three higher-timeframe blocks. The 100k time and memory are left to #82, which is the verification issue.
  - The first manual run wrote to `target/research-demo` and left it; the CLI test uses `target/research-tests` and removes its directory first. No test reads or writes `data/`.

## Final verification

- `cargo test -p quantrule-analytics`: 44 passed, 0 failed, 1 ignored; 2 doctests.
- `cargo test -p quantrule-shell --test cli research_export`: 3 passed.
- `cargo test --workspace`: 738 passed, 0 failed, 2 ignored.
- `cargo fmt --all -- --check`: clean.
- `cargo clippy --all-targets -- -D warnings`: clean.
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`: clean.
- Status: `Verified`. Commit, push, pull request, and board transition follow the delivery step.
