# Experiments

An experiment is one pre-registered test of a policy/strategy/engine change or
of a market hypothesis. It records the question, the matrix, the splits, the
costs, the results, the triage, and the next action.

## Location

```text
docs/experiments/<YYYY-MM-DD>/<EXP-ID>-<slug>.md
```

## Record

```text
# Experiment <EXP-ID>: <title>

- Date: YYYY-MM-DD (UTC)
- Status: Proposed | Running | Complete | Stopped
- Owner: Governing Swarm
- Policy: <path>
- Strategy: <path>
- Data versions: ...
- Tickets: ...
- Timeframes: ...
- In-sample: ...
- Out-of-sample / walk-forward: ...
- Costs: ...
- Green criteria: ...
- Iteration cap: ...
- Stop conditions: ...

## Hypothesis

One paragraph, in deterministic rule terms.

## Matrix

| Cell | Ticket | Timeframe | Data version | Config | Runs |
|---|---|---|---|---|---|

## Results

| Cell | Return | Max drawdown | Trades | Profit factor | Exposure | OOS result |
|---|---|---|---|---|---|---|

## Triage

Program defect / model / strategy / execution / portfolio / analytics / data /
no edge.

## Decision

What was decided, by whom, and the next experiment.

## Limitations

...
```

## Runner

`tools/experiment/run-matrix.sh` runs one `backtest` per manifest cell in
manifest order with an isolated output root. The script is deterministic and
does not create the experiment record; the Governing Swarm writes that from the
results. See `tools/experiment/README.md`.

## Rules

- Register the green criteria and the maximum number of variations before
  running.
- Record every variation attempted.
- Do not change a threshold after seeing the result; start a new experiment.
- Analytics changes are measurement changes, never a performance lever.
- A program fix and a trading change are never mixed in one experiment.
- The experiment record is not a validation record; model/strategy changes also
  need the Quant Validator record.
