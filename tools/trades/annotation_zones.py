#!/usr/bin/env python3
"""Extract zone features from recorded entry/exit annotation contexts.

The human mark is a zone, not only one candle. This tool reads the 250-bar
context ending at the mark and the post window, builds a symmetric zone around
the mark, and publishes one feature row per annotation plus a grouped summary.

It is evaluation-only: it never changes a model event, a strategy decision, or a
trade. A zone feature that uses bars after the mark is descriptive and must not
be read as a causal signal.

Usage:
    python3 tools/trades/annotation_zones.py \
        --contexts data/.../annotations/human-2018-v12 \
                   data/.../annotations/human-2019-v12 \
        --out data/.../annotations/zones-2026-11-10
"""

from __future__ import annotations

import argparse
import csv
import json
import statistics as stats
import sys
from pathlib import Path
from typing import Any

DEFAULT_RADIUS = 3
DEFAULT_EMA_ATR = 0.2
DEFAULT_FIB_ATR = 0.2
FORWARD_HORIZONS = (4, 16, 50)
MFE_BARS = 50

ZONE_COLUMNS = [
    "year",
    "chart",
    "bar",
    "time",
    "side",
    "action",
    "quality",
    "note",
    "close",
    "atr",
    "rsi",
    "adx",
    "ema_stack",
    "structure_state",
    "macd_state",
    "rsi_zone",
    "vwap_distance_atr",
    "poc_distance_atr",
    "va_position",
    "range_position",
    "close_position",
    "lower_high_count",
    "lower_low_count",
    "higher_high_count",
    "higher_low_count",
    "lower_high_max",
    "lower_low_max",
    "higher_high_max",
    "higher_low_max",
    "fib_nearest_atr",
    "fib_nearest_ratio",
    "fib_position",
    "band_touches",
    "band_width_atr",
    "band_position",
    "band_center_distance_atr",
    "ema_fast_distance_atr",
    "ema_mid_distance_atr",
    "ema_slow_distance_atr",
    "ema_tap_fast",
    "ema_tap_mid",
    "ema_tap_slow",
    "ema_tap_any",
    "fib_test",
    "band_test",
    "fwd_4",
    "fwd_16",
    "fwd_50",
    "mfe_50",
    "mae_50",
]


def number(row: dict[str, str], field: str) -> float | None:
    """Read one numeric field, treating empty and null as unavailable."""
    value = row.get(field, "")
    if value in (None, "", "null"):
        return None
    try:
        return float(value)
    except ValueError:
        return None


def load_rows(path: Path) -> list[dict[str, str]]:
    """Read one context or post CSV."""
    with path.open(encoding="utf-8", newline="") as handle:
        return list(csv.DictReader(handle))


def annotation_root(root: Path, annotation: dict[str, Any]) -> Path:
    """Resolve one annotation directory, preferring the recorded path."""
    recorded = Path(str(annotation.get("path", "")))
    if recorded.is_dir():
        return recorded
    return root / f"annotation-{int(annotation['row']):03d}"


def zone_rows(
    context: list[dict[str, str]],
    post: list[dict[str, str]],
    radius: int,
) -> list[dict[str, str]]:
    """Build the mark zone: radius bars before, the mark, radius bars after."""
    if not context:
        return []
    before = context[-(radius + 1) :]
    return before + post[:radius]


def ema_tap(
    rows: list[dict[str, str]],
    field: str,
    tolerance_atr: float,
) -> bool:
    """Whether any zone bar touches the EMA within the ATR tolerance."""
    for row in rows:
        atr = number(row, "atr")
        ema = number(row, field)
        high = number(row, "high")
        low = number(row, "low")
        if atr is None or atr <= 0.0 or ema is None or high is None or low is None:
            continue
        if min(abs(high - ema), abs(low - ema)) <= tolerance_atr * atr:
            return True
    return False


