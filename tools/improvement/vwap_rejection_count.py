#!/usr/bin/env python3
"""Count VWAP-rejection events and their 1h stack context.

The tool is read-only over a `report model` bundle. It reads `events.csv` and
`readings.csv`, keeps the 15m `signal.vwap_rejection` events by their unique
event code (28; the shared type/subtype/source also covers the separate
`signal.vwap_reclaim` event), and joins
each event's available bar to the latest closed 1h bar using the same
close-time rule as the strategy context. It writes a deterministic JSON count
report and exits non-zero when the count gate fails.

Usage:
    python3 tools/improvement/absorption_count.py \
        --bundle data/improvement/2026-11-09/SR6-VWAP/count/2018 \
        --bundle data/improvement/2026-11-09/SR6-VWAP/count/2019 \
        --bundle data/improvement/2026-11-09/SR6-VWAP/count/2020 \
        --bundle data/improvement/2026-11-09/SR6-VWAP/count/2021 \
        --bundle data/improvement/2026-11-09/SR6-VWAP/count/2022 \
        --out data/improvement/2026-11-09/SR6-VWAP/count-gate \
        --threshold 30
"""

from __future__ import annotations

import argparse
import csv
import json
import sys
from bisect import bisect_right
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

EVENT_CODE = 28
EVENT_TYPE = 3
EVENT_SUBTYPE = 14
EVENT_SOURCE = 19
EXECUTION_SECS = 900
CONTEXT_SECS = 3600


def read_csv(path: Path) -> list[dict[str, str]]:
    with path.open(encoding="utf-8", newline="") as handle:
        return list(csv.DictReader(handle))


def optional_int(value: str | None) -> int | None:
    if value is None or value == "":
        return None
    return int(value)


def optional_float(value: str | None) -> float | None:
    if value is None or value == "":
        return None
    return float(value)


def side_of(direction: int) -> str | None:
    if direction > 0:
        return "long"
    if direction < 0:
        return "short"
    return None


def year_of(timestamp: int) -> int:
    return datetime.fromtimestamp(timestamp, tz=timezone.utc).year


def context_rows(bundle: Path) -> tuple[list[int], list[dict[str, str]]]:
    rows = [
        row
        for row in read_csv(bundle / "readings.csv")
        if int(row["timeframe_secs"]) == CONTEXT_SECS
    ]
    rows.sort(key=lambda row: int(row["timestamp"]))
    closes = [int(row["timestamp"]) + CONTEXT_SECS for row in rows]
    return closes, rows


def execution_rows(bundle: Path) -> dict[int, dict[str, str]]:
    return {
        int(row["bar"]): row
        for row in read_csv(bundle / "readings.csv")
        if int(row["timeframe_secs"]) == EXECUTION_SECS
    }


def as_of(closes: list[int], at: int) -> int | None:
    index = bisect_right(closes, at) - 1
    return None if index < 0 else index


def is_target_event(row: dict[str, str]) -> bool:
    """The unique event identity, not the shared type/subtype/source triple."""
    return (
        int(row["timeframe_secs"]) == EXECUTION_SECS
        and int(row["event_code"]) == EVENT_CODE
    )


