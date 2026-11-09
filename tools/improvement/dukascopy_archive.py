#!/usr/bin/env python3
"""Build a one-minute CSV from the Dukascopy historical archive.

The chart endpoint used by ``quantrule fetch`` is not complete for every year.
The historical archive publishes one LZMA-compressed file per UTC day:

    https://datafeed.dukascopy.com/datafeed/<SYMBOL>/<YYYY>/<MM-1>/<DD>/BID_candles_min_1.bi5

Each record is 24 big-endian bytes: seconds since 00:00 UTC, open, close, low,
high (integer prices), and volume (float32). BTCUSD prices are scaled by 10.
The tool downloads every day of the requested year, retries transient failures,
decodes the records, sorts and deduplicates by timestamp, and writes the
repository CSV contract.

Usage:
    python3 tools/improvement/dukascopy_archive.py \
        --symbol BTCUSD --year 2023 \
        --out data/sources/BTCUSD_M1_2023-01-01_2023-12-31.csv

The raw files are kept under ``data/scratch/dukascopy/<SYMBOL>-<YYYY>/`` so a
failed run can resume without re-downloading complete days. The tool is a
research data-acquisition utility; it is not part of the product shell.
"""

from __future__ import annotations

import argparse
import calendar
import concurrent.futures
import csv
import lzma
import struct
import sys
import time
import urllib.error
import urllib.request
from collections import Counter
from dataclasses import dataclass
from datetime import date, datetime, timedelta, timezone
from pathlib import Path

ARCHIVE_ROOT = "https://datafeed.dukascopy.com/datafeed"
USER_AGENT = "Mozilla/5.0 (X11; Linux x86_64)"
RECORD = struct.Struct(">Iiiiif")
PRICE_SCALE = 10.0
MAX_ATTEMPTS = 8


class ArchiveError(ValueError):
    """The archive bytes or request cannot be decoded."""


@dataclass(frozen=True)
class Candle:
    timestamp: datetime
    open: float
    high: float
    low: float
    close: float
    volume: float


def archive_url(symbol: str, day: date) -> str:
    """Return the daily 1m candle URL. The provider month is zero-based."""
    return f"{ARCHIVE_ROOT}/{symbol}/{day.year}/{day.month - 1:02d}/{day.day:02d}/BID_candles_min_1.bi5"


def decode_candles(blob: bytes, day: date, price_scale: float = PRICE_SCALE) -> list[Candle]:
    """Decode one daily ``.bi5`` payload into UTC candles."""
    try:
        raw = lzma.decompress(blob)
    except lzma.LZMAError as error:
        raise ArchiveError(f"{day}: compressed payload is not LZMA") from error
    if len(raw) % RECORD.size:
        raise ArchiveError(f"{day}: decompressed length {len(raw)} is not a multiple of {RECORD.size}")
    candles = []
    midnight = datetime(day.year, day.month, day.day, tzinfo=timezone.utc)
    for offset in range(0, len(raw), RECORD.size):
        seconds, open_, close, low, high, volume = RECORD.unpack(raw[offset : offset + RECORD.size])
        candles.append(
            Candle(
                timestamp=midnight + timedelta(seconds=seconds),
                open=open_ / price_scale,
                high=high / price_scale,
                low=low / price_scale,
                close=close / price_scale,
                volume=float(volume),
            )
        )
    return candles


def normalize(candles: list[Candle]) -> list[Candle]:
    """Sort by timestamp and keep the first record for each timestamp."""
    ordered = sorted(candles, key=lambda candle: candle.timestamp)
    unique: list[Candle] = []
    for candle in ordered:
        if unique and candle.timestamp == unique[-1].timestamp:
            continue
        unique.append(candle)
    return unique


def format_volume(volume: float) -> str:
    text = f"{volume:.6f}".rstrip("0").rstrip(".")
    return text or "0"


def is_flat_zero(candle: Candle) -> bool:
    """True when the archive repeated the previous price with no volume."""
    return candle.open == candle.high == candle.low == candle.close and round(candle.volume, 6) == 0.0


