#!/usr/bin/env python3
"""Extract the same context bundle for human-marked chart points.

The annotation CSV is explicit; the tool does not infer quality from a circle
style. Timestamps are mapped to the nearest recorded 15m bar.

Usage:
    python3 tools/trades/annotations.py \
        --bundle data/.../backtest \
        --annotations annotations.csv \
        --out data/.../annotations
"""

from __future__ import annotations

import argparse
import csv
import sys
from datetime import datetime, timezone
from pathlib import Path

from bundle import (
    BundleError,
    event_catalog,
    event_label,
    load_bundle,
    number_or_none,
    reading_summary,
    timestamp_utc,
    write_csv,
    write_json,
)
from trade_context import context_rows, post_rows

ACTIONS = {"entry", "exit"}
QUALITIES = {"good", "bad", "neutral"}
SIDES = {"long", "short"}


def parse_utc(value: str) -> int:
    """Parse an ISO UTC timestamp into seconds."""
    moment = datetime.fromisoformat(value.replace("Z", "+00:00"))
    if moment.tzinfo is None:
        moment = moment.replace(tzinfo=timezone.utc)
    return int(moment.timestamp())


def nearest_bar(bundle, seconds: int) -> tuple[int, int, int]:
    """Return the nearest bar index, its timestamp, and the distance in seconds."""
    best = min(bundle.readings, key=lambda row: abs(int(row["timestamp"]) - seconds))
    bar = int(best["bar"])
    timestamp = int(best["timestamp"])
    return bar, timestamp, abs(timestamp - seconds)


def read_annotations(path: Path) -> list[dict[str, str]]:
    """Read and validate the annotation CSV."""
    with path.open(encoding="utf-8", newline="") as handle:
        rows = list(csv.DictReader(handle))
    required = {"chart", "marked_time_utc", "action", "quality", "side"}
    if not rows:
        raise BundleError("the annotation file is empty")
    missing = required - set(rows[0])
    if missing:
        raise BundleError(f"annotations are missing columns {sorted(missing)}")
    for index, row in enumerate(rows, 2):
        if row["action"] not in ACTIONS:
            raise BundleError(f"row {index}: unknown action {row['action']!r}")
        if row["quality"] not in QUALITIES:
            raise BundleError(f"row {index}: unknown quality {row['quality']!r}")
        if row["side"] not in SIDES:
            raise BundleError(f"row {index}: unknown side {row['side']!r}")
        parse_utc(row["marked_time_utc"])
    return rows


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bundle", required=True, type=Path)
    parser.add_argument("--annotations", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("--window", type=int, default=250)
    parser.add_argument("--post", type=int, default=250)
    parser.add_argument("--timeframe-secs", type=int, default=900)
    parser.add_argument("--event-registry", type=Path, default=Path("docs/registry/indicators.json"))
    args = parser.parse_args(argv[1:])
    try:
        bundle = load_bundle(args.bundle, args.timeframe_secs)
        catalog = event_catalog(args.event_registry)
        annotations = read_annotations(args.annotations)
        index = []
        for number, annotation in enumerate(annotations, 1):
            seconds = parse_utc(annotation["marked_time_utc"])
            bar, timestamp, distance = nearest_bar(bundle, seconds)
            if distance > bundle.timeframe_secs // 2:
                raise BundleError(
                    f"row {number + 1}: {annotation['marked_time_utc']} is {distance}s "
                    f"from the nearest bar; the maximum is {bundle.timeframe_secs // 2}s"
                )
            rows = context_rows(bundle, bar, args.window)
            if not rows:
                raise BundleError(f"row {number + 1}: no context at bar {bar}")
            start_bar = int(rows[0]["bar"])
            events = bundle.events_between(start_bar, bar)
            post = post_rows(bundle, bar, args.post, entry=False)
            root = args.out / f"annotation-{number:03d}"
            write_csv(root / "context.csv", rows)
            write_csv(root / "post.csv", post)
            write_csv(root / "events.csv", events)
            summary = {
                "schema": "quantrule.annotation-context/v1",
                "annotation": annotation,
                "bar": bar,
                "time": timestamp_utc(timestamp),
                "distance_seconds": distance,
                "readings": reading_summary(rows[-1]),
                "events_at_bar": [
                    {
                        "code": int(event["event_code"]),
                        "id": event_label(int(event["event_code"]), catalog),
                        "type": int(event["event_type_code"]),
                        "subtype": int(event["event_subtype_code"]),
                        "source": int(event["source_code"]),
                        "direction": int(event["direction_code"]),
                        "occurred_bar": int(event["occurred_bar_index"]),
                        "available_bar": int(event["available_bar_index"]),
                        "values": {
                            "primary": number_or_none(event["primary_value"]),
                            "reference": number_or_none(event["reference_value"]),
                            "completeness": number_or_none(event["data_completeness"]),
                        },
                    }
                    for event in bundle.events_at(bar)
                ],
                "context_bars": len(rows),
                "post_bars": len(post),
            }
            write_json(root / "summary.json", summary)
            index.append({"row": number, "path": str(root), **summary})
        write_json(args.out / "index.json", {"annotations": index})
    except (BundleError, ValueError, KeyError, OSError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    print(f"wrote {len(index)} annotation contexts to {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
