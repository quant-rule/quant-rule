# 9. Command Reference

- Status: Current
- Audience: operator or agent looking up one command contract

Every command accepts the global flags `--profile <id>`, `--token <token>`, and
`--json`. The table lists only command-specific arguments. Usage and output
follow the `help` registry; the stable-error column names codes from
`quantrule-api`. The manual checker fails when a command is missing here or its
usage drifts from the registry.

## Health and profiles

| Command | Usage | Required | Optional | Output | Stable errors | Browser | Example |
|---|---|---|---|---|---|---|---|
| `health` | `health` | — | — | API version, aggregate state, image digest, service lines, reserved queue/worker counts | `auth.unauthorized`, `service.unavailable`, `internal` | Connection → health table | `quantrule health` |
| `profile list` | `profile list` | — | — | Active profile and one line per profile with counts | `auth.unauthorized`, `service.unavailable` | Profiles → profile table | `quantrule profile list` |
| `profile create` | `profile create <name> [--id <profile_id>]` | name | `--id` | `profile_id`, name, counts | `profile.invalid`, `auth.unauthorized` | Profiles → create form | `quantrule profile create "Research Lab"` |
| `profile use` | `profile use <profile_id>` | profile id | — | Active profile | `profile.not_found`, `profile.invalid` | Profile switcher | `quantrule profile use research-lab` |

## Dataset

| Command | Usage | Required | Optional | Output | Stable errors | Browser | Example |
|---|---|---|---|---|---|---|---|
| `dataset list` | `dataset list` | — | — | One line per dataset: version, name, candles, coverage, bytes | `profile.not_found`, `service.unavailable` | Datasets → table | `quantrule dataset list` |
| `dataset show` | `dataset show <data_version>` | data version | — | Record with object key, hash, coverage, quality, created time | `dataset.not_found`, `profile.not_found` | Datasets → Details | `quantrule dataset show <data_version>` |
| `dataset upload` | `dataset upload <csv-path> [--name <name>] [--tolerant]` | CSV path | `--name`, `--tolerant` | Dataset record and quality report | `dataset.invalid`, `upload.too_large` | Workbench → Data → Upload | `quantrule dataset upload candles.csv --name btc-1m` |

## Config

| Command | Usage | Required | Optional | Output | Stable errors | Browser | Example |
|---|---|---|---|---|---|---|---|
| `config list` | `config list` | — | — | Config id, name, origin, validity, hash | `profile.not_found`, `service.unavailable` | Configs → table | `quantrule config list` |
| `config show` | `config show <config_id>` | config id | — | Config id, name, origin, valid, message, hash | `config.invalid`, `profile.not_found` | Configs → Details | `quantrule config show <config_id>` |

## Run

| Command | Usage | Required | Optional | Output | Stable errors | Browser | Example |
|---|---|---|---|---|---|---|---|
| `run submit` | `run submit <data_version> <config_id> [--model-artifact <id>]` | data version, config id | `--model-artifact` | `run_id`, status, created | `run.invalid`, `run.model_artifact_invalid`, `config.invalid`, `dataset.not_found`, `queue.unavailable` | Configs → Submit | `quantrule run submit <data_version> <config_id>` |
| `run status` | `run status <run_id>` | run id | — | Identity, model artifact, stages, provenance, artifacts, digest, error | `run.not_found`, `profile.not_found` | Runs → run detail | `quantrule run status <run_id>` |
| `run list` | `run list [--status <state>] [--limit <n>]` | — | `--status`, `--limit` | One line per run with status, config, data, submitted, elapsed | `run.invalid`, `profile.not_found` | Runs → table | `quantrule run list --status succeeded --limit 20` |
| `run fetch` | `run fetch <run_id> <directory>` | run id, directory | — | Run id, directory, one line per verified artifact | `run.not_found`, `run.invalid`, `internal` | Run detail → Download | `quantrule run fetch <run_id> ./bundle` |
| `run cancel` | `run cancel <run_id>` | run id | — | Run detail after the transition | `run.not_found`, `run.conflict` | Runs → Cancel | `quantrule run cancel <run_id>` |

## Stage

