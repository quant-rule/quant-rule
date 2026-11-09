#!/usr/bin/env python3
"""Build a one-minute CSV from the Binance public spot archive.

The importer reads the monthly ``data.binance.vision`` kline zips, verifies the
published SHA-256 checksum for every month, validates every OHLCV row, and writes
one deterministic CSV in the repository's source format. It never writes a
partial output file: the CSV is committed through a sibling temporary file and a
rename.

The archive is a historical input, not a live feed. The importer performs no
strategy or indicator work and never enters the signal path.

Usage:
    python3 tools/data/binance_klines.py \
        --symbol BTCUSDT --start 2018-01 --end 2025-12 \
        --out data/sources/BTCUSDT_M1_2018-01-01_2025-12-31.csv

The raw monthly zips are cached under ``data/scratch/binance``. A cached file is
reused only when its checksum matches the published checksum.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import io
import json
import math
import sys
import time
import urllib.error
import urllib.request
import zipfile
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Iterator

BASE_URL = "https://data.binance.vision/data/spot/monthly/klines"
USER_AGENT = "quantrule-data-import/1"
TIMEOUT_SECONDS = 60
RETRIES = 3
# Binance switched the kline open-time field from milliseconds to microseconds
# for spot data from 2025-01-01. Values above this bound are microseconds.
MICROSECOND_BOUND = 10**14
EXPECTED_HEADER = "Date,Open,High,Low,Close,Volume"


class BinanceImportError(Exception):
    """The archive, checksum, or CSV row is invalid."""


@dataclass(frozen=True)
class MonthFile:
    """One monthly archive file and its published checksum."""

    year: int
    month: int
    url: str
    checksum_url: str


@dataclass
class ImportStats:
    """What one import produced."""

    symbol: str
    rows: int
    first_timestamp: int
    last_timestamp: int
    sha256: str
    months: int
    checksums: dict[str, str]
    normalized_unaligned: int = 0


def month_files(symbol: str, start: str, end: str) -> list[MonthFile]:
    """Return the inclusive monthly archive files for a symbol and range."""
    start_year, start_month = parse_month(start)
    end_year, end_month = parse_month(end)
    if (end_year, end_month) < (start_year, start_month):
        raise BinanceImportError("the end month precedes the start month")
    files: list[MonthFile] = []
    year, month = start_year, start_month
    while (year, month) <= (end_year, end_month):
        name = f"{symbol}-1m-{year:04d}-{month:02d}.zip"
        url = f"{BASE_URL}/{symbol}/1m/{name}"
        files.append(MonthFile(year, month, url, f"{url}.CHECKSUM"))
        month += 1
        if month == 13:
            year += 1
            month = 1
    return files


def parse_month(value: str) -> tuple[int, int]:
    """Parse ``YYYY-MM`` into a year and month."""
    try:
        parsed = datetime.strptime(value, "%Y-%m")
    except ValueError as error:
        raise BinanceImportError(f"invalid month {value!r}; expected YYYY-MM") from error
    return parsed.year, parsed.month


def checksum_from_text(text: str) -> str:
    """Extract the SHA-256 digest from a Binance CHECKSUM file."""
    token = text.strip().split(maxsplit=1)[0] if text.strip() else ""
    if len(token) != 64 or any(char not in "0123456789abcdefABCDEF" for char in token):
        raise BinanceImportError("the checksum file does not contain a SHA-256 digest")
    return token.lower()


def sha256_file(path: Path) -> str:
    """Compute the SHA-256 of a file without loading it into memory."""
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def download(url: str, target: Path) -> None:
    """Download a URL to a temporary file and rename it on success."""
    target.parent.mkdir(parents=True, exist_ok=True)
    temporary = target.with_name(target.name + ".part")
    request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    last_error: Exception | None = None
    for attempt in range(RETRIES):
        try:
            with urllib.request.urlopen(request, timeout=TIMEOUT_SECONDS) as response:
                with temporary.open("wb") as handle:
                    while True:
                        chunk = response.read(1024 * 1024)
                        if not chunk:
                            break
                        handle.write(chunk)
            temporary.replace(target)
            return
        except (urllib.error.URLError, TimeoutError, OSError) as error:
            last_error = error
            temporary.unlink(missing_ok=True)
            if attempt + 1 < RETRIES:
                time.sleep(1 + attempt)
    raise BinanceImportError(f"cannot download {url}: {last_error}")


def ensure_month(month: MonthFile, cache: Path) -> tuple[Path, str]:
    """Return the cached zip and verify it against the published checksum."""
    cache.mkdir(parents=True, exist_ok=True)
    name = Path(month.url).name
    zip_path = cache / name
    checksum_path = cache / (name + ".CHECKSUM")
    if not checksum_path.exists():
        download(month.checksum_url, checksum_path)
    expected = checksum_from_text(checksum_path.read_text(encoding="utf-8"))
    if not zip_path.exists() or sha256_file(zip_path) != expected:
        download(month.url, zip_path)
    actual = sha256_file(zip_path)
    if actual != expected:
        raise BinanceImportError(f"checksum mismatch for {name}: {actual} != {expected}")
    return zip_path, expected


def kline_timestamp(raw: str) -> tuple[int, bool]:
    """Convert a Binance kline open time to UTC seconds.

    Early Binance months contain a few thousand rows whose open time is offset
    by a fixed number of seconds (2018-02 is offset by 14.789 seconds). The data
    engine requires minute-aligned timestamps, so the importer floors those rows
    to the containing minute and records the count in the manifest. It never
    invents a missing row or shifts a normal row.
    """
    try:
        value = int(raw)
    except ValueError as error:
        raise BinanceImportError(f"invalid open time {raw!r}") from error
    if value > MICROSECOND_BOUND:
        value //= 1000
    aligned = value % 60_000 == 0
    if not aligned:
        value -= value % 60_000
    return value // 1000, aligned


def validate_row(
    row: list[str], where: str
) -> tuple[int, float, float, float, float, float, bool]:
    """Validate one kline row and return its numeric values."""
    if len(row) < 6:
        raise BinanceImportError(f"{where}: expected at least 6 columns, got {len(row)}")
    timestamp, aligned = kline_timestamp(row[0])
    try:
        open_price, high, low, close, volume = (float(value) for value in row[1:6])
    except ValueError as error:
        raise BinanceImportError(f"{where}: non-numeric OHLCV value") from error
    values = (open_price, high, low, close, volume)
    if not all(math.isfinite(value) for value in values):
        raise BinanceImportError(f"{where}: non-finite OHLCV value")
    if min(open_price, high, low, close) <= 0.0:
        raise BinanceImportError(f"{where}: price must be positive")
    if volume < 0.0:
        raise BinanceImportError(f"{where}: volume must be non-negative")
    if high < max(open_price, close, low) or low > min(open_price, close, high):
        raise BinanceImportError(f"{where}: inconsistent OHLC bounds")
    return (timestamp, open_price, high, low, close, volume, aligned)


def rows_from_zip(
    path: Path,
) -> Iterator[tuple[int, float, float, float, float, float, bool]]:
    """Yield validated rows from one monthly zip."""
    with zipfile.ZipFile(path) as archive:
        members = [name for name in archive.namelist() if name.endswith(".csv")]
        if len(members) != 1:
            raise BinanceImportError(f"{path.name}: expected one CSV member, got {len(members)}")
        with archive.open(members[0]) as raw:
            text = io.TextIOWrapper(raw, encoding="utf-8", newline="")
            for number, row in enumerate(csv.reader(text), 1):
                if not row:
                    continue
                if number == 1 and row[0].lower() in {"open_time", "open time"}:
                    continue
                yield validate_row(row, f"{path.name}:{number}")


def format_timestamp(seconds: int) -> str:
    """Format UTC seconds in the repository source timestamp shape."""
    return datetime.fromtimestamp(seconds, tz=timezone.utc).strftime("%Y-%m-%d %H:%M:%S+00:00")


def format_number(value: float) -> str:
    """Format a finite float with a decimal point for integral values."""
    text = repr(value)
    if "." not in text and "e" not in text and "E" not in text:
        text += ".0"
    return text


def import_range(symbol: str, start: str, end: str, out: Path, cache: Path) -> ImportStats:
    """Download, validate, and write one source CSV."""
    months = month_files(symbol, start, end)
    temporary = out.with_name(out.name + ".tmp")
    checksums: dict[str, str] = {}
    rows = 0
    normalized_unaligned = 0
    first: int | None = None
    last: int | None = None
    out.parent.mkdir(parents=True, exist_ok=True)
    try:
        with temporary.open("w", encoding="utf-8", newline="") as handle:
            handle.write(EXPECTED_HEADER + "\n")
            for month in months:
                zip_path, digest = ensure_month(month, cache)
                checksums[f"{month.year:04d}-{month.month:02d}"] = digest
                for timestamp, open_price, high, low, close, volume, aligned in rows_from_zip(
                    zip_path
                ):
                    if not aligned:
                        normalized_unaligned += 1
                    if last is not None and timestamp <= last:
                        raise BinanceImportError(
                            f"timestamps are not strictly increasing at {format_timestamp(timestamp)}"
                        )
                    if first is None:
                        first = timestamp
                    last = timestamp
                    rows += 1
                    handle.write(
                        ",".join(
                            (
                                format_timestamp(timestamp),
                                format_number(open_price),
                                format_number(high),
                                format_number(low),
                                format_number(close),
                                format_number(volume),
                            )
                        )
                        + "\n"
                    )
        if rows == 0 or first is None or last is None:
            raise BinanceImportError("the archive produced no rows")
        temporary.replace(out)
    except Exception:
        temporary.unlink(missing_ok=True)
        raise
    return ImportStats(
        symbol=symbol,
        rows=rows,
        first_timestamp=first,
        last_timestamp=last,
        sha256=sha256_file(out),
        months=len(months),
        checksums=checksums,
        normalized_unaligned=normalized_unaligned,
    )


def write_manifest(stats: ImportStats, out: Path, manifest: Path, data_version: str) -> None:
    """Write a deterministic import manifest."""
    document = {
        "schema": "quantrule.data-import/v1",
        "source": "binance-spot-monthly-klines",
        "symbol": stats.symbol,
        "timeframe": "1m",
        "range": {
            "from": format_timestamp(stats.first_timestamp),
            "to": format_timestamp(stats.last_timestamp),
        },
        "rows": stats.rows,
        "months": stats.months,
        "normalized_unaligned_rows": stats.normalized_unaligned,
        "sha256": stats.sha256,
        "path": str(out),
        "data_version": data_version,
        "monthly_checksums": stats.checksums,
    }
    manifest.parent.mkdir(parents=True, exist_ok=True)
    manifest.write_text(json.dumps(document, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--symbol", default="BTCUSDT")
    parser.add_argument("--start", required=True, help="first month, YYYY-MM")
    parser.add_argument("--end", required=True, help="last month, YYYY-MM")
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("--cache", type=Path, default=Path("data/scratch/binance"))
    parser.add_argument("--manifest", type=Path)
    parser.add_argument(
        "--data-version",
        default="binance-spot-btcusdt-m1-2018-01-01_2025-12-31-v1",
    )
    args = parser.parse_args(argv[1:])
    try:
        stats = import_range(args.symbol.upper(), args.start, args.end, args.out, args.cache)
    except BinanceImportError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    if args.manifest is not None:
        write_manifest(stats, args.out, args.manifest, args.data_version)
    print(
        f"wrote {args.out}: rows={stats.rows} months={stats.months} "
        f"normalized_unaligned={stats.normalized_unaligned} "
        f"first={format_timestamp(stats.first_timestamp)} "
        f"last={format_timestamp(stats.last_timestamp)} sha256={stats.sha256}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