def fib_test(rows: list[dict[str, str]], tolerance_atr: float) -> bool:
    """Whether any zone bar is within the Fibonacci tolerance."""
    values = [number(row, "fib_nearest_atr") for row in rows]
    return any(value is not None and value <= tolerance_atr for value in values)


def band_test(rows: list[dict[str, str]]) -> bool:
    """Whether any zone bar closes inside the selected band."""
    for row in rows:
        close = number(row, "close")
        lower = number(row, "band_lower")
        upper = number(row, "band_upper")
        if close is not None and lower is not None and upper is not None:
            if lower <= close <= upper:
                return True
    return False


def forward(
    mark: dict[str, str],
    post: list[dict[str, str]],
    side: str,
) -> dict[str, float | None]:
    """Side-adjusted forward returns and the 50-bar excursions."""
    base = number(mark, "close")
    sign = 1.0 if side == "long" else -1.0
    result: dict[str, float | None] = {}
    for horizon in FORWARD_HORIZONS:
        if base is None or base <= 0.0 or len(post) < horizon:
            result[f"fwd_{horizon}"] = None
            continue
        close = number(post[horizon - 1], "close")
        result[f"fwd_{horizon}"] = (
            None if close is None else sign * (close / base - 1.0)
        )
    window = post[:MFE_BARS]
    highs = [value for row in window if (value := number(row, "high")) is not None]
    lows = [value for row in window if (value := number(row, "low")) is not None]
    if base is None or base <= 0.0 or not highs or not lows:
        result["mfe_50"] = None
        result["mae_50"] = None
        return result
    high = max(highs)
    low = min(lows)
    if side == "long":
        result["mfe_50"] = high / base - 1.0
        result["mae_50"] = low / base - 1.0
    else:
        result["mfe_50"] = 1.0 - low / base
        result["mae_50"] = 1.0 - high / base
    return result


def mark_features(
    mark: dict[str, str],
    zone: list[dict[str, str]],
    ema_atr: float,
    fib_atr: float,
) -> dict[str, float | bool | None]:
    """Feature values at the mark and across the zone."""
    atr = number(mark, "atr")
    close = number(mark, "close")
    features: dict[str, float | bool | None] = {}
    for field in (
        "close",
        "atr",
        "rsi",
        "adx",
        "ema_stack",
        "structure_state",
        "macd_state",
        "rsi_zone",
        "vwap_distance_atr",
        "poc_distance_atr",
        "va_position",
        "range_position",
        "close_position",
        "lower_high_count",
        "lower_low_count",
        "higher_high_count",
        "higher_low_count",
        "fib_nearest_atr",
        "fib_nearest_ratio",
        "fib_position",
        "band_touches",
        "band_width_atr",
        "band_position",
    ):
        features[field] = number(mark, field)
    for field, label in (
        ("lower_high_count", "lower_high_max"),
        ("lower_low_count", "lower_low_max"),
        ("higher_high_count", "higher_high_max"),
        ("higher_low_count", "higher_low_max"),
    ):
        values = [number(item, field) for item in zone]
        present = [value for value in values if value is not None]
        features[label] = max(present) if present else None
    for field in ("ema_fast", "ema_mid", "ema_slow"):
        ema = number(mark, field)
        features[f"{field}_distance_atr"] = (
            None if atr is None or atr <= 0.0 or close is None or ema is None else (close - ema) / atr
        )
    center = number(mark, "band_center")
    features["band_center_distance_atr"] = (
        None if atr is None or atr <= 0.0 or close is None or center is None else (close - center) / atr
    )
    features["ema_tap_fast"] = ema_tap(zone, "ema_fast", ema_atr)
    features["ema_tap_mid"] = ema_tap(zone, "ema_mid", ema_atr)
    features["ema_tap_slow"] = ema_tap(zone, "ema_slow", ema_atr)
    features["ema_tap_any"] = bool(
        features["ema_tap_fast"] or features["ema_tap_mid"] or features["ema_tap_slow"]
    )
    features["fib_test"] = fib_test(zone, fib_atr)
    features["band_test"] = band_test(zone)
    return features