| Command | Usage | Required | Optional | Output | Stable errors | Browser | Example |
|---|---|---|---|---|---|---|---|
| `stage run model` | `stage run model <data_version> <config_id>` | data version, config id | — | `stage_run_id`, state, created | `stage.invalid`, `dataset.not_found`, `config.invalid` | Workbench → Model → submit | `quantrule stage run model <data_version> <config_id>` |
| `stage run strategy` | `stage run strategy <model_artifact_id> <config_id>` | model artifact, config id | — | `stage_run_id`, state, created | `stage.not_found`, `stage.invalid`, `config.invalid` | Workbench → Strategy → submit | `quantrule stage run strategy <model_artifact_id> <config_id>` |
| `stage run portfolio` | `stage run portfolio <decisions_artifact_id> <config_id>` | decisions artifact, config id | — | `stage_run_id`, state, created | `stage.not_found`, `stage.invalid`, `config.invalid` | Workbench → Portfolio → submit | `quantrule stage run portfolio <decisions_artifact_id> <config_id>` |
| `stage explain` | `stage explain <stage_run_id> --bar <n>` | stage id, bar | — | Decision, side, reason, positions, published evidence | `stage.not_found`, `stage.invalid` | Strategy → Explain bar | `quantrule stage explain <stage_run_id> --bar 12` |
| `stage list` | `stage list [--kind model\|strategy\|portfolio] [--status <state>] [--limit <n>]` | — | `--kind`, `--status`, `--limit` | One line per stage with kind, state, config, submitted, artifact | `stage.invalid`, `profile.not_found` | Workbench → stage table | `quantrule stage list --kind model --status succeeded` |
| `stage show` | `stage show <stage_run_id>` | stage id | — | Identity, state, provenance, attempts, error, artifact files, `evidence: false` | `stage.not_found` | Workbench → stage detail | `quantrule stage show <stage_run_id>` |
| `stage events` | `stage events <stage_run_id> --timeframe <tf>` | stage id, timeframe | — | Event table and one coverage line per report timeframe | `stage.not_found`, `stage.invalid` | Model → Read events | `quantrule stage events <stage_run_id> --timeframe 5m` |
| `stage fetch` | `stage fetch <stage_run_id> <directory>` | stage id, directory | — | Stage id, directory, one line per verified artifact | `stage.not_found`, `stage.invalid` | Stage detail → Download | `quantrule stage fetch <stage_run_id> ./stage-bundle` |
| `stage cancel` | `stage cancel <stage_run_id>` | stage id | — | Stage detail after the transition | `stage.not_found`, `stage.conflict` | Stage detail → Cancel | `quantrule stage cancel <stage_run_id>` |
| `stage delete` | `stage delete <stage_run_id>` | stage id | — | Deleted stage id | `stage.not_found`, `stage.conflict` | Stage detail → Delete | `quantrule stage delete <stage_run_id>` |

## Research

| Command | Usage | Required | Optional | Output | Stable errors | Browser | Example |
|---|---|---|---|---|---|---|---|
| `research create` | `research create --tier <model\|strategy\|portfolio> --hypothesis <text> --partition <name> --data-version <digest> --config <config_id> [--criteria <criteria_id>] --green-criteria <json> --code-commit <commit> [--baseline <id>] [--seed <n>] [--analytics <true\|false>]` | tier, hypothesis, partition, data version, config, green criteria, code commit; criteria for strategy/portfolio | baseline, seed, analytics; criteria for model only | `improvement_id`, tier, state, manifest, timestamps | `research.invalid`, `config.invalid`, `dataset.not_found` | Research → create form | `quantrule research create --tier model --hypothesis "a retest holds" --partition model_dev --data-version <data_version> --config <config_id> --green-criteria '{"return_target":0.1}' --code-commit <commit>` |
| `research submit` | `research submit <improvement_id>` | improvement id | — | Improvement detail with `pipeline_run_id`, `state: submitted` | `research.not_found`, `research.conflict` | Research → Submit pipeline run | `quantrule research submit <improvement_id>` |
| `research list` | `research list [--tier <tier>] [--state <state>] [--limit <n>]` | — | `--tier`, `--state`, `--limit` | One line per run with tier, state, decision, timestamps | `research.invalid`, `profile.not_found` | Research → table | `quantrule research list --tier model` |
| `research show` | `research show <improvement_id>` | improvement id | — | State, decision, note, manifest, run, digest, timestamps | `research.not_found` | Research → record detail | `quantrule research show <improvement_id>` |
| `research compare` | `research compare <improvement_id> [--baseline <id>]` | improvement id | `--baseline` | Candidate and baseline ids, metric rows, note | `research.not_found`, `research.conflict` | Research → Compare | `quantrule research compare <improvement_id> --baseline <baseline_id>` |
| `research decide` | `research decide <improvement_id> --decision <accepted\|rejected\|no-edge\|invalidated\|new-hypothesis> --note <text>` | improvement id, decision, note | — | Decided improvement detail | `research.not_found`, `research.conflict`, `research.invalid` | Research → Human decision | `quantrule research decide <improvement_id> --decision accepted --note "criteria passed"` |
| `baseline list` | `baseline list` | — | — | One line per baseline with tier, run, scope, date | `profile.not_found` | Research → baseline table | `quantrule baseline list` |
| `baseline show` | `baseline show <model\|strategy\|portfolio>` | tier | — | One baseline row | `research.not_found` | Research → baseline detail | `quantrule baseline show model` |
| `invalidation list` | `invalidation list` | — | — | One line per tombstone | `profile.not_found` | Research → invalidation list | `quantrule invalidation list` |

