# Execution Plan: The Research Dataset Export (AN5)

- Issue: [#80](https://github.com/quant-rule/quant-rule/issues/80) — feat: export CSV with manifest and data dictionary
- Branch: `feat/80-export`
- Pull request: [#96](https://github.com/quant-rule/quant-rule/pull/96)
- Status: Verified
- Date: 2026-10-24 (UTC)
- Contract: [`docs/analytics/2026-10-24/research-output.md`](../../analytics/2026-10-24/research-output.md) §2–§12
- Transparency log: [`docs/ai-transparency-log/2026-10-24/80-export.md`](../../ai-transparency-log/2026-10-24/80-export.md)

## 1. Intent

Turn the three layers into the research dataset on disk: the raw M1 file, `layer1.csv`, `layer2.csv`, `layer3.csv`, `manifest.json`, `data_dictionary.csv`, `data_dictionary.md`, `codes.csv`, and `README.md`. The export is deterministic and reachable as a shell command.

## 2. Current state

- `main` is at `4bbe5e6`; the layers (#79) and the full-series path (#66) are merged.
- The analytics crate has `resolution`, `alignment`, and `layers`; it has no export code.
- The shell reads an M1 CSV and builds the ladder; it does not write a research dataset.
- The help text, README command reference, and `help.txt` fixture list the existing commands.

## 3. Scope

Included:

- `quantrule-analytics::research`: the column registry, the layer writers, the dictionary, the code table, the manifest, the README, and the raw CSV.
- The shell command `research export <csv-path> <dataset> --data-version <id> [--root <dir>]`.
- The help text, README command reference, and help fixture.
- Tests: byte identity, widths, nulls, the missing block, the manifest, the dictionary, the codes, the README, the unsafe dataset, and the shell command (success, missing data version, unsafe dataset).

Excluded:

- The event, leg, level, and consensus files. The issue's scope names the layer files, and the records belong to the store epic (#64). The contract marks them as the dataset's record contract, not part of the layer export.
- The summary/relation analysis (#81) and the 100k dataset verification (#82).
- A new dependency.

## 4. Assumptions and uncertainty

- The raw file keeps the source header and timestamp form (`Date,Open,High,Low,Close,Volume`, `YYYY-MM-DD HH:MM:SS+00:00`), so the shell can read the dataset back with its existing commands. The processed layers use integer UTC seconds.
- The manifest lists the raw and layer data files; the metadata files are not listed, because the manifest cannot contain its own byte count and the dictionary/codes are self-describing.
- The export renders every file before writing any of them, then writes each file atomically. An I/O failure after earlier writes leaves those files; the contract states that limitation.
- The layer width is the contract's: 61, 73, and 155 + 67n. The export asserts it in tests.
- The layer3 file is wide; the 100k time and memory are reported in #82.

## 5. Decisions

| ID | Decision | Alternatives | Choice | Rationale |
|---|---|---|---|---|
| D1 | Record files | Include them; defer them | Defer them | The issue names the layer files, and the records are store tables (#64); the contract records the split |
| D2 | Raw format | Research format; source format | Source format | The dataset can be re-read by `candles`/`market-state`; the processed layers carry the research form |
| D3 | Manifest file list | All files; data files only | Raw and layer data files | The manifest cannot list its own bytes, and the metadata is self-describing |
| D4 | Write atomicity | Directory temp and rename; per-file atomic | Per-file atomic | Per-file is sufficient for an interrupted write; the whole-directory rename would need destructive replacement |
| D5 | Header/dictionary source | Separate lists; one registry | One registry with adjacent writers | The width and dictionary tests catch a column that is written but not documented |
| D6 | Float form | Fixed decimals; shortest round-trip | Shortest round-trip, `-0` as `0` | The contract's form; byte-stable |
| D7 | Missing higher block | Omit the columns; null fields | Null fields | The header is fixed by the configured ladder; the row keeps its width |
| D8 | Dataset validation | Accept any name; restrict | Restrict to letters, digits, `.`, `-`, `_`, not `.`/`..` | The name is a directory component and must not escape the root |
| D9 | `--root` | Fixed `data`; option | Option, default `data` | Tests and alternate layouts need a root without touching `data/` |
| D10 | Column constructor | Eight arguments; builder | Eight arguments with a narrow allow | The registry has 161 rows; a builder would add noise without a new invariant |

## 6. Acceptance criteria

- `research export` writes the raw, three layer files, and five metadata files under the root in the contract's layout.
- Two renders of the same input and configuration are byte-identical.
- The layer headers are 61, 73, and 356 columns; every row has the header width.
- A missing higher block is written as empty fields, and the aggregate ladder names only the closed timeframes.
- The dictionary has one row per column of every written file, and every layer3 header name appears in it.
- The code table covers the code registries used by the layer files.
- The manifest records the dataset, data version, model version, contract dates, ladder, structure scale, cap, row counts, and data file list.
- An unsafe dataset name, an empty data version, and an empty root are rejected.
- `fmt`, `clippy -D warnings`, `cargo test`, and `doc -D warnings` are clean.

## 7. Work breakdown

1. Add the column registry and the layer writers.
2. Add the dictionary, code table, manifest, README, and raw writer.
3. Add the `research export` parser, runner, dispatch, help, and README entry.
4. Add the analytics and CLI tests; regenerate `help.txt`.
5. Update the contract for the raw format, the data-file list, the record-file split, and per-file atomic writes.
6. Run the checks, write the plan and the log, commit, push, open the PR, move #80 to Done, merge, and clean up.
7. Continue with #81.

## 8. Verification

| Check | Result |
|---|---|
| `cargo test -p quantrule-analytics` | 44 passed, 0 failed, 1 ignored; 2 doctests |
| `cargo test -p quantrule-shell --test cli research_export` | 3 passed |
| `cargo test --workspace` | 738 passed, 0 failed, 2 ignored |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | clean |
| Fixture export (260 M1 bars) | 260 raw, 333 Layer 1/2, 260 Layer 3; 9 files |
| Byte identity | A second export to the same root matches every file |
| Widths | Layer 3 header = 356 columns |
| Dictionary | 496 rows: 6 raw + 61 + 73 + 356 |
| Codes | 95 rows |

## 9. Risks and recovery

| Risk | Mitigation |
|---|---|
| The record files are expected in the dataset | The contract marks them as the store's record contract; the layer export's scope is stated in the plan and PR |
| The wide layer3 is expensive at 100k | #82 measures and reports; the file is rendered in memory once |
| The header and writer drift | Both use the same column registry; the width and dictionary tests fail on a mismatch |
| The manifest paths depend on `--root` | The root is part of the configuration; the byte-identity test uses two roots and ignores the manifest paths |
| An untrusted dataset name escapes the root | The name is validated before any path is built |

## 10. Stop conditions

- A column cannot be traced to a layer field.
- The raw file cannot be read back by the shell's source format.
- The manifest would need a value the shell cannot supply without a clock.
- A check fails and the fix is outside this scope.

## 11. Delivery

| Item | Value |
|---|---|
| Commit | `b7a54e9` — `feat: export the research dataset` |
| Branch | `feat/80-export` |
| Pull request | [#96](https://github.com/quant-rule/quant-rule/pull/96) |
| Board | #80 moved to `Done` when the PR is open |

Merge, release, tag, and deployment remain with the human.
