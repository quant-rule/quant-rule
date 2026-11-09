# Local Data Cleanup — 2026-11-09

- Date: 2026-11-09 (UTC)
- Status: Complete
- Record ID: `CLEANUP-2026-11-09`
- Archive: `/home/zantgo/quant-rule-archive/2026-11-09/`
- Plan: [`btc-visual-benchmarks.md`](../../execution-plans/2026-11-09/btc-visual-benchmarks.md)
- Claim type: local workspace maintenance; no model, strategy, or edge claim.

## 1. What changed

The repository's tracked tree was already clean: the retired model, dashboard,
store, and chart surfaces were removed earlier, and `docs/registry/` records
them. The 44G footprint was local ignored evidence and build output.

| Action | Result |
|---|---|
| `cargo clean` on the main checkout | removed 8.8G of build cache |
| `data/scratch/binance/` | removed 199M of re-downloadable archive cache |
| `__pycache__` | removed from the working tree |
| Large historical evidence | moved to the archive and left as symlinks at the original paths |

## 2. Archived paths

The archive preserves the original relative paths. A symlink at the old path
still resolves, so historical documents and reproduction commands continue to
work.

| Original path | Size |
|---|---:|
| `data/model-v4/` | 9.8G |
| `data/model-gate/` | 3.4G |
| `data/session-spike/` | 905M |
| `data/prediction-pilot/` | 842M |
| `data/macd-pilot/` | 759M |
| `data/path-pilot/` | 697M |
| `data/structure-pilot/` | 695M |
| `data/improvement/2026-11-08/STRATEGY-2026-11-08-01/` | 15G |
| `data/improvement/2026-11-08/STRATEGY-2026-11-08-02/` | 1.1G |
| `data/improvement/2026-11-08/SR3-FEATURES/` | 1.8G |
| `data/improvement/2026-11-09/SR4-STACK-FADE/` | 1.3G |
| `data/improvement/2026-11-09/SR6-VWAP/` | 2.4G |

The local `data/` footprint fell from 44G to about 7.3G. The archive is about
38G. No tracked file was deleted.

## 3. Retained in place

- `data/sources/` — the only input data; immutable.
- `data/partitions.json`.
- `data/improvement/2026-11-09/SR9-15M-STRUCTURE/` — the current SR9 evidence
  and 30 charts.
- `data/improvement/2026-11-09/BINANCE-BTCUSDT/` — the cross-venue check and 73
  charts.
- `data/improvement/2026-11-09/SR5-ABSORPTION/`, `SR7-H3-CRYPTO/`,
  `SR8-H3-BUFFER/`, and `data/improvement/2026-11-08/SR2-*`.
- `data/research/`, `data/labs/`, `data/reports/`, `data/charts/`, `data/db/`,
  `data/notes/` — small and cited.

## 4. What was not deleted

Tracked historical configs (`config/model-v4/`, `config/compression*/`,
`config/research/`, `config/sr1..sr9/`) and historical tools (`tools/research/`,
`tools/improvement/`) remain. They are accepted evidence and CI exercises the
tools. Deleting them would require the
[Deprecation and Retirement Workflow](../../workflows/deprecation-retirement.md)
and would lose verified history for a few megabytes of savings.

## 5. Recovery

To restore a path:

```bash
rm /home/zantgo/Proyectos/quant-rule/data/<path>
mv /home/zantgo/quant-rule-archive/2026-11-09/data/<path> \
   /home/zantgo/Proyectos/quant-rule/data/<path>
```

To free the archive disk after accepting the loss of local reproducibility,
delete `/home/zantgo/quant-rule-archive/2026-11-09/`. That is a separate
destructive action and was not performed.

## 6. Verification

| Check | Result |
|---|---|
| `du -sh data` | 7.3G |
| `du -sh /home/zantgo/quant-rule-archive/2026-11-09` | 38G |
| Symlinks resolve for every archived path | pass |
| `git status --short` | no tracked data change |
| `python3 tools/check_links.py` | pass (see the plan verification) |
