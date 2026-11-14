# Local Research Memory from the Shell

> **Historical.** This lab records the v0.3.0 local surface. The local stage
> commands were removed from the shell in v1.0.0; set
> `BIN=./target/release/quantrule` before running a block, and see
> [the shell tutorial](../2026-11-14/shell-http.md) for the current surface.

- Date: 2026-11-12 (UTC)
- Commands: `memory`
- Claim type: Behaviour Claim

The shell and the desktop UI share the `quantrule.research-memory/v1` store
through the application service. The shell is the agent surface: it can record a
run, add metrics, claims, and notes, list evidence, and export or back up the
store.

## Open and inspect

```text
"$BIN" memory status --db /home/you/quant-rule-workspace/research.db
"$BIN" memory workspaces --db /home/you/quant-rule-workspace/research.db
```

`status` creates the database when it does not exist and prints its schema and
version. `workspaces` prints the stable workspace IDs and roots.

## Record a run and its evidence

```text
"$BIN" memory record --db /home/you/quant-rule-workspace/research.db --config config/examples/btcusd-2024/run.json --key run-2026-11-12-01 --started 1762900000
"$BIN" memory runs --db /home/you/quant-rule-workspace/research.db --workspace 1
"$BIN" memory metric set --db /home/you/quant-rule-workspace/research.db --run 1 --name profit_factor --kind real --value 1.25
"$BIN" memory claim add --db /home/you/quant-rule-workspace/research.db --run 1 --type "Behaviour Claim" --statement "the run is deterministic" --status accepted --created 1762900000
"$BIN" memory note add --db /home/you/quant-rule-workspace/research.db --workspace 1 --run 1 --body "first review" --created 1762900000
```

`record` computes the model and stores its counts with the run. A missing
measurement is null, never zero. The caller supplies every timestamp.

## Export and restore

```text
"$BIN" memory export --db /home/you/quant-rule-workspace/research.db --out /home/you/quant-rule-workspace/export.json
"$BIN" memory backup --db /home/you/quant-rule-workspace/research.db --out /home/you/quant-rule-workspace/backup.db
"$BIN" memory restore --backup /home/you/quant-rule-workspace/backup.db --target /home/you/quant-rule-workspace/restored.db
```

The logical export is atomic; the SQLite backup is consistent. Import accepts a
logical export only into an empty memory.
