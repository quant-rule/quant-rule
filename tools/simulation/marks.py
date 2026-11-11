#!/usr/bin/env python3
"""Write and validate simulation marks.

The schema is the human annotator's schema, unchanged:
``chart,bar,marked_time_utc,action,quality,side,note``. A mark is evidence of a
review, not a trading decision; the schema is kept compatible so the existing
annotator and ``tools/trades/annotations.py`` can read the AI marks.

Usage:
    from marks import read_marks, validate_mark, write_marks
    rows = read_marks(path)
"""

from __future__ import annotations

import csv
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Iterable

ANNOTATIONS_HEADER = ["chart", "bar", "marked_time_utc", "action", "quality", "side", "note"]
ACTIONS = {"entry", "exit"}
QUALITIES = {"good", "bad", "neutral"}
SIDES = {"long", "short"}
NOTE_LIMIT = 500


class MarkError(Exception):
    """A mark is missing, malformed, or outside the chart."""


def utc_from_seconds(seconds: int) -> str:
    """Format UTC seconds as an ISO timestamp."""
    return datetime.fromtimestamp(seconds, tz=timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def parse_utc(value: str) -> int:
    """Parse an ISO UTC timestamp into seconds."""
    moment = datetime.fromisoformat(value.replace("Z", "+00:00"))
    if moment.tzinfo is None:
        moment = moment.replace(tzinfo=timezone.utc)
    return int(moment.timestamp())


def chart_bars(chart: dict[str, Any]) -> dict[int, list[Any]]:
    """Return bar -> [bar, timestamp, high, low]."""
    return {int(row[0]): row for row in chart["bars"]}


def read_marks(path: Path) -> list[dict[str, str]]:
    """Read a marks CSV; a missing file is an empty list."""
    if not path.is_file():
        return []
    with path.open(encoding="utf-8", newline="") as handle:
        rows = list(csv.DictReader(handle))
    return [{key: row.get(key, "") for key in ANNOTATIONS_HEADER} for row in rows]


def write_marks(path: Path, rows: Iterable[dict[str, str]]) -> None:
    """Write marks atomically in a stable order."""
    ordered = sorted(
        rows,
        key=lambda row: (
            row.get("chart", ""),
            int(row["bar"]) if row.get("bar", "").isdigit() else 0,
            row.get("action", ""),
            row.get("side", ""),
        ),
    )
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(path.name + ".tmp")
    with temporary.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=ANNOTATIONS_HEADER)
        writer.writeheader()
        for row in ordered:
            writer.writerow({key: row.get(key, "") for key in ANNOTATIONS_HEADER})
    temporary.replace(path)


def annotation_key(value: dict[str, Any]) -> tuple[str, str, str, str]:
    """Return the identity of a mark."""
    return (
        str(value.get("chart", "")),
        str(value.get("bar", "")),
        str(value.get("action", "")),
        str(value.get("side", "")),
    )


def validate_mark(value: dict[str, Any], charts: dict[str, dict[str, Any]]) -> dict[str, str]:
    """Validate one mark and derive its timestamp from the chart bar."""
    chart_name = str(value.get("chart", ""))
    chart = charts.get(chart_name)
    if chart is None:
        raise MarkError(f"unknown chart {chart_name!r}")
    try:
        bar = int(value["bar"])
    except (KeyError, TypeError, ValueError) as error:
        raise MarkError("bar must be an integer") from error
    bars = chart_bars(chart)
    if bar not in bars:
        raise MarkError(f"bar {bar} is not in {chart_name}")
    action = str(value.get("action", ""))
    quality = str(value.get("quality", ""))
    side = str(value.get("side", ""))
    if action not in ACTIONS:
        raise MarkError(f"unknown action {action!r}")
    if quality not in QUALITIES:
        raise MarkError(f"unknown quality {quality!r}")
    if side not in SIDES:
        raise MarkError(f"unknown side {side!r}")
    note = str(value.get("note", "")).strip()
    if len(note) > NOTE_LIMIT:
        raise MarkError(f"note is longer than {NOTE_LIMIT} characters")
    return {
        "chart": chart_name,
        "bar": str(bar),
        "marked_time_utc": utc_from_seconds(int(bars[bar][1])),
        "action": action,
        "quality": quality,
        "side": side,
        "note": note,
    }


def validate_marks(rows: Iterable[dict[str, Any]], charts: list[dict[str, Any]]) -> list[dict[str, str]]:
    """Validate a set of marks against a chart manifest."""
    by_name = {chart["file"]: chart for chart in charts}
    validated = [validate_mark(row, by_name) for row in rows]
    seen: set[tuple[str, str, str, str]] = set()
    for row in validated:
        key = annotation_key(row)
        if key in seen:
            raise MarkError(f"duplicate mark {key}")
        seen.add(key)
    return validated


def upsert(rows: list[dict[str, str]], row: dict[str, str]) -> list[dict[str, str]]:
    """Replace a mark with the same chart/bar/action/side."""
    key = annotation_key(row)
    return [item for item in rows if annotation_key(item) != key] + [row]


def delete(rows: list[dict[str, str]], value: dict[str, Any]) -> list[dict[str, str]]:
    """Delete a mark by chart/bar/action/side."""
    key = annotation_key(value)
    return [item for item in rows if annotation_key(item) != key]


def count_by_quality(rows: Iterable[dict[str, str]]) -> dict[tuple[str, str], int]:
    """Count marks by quality and side."""
    counts: dict[tuple[str, str], int] = {}
    for row in rows:
        key = (row["quality"], row["side"])
        counts[key] = counts.get(key, 0) + 1
    return counts
