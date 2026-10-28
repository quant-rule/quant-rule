# ADR — Config-Driven Engines and Per-Engine Reports

- Date: 2026-10-28 (UTC)
- Status: Accepted
- Issue: [#168](https://github.com/quant-rule/quant-rule/issues/168)
- Program: [#141](https://github.com/quant-rule/quant-rule/issues/141)
- Plan: [`docs/execution-plans/2026-10-28/141-mvp-v1-final-contract.md`](../../execution-plans/2026-10-28/141-mvp-v1-final-contract.md)
- Config contract: [`docs/config/2026-10-28/per-engine-config.md`](../../config/2026-10-28/per-engine-config.md)
- Report contract: [`docs/reports/2026-10-28/per-engine-reports.md`](../../reports/2026-10-28/per-engine-reports.md)
- Supersedes in part: [`docs/adr/2026-10-27/analysis-and-backtest-boundaries.md`](../2026-10-27/analysis-and-backtest-boundaries.md)

## Context

The system is a pipeline: data engine, model engine, strategy engine, execution
engine, portfolio engine, and a research engine. Before this decision each stage
was reachable only through its own command with its own flags, and only the model
had a report. There was no full-execution run, no portfolio accounting, and no
single artifact set that said what a run had produced.

The MVP v1 program needs three things the earlier surface cannot express:

1. **A reproducible run.** A backtest is a claim that a result can be reproduced.
   That requires the whole configuration of the run to be recorded, not just the
   flags someone happened to pass. A long flag list is easy to omit, hard to
   review, and impossible to diff against a previous run.
2. **Engine ownership.** Each engine has its own inputs and its own output
   contract. A configuration field belongs to exactly one engine, and a change to
   one engine's parameters should not require editing a file owned by another.
3. **Report boundaries.** The model report must not publish fills, the execution
   report must not publish P&L, and the portfolio report must not publish axis
   labels. A monolithic report cannot enforce that boundary, because it holds
   every field in one place and every renderer can read every other engine's data.

The constraint from `AGENTS.md` is that the shell is the only layer allowed to do
I/O, and that no engine crate may gain a serialization dependency. Configuration
therefore has to be parsed at the shell boundary and handed to the engines as
plain Rust values.

## Decision

**One JSON file per engine, composed by `run.json`, parsed and validated in the
shell, and published as five independent report bundles.**

### Configuration

- Seven files: `run.json` plus one for each of the six engines (`data`, `model`,
  `strategy`, `execution`, `portfolio`, `analytics`).
- `run.json` is the composition root. It names the six files and the output root;
  it does not repeat engine values.
- JSON is parsed with `serde_json`, which the shell already depends on. No engine
  crate gains a serde dependency; each engine receives a validated Rust config
  value.
- Validation is strict: unknown fields, missing fields, wrong types, non-finite
  numbers, and out-of-range values are rejected before any candle is read or any
  byte is written.
- Relative paths resolve against the directory of the file that names them.
- Cross-config invariants are checked once, after all seven files load: the
  strategy timeframe is in the data ladder, the execution fill timeframe equals
  the strategy timeframe, and the analytics scale and timeframe set agree with
  the model and the ladder.
- The example configuration is committed under `config/examples/solusd-2025/`
  and is the specification's worked example.

### Commands

| Command | Meaning |
|---|---|
| `backtest --config <run.json> [--out] [--json]` | Run the full pipeline and write all five bundles |
| `report <engine> --config <run.json> [--out] [--json]` | Write exactly one bundle |
| `market-report <csv> ...` | Legacy alias for the model report; unchanged |
| `strategy-run <csv> ...` | Unchanged; decisions only |

The command line carries the config path, an optional output override, and an
optional JSON flag. It does not carry engine parameters.

### Reports

- Five bundles under `<out>/<engine>/`: `model`, `strategy`, `execution`,
  `portfolio`, `analytical`.
- A bundle is available if and only if its `report.json` exists. There is no
  manifest and no placeholder report.
- Each bundle has a fixed artifact list, recorded in `provenance.json` and
  asserted by a test.
- A boundary test scans the JSON keys and CSV headers of each bundle and fails on
  a forbidden name, so the separation is enforced by code rather than by review.
- `report.json` is written last, so a partial bundle is never available. On
  failure the command removes the partial bundle.
- `backtest` runs the pipeline once and renders all five bundles from the same
  result, so the model and analytical reports never recompute the model.

## Alternatives considered

### TOML instead of JSON

TOML reads better for hand-edited configuration, but the shell already depends on
`serde_json` for the model report and the strategy output, and it does not depend
on a TOML parser. Choosing TOML would add a dependency to parse a file whose shape
is a fixed tree with no repeated tables. The dependency cost is real and the
readability gain is small for this schema. Rejected.

### One mega-config

A single file holding every engine's parameters would be simpler to pass around,
but it would erase engine ownership: a field's owner would be a convention rather
than a file boundary, every engine change would touch the same file, and two runs
could not reuse one engine's block without copying it. The composition root keeps
the single entry point while preserving ownership. Rejected.

### A long flag list

Flags were the previous surface. They are not reviewable as a unit, they cannot be
diffed against a previous run, and a missing flag silently changes the result.
They also put engine parameters on the command line, which the product surface
should not carry. Rejected.

### A monolithic "full report"

One directory holding every artifact would be simple to describe and impossible
to boundary-check. The model report would share a file with P&L, the execution
report would be able to read equity, and the separation the program requires
would be a matter of renderer discipline. Five bundles make the boundary a
directory boundary and let each engine's report be generated alone. Rejected.

### YAML

Same dependency argument as TOML, with more parsing ambiguity (implicit typing,
anchors, multi-document streams). Rejected.

### Derive serde in every engine crate

This would put the schema next to the engine types, but it would add `serde` to
four crates and couple the wire format to the internal type layout. The shell
owns the boundary, so the shell owns the schema. Rejected.

## Consequences

### Positive

- A run is reproducible from seven committed files, and the run's configuration
  is recorded in `provenance.json` without absolute paths.
- Each engine owns its parameters; a change to execution does not touch the
  portfolio file.
- Validation happens before I/O, so a bad config leaves the filesystem untouched.
- Report boundaries are enforced by tests over keys and headers.
- A bundle can be generated alone, which keeps the five reports independently
  usable and independently testable.
- No new dependency.

### Negative

- Seven files per run is more to author than one. The example directory and the
  config reference carry the cost.
- The shell holds the schema, so a new engine field is a change in two places:
  the engine's Rust config type and the shell's schema module. A test over the
  example configs keeps them in step.
- Strict validation means an old config fails after a schema change rather than
  being silently defaulted. That is intended, and the schema version is the
  migration signal.

### Compatibility

- `market-report` keeps its four artifacts and its `report.json` envelope. It is
  the legacy alias, and its output is byte-identical to the four legacy artifacts
  of `report model`.
- `strategy-run` keeps its NDJSON contract and still creates no fill, position,
  equity, or P&L record.
- No existing `market-*`, `research`, `store`, `fetch`, or `candles` contract
  changes.

### Follow-up

- The config loader (#170) implements the schema and its validation table.
- The report writers (#173, #174, #175) implement the bundles and their boundary
  tests.
- The backtest command (#176) composes the five bundles from one run.
- Verification (#177) checks byte identity across two full-year runs and the
  compatibility of `market-report` and `report model`.
