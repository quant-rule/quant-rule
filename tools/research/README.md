# Research tooling

> Status: current for the MR5.x research platform; retained for the M2.x–M4.x
> programme records and the reproducibility checks in CI. The scripts are not
> part of the M1.0 shell surface; the current model surface is `report model` and
> `events`.

The package validates programme manifests, resolves programme cells, runs the
model or shell cells, and checks exploratory coverage. Every cell may carry its
own model config and a sequence/confluence/column condition; the path report is
descriptive. The scripts are offline and deterministic: they take dates from the
manifest, never the wall clock, sort the collections they write, and never
compute a trading signal.

## Raw-edge diagnostic

`raw_edge.py` is an evaluation tool, not a programme runner. It reconstructs a
published entry signal from a model bundle's events and readings, enters at the
next bar open, and measures forward returns, MFE/MAE, and barrier outcomes with
no stop, target, halt, or position state. It writes `raw_edge.json` and
`raw_edge.md` only; it does not produce a strategy, execution, or portfolio
artifact and it never feeds a result back into a rule.

```sh
python3 tools/research/raw_edge.py --model <model-bundle> --out <dir>
```

`test_raw_edge.py` covers the forward arithmetic, the short-side mirror, the
same-bar stop-first tie, the signal variants, and the bundle parsing.

`event_edge.py` generalizes the screen to a configurable event signal. It reads a
small signal spec, matches the listed event identities and optional states within
the window, and writes the same forward and barrier measurements with an
all-bars control. It enforces the event-composite subtype range (1–7) so a spec
cannot silently rely on an event the strategy engine cannot read.

```sh
python3 tools/research/event_edge.py --spec <signal.json> --model <model-bundle> --out <dir>
```

`test_event_edge.py` covers spec validation, the event-identity parsing, the
window matching, the direction filter, and the control output.

`breakout_edge.py` implements the SR13 fresh-onset 5m rule: the first strict
close outside the accepted onset range within a fixed window, entry at the next
bar open, and fixed-horizon returns with no stop, target, halt, or position
state. It reconstructs the rule from the model bundle and reports the
unconditional, fade, random-direction, back-lag, non-coiling placebo, and
compression-only controls. It is a diagnostic and writes only JSON and Markdown.

```sh
python3 tools/research/breakout_edge.py --model <model-bundle> --out <dir> \
  --window 3 --horizons 1,3,6,12 --primary 6 --cost-bps 6 --stress-bps 9 --seed 0
```

`test_breakout_edge.py` covers the long and short break, the strict boundary,
window expiry, next-onset supersession, gap-open entry, end-of-series safety,
the non-overlapping sample, and the statistics.