## Inspect

| Command | Usage | Required | Optional | Output | Stable errors | Browser | Example |
|---|---|---|---|---|---|---|---|
| `inspect candles` | `inspect candles <data_version>` | data version | — | Version, name, candles, first/last bar, bytes | `dataset.not_found` | Inspect → Candles | `quantrule inspect candles <data_version>` |
| `inspect timeframes` | `inspect timeframes <data_version>` | data version | — | One line per timeframe with bars, partial buckets, coverage | `dataset.not_found` | Inspect → Timeframes | `quantrule inspect timeframes <data_version>` |
| `inspect events` | `inspect events <data_version> --timeframe <tf>` | data version, timeframe | — | One JSON event record per line in availability order | `dataset.not_found`, `run.invalid` | Inspect → Events | `quantrule inspect events <data_version> --timeframe 5m` |

## Simulation

| Command | Usage | Required | Optional | Output | Stable errors | Browser | Example |
|---|---|---|---|---|---|---|---|
| `simulation list` | `simulation list [--status <status>] [--limit <n>]` | — | `--status`, `--limit` | One line per simulation with status, windows, marks, premise | `simulation.not_found`, `profile.not_found` | Simulation → table | `quantrule simulation list --status marking` |
| `simulation create` | `simulation create --premise <text> --bundle <digest> --seed <n> --criteria <text>` | premise, bundle, seed, criteria | — | Server-assigned `sim_id` | `simulation.mark_invalid`, `service.unavailable` | Simulation → create form | `quantrule simulation create --premise "a higher low holds" --bundle <data_version> --seed 7 --criteria "good entry within five bars"` |
| `simulation windows` | `simulation windows <sim_id>` | simulation id | — | Deterministic window selection | `simulation.not_found` | Simulation run → windows | `quantrule simulation windows <sim_id>` |
| `simulation mark` | `simulation mark <sim_id> --chart <chart> --bar <n> --action entry\|exit --quality good\|bad\|neutral --side long\|short [--note <text>]` | simulation id, chart, bar, action, quality, side | `--note` | Server-assigned mark identity | `simulation.not_found`, `simulation.mark_invalid`, `simulation.locked` | Simulation run → mark form | `quantrule simulation mark <sim_id> --chart 1 --bar 12 --action entry --quality good --side long` |
| `simulation marks` | `simulation marks <sim_id> [--origin human\|oracle]` | simulation id | `--origin` | One JSON mark record per line | `simulation.not_found` | Simulation run → marks table | `quantrule simulation marks <sim_id> --origin human` |
| `simulation oracle` | `simulation oracle <sim_id>` | simulation id | — | One JSON mark record per line | `simulation.not_found` | Simulation run → Run oracle | `quantrule simulation oracle <sim_id>` |
| `simulation check` | `simulation check <sim_id>` | simulation id | — | Check table and verdict | `simulation.not_found`, `simulation.check_incomplete` | Simulation run → check | `quantrule simulation check <sim_id>` |
| `simulation decide` | `simulation decide <sim_id> <test\|refine\|stop> [--note <text>]` | simulation id, decision | `--note` | Decision record | `simulation.not_found`, `simulation.locked` | Simulation run → decision | `quantrule simulation decide <sim_id> test --note "the rule is testable"` |

## Verify

| Command | Usage | Required | Optional | Output | Stable errors | Browser | Example |
|---|---|---|---|---|---|---|---|
| `verify` | `verify <run_id>` | run id | — | Run id and recorded canonical digest | `run.not_found`, `run.invalid` | Run detail → Verify | `quantrule verify <run_id>` |

## Help

| Command | Usage | Required | Optional | Output | Stable errors | Browser | Example |
|---|---|---|---|---|---|---|---|
| `help` | `help [<command>\|all\|glossary\|workflows]` | — | topic | Plain-text command map or contract | `internal` | Help → command table | `quantrule help run submit` |
