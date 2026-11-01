#!/usr/bin/env python3
"""Measure event-direction confidence for one CSV source and timeframe.

The command runs ``events`` and ``model-series`` for the same source and
timeframe. Each event is joined to the close series at ``available_bar``, never
``occurred_bar``, so a result cannot use a price that was not knowable when the
event became available. Forward returns are evaluated at a pre-registered
horizon grid, grouped by event type and subtype.

The model is unchanged. This is evaluation only.
"""

from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path
from typing import Any

from common import REPO_ROOT, ToolError, is_number, resolve_shell, run_process, sign

Z_95 = 1.959963984540054
DEFAULT_HORIZONS = (1, 3, 5, 10, 20)


def parse_arguments(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        prog="event_confidence.py",
        description=(
            "Join event records to the available-bar close series and report "
            "directional hit rates by type, subtype, and horizon."
        ),
        epilog=(
            "Example: python3 tools/improvement/event_confidence.py "
            "--source data/sources/BTCUSD_M1_2021-01-01_2021-02-14.csv "
            "--timeframe 15m"
        ),
    )
    parser.add_argument("--source", required=True, help="CSV candle source")
    parser.add_argument(
        "--timeframe",
        required=True,
        choices=("1m", "5m", "15m", "1h", "4h"),
        help="event timeframe to evaluate",
    )
    parser.add_argument(
        "--qr",
        default="target/debug/quantrule",
        help="shell binary (default: target/debug/quantrule)",
    )
    parser.add_argument(
        "--horizons",
        default="1,3,5,10,20",
        help="comma-separated forward horizons in bars (default: 1,3,5,10,20)",
    )
    return parser.parse_args(argv)


def parse_horizons(value: str) -> tuple[int, ...]:
    horizons: list[int] = []
    for field in value.split(","):
        field = field.strip()
        if not field:
            raise ToolError("horizons must be a comma-separated list of positive integers")
        try:
            horizon = int(field)
        except ValueError:
            raise ToolError(f"invalid horizon {field!r}; expected a positive integer") from None
        if horizon <= 0:
            raise ToolError(f"invalid horizon {field!r}; expected a positive integer")
        if horizon not in horizons:
            horizons.append(horizon)
    if not horizons:
        raise ToolError("at least one horizon is required")
    return tuple(sorted(horizons))


def parse_ndjson(text: str, label: str) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    for number, line in enumerate(text.splitlines(), 1):
        if not line.strip():
            continue
        try:
            value = json.loads(line)
        except json.JSONDecodeError as error:
            raise ToolError(
                f"{label} line {number}: invalid JSON: {error.msg} "
                f"(column {error.colno})"
            ) from None
        if not isinstance(value, dict):
            raise ToolError(f"{label} line {number}: record is not an object")
        records.append(value)
    return records


def close_series(records: list[dict[str, Any]]) -> dict[int, float]:
    closes: dict[int, float] = {}
    for record in records:
        index = record.get("series_index")
        close = record.get("close")
        if not isinstance(index, int) or isinstance(index, bool):
            continue
        if is_number(close) and float(close) > 0.0:
            closes[index] = float(close)
    return closes


def event_rows(
    events: list[dict[str, Any]],
    closes: dict[int, float],
    timeframe: str,
    horizons: tuple[int, ...],
) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for event in events:
        if event.get("timeframe") != timeframe:
            continue
        direction = event.get("direction")
        available = event.get("available_bar")
        if not isinstance(direction, (int, float)) or isinstance(direction, bool):
            continue
        if direction == 0 or not isinstance(available, int) or isinstance(available, bool):
            continue
        close = closes.get(available)
        if close is None or close <= 0.0:
            continue
        for horizon in horizons:
            target = closes.get(available + horizon)
            if target is None or target <= 0.0:
                continue
            forward = target / close - 1.0
            if not math.isfinite(forward):
                continue
            rows.append(
                {
                    "type": str(event.get("type_label", "unknown")),
                    "subtype": str(event.get("subtype_label", "unknown")),
                    "horizon": horizon,
                    "direction": int(direction),
                    "forward": forward,
                    "hit": sign(float(direction)) == sign(forward),
                }
            )
    return rows


def lower_bound(rate: float | None, count: int) -> float | None:
    if rate is None or count <= 0:
        return None
    margin = Z_95 * math.sqrt(rate * (1.0 - rate) / count)
    return max(0.0, rate - margin)


def grouped(rows: list[dict[str, Any]], keys: tuple[str, ...]) -> list[dict[str, Any]]:
    groups: dict[tuple[Any, ...], list[dict[str, Any]]] = {}
    for row in rows:
        key = tuple(row[name] for name in keys) + (row["horizon"],)
        groups.setdefault(key, []).append(row)
    output: list[dict[str, Any]] = []
    for key in sorted(groups):
        values = groups[key]
        hits = sum(1 for row in values if row["hit"])
        count = len(values)
        rate = hits / count if count else None
        entry = dict(zip(keys, key[:-1]))
        entry["horizon"] = key[-1]
        entry["evaluated"] = count
        entry["hits"] = hits
        entry["hit_rate"] = rate
        entry["lower_bound_95"] = lower_bound(rate, count)
        output.append(entry)
    return output


def main(argv: list[str] | None = None) -> int:
    args = parse_arguments(argv)
    try:
        horizons = parse_horizons(args.horizons)
    except ToolError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    source = Path(args.source).expanduser()
    if not source.is_file():
        print(f"error: source not found: {source}", file=sys.stderr)
        return 1
    try:
        shell = resolve_shell(args.qr)
        source = source.resolve()
        events_process = run_process(
            [str(shell), "events", str(source), "--window", "full"], REPO_ROOT
        )
        series_process = run_process(
            [
                str(shell),
                "model-series",
                str(source),
                "--timeframe",
                args.timeframe,
                "--window",
                "full",
                "--bars",
                "100000",
            ],
            REPO_ROOT,
        )
    except ToolError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    for label, process in (("events", events_process), ("model-series", series_process)):
        if process.returncode != 0:
            if process.stderr.strip():
                print(process.stderr.rstrip(), file=sys.stderr)
            print(f"error: {label} exited with code {process.returncode}", file=sys.stderr)
            return 1
    try:
        events = parse_ndjson(events_process.stdout, "events")
        series = parse_ndjson(series_process.stdout, "model-series")
        closes = close_series(series)
        rows = event_rows(events, closes, args.timeframe, horizons)
    except ToolError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    if not rows:
        print("error: no directional event with an available forward bar", file=sys.stderr)
        return 1
    report = {
        "schema": "quantrule.event-confidence/v1",
        "source": args.source,
        "timeframe": args.timeframe,
        "horizons": list(horizons),
        "events": len({(row["type"], row["subtype"]) for row in rows}),
        "by_type": grouped(rows, ("type",)),
        "by_subtype": grouped(rows, ("type", "subtype")),
    }
    try:
        text = json.dumps(report, indent=2, ensure_ascii=False, allow_nan=False)
    except (TypeError, ValueError) as error:
        print(f"error: cannot encode report: {error}", file=sys.stderr)
        return 1
    sys.stdout.write(text + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
