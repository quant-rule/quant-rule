# W6a Event Order and Aggregate Guard Validation

- Date: 2026-10-31 (UTC)
- Work item: #266 (W6a)
- Findings: AUD-2026-10-31-09, -23
- Validator: Quant Validator (AI), sequential read-only pass
- Claim type: program correctness fix; no edge claim

## Reproduction

| Check | Baseline | Fixed |
|---|---|---|
| Bar 28 order | Minor-pivot test before EMA20 break | EMA20 break before minor-pivot test |
| Bar 37 signal order | Structure before Momentum | Momentum before Structure |
| Divergence direction 2 | Panic on array index | Count ignored, no panic |

The shell regression test fails with the old full sort and passes with the
availability/occurrence sort.

## Commands

| Command | Observed |
|---|---|
| `cargo test -p quantrule-shell --test cli events_keep_the_contract_order_on_one_bar` | Passed |
| `cargo test -p quantrule-model` | Passed |
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --all-targets -- -D warnings` | Passed |
| `cargo test --workspace` | 45 suites, 0 failures |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | Passed |

## Limitations

- Same-agent sequential pass, not independent.
- The out-of-range aggregate direction is guarded but not exercised by a
  dedicated fixture; the shell event-order test is the regression guard for the
  order finding.

## Verdict

Evidence supports Fixed for AUD-2026-10-31-09 and -23. Not an edge claim.