def count_events(bundle: Path) -> dict[str, Any]:
    closes, contexts = context_rows(bundle)
    executions = execution_rows(bundle)
    events = [
        row
        for row in read_csv(bundle / "events.csv")
        if is_target_event(row)
    ]
    categories = ("all", "a1", "a2", "a3")
    totals = {name: {"long": 0, "short": 0} for name in categories}
    by_year: dict[str, dict[str, dict[str, int]]] = {}
    skipped = {"missing_bar": 0, "missing_context": 0, "zero_direction": 0}

    for event in events:
        available = optional_int(event["available_bar_index"])
        if available is None or available not in executions:
            skipped["missing_bar"] += 1
            continue
        execution = executions[available]
        timestamp = int(execution["timestamp"])
        context_index = as_of(closes, timestamp + EXECUTION_SECS)
        if context_index is None:
            skipped["missing_context"] += 1
            continue
        direction = optional_int(event["direction_code"]) or 0
        side = side_of(direction)
        if side is None:
            skipped["zero_direction"] += 1
            continue
        context = contexts[context_index]
        stack = optional_int(context.get("ema_stack"))
        adx = optional_int(context.get("adx_regime"))
        year = str(year_of(timestamp))

        totals["all"][side] += 1
        totals["a3"][side] += 1
        if stack == direction:
            totals["a1"][side] += 1
            if adx == 1:
                totals["a2"][side] += 1
        year_counts = by_year.setdefault(year, {name: {"long": 0, "short": 0} for name in categories})
        year_counts["all"][side] += 1
        year_counts["a3"][side] += 1
        if stack == direction:
            year_counts["a1"][side] += 1
            if adx == 1:
                year_counts["a2"][side] += 1

    return {
        "schema": "quantrule.absorption-count/v1",
        "execution_timeframe_secs": EXECUTION_SECS,
        "context_timeframe_secs": CONTEXT_SECS,
        "event": {
            "code": EVENT_CODE,
            "event_type": EVENT_TYPE,
            "subtype": EVENT_SUBTYPE,
            "source": EVENT_SOURCE,
        },
        "totals": totals,
        "by_year": {year: by_year[year] for year in sorted(by_year)},
        "events": len(events),
        "skipped": skipped,
    }


def merge_counts(counts: list[dict[str, Any]]) -> dict[str, Any]:
    categories = ("all", "a1", "a2", "a3")
    totals = {name: {"long": 0, "short": 0} for name in categories}
    by_year: dict[str, dict[str, dict[str, int]]] = {}
    skipped = {"missing_bar": 0, "missing_context": 0, "zero_direction": 0}
    events = 0
    for count in counts:
        for name in categories:
            for side in ("long", "short"):
                totals[name][side] += count["totals"][name][side]
        for year, values in count["by_year"].items():
            target = by_year.setdefault(year, {name: {"long": 0, "short": 0} for name in categories})
            for name in categories:
                for side in ("long", "short"):
                    target[name][side] += values[name][side]
        for key in skipped:
            skipped[key] += count["skipped"][key]
        events += count["events"]
    return {
        "schema": "quantrule.absorption-count/v1",
        "execution_timeframe_secs": EXECUTION_SECS,
        "context_timeframe_secs": CONTEXT_SECS,
        "event": {
            "code": EVENT_CODE,
            "event_type": EVENT_TYPE,
            "subtype": EVENT_SUBTYPE,
            "source": EVENT_SOURCE,
        },
        "totals": totals,
        "by_year": {year: by_year[year] for year in sorted(by_year)},
        "events": events,
        "skipped": skipped,
        "bundles": len(counts),
    }


def gate_report(count: dict[str, Any], threshold: int) -> dict[str, Any]:
    a1 = count["totals"]["a1"]
    return {
        "threshold": threshold,
        "a1_long": a1["long"],
        "a1_short": a1["short"],
        "pass": a1["long"] >= threshold and a1["short"] >= threshold,
    }


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--bundle",
        action="append",
        required=True,
        help="model bundle directory; repeat for per-year bundles",
    )
    parser.add_argument("--out", required=True, help="output directory")
    parser.add_argument("--threshold", type=int, default=30, help="minimum A1 events per side")
    args = parser.parse_args(argv[1:])
    bundles = [Path(path).resolve() for path in args.bundle]
    out = Path(args.out).resolve()
    out.mkdir(parents=True, exist_ok=True)
    count = merge_counts([count_events(bundle) for bundle in bundles])
    count["gate"] = gate_report(count, args.threshold)
    (out / "count.json").write_text(json.dumps(count, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(
        f"A1 long={count['gate']['a1_long']} short={count['gate']['a1_short']} "
        f"threshold={args.threshold} pass={str(count['gate']['pass']).lower()}"
    )
    return 0 if count["gate"]["pass"] else 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