def summarize(candles: list[Candle]) -> dict[str, object]:
    """Summarize row completeness and the flat zero-volume placeholder share.

    A full row count is not market coverage: the archive can emit a flat
    zero-volume candle for every minute it has no ticks. A caller must check
    this before admitting the series to a backtest.
    """
    hours = Counter(candle.timestamp.strftime("%Y-%m-%d %H") for candle in candles)
    full_hours = sum(1 for count in hours.values() if count == 60)
    if candles:
        first_hour = candles[0].timestamp.replace(minute=0, second=0, microsecond=0)
        last_hour = candles[-1].timestamp.replace(minute=0, second=0, microsecond=0)
        expected_hours = int((last_hour - first_hour).total_seconds() // 3600) + 1
    else:
        expected_hours = 0
    flat = 0
    longest = 0
    longest_start = None
    run = 0
    run_start = None
    for candle in candles:
        if is_flat_zero(candle):
            if run == 0:
                run_start = candle.timestamp
            run += 1
            flat += 1
            if run > longest:
                longest = run
                longest_start = run_start
        else:
            run = 0
    return {
        "rows": len(candles),
        "first": candles[0].timestamp.isoformat() if candles else None,
        "last": candles[-1].timestamp.isoformat() if candles else None,
        "flat_zero_bars": flat,
        "flat_zero_share": flat / len(candles) if candles else None,
        "hours_present": len(hours),
        "expected_hours": expected_hours,
        "full_hours": full_hours,
        "coverage_share": full_hours / expected_hours if expected_hours else None,
        "longest_flat_run": longest,
        "longest_flat_start": longest_start.isoformat() if longest_start else None,
    }


def write_csv(candles: list[Candle], path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.writer(handle)
        writer.writerow(["Date", "Open", "High", "Low", "Close", "Volume"])
        for candle in candles:
            writer.writerow(
                [
                    candle.timestamp.strftime("%Y-%m-%d %H:%M:%S+00:00"),
                    f"{candle.open:.1f}",
                    f"{candle.high:.1f}",
                    f"{candle.low:.1f}",
                    f"{candle.close:.1f}",
                    format_volume(candle.volume),
                ]
            )


def http_get(url: str, timeout: float = 45.0) -> bytes:
    request = urllib.request.Request(
        url,
        headers={"User-Agent": USER_AGENT, "Referer": "https://www.dukascopy.com/"},
    )
    with urllib.request.urlopen(request, timeout=timeout) as response:
        return response.read()


def fetch_day(
    symbol: str,
    day: date,
    raw_dir: Path,
    fetch=http_get,
    attempts: int = MAX_ATTEMPTS,
) -> tuple[date, str]:
    """Download one day, reusing a cached non-empty raw file when present."""
    path = raw_dir / f"{day.isoformat()}.bi5"
    if path.exists() and path.stat().st_size:
        return day, "cached"
    url = archive_url(symbol, day)
    last = "unknown"
    for attempt in range(1, attempts + 1):
        try:
            payload = fetch(url)
        except urllib.error.HTTPError as error:
            last = f"http {error.code}"
            if error.code == 404:
                return day, "missing"
        except (urllib.error.URLError, TimeoutError, OSError) as error:
            last = str(error)
        else:
            if payload:
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(payload)
                return day, "ok"
            last = "empty"
        time.sleep(min(1.5 * attempt, 10.0))
    return day, f"failed: {last}"


def build_year(
    symbol: str,
    year: int,
    out: Path,
    raw_dir: Path,
    workers: int = 6,
    fetch=http_get,
    price_scale: float = PRICE_SCALE,
) -> dict[str, object]:
    """Download and decode one year. Return a deterministic summary."""
    days = [date(year, 1, 1) + timedelta(days=offset) for offset in range(366 if calendar.isleap(year) else 365)]
    raw_dir.mkdir(parents=True, exist_ok=True)
    statuses: dict[date, str] = {}
    with concurrent.futures.ThreadPoolExecutor(max_workers=max(1, workers)) as pool:
        futures = {pool.submit(fetch_day, symbol, day, raw_dir, fetch): day for day in days}
        for future in concurrent.futures.as_completed(futures):
            day, status = future.result()
            statuses[day] = status
    problems = {day.isoformat(): status for day, status in statuses.items() if status not in {"ok", "cached"}}

    candles: list[Candle] = []
    for day in days:
        path = raw_dir / f"{day.isoformat()}.bi5"
        if path.exists() and path.stat().st_size:
            candles.extend(decode_candles(path.read_bytes(), day, price_scale))
    candles = normalize(candles)
    write_csv(candles, out)
    return {
        "symbol": symbol,
        "year": year,
        "days": len(days),
        "missing_or_failed": problems,
        "out": str(out),
        "quality": summarize(candles),
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--symbol", default="BTCUSD")
    parser.add_argument("--year", type=int, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--raw-dir", type=Path)
    parser.add_argument("--workers", type=int, default=6)
    parser.add_argument("--price-scale", type=float, default=PRICE_SCALE)
    args = parser.parse_args(argv)
    raw_dir = args.raw_dir or Path("data/scratch/dukascopy") / f"{args.symbol}-{args.year}"
    summary = build_year(args.symbol, args.year, args.out, raw_dir, args.workers, price_scale=args.price_scale)
    quality = summary["quality"]
    print(
        f"rows={quality['rows']} full_hours={quality['full_hours']}/{quality['expected_hours']} "
        f"flat_zero={quality['flat_zero_bars']} longest_flat={quality['longest_flat_run']} "
        f"out={summary['out']}"
    )
    problems = summary["missing_or_failed"]
    if problems:
        for day, status in sorted(problems.items()):
            print(f"  {day}: {status}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
