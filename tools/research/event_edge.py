#!/usr/bin/env python3
"""Measure the raw forward edge of a configurable event signal.

The tool reads one or more model bundles, reconstructs a signal from published
events and states, and measures forward returns, MFE/MAE, and barrier outcomes
with no stop, target, halt, or position state. It is an evaluation tool: it
writes only a report and never a strategy, execution, or portfolio artifact.

The signal spec is a small JSON document:

```json
{
  "id": "h1-long",
  "side": "long",
  "window": 2,
  "events": [
    {"type": 1, "subtype": 3, "source": 8, "direction": -1},
    {"type": 1, "subtype": 2, "source": 8, "direction": 1}
  ]
}
```

All event conditions must be satisfied within the window ending at the signal
bar. The signal side is fixed by the spec. The event-composite engine does not
enforce order inside the window; this tool matches the engine's window
semantics, not a stronger sequence rule.
"""

from __future__ import annotations

import argparse
import csv
import json
import math
from dataclasses import dataclass
from pathlib import Path

import raw_edge

TIMEFRAME_SECS = 900
SIDES = ("long", "short")
DEFAULT_HORIZONS = (1, 3, 5, 10, 20)


@dataclass(frozen=True)
class EventCondition:
    type: int
    subtype: int
    source: int
    direction: int


@dataclass(frozen=True)
class SignalSpec:
    id: str
    side: str
    window: int
    events: tuple[EventCondition, ...]
    states: tuple[tuple[str, int], ...] = ()


@dataclass(frozen=True)
class EventBundle:
    label: str
    bars: tuple[raw_edge.Bar, ...]
    events: dict[tuple[int, int, int], dict[int, frozenset[int]]]


def load_spec(path: Path) -> SignalSpec:
    """Loads and validates one signal spec."""
    document = json.loads(path.read_text(encoding="utf-8"))
    if document.get("schema") != "quantrule.event-edge-signal/v1":
        raise ValueError(f"{path}: unsupported schema")
    side = document.get("side")
    if side not in SIDES:
        raise ValueError(f"{path}: side must be long or short")
    window = document.get("window")
    if not isinstance(window, int) or window < 1:
        raise ValueError(f"{path}: window must be a positive integer")
    raw_events = document.get("events")
    if not isinstance(raw_events, list) or not raw_events:
        raise ValueError(f"{path}: at least one event is required")
    events: list[EventCondition] = []
    for raw in raw_events:
        event = EventCondition(
            type=int(raw["type"]),
            subtype=int(raw["subtype"]),
            source=int(raw["source"]),
            direction=int(raw["direction"]),
        )
        if event.type not in (1, 2, 3):
            raise ValueError(f"{path}: type must be 1..3")
        if event.subtype not in range(1, 8):
            raise ValueError(f"{path}: subtype must be 1..7 for the event-composite language")
        if event.source not in range(1, 58):
            raise ValueError(f"{path}: source must be 1..57")
        if event.direction not in (-1, 0, 1):
            raise ValueError(f"{path}: direction must be -1, 0, or 1")
        events.append(event)
    states: list[tuple[str, int]] = []
    for raw in document.get("states", []):
        field = raw.get("field")
        if field not in ("ema_stack", "structure_state"):
            raise ValueError(f"{path}: unsupported state field {field!r}")
        states.append((field, int(raw["value"])))
    return SignalSpec(
        id=str(document["id"]),
        side=side,
        window=window,
        events=tuple(events),
        states=tuple(states),
    )


def parse_all_events(path: Path) -> dict[tuple[int, int, int], dict[int, frozenset[int]]]:
    """Reads every 15m event, keyed by identity and available bar."""
    events: dict[tuple[int, int, int], dict[int, set[int]]] = {}
    with path.open(newline="", encoding="utf-8") as handle:
        for row in csv.DictReader(handle):
            if raw_edge._int(row.get("timeframe_secs", "")) != TIMEFRAME_SECS:
                continue
            event_type = raw_edge._int(row.get("event_type_code", ""))
            subtype = raw_edge._int(row.get("event_subtype_code", ""))
            source = raw_edge._int(row.get("source_code", ""))
            bar = raw_edge._int(row.get("available_bar_index", ""))
            direction = raw_edge._int(row.get("direction_code", ""))
            if None in (event_type, subtype, source, bar, direction):
                continue
            events.setdefault((event_type, subtype, source), {}).setdefault(bar, set()).add(direction)
    return {
        key: {bar: frozenset(directions) for bar, directions in by_bar.items()}
        for key, by_bar in events.items()
    }


def load_bundle(model_dir: Path, label: str | None = None) -> EventBundle:
    """Loads one model bundle."""
    readings = model_dir / "readings.csv"
    events = model_dir / "events.csv"
    if not readings.is_file() or not events.is_file():
        raise FileNotFoundError(f"{model_dir} is not a model bundle")
    return EventBundle(
        label=label or str(model_dir),
        bars=raw_edge.parse_readings(readings),
        events=parse_all_events(events),
    )


