#!/usr/bin/env python3
"""Measure model-axis confidence on one CSV source and one timeframe.

The command runs ``model-series`` over the full window, joins each bar with the
bar ``h`` places later in the same series, and reports:

* direction and momentum hit rate at a pre-registered horizon grid (sign of the
  axis against sign of the forward return), with coverage and a 95%
  normal-approximation lower bound;
* ordered structure-code and progression-stage buckets with counts, means, and
  rank monotonicity at the same horizons.

No random split, bootstrap, or fitted threshold is used. A gap in the series is
skipped rather than joined across.
"""

from __future__ import annotations

import argparse
import csv
import json
import math
import sys
from pathlib import Path
from typing import Any, Iterable

from common import (
    REPO_ROOT,
    ToolError,
    is_number,
    mean,
    resolve_shell,
    run_process,
    sign,
)

Z_95 = 1.959963984540054

DEFAULT_HORIZONS = (1, 3, 5, 10, 20)
PRIMARY_HORIZONS = {
    "direction": 5,
    "momentum": 3,
    "structure": 5,
    "progression": 5,
}
HORIZONS = PRIMARY_HORIZONS
AXIS_PATHS = {
    "direction": ("direction", "value"),
    "momentum": ("momentum", "force"),
    "structure": ("structure", "score"),
    "progression": ("progression", "progress"),
}
BUCKET_PATHS = {
    "structure": ("structure", "code"),
    "progression": ("progression", "stage"),
}
BUCKET_NAMES = {"structure": "code", "progression": "stage"}

REGIME_FIELDS = {
    "atr_ratio": ("context", "atr_ratio"),
    "rvol_ratio": ("context", "rvol_ratio"),
    "context_code": ("context", "code"),
}


def regime_bucket(field: str, value: float) -> str | None:
    """Map one published context value to a pre-registered bucket."""
    if field == "atr_ratio":
        if value < 0.8:
            return "atr<0.8"
        if value <= 1.2:
            return "atr0.8-1.2"
        return "atr>1.2"
    if field == "rvol_ratio":
        if value < 1.0:
            return "rvol<1.0"
        if value <= 2.0:
            return "rvol1.0-2.0"
        return "rvol>2.0"
    if field == "context_code":
        return f"code={int(value)}"
    return None

CSV_HEADER = (
    "axis",
    "kind",
    "bucket",
    "horizon",
    "bars",
    "coverage",
    "evaluated",
    "hits",
    "hit_rate",
    "lower_bound_95",
    "count",
    "mean_axis_value",
    "mean_forward_return",
)


