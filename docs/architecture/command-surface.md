# Command Surface — the Gateway Client

- Status: Current; v1.0.0 distributed surface
- Owner: Human
- Part of: [`docs/architecture/README.md`](README.md)
- Claim type: Behaviour Claim

## 1. Why the surface is command-first

QuantRule has one client contract and two clients. The browser client and the
command shell both speak `quantrule.api/v1` through the gateway
([`http-api.md`](http-api.md)). The shell is primarily for agents and remains the
stable command interface; the browser client is the human workstation. Feature
parity is required, and a capability that only one client can reach is a parity
gap, not a completed capability.

The shell is a thin client. It parses one command, makes one gateway call, and
renders the contract type that comes back. It has no engine dependency, opens no
local database, and executes no pipeline stage. Every capability is a server
capability; the shell is one way to type it.

The surface follows six rules:

1. **Command-first.** Every command has a stable name, its arguments, its output
   shape, and its error behavior.
2. **Reference-first.** A command names an identifier — `data_version`,
   `config_id`, `run_id`, `sim_id` — never a filesystem path. The server owns its
   filesystem.
3. **One call.** A command makes one gateway operation and prints its result.
   `run fetch` is the exception: it reads the run record and then downloads each
   named artifact, because the contract exposes artifacts one at a time.
4. **No stubs.** A command appears in `help`, the README, and a tutorial only
   when it is implemented and tested.
5. **Deterministic output.** The same response prints the same bytes. Lists are
   sorted where the gateway order is not part of the contract; no command reads
   the local clock or renders a timestamp the server did not send.
6. **Human acceptance.** The server measures, decides, fills, accounts, and
   evaluates. The shell displays. No command declares an edge or accepts a
   strategy.

## 2. Stage contract

The pipeline stages still exist; the shell no longer runs them. A run is the
full pipeline, and inspection is read-only.

| Stage | Engine | Reached through |
|---:|---|---|
| 1 | Data Engine | `dataset upload`, `inspect candles`, `inspect timeframes` |
| 2 | Model Engine | `run submit`, `inspect events` |
| 3 | Strategy Engine | `run submit` |
| 4 | Execution Engine | `run submit` |
| 5 | Portfolio Engine | `run submit` |
| 6 | Qualification Engine | the run's evidence artifact set |
| 7 | Analytics Engine | the run's result artifact set |
| — | Backtest orchestration | `run submit`, `run status`, `run fetch` |

The stage names are fixed. Model "predictions" are measurements; strategy
"edge" is a research verdict about decisions, not an engine output;
qualification "verdict" is evidence status, not acceptance.

## 3. Command registry

A command is **Implemented** only when it has a `CommandHelp` entry in
[`crates/quantrule-shell/src/command/help.rs`](../../crates/quantrule-shell/src/command/help.rs),
a row in the README command table, and a runnable example in a tutorial. The
checker [`tools/check_command_surface.py`](../../tools/check_command_surface.py)
enforces the distinction.

| Command | Status | Input | Output |
|---|---|---|---|
| `health` | Implemented | — | service states, queue depth, worker count |
| `dataset list` | Implemented | — | datasets with `data_version`, candles, coverage |
| `dataset show` | Implemented | `data_version` | one dataset record |
| `dataset upload` | Implemented | CSV file, optional name | `data_version` and quality |
| `config list` | Implemented | — | configs with validity |
| `config show` | Implemented | `config_id` | one configuration summary |
| `run submit` | Implemented | `data_version`, `config_id` | `run_id`, state |
| `run status` | Implemented | `run_id` | state, stages, provenance, digest |
| `run list` | Implemented | `status`, `limit` | run summaries |
| `run fetch` | Implemented | `run_id`, directory | artifact set |
| `run cancel` | Implemented | `run_id` | new state |
| `inspect candles` | Implemented | `data_version` | counts and coverage |
| `inspect timeframes` | Implemented | `data_version` | bars and partial buckets |
| `inspect events` | Implemented | `data_version`, `timeframe` | event batch |
| `verify` | Implemented | `run_id` | recorded canonical digest |
| `profile list` | Implemented | — | profiles with counts |
| `profile create` | Implemented | `name`, optional id | `profile_id` |
| `profile use` | Implemented | `profile_id` | active profile |
| `simulation list` | Implemented | `status`, `limit` | simulation run summaries |
| `simulation create` | Implemented | premise, bundle, seed, criteria | `sim_id` |
| `simulation windows` | Implemented | `sim_id` | deterministic window selection |
| `simulation mark` | Implemented | `sim_id`, chart, bar, action, quality, side, note | mark id |
| `simulation marks` | Implemented | `sim_id`, origin | human and oracle marks |
| `simulation oracle` | Implemented | `sim_id` | deterministic oracle marks |
| `simulation check` | Implemented | `sim_id` | event-study table and verdict |
| `simulation decide` | Implemented | `sim_id`, test/refine/stop, note | decision record |
| `help` | Implemented | topic | the command map |

