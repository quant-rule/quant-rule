#!/usr/bin/env python3
"""Audit one OHLCV source and optionally compare it with another venue.

The audit is read-only. It reports row integrity, per-year M1 coverage, full
15m/1h/4h bucket coverage, flat zero-volume bars, gaps, and a timestamp-aligned
cross-venue close comparison. It never fills a gap, rewrites a source, or enters
the signal path.

Usage:
    python3 tools/data/audit_coverage.py \
        --source data/sources/BTCUSDT_M1_2018-01-01_2025-12-31.csv \
        --instrument BTCUSDT \
        --out data/improvement/2026-11-09/BINANCE-BTCUSDT/coverage.json \
        --compare data/sources/BTCUSD_M1_2018-01-01_2024-12-31.csv \
        --compare-from 2023-01-01 --compare-to 2023-12-31
"""

from __future__ import annotations

import argparse
import csv
import json
import statistics
import sys
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Iterator

HEADER = "Date,Open,High,Low,Close,Volume"
TIMEFRAMES = (15, 60, 240)


class AuditError(Exception):
    """The source or comparison cannot be audited."""


@dataclass(frozen=True)
class Row:
    timestamp: int
    open: float
    high: float
    low: float
    close: float
    volume: float


def parse_timestamp(value: str) -> int:
    """Parse the repository source timestamp shape."""
    try:
        moment = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError as error:
        raise AuditError(f"invalid timestamp {value!r}") from error
    if moment.tzinfo is None:
        moment = moment.replace(tzinfo=timezone.utc)
    return int(moment.timestamp())


def format_timestamp(seconds: int) -> str:
    """Format UTC seconds in the repository source shape."""
    return datetime.fromtimestamp(seconds, tz=timezone.utc).strftime("%Y-%m-%d %H:%M:%S+00:00")


def iter_rows(path: Path, start: int | None = None, end: int | None = None) -> Iterator[Row]:
    """Yield parsed rows, optionally limited to a timestamp range."""
    with path.open(encoding="utf-8", newline="") as handle:
        reader = csv.reader(handle)
        try:
            header = next(reader)
        except StopIteration as error:
            raise AuditError(f"{path} is empty") from error
        if ",".join(header) != HEADER:
            raise AuditError(f"{path} header is {','.join(header)!r}, expected {HEADER!r}")
        for number, fields in enumerate(reader, 2):
            if not fields:
                continue
            if len(fields) != 6:
                raise AuditError(f"{path}:{number} has {len(fields)} columns, expected 6")
            timestamp = parse_timestamp(fields[0])
            if start is not None and timestamp < start:
                continue
            if end is not None and timestamp > end:
                break
            try:
                open_price, high, low, close, volume = (float(value) for value in fields[1:])
            except ValueError as error:
                raise AuditError(f"{path}:{number} has a non-numeric value") from error
            yield Row(timestamp, open_price, high, low, close, volume)


def bucket_coverage(flags: bytearray, bucket_minutes: int) -> dict[str, int | float]:
    """Count fully present buckets in one year's minute flags."""
    expected = len(flags) // bucket_minutes
    full = 0
    for bucket in range(expected):
        start = bucket * bucket_minutes
        if all(flags[start : start + bucket_minutes]):
            full += 1
    return {
        "expected": expected,
        "full": full,
        "coverage": full / expected if expected else 0.0,
    }


