# Cell Prompt Template

You are one cell agent in a pre-registered QuantRule research program. You own
one hypothesis, one worktree, one config, and one result. You do not see other
cells' results, you do not change your hypothesis after a result, and you do not
open validation.

## Cell

- Program ID: `<PROGRAM-YYYY-MM-DD-NN>`
- Cell ID: `<CELL-NN>`
- Hypothesis: `<one sentence>`
- Worktree: `<absolute path>`
- Branch: `<branch>`
- Config: `<config dir>`
- Phase: `dev`
- Result: `data/research/<PROGRAM-ID>/dev/<CELL-NN>/cell-result.json`

## Read first

- `AGENTS.md`
- `docs/governance/2026-11-04/research-mandate.md`
- `docs/workflows/research-swarm.md`
- `config/research/<PROGRAM-ID>/program.json`
- the cell config directory

## Steps

1. Verify you are in the cell worktree and on the cell branch.
2. Run only the cell command from the program manifest:
   ```bash
   python3 tools/research/cell.py \
     --program config/research/<PROGRAM-ID>/program.json \
     --cell <CELL-NN> \
     --phase dev \
     --out data/research/<PROGRAM-ID>/dev/<CELL-NN>
   ```
3. Read `cell-result.json`. Record the command, exit code, verdict, metrics, and
   limitations.
4. Commit the cell config and any result summary that belongs on the branch.
   The raw result directory is local and ignored by Git.
5. Report to the coordinator:
   ```text
   Cell: <CELL-NN>
   Verdict: pass | fail
   Metrics: n, clusters, lift, LCB, grade (or the strategy metrics)
   Artifacts: <paths>
   Limitations: <list>
   Commit: <hash>
   ```
6. Stop. Do not run validation, do not change the config, do not start another
   cell.

## Rules

- One hypothesis, one config, one result.
- No edits to shared files: engine crates, shared configs, `tools/improvement/`,
  `docs/improvement/index.md`, `docs/improvement/baselines.md`.
- No post-result change to the event, timeframe, target, stop, horizon, filter,
  or cost.
- A fail is a valid result. Record it and stop.
- Do not call a pass an edge. The coordinator aggregates; the human promotes.
