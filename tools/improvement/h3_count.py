#!/usr/bin/env python3
"""Count H3 compression-fade events and their 1h stack/ADX context.

The tool is read-only over a `report model` bundle. It reads `events.csv` and
`readings.csv`, keeps the 1h `signal.compression_breakout` events (code 17), and
counts the H3 fade condition on the same bar: the 1h EMA stack equals the event
direction and 1h `adx_regime` is 0. The H3 side is opposite the event, so a
negative event is a long fade and a positive event is a short fade.

Usage:
    python3 tools/improvement/h3_count.py \
        --bundle data/improvement/2026-11-09/SR7-H3-CRYPTO/count/BTCUSD/2018/model \
        --bundle ... \
        --instrument BTCUSD \
        --out data/improvement/2026-11-09/SR7-H3-CRYPTO/count-gate/BTCUSD
"""

from __future__ import annotations

import argparse
import csv
import json
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

EVENT_CODE = 17
EXECUTION_SECS = 3600


def read_csv(path: Path) -> list[dict[str, str]]:
    with path.open(encoding="utf-8", newline="") as handle:
        return list(csv.DictReader(handle))


def optional_int(value: str | None) -> int | None:
    if value is None or value == "":
        return None
    return int(value)


def side_of(event_direction: int) -> str | None:
    """The H3 fade is opposite the event direction."""
    if event_direction < 0:
        return "long"
    if event_direction > 0:
        return "short"
    return None


def is_target_event(row: dict[str, str]) -> bool:
    return (
        int(row["timeframe_secs"]) == EXECUTION_SECS
        and int(row["event_code"]) == EVENT_CODE
    )


def is_h3(direction: int, stack: int | None, adx: int | None) -> bool:
    return stack == direction and adx == 0


def count_events(bundle: Path, instrument: str) -> dict[str, Any]:
    readings = {
        int(row["bar"]): row
        for row in read_csv(bundle / "readings.csv")
        if int(row["timeframe_secs"]) == EXECUTION_SECS
    }
    events = [row for row in read_csv(bundle / "events.csv") if is_target_event(row)]
    categories = ("all", "stack", "h3")
    totals = {name: {"long": 0, "short": 0} for name in categories}
    by_year: dict[str, dict[str, dict[str, int]]] = {}
    skipped = {"missing_bar": 0, "zero_direction": 0}

    for event in events:
        bar = optional_int(event["available_bar_index"])
        if bar is None or bar not in readings:
            skipped["missing_bar"] += 1
            continue
        direction = optional_int(event["direction_code"]) or 0
        side = side_of(direction)
        if side is None:
            skipped["zero_direction"] += 1
            continue
        reading = readings[bar]
        stack = optional_int(reading.get("ema_stack"))
        adx = optional_int(reading.get("adx_regime"))
        timestamp = int(reading["timestamp"])
        year = str(datetime.fromtimestamp(timestamp, tz=timezone.utc).year)
        year_counts = by_year.setdefault(
            year, {name: {"long": 0, "short": 0} for name in categories}
        )
        totals["all"][side] += 1
        year_counts["all"][side] += 1
        if stack == direction:
            totals["stack"][side] += 1
            year_counts["stack"][side] += 1
        if is_h3(direction, stack, adx):
            totals["h3"][side] += 1
            year_counts["h3"][side] += 1

    return {
        "schema": "quantrule.h3-count/v1",
        "instrument": instrument,
        "execution_timeframe_secs": EXECUTION_SECS,
        "event": {"code": EVENT_CODE, "event_type": 3, "subtype": 7, "source": 5},
        "totals": totals,
        "by_year": {year: by_year[year] for year in sorted(by_year)},
        "events": len(events),
        "skipped": skipped,
    }


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bundle", action="append", required=True, help="model bundle directory")
    parser.add_argument("--instrument", required=True, help="instrument label")
    parser.add_argument("--out", required=True, help="output directory")
    args = parser.parse_args(argv[1:])
    bundles = [Path(path).resolve() for path in args.bundle]
    out = Path(args.out).resolve()
    out.mkdir(parents=True, exist_ok=True)
    counts = [count_events(bundle, args.instrument) for bundle in bundles]
    categories = ("all", "stack", "h3")
    totals = {name: {"long": 0, "short": 0} for name in categories}
    by_year: dict[str, dict[str, dict[str, int]]] = {}
    skipped = {"missing_bar": 0, "zero_direction": 0}
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
    result = {
        "schema": "quantrule.h3-count/v1",
        "instrument": args.instrument,
        "execution_timeframe_secs": EXECUTION_SECS,
        "event": {"code": EVENT_CODE, "event_type": 3, "subtype": 7, "source": 5},
        "totals": totals,
        "by_year": {year: by_year[year] for year in sorted(by_year)},
        "events": events,
        "skipped": skipped,
        "bundles": len(bundles),
    }
    (out / "count.json").write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    h3 = result["totals"]["h3"]
    print(f"{args.instrument} H3 long={h3['long']} short={h3['short']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
