#!/usr/bin/env python3
"""ESANDP500 1h confirmed-pivot Level prediction pilot.

Primary question: after price tests a confirmed pivot level, is the next 1h
return more often in the direction of the event's close side than the
directional base rate?

The pilot is prediction-only. It does not model costs, execution, or a strategy.
Development and validation are reported separately; validation is not used for
tuning.

The event preparation is pre-registered:

- Level events only, confirmed pivot sources, reference codes >= 1001.
- One fresh test per reference: the first bar of a run of consecutive test bars.
- One vote per bar; a bar with conflicting directions is dropped as ambiguous.
- The next 1h bar must be exactly 3,600 seconds later, so a session gap is never
  treated as a one-hour return.

Usage:
    python3 tools/improvement/structure_pilot.py \
        --dev data/structure-pilot/esanp500-2018/model \
              data/structure-pilot/esanp500-2019/model \
              data/structure-pilot/esanp500-2020/model \
        --validation data/structure-pilot/esanp500-2021/model \
                    data/structure-pilot/esanp500-2022/model \
        --out data/structure-pilot/STRUCTURE
"""

from __future__ import annotations

import argparse
import csv
import json
import math
import sys
from collections import defaultdict
from pathlib import Path
from typing import Any

TIMEFRAME_SECS = 3600
PRIMARY_HORIZON = 1
Z_95 = 1.959963984540054
MIN_EVENTS = 100
MIN_LIFT = 0.01
PIVOT_REFERENCE_MIN = 1001
PIVOT_SOURCES = ("7", "8")
SUBTYPES = {
    "test": "1",
    "cross": "2",
    "break": "3",
}