def signal_bars(bundle: EventBundle, spec: SignalSpec) -> list[int]:
    """Returns the signal bars for one bundle."""
    selected: list[int] = []
    for bar in bundle.bars:
        if bar.atr <= 0.0:
            continue
        matched = True
        for condition in spec.events:
            by_bar = bundle.events.get(
                (condition.type, condition.subtype, condition.source), {}
            )
            found = False
            for index in range(bar.index - spec.window + 1, bar.index + 1):
                directions = by_bar.get(index)
                if directions and (
                    condition.direction == 0 or condition.direction in directions
                ):
                    found = True
                    break
            if not found:
                matched = False
                break
        if not matched:
            continue
        for field, value in spec.states:
            if field == "ema_stack" and bar.ema_stack != value:
                matched = False
            elif field == "structure_state" and bar.structure_state != value:
                matched = False
        if matched:
            selected.append(bar.index)
    return selected


def records_for(
    bundles: list[EventBundle],
    side: str,
    indices: list[list[int]],
    horizons: tuple[int, ...],
    target_atr: float,
    stop_atr: float,
    barrier_bars: int,
    cost_bps: float,
) -> list[dict[str, object]]:
    """Measures the selected bars across bundles."""
    records: list[dict[str, object]] = []
    for bundle, selected in zip(bundles, indices, strict=True):
        position = {bar.index: index for index, bar in enumerate(bundle.bars)}
        for index in selected:
            record = raw_edge.forward_metrics(
                bundle.bars,
                position[index],
                side,
                horizons,
                target_atr,
                stop_atr,
                barrier_bars,
                cost_bps,
            )
            if record is not None:
                records.append(record)
    return records


def evaluate(
    bundles: list[EventBundle],
    spec: SignalSpec,
    horizons: tuple[int, ...],
    target_atr: float,
    stop_atr: float,
    barrier_bars: int,
    cost_bps: float,
) -> dict[str, object]:
    """Evaluates one spec across the bundles."""
    selected = [signal_bars(bundle, spec) for bundle in bundles]
    records = records_for(
        bundles, spec.side, selected, horizons, target_atr, stop_atr, barrier_bars, cost_bps
    )
    sample = raw_edge.summarize(records, horizons)
    sample["signals"] = sum(len(indices) for indices in selected)

    non_overlapping: list[list[int]] = []
    for bundle, indices in zip(bundles, selected, strict=True):
        non_overlapping.append(raw_edge.non_overlapping(indices, max(horizons)))
    non_records = records_for(
        bundles,
        spec.side,
        non_overlapping,
        horizons,
        target_atr,
        stop_atr,
        barrier_bars,
        cost_bps,
    )
    non = raw_edge.summarize(non_records, horizons)
    non["signals"] = sum(len(indices) for indices in non_overlapping)

    all_indices = [
        [bar.index for bar in bundle.bars if bar.atr > 0.0] for bundle in bundles
    ]
    all_records = records_for(
        bundles,
        spec.side,
        all_indices,
        horizons,
        target_atr,
        stop_atr,
        barrier_bars,
        cost_bps,
    )
    all_bars = raw_edge.summarize(all_records, horizons)
    all_bars["signals"] = sum(len(indices) for indices in all_indices)

    return {
        "schema": "quantrule.event-edge/v1",
        "signal": {
            "id": spec.id,
            "side": spec.side,
            "window": spec.window,
            "events": [
                {
                    "type": event.type,
                    "subtype": event.subtype,
                    "source": event.source,
                    "direction": event.direction,
                }
                for event in spec.events
            ],
            "states": [{"field": field, "value": value} for field, value in spec.states],
        },
        "timeframe": "15m",
        "cost_bps": cost_bps,
        "horizons": list(horizons),
        "barrier": {
            "target_atr": target_atr,
            "stop_atr": stop_atr,
            "bars": barrier_bars,
            "tie_break": "stop_first",
            "entry": "next_bar_open",
        },
        "bundles": [
            {"label": bundle.label, "bars": len(bundle.bars)} for bundle in bundles
        ],
        "sample": sample,
        "non_overlapping": non,
        "all_bars": all_bars,
    }


