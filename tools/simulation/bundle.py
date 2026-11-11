#!/usr/bin/env python3
"""Load a recorded model bundle for a simulation run.

The loader is read-only. It never reruns the model, the strategy, or the
portfolio, and it never reaches a live source. The bundle is the model stage's
published readings, events, levels, and provenance.

Usage:
    from bundle import load_bundle
    bundle = load_bundle(Path("data/simulation/_data/M1.3-2018-2022/report/model"))
"""

from __future__ import annotations

import csv
import json
from dataclasses import dataclass, field, replace
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Iterable

DEFAULT_TIMEFRAME_SECS = 900
REQUIRED_READING_COLUMNS = {
    "timeframe_secs",
    "bar",
    "timestamp",
    "open",
    "high",
    "low",
    "close",
    "volume",
    "atr",
    "ema_fast",
    "ema_mid",
    "ema_slow",
}
M13_READING_COLUMNS = {
    "fib_nearest_atr",
    "band_upper",
    "band_lower",
    "band_position",
}
M13_EVENT_CODES = {149, 150}


class BundleError(Exception):
    """The bundle is missing, malformed, or outside the M1.3 contract."""


def optional_float(value: str | None) -> float | None:
    """Parse an optional numeric field, preserving the missing/null distinction."""
    if value is None or value in ("", "null", "None", "NaN", "nan"):
        return None
    return float(value)


def optional_int(value: str | None) -> int | None:
    """Parse an optional integer field."""
    parsed = optional_float(value)
    return None if parsed is None else int(parsed)


def read_csv(path: Path) -> list[dict[str, str]]:
    """Read a CSV with a header."""
    with path.open(encoding="utf-8", newline="") as handle:
        return list(csv.DictReader(handle))


def read_json(path: Path) -> dict[str, Any]:
    """Read a JSON object."""
    return json.loads(path.read_text(encoding="utf-8"))


