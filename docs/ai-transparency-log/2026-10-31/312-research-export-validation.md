# AI Transparency Log — Research Export Validation

- Date: 2026-10-31 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash), sequential role passes
- Work item: #312, branch `fix/312-research-export-validation`
- Plan: `docs/execution-plans/2026-10-31/full-system-audit.md` (W5)
- Audit: `docs/audits/2026-10-31/full-system-audit.md` (AUD-2026-10-31-46, -51)

## Intent and boundary

The human approved the full-system audit plan on 2026-10-31. W5 makes the
research export run the validation its contract promises and records the
configured ladder in the manifest, including a configured higher timeframe that
has no bars. The change is confined to the analytics research export and its
tests. It does not change model values, the strategy, execution, portfolio
accounting, or a research contract threshold. It adds no dependency.

## Implemented

- Added `research/validate.rs`. Before returning any file it checks:
  - the 100,000 M1 cap;
  - layer1/layer2/consensus row counts and layer3/M1 counts;
  - layer1 bar identity and timeframe grouping;
  - layer2 and consensus alignment with layer1;
  - layer3 base identity and M1 layer2 equality;
  - axis resolution range and null/reason consistency;
  - as-of higher-close and staleness causality;
  - event, level, pivot, and confirmed-leg availability;
  - dictionary completeness against the four data headers;
  - code completeness against `codes.csv`;
  - the absence of non-finite values in every rendered data field.
- The manifest and README now record the configured ladder, built from M1 plus
  the explicit higher ladder, rather than only the timeframes with present rows.
- Corrected the column registry: the aggregate event-subtype **count** columns
  no longer claim an event-subtype code registry. They are counts, not codes;
  the validation exposed the mismatch.
- Added eight research tests covering the manifest ladder and a validation
  failure for row counts, resolution range, staleness, event availability, code
  completeness, non-finite values, and the M1 cap.

## Verification and evidence

Commands actually run, with observed results:

| Command | Observed |
|---|---|
| `cargo test -p quantrule-analytics --lib research::tests` | 18 passed, 0 failed |
| `cargo test -p quantrule-analytics --test research_verification` | 7 passed, 0 failed, 1 ignored |
| `cargo test -p quantrule-shell --test cli research` | 5 passed, 0 failed |
| `cargo test -p quantrule-shell --test cap_workflow` | 1 passed, 0 failed |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo test --workspace` | 1,551 passed, 0 failed, 7 ignored; 47 suites |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | exit 0 |
| `git diff --check` | clean |

The valid fixture, the 600-bar verification dataset, the CLI research export,
and the over-cap workflow all pass the validation unchanged.

## Learning and rejected suggestions

- The code-completeness check found that aggregate event-count columns carried
  an event-subtype code registry. A count can be 0, 3, or 100; it is not a code.
  Clearing the registry is the correct contract representation, not a validation
  exemption.
- The first validation run rejected a test that rendered a one-candle raw file
  against a five-candle layer set. The layer3/M1 row-count check is right: the
  test was building an inconsistent dataset to inspect the raw format. The test
  now builds a matching one-candle set.
- I considered validating every aggregate float structurally, but the rendered
  finite scan covers every numeric field that is actually written, including
  aggregates and metrics, without duplicating the model's aggregate types.
- The record availability checks run on the in-memory `LayerSet` records. The
  layer export does not write the record files; the store owns that persistence.

## Limitations

- The validation is sequential and same-agent; no Quant Validator record is
  required because no model or strategy behavior changes.
- The 100k benchmark remains ignored; the cap itself is checked before render.
- The tests use deterministic fixtures, not live market data.
- The dictionary/code checks validate the four data files; the record files are
  outside this export.