def annotation_row(
    root: Path,
    annotation: dict[str, Any],
    radius: int,
    ema_atr: float,
    fib_atr: float,
) -> dict[str, Any]:
    """Build one output row."""
    ann_root = annotation_root(root, annotation)
    context = load_rows(ann_root / "context.csv")
    post = load_rows(ann_root / "post.csv")
    if not context:
        raise ValueError(f"{ann_root}: empty context")
    mark = context[-1]
    zone = zone_rows(context, post, radius)
    row: dict[str, Any] = {
        "year": str(annotation["annotation"]["marked_time_utc"])[:4],
        "chart": annotation["annotation"]["chart"],
        "bar": annotation["bar"],
        "time": annotation["time"],
        "side": annotation["annotation"]["side"],
        "action": annotation["annotation"]["action"],
        "quality": annotation["annotation"]["quality"],
        "note": annotation["annotation"].get("note", ""),
    }
    row.update(mark_features(mark, zone, ema_atr, fib_atr))
    row.update(forward(mark, post, annotation["annotation"]["side"]))
    return row


def median(values: list[float | None]) -> float | None:
    present = [value for value in values if value is not None]
    return stats.median(present) if present else None


def rate(values: list[Any]) -> float | None:
    present = [value for value in values if value is not None]
    return sum(1 for value in present if value) / len(present) if present else None


def summarize(rows: list[dict[str, Any]]) -> dict[str, Any]:
    """Group rows by year/side/action and by side/action."""
    groups: dict[str, list[dict[str, Any]]] = {}
    for row in rows:
        for key in (
            f"{row['year']}|{row['side']}|{row['action']}",
            f"all|{row['side']}|{row['action']}",
        ):
            groups.setdefault(key, []).append(row)
    result: dict[str, Any] = {"groups": {}, "count": len(rows)}
    for key in sorted(groups):
        items = groups[key]
        entry: dict[str, Any] = {
            "count": len(items),
            "ema_tap_any_rate": rate([row["ema_tap_any"] for row in items]),
            "ema_tap_fast_rate": rate([row["ema_tap_fast"] for row in items]),
            "ema_tap_mid_rate": rate([row["ema_tap_mid"] for row in items]),
            "ema_tap_slow_rate": rate([row["ema_tap_slow"] for row in items]),
            "fib_test_rate": rate([row["fib_test"] for row in items]),
            "band_test_rate": rate([row["band_test"] for row in items]),
            "fwd_4_median": median([row["fwd_4"] for row in items]),
            "fwd_16_median": median([row["fwd_16"] for row in items]),
            "fwd_50_median": median([row["fwd_50"] for row in items]),
            "mfe_50_median": median([row["mfe_50"] for row in items]),
            "mae_50_median": median([row["mae_50"] for row in items]),
        }
        for field in ("lower_high_max", "lower_low_max", "higher_high_max", "higher_low_max"):
            entry[f"{field}_gte2_rate"] = rate(
                [row[field] is not None and row[field] >= 2 for row in items]
            )
        for field in (
            "rsi",
            "adx",
            "vwap_distance_atr",
            "poc_distance_atr",
            "va_position",
            "range_position",
            "close_position",
            "lower_high_count",
            "lower_low_count",
            "higher_high_count",
            "higher_low_count",
            "lower_high_max",
            "lower_low_max",
            "higher_high_max",
            "higher_low_max",
            "fib_nearest_atr",
            "fib_position",
            "band_touches",
            "band_width_atr",
            "band_position",
            "band_center_distance_atr",
            "ema_fast_distance_atr",
            "ema_mid_distance_atr",
            "ema_slow_distance_atr",
        ):
            entry[f"{field}_median"] = median([row[field] for row in items])
        if key.startswith("all|"):
            entry["fwd_4_hit"] = rate(
                [
                    (row["fwd_4"] > 0) if row["action"] == "entry" else (row["fwd_4"] < 0)
                    for row in items
                    if row["fwd_4"] is not None
                ]
            )
            entry["fwd_16_hit"] = rate(
                [
                    (row["fwd_16"] > 0) if row["action"] == "entry" else (row["fwd_16"] < 0)
                    for row in items
                    if row["fwd_16"] is not None
                ]
            )
            entry["fwd_50_hit"] = rate(
                [
                    (row["fwd_50"] > 0) if row["action"] == "entry" else (row["fwd_50"] < 0)
                    for row in items
                    if row["fwd_50"] is not None
                ]
            )
        result["groups"][key] = entry
    return result


