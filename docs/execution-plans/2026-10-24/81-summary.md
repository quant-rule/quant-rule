# Execution Plan: Summary and Relation Analysis (AN6)

- Issue: [#81](https://github.com/quant-rule/quant-rule/issues/81) — feat: add summary and relation analysis
- Branch: `feat/81-summary`
- Pull request: pending
- Status: Verified
- Date: 2026-10-24 (UTC)
- Contract: [`docs/analytics/2026-10-24/research-output.md`](../../analytics/2026-10-24/research-output.md) §5–§7
- Transparency log: [`docs/ai-transparency-log/2026-10-24/81-summary.md`](../../ai-transparency-log/2026-10-24/81-summary.md)

## 1. Intent

Give the research dataset a deterministic analysis surface: counts, nulls, ranges, ordinal histograms, and pairwise continuous correlations per timeframe. The analysis must count ordinal codes rather than average them and must not produce a cross-axis composite.

## 2. Current state

- `main` is at `b2d2edb`; the layers (#79) and the export (#80) are merged.
- The analytics crate has `layers`, `resolution`, `alignment`, `research`, and no analysis.
- The shell has `research export`; it has no summary command.

## 3. Scope

Included:

- `quantrule-analytics::summary`: `AxisSummary`, `RelationSummary`, `DatasetSummary`, and `summarize`.
- The shell command `research summary <csv-path>`, its parser, both dispatchers, the help text, and the README.
- Tests for the counts, ranges, means, ordinal histograms, null handling, perfect positive and negative relations, a constant series, an empty set, determinism, and the CLI output.
- Regenerated `help.txt`.

Excluded:

- Rounding or smoothing of the reported numbers; they use the export's float form.
- A cross-axis composite or a single score.
- The 100k dataset verification (#82).
- A new dependency.

## 4. Assumptions and uncertainty

- The summary is computed from the same layer build as the export, not by re-reading the CSV files. The export is byte-identical for the same input, so the analysis corresponds to the exported layers.
- The correlation uses the resolution's continuous values, which are the contract's comparable values. Progression is clamped in that value; the raw `progress` is not used.
- Pearson correlation is undefined for fewer than two paired values or a zero-variance side; the summary reports null.
- The summary is a description of the dataset, not a signal or a recommendation.

## 5. Decisions

| ID | Decision | Alternatives | Choice | Rationale |
|---|---|---|---|---|
| D1 | Input | Re-read the exported CSVs; build the layers | Build the layers | The export is byte-identical for the same input; parsing 356 columns back adds no evidence and a second parser |
| D2 | Summary scope | One line per file; one line per timeframe and axis | Per timeframe and axis | The same axis has different behavior per timeframe; pooling would hide it |
| D3 | Ordinals | Mean of the codes; histogram | Histogram | The contract forbids averaging ordinal codes |
| D4 | Relations | One composite score; pairwise correlations | Pairwise Pearson | A composite would combine axes; the contract forbids it |
| D5 | Undefined correlation | Zero; omit the line | Null `r` | A valid zero is a value; an undefined relation is not |
| D6 | Accumulation | Any order; row order | Row order | Deterministic floats; the same layers produce the same numbers |
| D7 | Clamp | Report overshoot; clamp | Clamp to `[-1, 1]` | A Pearson coefficient cannot exceed the range; floating error can |
| D8 | Rendering | In the engine; in the shell | In the shell | The engine publishes numbers; the shell maps them to text |
| D9 | Command | A dataset name under `--root`; a CSV path | CSV path | The summary is a pure function of the source; the export dataset is optional |
| D10 | Shared load | Duplicate the export's load; extract a helper | Extract `load_layers` | One build path for export and summary |

## 6. Acceptance criteria

- `summary` prints the three layer counts.
- For every timeframe and axis, it prints the present count, null count, min, max, mean, ten ordinal counts, and the ordinal-null count.
- For every timeframe and axis pair, it prints the paired count and the correlation, or `null`.
- The ordinal counts sum to the present values, and the mean is computed from the continuous values only.
- A constant series reports a null correlation; a perfect relation reports ±1 within floating tolerance.
- Two runs print the same text.
- `fmt`, `clippy -D warnings`, `cargo test`, and `doc -D warnings` are clean.

## 7. Work breakdown

1. Add `summary` to the analytics crate.
2. Add the shell command, the shared load, the rendering, the dispatchers, and the help/README entries.
3. Add the unit and CLI tests; regenerate `help.txt`.
4. Run the checks, write the plan and the log, commit, push, open the PR, move #81 to Done, merge, and clean up.
5. Continue with #82.

## 8. Verification

| Check | Result |
|---|---|
| `cargo test -p quantrule-analytics summary` | 7 passed |
| `cargo test -p quantrule-shell --test cli research_summary` | 1 passed |
| `cargo test --workspace` | 746 passed, 0 failed, 2 ignored |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | clean |
| Fixture summary | 41 lines: 1 summary, 16 axes, 24 relations |

## 9. Risks and recovery

| Risk | Mitigation |
|---|---|
| The summary diverges from the exported layers | Both use the same `load_layers`; the export test pins the layer widths and row counts |
| A long float makes the output noisy | The values are exact and deterministic; rounding is a separate presentation decision |
| A correlation is undefined for short timeframes | The line is printed with `r=null`; the H1 rows in the fixture show it |
| The summary is mistaken for a signal | The command is descriptive; the README says it does not combine the axes |
| The 100k summary is slow | The summary is linear; #82 measures the dataset run |

## 10. Stop conditions

- A required statistic needs data the layer set does not carry.
- A correlation would need an ordinal code.
- A check fails and the fix is outside this scope.

## 11. Delivery

| Item | Value |
|---|---|
| Commit | pending |
| Branch | `feat/81-summary` |
| Pull request | pending |
| Board | #81 moved to `Done` when the PR is open |

Merge, release, tag, and deployment remain with the human.
