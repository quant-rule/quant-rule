# QuantRule - User Journey and Value

## 1. The operator

One systematic researcher. They own the candles, choose the hypothesis, submit
batch runs, and decide whether to accept, revise, or stop. They are not a tenant,
not an account, and not a team. A reviewer is the secondary reader: someone who
must reproduce a result from an image digest and a canonical digest.

## 2. The first run

| # | The operator does | Sees | Value |
|---|---|---|---|
| 1 | Clones the repository and runs `make up` | `ready -> http://localhost:8080` | one command, no configuration file |
| 2 | Opens the URL | `connected . v1.0.0` | a client that is only a browser |
| 3 | Creates a profile, or keeps `default` | the switcher shows the active profile | research lines stay separate |
| 4 | Uploads a candle CSV once | `data_version`, candles, coverage | data identified by content |
| 5 | Picks a config, sees `valid`, submits | `202`, the run appears `queued` | submission returns immediately |
| 6 | Closes the laptop | - | the run does not depend on the session |
| 7 | Returns and opens the URL | `succeeded`, stages, provenance, digest, artifacts | evidence with an audit trail |
| 8 | Copies the digest, or runs `make verify` | one hash | reproducible, comparable, defensible |

## 3. The research loop

Before committing to a run, the operator can explore:

| # | Does | Sees | Value |
|---|---|---|---|
| 1 | Creates a simulation run from a recorded bundle and a premise | a seeded window selection | a bounded, reproducible sample |
| 2 | Opens a window | candles, EMAs, events, levels | the context, with the future visible |
| 3 | Marks entries and exits as good or bad | marks drawn on the chart | judgment captured in a schema |
| 4 | Runs the oracle | the deterministic marks for the same window | a mechanical second opinion |
| 5 | Reviews the difference | a candidate rule with at most two conditions | a testable hypothesis |
| 6 | Runs the check on a later, unmarked window | an event study, 30-event minimum | a screen, not an edge |
| 7 | Decides test, refine, or stop | a decision record | a hypothesis for Strategy Research, or a stop |

The loop is hindsight by design. It never feeds a run, and a strategy can never
read it.

## 4. Remote compute

The operator starts the same stack on a server, points the browser at it, and
submits from any machine. Closing the laptop changes nothing, because the run is
a server job.

## 5. Profiles

Profiles organise research: one per instrument, timeframe set, or campaign. They
share the content-addressed bytes and keep separate records. They are not a
security boundary, and the product never calls them one.

## 6. The desktop shell

A GitHub Release asset installs a Tauri window. It asks for the gateway URL once,
then behaves exactly like the browser: same screens, same profiles, same data. It
contains no engine and reads no local file.

## 7. What the product is worth

| Value | Because |
|---|---|
| **Reproducible** | The same inputs and image digest produce the same data artifacts |
| **Auditable** | Every result names its data version, config hash, model version, and image digest |
| **Detachable** | A run outlives the client that submitted it |
| **Portable** | Any machine with the URL uses it; the compute can move |
| **Organised** | Profiles keep separate research lines separate |
| **Sovereign** | No account, no vendor; candle data stays on the operator's network |
| **Honest** | A negative result is valid and the workflow can stop |

## 8. What the operator never does

- Type a filesystem path.
- Create an account.
- Install Docker to use the desktop shell.
- Compare a bundle by hand.
- Wait with a terminal open.
- Lose a research line in a single undifferentiated list.