def write_csv(path: Path, rows: list[dict[str, Any]]) -> None:
    """Write a deterministic CSV with the fixed column order."""
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=ZONE_COLUMNS)
        writer.writeheader()
        for row in rows:
            writer.writerow({key: row.get(key, "") for key in ZONE_COLUMNS})


def write_json(path: Path, value: Any) -> None:
    """Write deterministic pretty JSON."""
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def write_markdown(path: Path, summary: dict[str, Any], radius: int) -> None:
    """Write a compact human-readable summary."""
    lines = [
        "# Annotation zone summary",
        "",
        f"- Zone radius: {radius} bars each side of the mark",
        f"- Annotations: {summary['count']}",
        "",
        "| Group | n | EMA tap | Fib test | Band test | 4-bar | 16-bar | 50-bar | 50-bar hit |",
        "|---|---:|---:|---:|---:|---:|---:|---:|---:|",
    ]
    for key in sorted(summary["groups"]):
        entry = summary["groups"][key]
        fmt = lambda value: "null" if value is None else f"{value * 100:+.2f}%"
        hit = entry.get("fwd_50_hit")
        lines.append(
            f"| {key.replace('|', ' ')} | {entry['count']} | "
            f"{fmt(entry['ema_tap_any_rate'])} | {fmt(entry['fib_test_rate'])} | "
            f"{fmt(entry['band_test_rate'])} | {fmt(entry['fwd_4_median'])} | "
            f"{fmt(entry['fwd_16_median'])} | {fmt(entry['fwd_50_median'])} | "
            f"{fmt(hit)} |"
        )
    lines += [
        "",
        "A positive entry return and a negative exit return are favorable. The",
        "zone includes bars after the mark, so this table is descriptive, not a",
        "causal signal.",
    ]
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--contexts", nargs="+", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("--radius", type=int, default=DEFAULT_RADIUS)
    parser.add_argument("--ema-atr", type=float, default=DEFAULT_EMA_ATR)
    parser.add_argument("--fib-atr", type=float, default=DEFAULT_FIB_ATR)
    args = parser.parse_args(argv[1:])
    if args.radius < 0 or args.ema_atr < 0.0 or args.fib_atr < 0.0:
        print("error: radius and tolerances must be non-negative", file=sys.stderr)
        return 1
    rows: list[dict[str, Any]] = []
    try:
        for root in args.contexts:
            index = json.loads((root / "index.json").read_text(encoding="utf-8"))
            for annotation in index["annotations"]:
                rows.append(
                    annotation_row(root, annotation, args.radius, args.ema_atr, args.fib_atr)
                )
    except (OSError, ValueError, KeyError, json.JSONDecodeError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    rows.sort(key=lambda row: (row["year"], row["chart"], int(row["bar"]), row["side"], row["action"]))
    summary = summarize(rows)
    write_csv(args.out / "zones.csv", rows)
    write_json(args.out / "summary.json", summary)
    write_markdown(args.out / "summary.md", summary, args.radius)
    print(f"wrote {len(rows)} zone rows to {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
