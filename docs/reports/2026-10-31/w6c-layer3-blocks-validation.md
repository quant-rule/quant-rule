# W6c Layer 3 Block Validation

- Date: 2026-10-31 (UTC)
- Work item: #274 (W6c)
- Finding: AUD-2026-10-31-08
- Validator: Quant Validator (AI), sequential read-only pass

## Reproduction

| Check | Baseline | Fixed |
|---|---|---|
| `research export single.csv` Layer 3 width | 155 columns | 356 columns |

The 356 width is the default three higher-timeframe blocks (155 + 3*67), even
though every higher series is empty; each block is null.

## Commands

| Command | Observed |
|---|---|
| `research export single.csv` | 356 Layer 3 columns |
| `cargo test -p quantrule-analytics` | 68 + 7 + 2 passed |
| `cargo build -p quantrule-shell` | Passed |

## Limitations

- Same-agent sequential pass.
- The full workspace gates are recorded in the PR.

## Verdict

Evidence supports Fixed for AUD-2026-10-31-08. Not an edge claim.
