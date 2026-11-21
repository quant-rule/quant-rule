# Research tooling

- Status: Current; the Rust research commands replace the retired Python stack
- Commands: `qr-governance research program|batch|cell|registry|report`
- Model cell: `qr-governance improvement model-cell`
- Historical records: Git history and the committed `config/research/` manifests

The M2.x–M4.x Python runners, the coverage checker, the grid builders, and the
diagnostic edge screens were retired with the frozen research stack. They are
not current tooling and are not invoked by CI. The committed program manifests,
cell configs, and improvement records remain the audit trail.

## Current commands

| Command | Purpose |
|---|---|
| `qr-governance research program` | Validate a program manifest and phase |
| `qr-governance research batch` | Run selected cells in config or worktree mode |
| `qr-governance research cell` | Run one cell and write `cell-result.json` |
| `qr-governance research registry` | Write the program record from a report |
| `qr-governance research report` | Aggregate program results |
| `qr-governance improvement model-cell` | Merge yearly bundles and run the pooled prediction evaluation |

The result adapters registered in `qr-governance` are `path_validation`,
`strategy_evaluation`, `model_prediction`, `summary_json`, and `none`.