def parse_arguments(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        prog="model_confidence.py",
        description=(
            "Run model-series on a CSV source and compute directional hit rates "
            "for direction and momentum across a horizon grid, plus ordered "
            "bucket means and monotonicity for structure and progression."
        ),
        epilog=(
            "Example: python3 tools/improvement/model_confidence.py "
            "--source data/sources/BTCUSD_M1_2024.csv --timeframe 1h"
        ),
    )
    parser.add_argument("--source", required=True, help="CSV candle source")
    parser.add_argument(
        "--timeframe",
        required=True,
        choices=("1m", "5m", "15m", "1h", "4h"),
        help="timeframe passed to model-series",
    )
    parser.add_argument(
        "--qr",
        default="target/debug/quantrule",
        help="shell binary (default: target/debug/quantrule)",
    )
    parser.add_argument(
        "--format",
        choices=("json", "csv"),
        default="json",
        help="stdout format (default: json)",
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


def nested(record: dict[str, Any], path: Iterable[str]) -> Any:
    value: Any = record
    for key in path:
        if not isinstance(value, dict):
            return None
        value = value.get(key)
    return value


def numeric_axis(record: dict[str, Any], path: Iterable[str]) -> float | None:
    value = nested(record, path)
    return float(value) if is_number(value) else None


def series_index(record: dict[str, Any], position: int) -> int | None:
    value = record.get("series_index")
    if isinstance(value, int) and not isinstance(value, bool):
        return value
    if isinstance(value, float) and value.is_integer():
        return int(value)
    return None


def parse_records(stdout: str) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    for number, line in enumerate(stdout.splitlines(), 1):
        if not line.strip():
            continue
        try:
            value = json.loads(line)
        except json.JSONDecodeError as error:
            raise ToolError(
                f"model-series line {number}: invalid JSON: {error.msg} "
                f"(column {error.colno})"
            ) from None
        if not isinstance(value, dict):
            raise ToolError(f"model-series line {number}: record is not an object")
        close = value.get("close")
        if not is_number(close):
            raise ToolError(f"model-series line {number}: close is not a finite number")
        records.append(value)
    if not records:
        raise ToolError("model-series produced no records")
    return records


def index_records(
    records: list[dict[str, Any]],
) -> tuple[list[dict[str, Any]], list[str]]:
    notes: list[str] = []
    positions = [series_index(record, position) for position, record in enumerate(records)]
    if any(position is None for position in positions):
        positions = list(range(len(records)))
        notes.append("series_index is missing on at least one record; record order was used")

    indexed = [
        {
            "series_index": position,
            "close": float(record["close"]),
            "record": record,
        }
        for position, record in zip(positions, records)
    ]
    indexed.sort(key=lambda item: item["series_index"])

    seen: set[int] = set()
    for item in indexed:
        position = item["series_index"]
        if position in seen:
            raise ToolError(f"model-series repeats series_index {position}")
        seen.add(position)

    original = positions or []
    if not notes and original != sorted(original):
        notes.append("records were sorted by series_index before joining")
    return indexed, notes


def forward_pairs(
    indexed: list[dict[str, Any]], horizon: int
) -> tuple[list[tuple[dict[str, Any], float]], int]:
    """Join each bar with the bar exactly ``horizon`` series steps later."""
    by_index = {item["series_index"]: item for item in indexed}
    pairs: list[tuple[dict[str, Any], float]] = []
    skipped = 0
    for item in indexed:
        target = by_index.get(item["series_index"] + horizon)
        if target is None or item["close"] <= 0 or target["close"] <= 0:
            skipped += 1
            continue
        forward = target["close"] / item["close"] - 1.0
        if not math.isfinite(forward):
            skipped += 1
            continue
        pairs.append((item, forward))
    return pairs, skipped


def coverage(indexed: list[dict[str, Any]], axis: str) -> float | None:
    if not indexed:
        return None
    present = sum(
        1 for item in indexed if is_number(nested(item["record"], AXIS_PATHS[axis]))
    )
    return present / len(indexed)


def lower_bound(rate: float | None, count: int) -> float | None:
    if rate is None or count <= 0:
        return None
    margin = Z_95 * math.sqrt(rate * (1.0 - rate) / count)
    return max(0.0, rate - margin)


def average_ranks(values: list[float]) -> list[float]:
    """Return average ranks, with ties sharing the mean rank."""
    order = sorted(range(len(values)), key=lambda index: values[index])
    ranks = [0.0] * len(values)
    index = 0
    while index < len(order):
        end = index + 1
        while end < len(order) and values[order[end]] == values[order[index]]:
            end += 1
        rank = (index + 1 + end) / 2.0
        for position in order[index:end]:
            ranks[position] = rank
        index = end
    return ranks


def spearman(xs: list[float], ys: list[float]) -> float | None:
    """Spearman rank correlation without a dependency."""
    if len(xs) != len(ys) or len(xs) < 3:
        return None
    rx = average_ranks(xs)
    ry = average_ranks(ys)
    mean_x = mean(rx)
    mean_y = mean(ry)
    if mean_x is None or mean_y is None:
        return None
    numerator = sum((x - mean_x) * (y - mean_y) for x, y in zip(rx, ry))
    denominator = math.sqrt(
        sum((x - mean_x) ** 2 for x in rx) * sum((y - mean_y) ** 2 for y in ry)
    )
    if denominator == 0.0:
        return None
    value = numerator / denominator
    return value if math.isfinite(value) else None


def bucket_monotonicity(buckets: list[dict[str, Any]], name: str) -> tuple[float | None, bool, bool]:
    ordered = [
        bucket
        for bucket in buckets
        if bucket.get("mean_forward_return") is not None
    ]
    if len(ordered) < 3:
        return None, False, False
    keys = [float(bucket[name]) for bucket in ordered]
    returns = [float(bucket["mean_forward_return"]) for bucket in ordered]
    correlation = spearman(keys, returns)
    nondecreasing = all(a <= b for a, b in zip(returns, returns[1:]))
    nonincreasing = all(a >= b for a, b in zip(returns, returns[1:]))
    return correlation, nondecreasing, nonincreasing


def hit_rate_at(
    indexed: list[dict[str, Any]], axis: str, horizon: int
) -> tuple[dict[str, Any], list[str]]:
    notes: list[str] = []
    pairs, skipped = forward_pairs(indexed, horizon)
    evaluated = [
        (numeric_axis(item["record"], AXIS_PATHS[axis]), forward)
        for item, forward in pairs
    ]
    evaluated = [(value, forward) for value, forward in evaluated if value is not None]
    hits = sum(1 for value, forward in evaluated if sign(value) == sign(forward))
    count = len(evaluated)
    rate = hits / count if count else None
    data = {
        "horizon": horizon,
        "coverage": coverage(indexed, axis),
        "unpaired": skipped,
        "evaluated": count,
        "hits": hits,
        "hit_rate": rate,
        "lower_bound_95": lower_bound(rate, count),
    }
    if count == 0:
        notes.append(f"{axis}: no non-null axis value has a forward bar at {horizon}")
    return data, notes


def regime_hit_rate(
    indexed: list[dict[str, Any]], axis: str, horizon: int, field: str
) -> list[dict[str, Any]]:
    """Hit rate by one pre-registered context bucket at one horizon."""
    pairs, _ = forward_pairs(indexed, horizon)
    path = REGIME_FIELDS[field]
    buckets: dict[str, list[tuple[float, float]]] = {}
    for item, forward in pairs:
        axis_value = numeric_axis(item["record"], AXIS_PATHS[axis])
        raw_regime = nested(item["record"], path)
        if axis_value is None or not is_number(raw_regime):
            continue
        label = regime_bucket(field, float(raw_regime))
        if label is None:
            continue
        buckets.setdefault(label, []).append((axis_value, forward))
    rows = []
    for label in sorted(buckets):
        values = buckets[label]
        hits = sum(1 for value, forward in values if sign(value) == sign(forward))
        count = len(values)
        rate = hits / count if count else None
        rows.append(
            {
                "field": field,
                "bucket": label,
                "evaluated": count,
                "hits": hits,
                "hit_rate": rate,
                "lower_bound_95": lower_bound(rate, count),
            }
        )
    return rows


def hit_rate_axis(
    indexed: list[dict[str, Any]], axis: str, horizons: tuple[int, ...]
) -> tuple[dict[str, Any], list[str]]:
    notes: list[str] = []
    by_horizon: list[dict[str, Any]] = []
    for horizon in horizons:
        data, axis_notes = hit_rate_at(indexed, axis, horizon)
        by_horizon.append(data)
        notes.extend(axis_notes)
    primary = next(
        data for data in by_horizon if data["horizon"] == PRIMARY_HORIZONS[axis]
    )
    data = dict(primary)
    data["primary_horizon"] = PRIMARY_HORIZONS[axis]
    data["by_horizon"] = by_horizon
    data["by_regime"] = {
        field: regime_hit_rate(indexed, axis, PRIMARY_HORIZONS[axis], field)
        for field in REGIME_FIELDS
    }
    return data, notes


def bucket_at(
    indexed: list[dict[str, Any]], axis: str, horizon: int
) -> tuple[dict[str, Any], list[str]]:
    notes: list[str] = []
    pairs, skipped = forward_pairs(indexed, horizon)
    forward_by_index = {item["series_index"]: forward for item, forward in pairs}
    bucket_path = BUCKET_PATHS[axis]
    name = BUCKET_NAMES[axis]

    buckets: dict[float | str, dict[str, list[float]]] = {}
    non_numeric = 0
    for item in indexed:
        code = nested(item["record"], bucket_path)
        if not is_number(code):
            non_numeric += 1
            continue
        key: float | str = float(code)
        entry = buckets.setdefault(key, {"values": [], "forwards": []})
        bucket_series_index = item["series_index"]
        value = numeric_axis(item["record"], AXIS_PATHS[axis])
        if value is not None:
            entry["values"].append(value)
            forward = forward_by_index.get(bucket_series_index)
            if forward is not None:
                entry["forwards"].append(forward)

    def order(key: float | str) -> tuple[int, float]:
        if isinstance(key, float):
            return (0, key)
        return (1, 0.0)

    rows = []
    for key in sorted(buckets, key=order):
        entry = buckets[key]
        row = {
            name: int(key) if isinstance(key, float) and key.is_integer() else key,
            "count": len(entry["values"]),
            "forward_count": len(entry["forwards"]),
            "mean_axis_value": mean(entry["values"]),
            "mean_forward_return": mean(entry["forwards"]),
        }
        rows.append(row)

    if non_numeric:
        notes.append(
            f"{axis}: {non_numeric} record(s) have a null or non-numeric "
            f"{'.'.join(bucket_path)} and were left out of the buckets"
        )
    correlation, nondecreasing, nonincreasing = bucket_monotonicity(rows, name)
    data = {
        "horizon": horizon,
        "coverage": coverage(indexed, axis),
        "unpaired": skipped,
        "bucket_field": ".".join(bucket_path),
        "spearman": correlation,
        "monotonic_nondecreasing": nondecreasing,
        "monotonic_nonincreasing": nonincreasing,
        "buckets": rows,
    }
    return data, notes


def bucket_axis(
    indexed: list[dict[str, Any]], axis: str, horizons: tuple[int, ...]
) -> tuple[dict[str, Any], list[str]]:
    notes: list[str] = []
    by_horizon: list[dict[str, Any]] = []
    for horizon in horizons:
        data, axis_notes = bucket_at(indexed, axis, horizon)
        by_horizon.append(data)
        notes.extend(axis_notes)
    primary = next(
        data for data in by_horizon if data["horizon"] == PRIMARY_HORIZONS[axis]
    )
    data = dict(primary)
    data["primary_horizon"] = PRIMARY_HORIZONS[axis]
    data["by_horizon"] = by_horizon
    return data, notes


def build_report(
    indexed: list[dict[str, Any]], notes: list[str], horizons: tuple[int, ...]
) -> dict[str, Any]:
    axes: dict[str, Any] = {}
    for axis in ("direction", "momentum"):
        data, axis_notes = hit_rate_axis(indexed, axis, horizons)
        axes[axis] = data
        notes.extend(axis_notes)
    for axis in ("structure", "progression"):
        data, axis_notes = bucket_axis(indexed, axis, horizons)
        axes[axis] = data
        notes.extend(axis_notes)
    return {
        "schema": "quantrule.model-confidence/v1",
        "bars": len(indexed),
        "horizons": HORIZONS,
        "horizon_grid": list(horizons),
        "lower_bound_method": "p - 1.96 * sqrt(p * (1 - p) / n)",
        "axes": axes,
        "notes": notes,
    }


def format_value(value: Any) -> str:
    if value is None:
        return ""
    if isinstance(value, bool):
        return "true" if value else "false"
    return str(value)


def write_csv(report: dict[str, Any]) -> None:
    writer = csv.writer(sys.stdout, lineterminator="\n")
    writer.writerow(CSV_HEADER)
    for axis in ("direction", "momentum"):
        data = report["axes"][axis]
        writer.writerow(
            [
                axis,
                "hit_rate",
                "",
                data["horizon"],
                report["bars"],
                format_value(data["coverage"]),
                data["evaluated"],
                data["hits"],
                format_value(data["hit_rate"]),
                format_value(data["lower_bound_95"]),
                "",
                "",
                "",
            ]
        )
    for axis, name in (("structure", "code"), ("progression", "stage")):
        data = report["axes"][axis]
        for bucket in data["buckets"]:
            writer.writerow(
                [
                    axis,
                    "bucket",
                    format_value(bucket[name]),
                    data["horizon"],
                    report["bars"],
                    format_value(data["coverage"]),
                    "",
                    "",
                    "",
                    "",
                    bucket["count"],
                    format_value(bucket["mean_axis_value"]),
                    format_value(bucket["mean_forward_return"]),
                ]
            )


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
        command = [
            str(shell),
            "model-series",
            str(source),
            "--timeframe",
            args.timeframe,
            "--window",
            "full",
            "--bars",
            "100000",
        ]
        process = run_process(command, REPO_ROOT)
    except ToolError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1

    if process.returncode != 0:
        if process.stderr.strip():
            print(process.stderr.rstrip(), file=sys.stderr)
        print(
            f"error: model-series exited with code {process.returncode}",
            file=sys.stderr,
        )
        return 1

    try:
        records = parse_records(process.stdout)
        indexed, notes = index_records(records)
    except ToolError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1

    report = build_report(indexed, notes, horizons)
    report = {
        "schema": report["schema"],
        "source": args.source,
        "timeframe": args.timeframe,
        "command": [args.qr, "model-series", args.source, "--timeframe", args.timeframe, "--window", "full"],
        "bars": report["bars"],
        "horizons": report["horizons"],
        "horizon_grid": report["horizon_grid"],
        "lower_bound_method": report["lower_bound_method"],
        "axes": report["axes"],
        "notes": report["notes"],
    }

    if args.format == "csv":
        write_csv(report)
    else:
        try:
            text = json.dumps(
                report, indent=2, ensure_ascii=False, allow_nan=False
            )
        except (TypeError, ValueError) as error:
            print(f"error: cannot encode report: {error}", file=sys.stderr)
            return 1
        sys.stdout.write(text + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