Every command accepts `--profile <id>`, which sets `X-QuantRule-Profile`, and
`--json`, which prints the gateway JSON instead of the human rendering. With no
flag, the shell uses the stored default and then `default`.

## 4. Output and identity rule

- The shell prints the values the gateway sent. It does not average, relabel, or
  derive a measurement. A missing value is `none` or a null in the JSON view,
  never zero.
- `run fetch` verifies each downloaded artifact against the record's size and
  SHA-256 before writing it, and refuses an artifact name that is not a safe
  relative path.
- A failed call prints the envelope's stable code and message, for example
  `error: run.not_found: run is unknown`. The shell never turns a refused profile
  into a silent default.
- The shell stores only the local default profile, under `QUANTRULE_HOME`
  (default `$HOME/.quantrule`). It stores no result, no token, and no data.

The gateway base comes from `QUANTRULE_API_URL`; the default is
`http://127.0.0.1:8080/api/v1`.

## 5. Discovery contract

A command exists only when all five surfaces agree:

| Surface | What it carries |
|---|---|
| `help.rs` | The name, usage, summary, output shape, and contract notes. |
| README command table | The current command set and a one-line output description. |
| Tutorial | At least one runnable invocation. |
| Command registry table | The status, input, and output. |
| Checker | A failing gate when any of the other four disagree. |

The checker reads the implemented commands from `help.rs`, the README table, the
registry table above, and the fenced code blocks under `docs/tutorials/`. It
fails when an implemented command is missing from any surface, when a README or
registry row names a command `help.rs` does not have, when a tutorial invokes an
unimplemented command, or when a `CommandHelp` entry has an empty usage,
summary, or output contract.

## 6. Qualification Engine boundary

The Qualification Engine is stage 6. It answers one question: **does the
evidence satisfy the human-owned acceptance criteria?** It does not answer
whether the strategy is profitable in general, and it does not accept the
strategy.

- The **human owns** the account objective, risk envelope, universe, thresholds,
  partitions, costs, and pass/fail rules. They are recorded before the run.
- The engine **evaluates** the evidence bundle and emits a verdict of `pass`,
  `fail`, or `insufficient evidence`, with reasons and evidence references.
- The engine **never** declares an edge, chooses a threshold after seeing the
  result, drops a failing criterion, or writes the human's acceptance.
- The **human makes the final acceptance decision** in a separate record. A
  `pass` may still be rejected for residual risk; a `fail` cannot be accepted as
  an edge.

The criteria contract is in
[`docs/config/2026-11-06/qualification-criteria.md`](../config/2026-11-06/qualification-criteria.md).

## 7. Adding a command

A new command lands as one coherent change:

1. Add the server capability and its contract route; the shell gains no engine.
2. Add the `CommandHelp` entry with usage, summary, output, and notes.
3. Add the README row and the tutorial invocation.
4. Add the registry row.
5. Add the printed-output test against the fixture server.
6. Run `python3 tools/check_command_surface.py` and the full gate.

A change to an existing command's meaning is not part of this checklist. It is a
contract change reserved to the human.

## 8. Sequence

| Phase | Deliverable | Status |
|---:|---|---|
| 1 | Command and artifact contracts, qualification criteria, drift checker | Merged |
| 2 | Strategy commands and decisions artifact | Merged |
| 3 | Execution command and fills artifact | Merged |
| 4 | Portfolio command and performance artifact | Merged |
| 5 | Backtest orchestration command | Merged |
| 6 | Qualification command and record | Merged |
| 7 | Analytics evaluation command | Merged |
| 8 | Independent stage verification and closure | Merged and verified locally; human acceptance pending |
| 9 | v1.0.0 gateway client (this surface) | Implemented; parity and server dependencies tracked |

## 9. Removed in v1.0.0

`candles`, `candles-tolerant`, `timeframes`, `fetch`, `events`, `report model`,
`strategy explain`, `strategy run`, `execution run`, `portfolio replay`,
`backtest run`, `qualify`, `analytics evaluate`, `run check`, and `memory` are
removed from the shell. Their capability moves into the run and into read-only
inspection. Local source fetch and local stage execution are gone; the shell has
no engine dependency. The retirement is recorded by `DIST-018`.