def utc_from_seconds(seconds: int) -> str:
    """Format UTC seconds as an ISO timestamp."""
    return datetime.fromtimestamp(seconds, tz=timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def parse_utc(value: str) -> int:
    """Parse an ISO UTC timestamp into seconds."""
    moment = datetime.fromisoformat(value.replace("Z", "+00:00"))
    if moment.tzinfo is None:
        moment = moment.replace(tzinfo=timezone.utc)
    return int(moment.timestamp())


def resolve_model_dir(root: Path) -> Path:
    """Accept either a model bundle or a run directory that contains one."""
    if (root / "readings.csv").is_file():
        return root
    nested = root / "model"
    if (nested / "readings.csv").is_file():
        return nested
    raise BundleError(f"{root} has no readings.csv and no model/readings.csv")


@dataclass(frozen=True)
class Bar:
    """One model reading row for a single bar."""

    bar: int
    timestamp: int
    open: float
    high: float
    low: float
    close: float
    volume: float
    atr: float | None
    ema_fast: float | None
    ema_mid: float | None
    ema_slow: float | None
    rsi: float | None = None
    rvol: float | None = None
    adx: float | None = None
    ema_stack: int | None = None
    rsi_zone: int | None = None
    atr_ratio: float | None = None
    raw: dict[str, str] = field(default_factory=dict, compare=False)

    def value(self, column: str) -> float | None:
        """Return a numeric reading from the raw row."""
        return optional_float(self.raw.get(column))


@dataclass(frozen=True)
class Event:
    """One published event record."""

    event_index: int
    code: int
    type_code: int
    subtype_code: int
    source_code: int
    occurred_bar: int
    available_bar: int
    direction: int
    reference_code: int | None
    primary: float | None
    reference: float | None
    completeness: float | None

    def label(self, catalog: dict[int, str]) -> str:
        """Return the registry name, or a stable fallback."""
        return catalog.get(self.code, f"event.{self.code}")


@dataclass(frozen=True)
class Level:
    """One confirmed level record."""

    reference_code: int
    scale_code: int
    kind_code: int
    price: float
    occurred_bar: int
    available_bar: int
    distance_atr: float | None


@dataclass
class Bundle:
    """One model bundle, filtered to one timeframe and one date range."""

    root: Path
    timeframe_secs: int
    bars: list[Bar]
    events: list[Event]
    levels: list[Level]
    provenance: dict[str, Any]
    bar_index: dict[int, int] = field(init=False)
    events_by_bar: dict[int, list[Event]] = field(init=False)
    levels_by_bar: dict[int, list[Level]] = field(init=False)

    def __post_init__(self) -> None:
        self.bar_index = {bar.bar: index for index, bar in enumerate(self.bars)}
        self.events_by_bar = {}
        for event in self.events:
            self.events_by_bar.setdefault(event.available_bar, []).append(event)
        for rows in self.events_by_bar.values():
            rows.sort(key=lambda event: (event.available_bar, event.event_index))
        self.levels_by_bar = {}
        for level in self.levels:
            self.levels_by_bar.setdefault(level.available_bar, []).append(level)

    @property
    def data_version(self) -> str:
        """The recorded data version, or an explicit unknown marker."""
        return str(self.provenance.get("data_version", "unknown"))

    @property
    def model_version(self) -> str:
        """The recorded model package version, or an explicit unknown marker."""
        return str(self.provenance.get("model_version", "unknown"))

    @property
    def first_timestamp(self) -> int:
        """The first bar timestamp."""
        return self.bars[0].timestamp

    @property
    def last_timestamp(self) -> int:
        """The last bar timestamp."""
        return self.bars[-1].timestamp

    def index_of(self, bar: int) -> int:
        """Return the list index for a bar id, or raise."""
        try:
            return self.bar_index[bar]
        except KeyError as error:
            raise BundleError(f"bar {bar} is not in the bundle") from error

    def bar_at(self, index: int) -> Bar:
        """Return the bar at a list index."""
        if not 0 <= index < len(self.bars):
            raise BundleError(f"bar index {index} is outside 0..{len(self.bars) - 1}")
        return self.bars[index]

    def events_at(self, bar: int) -> list[Event]:
        """Return events available at one bar id."""
        return list(self.events_by_bar.get(bar, []))


def default_model_dir(root: Path) -> Path:
    """Resolve a model directory and make the error message actionable."""
    try:
        return resolve_model_dir(root)
    except BundleError as error:
        raise BundleError(
            f"{error}; pass either the model bundle or the run directory that contains it"
        ) from error


def load_bundle(
    root: Path,
    timeframe_secs: int = DEFAULT_TIMEFRAME_SECS,
    from_ts: int | None = None,
    to_ts: int | None = None,
    required_columns: Iterable[str] | None = None,
) -> Bundle:
    """Load and validate one recorded model bundle.

    Events are filtered by ``available_bar_index``; a bar is included only when
    the event was knowable there. The bundle is sorted by timestamp and bar, so
    the result does not depend on the CSV row order.
    """
    model_dir = default_model_dir(root)
    readings_path = model_dir / "readings.csv"
    events_path = model_dir / "events.csv"
    if not events_path.is_file():
        raise BundleError(f"missing events: {events_path}")
    rows = read_csv(readings_path)
    if not rows:
        raise BundleError(f"{readings_path} is empty")
    required = set(required_columns or REQUIRED_READING_COLUMNS)
    missing = required - set(rows[0])
    if missing:
        raise BundleError(f"{readings_path} is missing columns {sorted(missing)}")

    bars: list[Bar] = []
    for row in rows:
        if int(row["timeframe_secs"]) != timeframe_secs:
            continue
        timestamp = int(row["timestamp"])
        if from_ts is not None and timestamp < from_ts:
            continue
        if to_ts is not None and timestamp > to_ts:
            continue
        bars.append(
            Bar(
                bar=int(row["bar"]),
                timestamp=timestamp,
                open=float(row["open"]),
                high=float(row["high"]),
                low=float(row["low"]),
                close=float(row["close"]),
                volume=float(row["volume"]),
                atr=optional_float(row["atr"]),
                ema_fast=optional_float(row["ema_fast"]),
                ema_mid=optional_float(row["ema_mid"]),
                ema_slow=optional_float(row["ema_slow"]),
                rsi=optional_float(row.get("rsi")),
                rvol=optional_float(row.get("rvol")),
                adx=optional_float(row.get("adx")),
                ema_stack=optional_int(row.get("ema_stack")),
                rsi_zone=optional_int(row.get("rsi_zone")),
                atr_ratio=optional_float(row.get("atr_ratio")),
                raw=row,
            )
        )
    if not bars:
        raise BundleError(f"no {timeframe_secs}s readings in {readings_path}")
    bars.sort(key=lambda bar: (bar.timestamp, bar.bar))
    seen: set[int] = set()
    for bar in bars:
        if bar.bar in seen:
            raise BundleError(f"duplicate bar {bar.bar} in {readings_path}")
        seen.add(bar.bar)

    events: list[Event] = []
    for row in read_csv(events_path):
        if int(row["timeframe_secs"]) != timeframe_secs:
            continue
        available = int(row["available_bar_index"])
        if available not in seen:
            continue
        events.append(
            Event(
                event_index=int(row["event_index"]),
                code=int(row["event_code"]),
                type_code=int(row["event_type_code"]),
                subtype_code=int(row["event_subtype_code"]),
                source_code=int(row["source_code"]),
                occurred_bar=int(row["occurred_bar_index"]),
                available_bar=available,
                direction=int(row["direction_code"]),
                reference_code=optional_int(row.get("reference_code")),
                primary=optional_float(row.get("primary_value")),
                reference=optional_float(row.get("reference_value")),
                completeness=optional_float(row.get("data_completeness")),
            )
        )
    events.sort(key=lambda event: (event.available_bar, event.event_index))

    levels: list[Level] = []
    levels_path = model_dir / "levels.csv"
    if levels_path.is_file():
        for row in read_csv(levels_path):
            if int(row["timeframe_secs"]) != timeframe_secs:
                continue
            available = int(row["available_bar_index"])
            if available not in seen:
                continue
            levels.append(
                Level(
                    reference_code=int(row["reference_code"]),
                    scale_code=int(row["scale_code"]),
                    kind_code=int(row["kind_code"]),
                    price=float(row["price"]),
                    occurred_bar=int(row["occurred_bar_index"]),
                    available_bar=available,
                    distance_atr=optional_float(row.get("distance_atr")),
                )
            )
    levels.sort(key=lambda level: (level.available_bar, level.price, level.occurred_bar))

    provenance: dict[str, Any] = {}
    provenance_path = model_dir / "provenance.json"
    if provenance_path.is_file():
        provenance = read_json(provenance_path)

    return Bundle(
        root=model_dir,
        timeframe_secs=timeframe_secs,
        bars=bars,
        events=events,
        levels=levels,
        provenance=provenance,
    )


def load_bundles(
    roots: list[Path],
    timeframe_secs: int = DEFAULT_TIMEFRAME_SECS,
    from_ts: int | None = None,
    to_ts: int | None = None,
    required_columns: Iterable[str] | None = None,
    data_version: str | None = None,
) -> Bundle:
    """Load and concatenate per-year reports into one ordered bundle.

    The model report is capped at 600,000 source candles, so a five-year 1m
    source is reported per year. The merge keeps the recorded events and levels,
    sorts by timestamp, and rejects an overlap or duplicate bar.
    """
    if not roots:
        raise BundleError("at least one bundle root is required")
    bundles = [
        load_bundle(root, timeframe_secs, None, None, required_columns) for root in roots
    ]
    timeframe = bundles[0].timeframe_secs
    bars: list[Bar] = []
    events: list[Event] = []
    levels: list[Level] = []
    offset = 0
    for bundle in bundles:
        if bundle.timeframe_secs != timeframe:
            raise BundleError("the bundles have different timeframes")
        missing = M13_READING_COLUMNS - set(bundle.bars[0].raw)
        if missing:
            raise BundleError(f"{bundle.root} is missing the M1.3 columns {sorted(missing)}")
        # Each per-year report restarts its bar index at zero. Give the merged
        # series one global bar identity and remap the event and level records
        # before they are compared.
        id_map = {bar.bar: offset + index for index, bar in enumerate(bundle.bars)}
        for bar in bundle.bars:
            bars.append(replace(bar, bar=id_map[bar.bar]))
        for event in bundle.events:
            events.append(
                replace(
                    event,
                    occurred_bar=id_map[event.occurred_bar],
                    available_bar=id_map[event.available_bar],
                )
            )
        for level in bundle.levels:
            levels.append(
                replace(
                    level,
                    occurred_bar=id_map[level.occurred_bar],
                    available_bar=id_map[level.available_bar],
                )
            )
        offset += len(bundle.bars)
    bars.sort(key=lambda bar: (bar.timestamp, bar.bar))
    for previous, current in zip(bars, bars[1:]):
        if previous.timestamp >= current.timestamp:
            raise BundleError(
                f"the bundles overlap or repeat a timestamp at {utc_from_seconds(current.timestamp)}"
            )
    if from_ts is not None or to_ts is not None:
        bars = [
            bar
            for bar in bars
            if (from_ts is None or bar.timestamp >= from_ts)
            and (to_ts is None or bar.timestamp <= to_ts)
        ]
        keep = {bar.bar for bar in bars}
        events = [event for event in events if event.available_bar in keep]
        levels = [level for level in levels if level.available_bar in keep]
    events.sort(key=lambda event: (event.available_bar, event.event_index))
    levels.sort(key=lambda level: (level.available_bar, level.price, level.occurred_bar))
    provenance = dict(bundles[0].provenance)
    provenance["source_reports"] = [str(bundle.root) for bundle in bundles]
    if data_version:
        provenance["data_version"] = data_version
    else:
        provenance["data_version"] = " | ".join(bundle.data_version for bundle in bundles)
    return Bundle(
        root=bundles[0].root,
        timeframe_secs=timeframe,
        bars=bars,
        events=events,
        levels=levels,
        provenance=provenance,
    )


def load_catalog(registry_path: Path) -> dict[int, str]:
    """Load the event-code to registry-name map."""
    registry = read_json(registry_path)
    events = registry.get("events")
    if not isinstance(events, list):
        raise BundleError(f"{registry_path} has no events list")
    catalog: dict[int, str] = {}
    for event in events:
        code = event.get("code")
        name = event.get("id")
        if isinstance(code, int) and isinstance(name, str):
            catalog[code] = name
    if not catalog:
        raise BundleError(f"{registry_path} has no coded events")
    return catalog


def validate_m13(bundle: Bundle) -> None:
    """Require the M1.3 extended readings and Fib/band event codes.

    The check is deliberately structural: the report must publish the extended
    columns and at least one Fib or band event. A report without them is an
    older model and is rejected, never silently treated as M1.3.
    """
    missing = M13_READING_COLUMNS - set(bundle.bars[0].raw)
    if missing:
        raise BundleError(f"the bundle is missing the M1.3 columns {sorted(missing)}")
    codes = {event.code for event in bundle.events}
    if not (M13_EVENT_CODES & codes):
        raise BundleError(
            "the bundle has no level.fib_test or level.band_test event; "
            "it is not an M1.3 report"
        )
