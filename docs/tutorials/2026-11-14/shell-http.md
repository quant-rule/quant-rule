# Tutorial — The Shell as a Gateway Client

- Date: 2026-11-14
- Status: Current for v1.0.0
- Audience: operator, agent, reviewer
- Related: [`command-surface.md`](../../architecture/command-surface.md),
  [`http-api.md`](../../architecture/http-api.md)

The shell is a thin client over `quantrule.api/v1`. It parses a command, makes
one gateway call, and prints the result. Every command accepts `--profile <id>`
and `--json`.

```bash
export QUANTRULE_API_URL=http://localhost:8080/api/v1
export QUANTRULE_HOME="$HOME/.quantrule"
```

## 1. Connection and profiles

```bash
quantrule health
quantrule profile list
quantrule profile create "Research Lab"
quantrule profile use research-lab
quantrule health --profile default
```

`profile create` derives a lowercase, hyphenated id unless `--id` names one.
`profile use` stores the local default only after the gateway lists it. The
`--profile` flag overrides the stored value and is never written.

## 2. Datasets and configs

```bash
quantrule dataset list
quantrule dataset upload candles.csv --name btc-1m
quantrule dataset upload candles.csv --tolerant
quantrule dataset show <data_version>
quantrule config list
quantrule config show <config_id>
quantrule inspect candles <data_version>
quantrule inspect timeframes <data_version>
quantrule inspect events <data_version> --timeframe 5m
```

The gateway owns the hash and the upload limit. The shell never constructs a
`data_version`.

## 3. A run

```bash
quantrule run submit <data_version> <config_id>
quantrule run status <run_id>
quantrule run list --status succeeded --limit 20
quantrule run fetch <run_id> ./bundle
quantrule verify <run_id>
quantrule run cancel <run_id>
```

`run fetch` writes each named artifact and checks its size and SHA-256 against
the run record before writing it. `verify` prints the digest the gateway
recorded; it does not recompute it. A failed call keeps its stable code:

```text
error: run.not_found: run is unknown
```

## 4. Simulation research

```bash
quantrule simulation create \
  --premise "a higher low into a confirmed level holds" \
  --bundle <data_version> --seed 7 --criteria "good entry within five bars"
quantrule simulation list --status marking
quantrule simulation windows <sim_id>
quantrule simulation mark <sim_id> --chart w1 --bar 12 \
  --action entry --quality good --side long --note "higher low"
quantrule simulation marks <sim_id> --origin human
quantrule simulation oracle <sim_id>
quantrule simulation check <sim_id>
quantrule simulation decide <sim_id> test --note "the rule is testable"
```

The server assigns `sim_id` and `marked_time_utc`. The client clock is not
trusted. `check` refuses fewer than 30 events, and marking after a decision is
refused with `simulation.locked`.

## 5. Help and JSON

```bash
quantrule help
quantrule help run submit
quantrule help all
quantrule help glossary
quantrule help workflows
quantrule run status <run_id> --json
```

`--json` prints the gateway value, so an agent reads the contract shape rather
than parsing the human table.

## Failure you should be able to explain

| Symptom | Cause |
|---|---|
| `profile.not_found` | The header names a profile the gateway does not have |
| `run.not_found` | The run id is unknown in this profile |
| `config.invalid` | The config failed validation, or the id is unknown |
| `simulation.check_incomplete` | Fewer than 30 events, or an unmarked window |
| `cannot reach the gateway` | `QUANTRULE_API_URL` is wrong or the stack is down |