def render_markdown(result: dict[str, object]) -> str:
    """Renders one event-edge report."""
    signal = result["signal"]
    lines = [
        f"# Event Edge — {signal['id']}",
        "",
        f"Side: {signal['side']}. Window: {signal['window']} bars. "
        f"Costs: {result['cost_bps']} bps. "
        f"Barrier: {result['barrier']['target_atr']}:{result['barrier']['stop_atr']} ATR, "
        f"{result['barrier']['bars']} bars, stop first.",
        "",
        "## Events",
        "",
        "| Type | Subtype | Source | Direction |",
        "|---:|---:|---:|---:|",
    ]
    for event in signal["events"]:
        lines.append(
            f"| {event['type']} | {event['subtype']} | {event['source']} | {event['direction']} |"
        )
    if signal["states"]:
        lines += ["", "States: " + ", ".join(
            f"{state['field']}={state['value']}" for state in signal["states"]
        )]
    sample = result["sample"]
    lines += [
        "",
        "## Sample",
        "",
        "| Bars | Count | Mean gross | Mean net | Median net | Hit rate | MFE | MAE |",
        "|---:|---:|---:|---:|---:|---:|---:|---:|",
    ]
    for horizon in result["horizons"]:
        row = sample["horizons"][str(horizon)]
        lines.append(
            f"| {horizon} | {row['count']} | {_fmt(row.get('mean_gross_pct'))}% | "
            f"{_fmt(row.get('mean_net_pct'))}% | {_fmt(row.get('median_gross_pct'))}% | "
            f"{_pct(row.get('hit_rate'))} | {_fmt(row.get('mean_mfe_atr'))} | "
            f"{_fmt(row.get('mean_mae_atr'))} |"
        )
    barrier = sample["barrier"]
    non = result["non_overlapping"]
    all_bars = result["all_bars"]
    lines += [
        "",
        "## Barrier and controls",
        "",
        "| View | Signals | Target | Stop | Timeout | Mean R | 95% CI |",
        "|---|---:|---:|---:|---:|---:|---|",
    ]
    for label, summary in (
        ("sample", sample),
        ("non-overlapping", non),
        ("all bars", all_bars),
    ):
        row = summary["barrier"]
        lines.append(
            f"| {label} | {summary['signals']} | {_pct(row.get('target_rate'))} | "
            f"{_pct(row.get('stop_rate'))} | {_pct(row.get('timeout_rate'))} | "
            f"{_fmt(row.get('mean_r'))} | {_ci(row.get('ci95_r'))} |"
        )
    lines += [
        "",
        f"Sample 20-bar net: {_fmt(sample['horizons']['20'].get('mean_net_pct'))}% "
        f"(95% CI {_ci(sample['horizons']['20'].get('ci95_net_pct'))}).",
        f"Non-overlapping 20-bar net: {_fmt(non['horizons']['20'].get('mean_net_pct'))}% "
        f"(95% CI {_ci(non['horizons']['20'].get('ci95_net_pct'))}).",
        f"All-bars 20-bar net: {_fmt(all_bars['horizons']['20'].get('mean_net_pct'))}% "
        f"(95% CI {_ci(all_bars['horizons']['20'].get('ci95_net_pct'))}).",
    ]
    return "\n".join(lines) + "\n"


def _fmt(value: object) -> str:
    if value is None:
        return "—"
    return f"{float(value):.4f}"


def _pct(value: object) -> str:
    if value is None:
        return "—"
    return f"{float(value) * 100.0:.1f}%"


def _ci(value: object) -> str:
    if not isinstance(value, list) or len(value) != 2:
        return "—"
    return f"{float(value[0]):.4f}, {float(value[1]):.4f}"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--spec", action="append", required=True, help="signal spec JSON")
    parser.add_argument("--model", action="append", required=True, help="model bundle directory")
    parser.add_argument("--out", required=True, help="output directory")
    parser.add_argument("--horizons", default="1,3,5,10,20")
    parser.add_argument("--target-atr", type=float, default=2.0)
    parser.add_argument("--stop-atr", type=float, default=1.0)
    parser.add_argument("--barrier-bars", type=int, default=20)
    parser.add_argument("--cost-bps", type=float, default=6.0)
    args = parser.parse_args()
    horizons = tuple(sorted({int(value) for value in args.horizons.split(",")}))
    if not horizons or min(horizons) <= 0:
        raise ValueError("horizons must be positive")
    bundles = [load_bundle(Path(path)) for path in args.model]
    bundles.sort(key=lambda bundle: bundle.label)
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    for spec_path in args.spec:
        spec = load_spec(Path(spec_path))
        result = evaluate(
            bundles,
            spec,
            horizons,
            args.target_atr,
            args.stop_atr,
            args.barrier_bars,
            args.cost_bps,
        )
        (out / f"{spec.id}.json").write_text(
            json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
        (out / f"{spec.id}.md").write_text(render_markdown(result), encoding="utf-8")
        sample = result["sample"]
        twenty = sample["horizons"]["20"]
        print(
            f"{spec.id}: signals {sample['signals']} | 20-bar net "
            f"{twenty.get('mean_net_pct')}% | hit {twenty.get('hit_rate')} | "
            f"target {sample['barrier'].get('target_rate')} | "
            f"mean R {sample['barrier'].get('mean_r')} | "
            f"all-bars mean R {result['all_bars']['barrier'].get('mean_r')}"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