def audit_source(path: Path, instrument: str) -> dict[str, object]:
    """Audit one source file."""
    years: dict[int, bytearray] = {}
    rows = 0
    duplicates = 0
    out_of_order = 0
    flat_zero = 0
    flat_by_year: dict[int, int] = {}
    first: int | None = None
    last: int | None = None
    previous_timestamp: int | None = None
    gap_count = 0
    gap_minutes = 0
    longest_gap = 0
    for row in iter_rows(path):
        rows += 1
        if first is None:
            first = row.timestamp
        last = row.timestamp
        moment = datetime.fromtimestamp(row.timestamp, tz=timezone.utc)
        length = 366 * 24 * 60 if _is_leap(moment.year) else 365 * 24 * 60
        flags = years.setdefault(moment.year, bytearray(length))
        minute = int(
            (moment - datetime(moment.year, 1, 1, tzinfo=timezone.utc)).total_seconds() // 60
        )
        if flags[minute]:
            duplicates += 1
        flags[minute] = 1
        if previous_timestamp is not None:
            if row.timestamp <= previous_timestamp:
                out_of_order += 1
            difference = (row.timestamp - previous_timestamp) // 60 - 1
            if difference > 0:
                gap_count += 1
                gap_minutes += difference
                longest_gap = max(longest_gap, difference)
        previous_timestamp = row.timestamp
        if row.open == row.high == row.low == row.close and row.volume == 0.0:
            flat_zero += 1
            flat_by_year[moment.year] = flat_by_year.get(moment.year, 0) + 1
    if rows == 0 or first is None or last is None:
        raise AuditError(f"{path} has no rows")
    coverage: dict[str, object] = {}
    for year, flags in sorted(years.items()):
        entry: dict[str, object] = {
            "present_minutes": sum(flags),
            "expected_minutes": len(flags),
            "coverage": sum(flags) / len(flags),
            "flat_zero": flat_by_year.get(year, 0),
        }
        for timeframe in TIMEFRAMES:
            entry[f"full_{timeframe}m"] = bucket_coverage(flags, timeframe)
        coverage[str(year)] = entry
    return {
        "instrument": instrument,
        "path": str(path),
        "rows": rows,
        "first": format_timestamp(first),
        "last": format_timestamp(last),
        "duplicates": duplicates,
        "out_of_order": out_of_order,
        "flat_zero": flat_zero,
        "gaps": {"count": gap_count, "missing_minutes": gap_minutes, "longest_minutes": longest_gap},
        "coverage": coverage,
    }


def _is_leap(year: int) -> bool:
    return year % 4 == 0 and (year % 100 != 0 or year % 400 == 0)


def compare_sources(
    source: Path,
    compare: Path,
    start: int,
    end: int,
) -> dict[str, object]:
    """Merge-join two timestamp-ordered sources and compare closes."""
    left = iter_rows(source, start, end)
    right = iter_rows(compare, start, end)
    a = next(left, None)
    b = next(right, None)
    overlap = 0
    ohlc_mismatch = 0
    close_differences: list[float] = []
    basis: list[float] = []
    while a is not None and b is not None:
        if a.timestamp < b.timestamp:
            a = next(left, None)
        elif a.timestamp > b.timestamp:
            b = next(right, None)
        else:
            overlap += 1
            if (
                abs(a.open - b.open) > 1e-9
                or abs(a.high - b.high) > 1e-9
                or abs(a.low - b.low) > 1e-9
                or abs(a.close - b.close) > 1e-9
            ):
                ohlc_mismatch += 1
            difference = abs(a.close - b.close)
            close_differences.append(difference)
            if b.close != 0.0:
                basis.append((a.close - b.close) / b.close)
            a = next(left, None)
            b = next(right, None)
    if not close_differences:
        return {"overlap_rows": 0}
    return {
        "overlap_rows": overlap,
        "ohlc_mismatch_rows": ohlc_mismatch,
        "close_difference": {
            "median": statistics.median(close_differences),
            "p95": percentile(close_differences, 0.95),
            "max": max(close_differences),
        },
        "basis_fraction": {
            "median": statistics.median(basis),
            "p05": percentile(basis, 0.05),
            "p95": percentile(basis, 0.95),
            "min": min(basis),
            "max": max(basis),
        },
    }


def percentile(values: list[float], fraction: float) -> float:
    """Linear-interpolated percentile of a finite sample."""
    ordered = sorted(values)
    if not ordered:
        raise AuditError("cannot take a percentile of an empty sample")
    position = (len(ordered) - 1) * fraction
    lower = int(position)
    upper = min(lower + 1, len(ordered) - 1)
    weight = position - lower
    return ordered[lower] * (1.0 - weight) + ordered[upper] * weight


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", required=True, type=Path)
    parser.add_argument("--instrument", required=True)
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("--compare", type=Path)
    parser.add_argument("--compare-from", default="2023-01-01")
    parser.add_argument("--compare-to", default="2023-12-31")
    args = parser.parse_args(argv[1:])
    try:
        report = audit_source(args.source, args.instrument)
        if args.compare is not None:
            start = parse_timestamp(f"{args.compare_from} 00:00:00+00:00")
            end = parse_timestamp(f"{args.compare_to} 23:59:00+00:00")
            report["comparison"] = {
                "path": str(args.compare),
                "from": args.compare_from,
                "to": args.compare_to,
                **compare_sources(args.source, args.compare, start, end),
            }
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    except AuditError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    print(f"wrote {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
