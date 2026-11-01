# AI Transparency Log — Output-Path Collisions

- Date: 2026-10-31 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash), sequential role passes
- Work item: #308, branch `fix/308-output-path-collision`
- Plan: `docs/execution-plans/2026-10-31/full-system-audit.md` (W1)
- Audit: `docs/audits/2026-10-31/full-system-audit.md` (AUD-2026-10-31-41)

## Intent and boundary

The human approved the full-system audit plan on 2026-10-31. W1 closes the Major
finding that `fetch` and `market-chart` compared output paths only as raw
strings, so a CSV target equal to the PNG's temporary path, or an input CSV
spelled with a `.` alias, could be overwritten.

The change is confined to shell output-path validation and the atomic export
helpers. It does not touch the signal path, a strategy rule, an indicator,
execution, portfolio accounting, or any diagnosis threshold. It adds no
dependency and no I/O outside the shell.

## Implemented

- Added `export::outputs_conflict`, which resolves `.`/`..`, a symlinked parent,
  and an existing target; compares each final path with the other output's
  temporary path; and rejects two existing paths that are the same file by
  device/inode on Unix.
- Changed the atomic temporary name from `<target>.tmp` to
  `<target>.tmp.<pid>`, so two processes writing the same target do not share a
  temporary file. This is the approved D1=C defense in depth.
- `fetch` uses the comparison in the parser and again in `run_fetch` before the
  first write. `market-chart` uses it in the parser before reading the source.
- Added tests for a final/temp collision, a `.` alias, distinct outputs, a hard
  link, the `fetch` runtime collision, and the two command parsers.
- Updated the `fetch` and `market-chart` error text to name the resolved
  file/temporary-path rule.
- Updated the `fetch` test helper to the process-scoped temporary name.

## Verification and evidence

Commands actually run, with observed results:

| Command | Observed |
|---|---|
| `cargo test -p quantrule-shell --bin quantrule export::tests` | 10 passed, 0 failed |
| `cargo test -p quantrule-shell --bin quantrule fetch::tests` | 13 passed, 0 failed |
| `cargo test -p quantrule-shell --bin quantrule command::tests::rejects_a` | 9 passed, 0 failed |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo test --workspace` | 1,525 passed, 0 failed, 7 ignored; 47 suites |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | exit 0 |
| `git diff --check` | clean |

The targeted tests exercise the command parser and the `run_fetch` entry point,
so the collision is rejected before the mock transport is called or a file is
created. The existing golden CSV/PNG tests still write the normal pair.

## Learning and rejected suggestions

- The audit's example used the deterministic `<target>.tmp` name. The approved
  D1=C added the process-scoped suffix, so the exact old example is no longer a
  collision; the test now uses the actual temporary path. The resolved-path
  comparison remains the authoritative guard, and the unique suffix protects
  concurrent processes.
- `outputs_conflict` is symmetric and rejects `--csv out.csv --png
  out.csv.tmp` even though the CSV temporary file is renamed before the PNG
  final is written. This is deliberately conservative: the rule is "no output
  may equal another output's final or temporary path", and the unusual command
  is safer rejected than special-cased.
- The parser now resolves paths, which is read-only filesystem access in the
  shell. That is within the shell boundary; no engine reads a path or a file.
- I considered adding a second runtime check in `run_market_chart`, but its only
  caller is the parsed command dispatch, so the parser guard is the command
  boundary. The `fetch` runtime check is kept because `run_fetch` is also
  exercised directly by tests.

## Limitations

- The device/inode comparison is Unix-only; other platforms rely on the
  resolved-path comparison. The workspace's CI target is Linux.
- The check and the write are not one atomic operation. A path changed by
  another process between the check and the rename could still race; the unique
  temporary name and the atomic rename reduce the window. A full guarantee
  would require descriptor-relative operations.
- The tests use the mock transport and local artifacts, not a live provider.
- No Quant Validator record is required: this is shell output safety, not model
  or strategy behavior.