def parse_arguments(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dev", nargs="+", required=True, help="development model bundle dirs")
    parser.add_argument("--validation", nargs="+", required=True, help="validation model bundle dirs")
    parser.add_argument("--out", required=True, help="output directory")
    return parser.parse_args(argv)


def read_readings(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    with path.open(newline="", encoding="utf-8") as handle:
        for row in csv.DictReader(handle):
            if row.get("timeframe_secs") != str(TIMEFRAME_SECS):
                continue
            try:
                rows.append(
                    {
                        "bar": int(row["bar"]),
                        "timestamp": int(row["timestamp"]),
                        "close": float(row["close"]),
                    }
                )
            except (KeyError, TypeError, ValueError):
                continue
    rows.sort(key=lambda row: row["bar"])
    return rows


def read_events(path: Path) -> list[dict[str, Any]]:
    events: list[dict[str, Any]] = []
    with path.open(newline="", encoding="utf-8") as handle:
        for row in csv.DictReader(handle):
            if row.get("timeframe_secs") != str(TIMEFRAME_SECS):
                continue
            if row.get("event_type_code") != "1":
                continue
            subtype = row.get("event_subtype_code")
            if subtype not in SUBTYPES.values():
                continue
            if row.get("source_code") not in PIVOT_SOURCES:
                continue
            reference = optional_int(row.get("reference_code"))
            if reference is None or reference < PIVOT_REFERENCE_MIN:
                continue
            direction = optional_int(row.get("direction_code"))
            if direction not in (-1, 1):
                continue
            completeness = optional_float(row.get("data_completeness"))
            if completeness is None or completeness < 1.0:
                continue
            bar = optional_int(row.get("available_bar_index"))
            if bar is None or bar < 0:
                continue
            events.append(
                {
                    "bar": bar,
                    "subtype": subtype,
                    "source": row["source_code"],
                    "reference": reference,
                    "direction": direction,
                    "event_index": optional_int(row.get("event_index")) or 0,
                }
            )
    events.sort(key=lambda event: (event["bar"], event["event_index"], event["reference"]))
    return events


def prepare_subtype(
    events: list[dict[str, Any]], subtype: str
) -> tuple[list[dict[str, Any]], list[dict[str, Any]], int]:
    subset = sorted(
        (event for event in events if event["subtype"] == subtype),
        key=lambda event: (event["bar"], event["event_index"], event["reference"]),
    )
    fresh: list[dict[str, Any]] = []
    last_raw_bar: dict[int, int] = {}
    for event in subset:
        previous = last_raw_bar.get(event["reference"])
        if previous is None or event["bar"] > previous + 1:
            fresh.append(event)
        last_raw_bar[event["reference"]] = event["bar"]

    by_bar: dict[int, list[dict[str, Any]]] = defaultdict(list)
    for event in fresh:
        by_bar[event["bar"]].append(event)
    prepared: list[dict[str, Any]] = []
    conflicts = 0
    for bar in sorted(by_bar):
        directions = {event["direction"] for event in by_bar[bar]}
        if len(directions) != 1:
            conflicts += 1
            continue
        prepared.append(
            min(
                by_bar[bar],
                key=lambda event: (event["event_index"], event["reference"]),
            )
        )
    return fresh, prepared, conflicts


def optional_int(value: Any) -> int | None:
    if value is None or value == "":
        return None
    try:
        return int(value)
    except (TypeError, ValueError):
        return None


def optional_float(value: Any) -> float | None:
    if value is None or value == "":
        return None
    try:
        return float(value)
    except (TypeError, ValueError):
        return None


def sign(value: float) -> int:
    if value > 0.0:
        return 1
    if value < 0.0:
        return -1
    return 0


def forward(rows: list[dict[str, Any]], index: int) -> float | None:
    if index < 0 or index + PRIMARY_HORIZON >= len(rows):
        return None
    current = rows[index]
    target = rows[index + PRIMARY_HORIZON]
    if target["timestamp"] - current["timestamp"] != TIMEFRAME_SECS:
        return None
    return target["close"] / current["close"] - 1.0


def base_stats(rows: list[dict[str, Any]]) -> dict[str, Any]:
    total = 0
    up = 0
    down = 0
    for index in range(len(rows) - PRIMARY_HORIZON):
        value = forward(rows, index)
        if value is None:
            continue
        total += 1
        if value > 0.0:
            up += 1
        elif value < 0.0:
            down += 1
    if total == 0:
        return {"n": 0, "up": 0.0, "down": 0.0, "var_up": 0.0, "var_down": 0.0}
    base_up = up / total
    base_down = down / total
    return {
        "n": total,
        "up": base_up,
        "down": base_down,
        "var_up": base_up * (1.0 - base_up) / total,
        "var_down": base_down * (1.0 - base_down) / total,
    }


def summarize_bundles(dirs: list[str]) -> dict[str, Any]:
    cells = {
        key: {
            "raw": 0,
            "fresh": 0,
            "prepared": 0,
            "conflicts": 0,
            "dropped_gap": 0,
            "n": 0,
            "hits": 0,
            "long_n": 0,
            "short_n": 0,
            "base_sum": 0.0,
            "per_bundle": {},
        }
        for key in SUBTYPES
    }
    breakdown: dict[str, dict[str, Any]] = {
        f"{source}/{direction}": {"n": 0, "hits": 0, "base_sum": 0.0}
        for source in PIVOT_SOURCES
        for direction in ("long", "short")
    }
    for directory in dirs:
        root = Path(directory).expanduser()
        rows = read_readings(root / "readings.csv")
        events = read_events(root / "events.csv")
        stats = base_stats(rows)
        if stats["n"] == 0:
            continue
        bundle = str(root)
        for key, subtype in SUBTYPES.items():
            cell = cells[key]
            cell["raw"] += sum(1 for event in events if event["subtype"] == subtype)
            fresh, prepared, conflicts = prepare_subtype(events, subtype)
            cell["fresh"] += len(fresh)
            cell["prepared"] += len(prepared)
            cell["conflicts"] += conflicts
            up_b = 0
            down_b = 0
            for event in prepared:
                value = forward(rows, event["bar"])
                if value is None:
                    cell["dropped_gap"] += 1
                    continue
                cell["n"] += 1
                expected = stats["up"] if event["direction"] == 1 else stats["down"]
                cell["base_sum"] += expected
                if event["direction"] == 1:
                    cell["long_n"] += 1
                    up_b += 1
                else:
                    cell["short_n"] += 1
                    down_b += 1
                if sign(value) == event["direction"]:
                    cell["hits"] += 1
                if key == "test":
                    label = f"{event['source']}/{'long' if event['direction'] == 1 else 'short'}"
                    bucket = breakdown[label]
                    bucket["n"] += 1
                    bucket["base_sum"] += expected
                    if sign(value) == event["direction"]:
                        bucket["hits"] += 1
            cell["per_bundle"][bundle] = {"up": up_b, "down": down_b, "stats": stats}

    result = {}
    for key, cell in cells.items():
        n = cell["n"]
        if n:
            p = cell["hits"] / n
            base = cell["base_sum"] / n
            base_var = 0.0
            for bundle in cell["per_bundle"].values():
                w_up = bundle["up"] / n
                w_down = bundle["down"] / n
                base_var += w_up * w_up * bundle["stats"]["var_up"]
                base_var += w_down * w_down * bundle["stats"]["var_down"]
            se = math.sqrt(max(p * (1.0 - p) / n + base_var, 0.0))
            hit_lcb = p - Z_95 * math.sqrt(max(p * (1.0 - p) / n, 0.0))
            lift = p - base
            lift_lcb = lift - Z_95 * se
        else:
            p = None
            base = None
            hit_lcb = None
            lift = None
            lift_lcb = None
        result[key] = {
            "n": n,
            "raw": cell["raw"],
            "fresh": cell["fresh"],
            "prepared": cell["prepared"],
            "conflicts": cell["conflicts"],
            "dropped_gap": cell["dropped_gap"],
            "coverage": (n / cell["raw"]) if cell["raw"] else 0.0,
            "hits": cell["hits"],
            "hit_rate": p,
            "base_rate": base,
            "lift": lift,
            "hit_lcb": hit_lcb,
            "lift_lcb": lift_lcb,
            "long_n": cell["long_n"],
            "short_n": cell["short_n"],
        }

    for label, bucket in breakdown.items():
        n = bucket["n"]
        bucket["hit_rate"] = bucket["hits"] / n if n else None
        bucket["base_rate"] = bucket["base_sum"] / n if n else None
        bucket["lift"] = (
            bucket["hit_rate"] - bucket["base_rate"]
            if n and bucket["hit_rate"] is not None and bucket["base_rate"] is not None
            else None
        )
    return {"cells": result, "breakdown": breakdown}


def passes(cell: dict[str, Any]) -> bool:
    return (
        cell["n"] >= MIN_EVENTS
        and cell["lift"] is not None
        and cell["lift"] >= MIN_LIFT
        and cell["lift_lcb"] is not None
        and cell["lift_lcb"] > 0.0
    )


def decision(dev: dict[str, Any], validation: dict[str, Any]) -> str:
    primary = dev["cells"]["test"]
    check = validation["cells"]["test"]
    if primary["n"] < MIN_EVENTS:
        return "inconclusive"
    if not passes(primary):
        return "stop"
    if check["n"] < MIN_EVENTS:
        return "validation_inconclusive"
    if passes(check):
        return "proceed_to_progression"
    return "development_only"


def fmt(value: float | None) -> str:
    return "-" if value is None else f"{value:.4f}"


def markdown(document: dict[str, Any]) -> str:
    lines = [
        "# ESANDP500 Pivot-Level Structure Pilot",
        "",
        f"- Decision: **{document['decision']}**",
        "- Primary event: confirmed pivot Level test/retest",
        f"- Primary horizon: {PRIMARY_HORIZON} bar",
        "- Primary comparison: directional base rate",
        "",
        "## Primary test",
        "",
        "| Partition | Raw | Fresh | Bars | Valid n | Coverage | Hit | Base | Lift | Hit LCB | Lift LCB |",
        "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|",
    ]
    for partition in ("dev", "validation"):
        cell = document[partition]["cells"]["test"]
        lines.append(
            "| {p} | {raw} | {fresh} | {prepared} | {n} | {c:.4f} | {hit} | {base} | {lift} | {hl} | {ll} |".format(
                p=partition,
                raw=cell["raw"],
                fresh=cell["fresh"],
                prepared=cell["prepared"],
                n=cell["n"],
                c=cell["coverage"],
                hit=fmt(cell["hit_rate"]),
                base=fmt(cell["base_rate"]),
                lift=fmt(cell["lift"]),
                hl=fmt(cell["hit_lcb"]),
                ll=fmt(cell["lift_lcb"]),
            )
        )

    lines.extend(
        [
            "",
            "## Secondary events",
            "",
            "| Partition | Event | n | Hit | Base | Lift | Hit LCB | Lift LCB |",
            "|---|---|---:|---:|---:|---:|---:|---:|",
        ]
    )
    for partition in ("dev", "validation"):
        for key in ("cross", "break"):
            cell = document[partition]["cells"][key]
            lines.append(
                "| {p} | {e} | {n} | {hit} | {base} | {lift} | {hl} | {ll} |".format(
                    p=partition,
                    e=key,
                    n=cell["n"],
                    hit=fmt(cell["hit_rate"]),
                    base=fmt(cell["base_rate"]),
                    lift=fmt(cell["lift"]),
                    hl=fmt(cell["hit_lcb"]),
                    ll=fmt(cell["lift_lcb"]),
                )
            )

    lines.extend(
        [
            "",
            "## Descriptive breakdown of the primary test",
            "",
            "| Partition | Source/direction | n | Hit | Base | Lift |",
            "|---|---|---:|---:|---:|---:|",
        ]
    )
    for partition in ("dev", "validation"):
        for label in sorted(document[partition]["breakdown"]):
            bucket = document[partition]["breakdown"][label]
            lines.append(
                "| {p} | {label} | {n} | {hit} | {base} | {lift} |".format(
                    p=partition,
                    label=label,
                    n=bucket["n"],
                    hit=fmt(bucket["hit_rate"]),
                    base=fmt(bucket["base_rate"]),
                    lift=fmt(bucket["lift"]),
                )
            )

    lines.extend(
        [
            "",
            "## Rules",
            "",
            "A hit is `sign(next_1h_return) == direction_code`; a zero return is a miss. The directional base rate is the per-bundle probability of an up bar for long events and a down bar for short events, weighted by the prepared event mix.",
            "",
            "A primary pass needs `n >= 100`, point lift `>= +0.01`, and lift LCB `> 0.00` on both partitions. The lift LCB uses `se = sqrt(p*(1-p)/n + var(base))`. `stop` means development failed; `development_only` means validation did not confirm; `validation_inconclusive` means validation had fewer than 100 valid events.",
            "",
            "The event is fresh when it is the first test for its reference after at least one bar without a test. A bar with conflicting fresh directions is dropped. A next bar that is not exactly 3,600 seconds later is not a one-hour return and is dropped.",
            "",
            "`raw` is the filtered event count, `fresh` is after the consecutive-bar rule, `bars` is after the one-vote-per-bar rule, and `valid n` is after the exact-1h rule. The descriptive breakdown is not a promotion path.",
            "",
        ]
    )
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    args = parse_arguments(argv)
    dev = summarize_bundles(args.dev)
    validation = summarize_bundles(args.validation)
    result = {
        "schema": "quantrule.structure-pilot/v1",
        "question": (
            "Does a confirmed pivot Level test predict the next 1h return sign "
            "better than the directional base rate on ESANDP500?"
        ),
        "asset": "ESANDP500",
        "timeframe_secs": TIMEFRAME_SECS,
        "primary_horizon": PRIMARY_HORIZON,
        "primary_event": "level.test",
        "rules": {
            "min_events": MIN_EVENTS,
            "min_lift": MIN_LIFT,
            "pivot_reference_min": PIVOT_REFERENCE_MIN,
            "pivot_sources": list(PIVOT_SOURCES),
            "exact_next_bar_secs": TIMEFRAME_SECS,
        },
        "dev": dev,
        "validation": validation,
        "decision": decision(dev, validation),
    }
    out = Path(args.out).expanduser()
    out.mkdir(parents=True, exist_ok=True)
    (out / "result.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    (out / "result.md").write_text(markdown(result), encoding="utf-8")
    print(f"ok structure pilot -> {out} decision={result['decision']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
