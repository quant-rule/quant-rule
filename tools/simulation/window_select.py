#!/usr/bin/env python3
"""Select deterministic chart windows for a simulation run.

The selector reads a recorded bundle and one or two event codes. It builds a
window around each candidate bar, keeps the event's available bar inside the
candidate zone, removes heavily overlapping windows, and takes a seeded,
year-stratified sample. The same bundle, events, and seed produce the same
``windows.json``.

Usage:
    from window_select import WindowSpec, select_windows
    windows = select_windows(bundle, WindowSpec(event_codes=(149,), seed=0))
"""

from __future__ import annotations

import json
import random
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from bundle import Bundle, BundleError, Event, utc_from_seconds


class SelectError(Exception):
    """The window selection cannot be built from the bundle."""


@dataclass(frozen=True)
class WindowSpec:
    """The pre-registered selection parameters for one run."""

    event_codes: tuple[int, ...]
    window_bars: int = 400
    candidate_zone: tuple[int, int] = (80, 120)
    max_windows: int = 48
    min_windows: int = 30
    seed: int = 0
    overlap_bars: int = 40
    from_ts: int | None = None
    to_ts: int | None = None

    def validate(self) -> None:
        """Reject a spec that cannot produce a useful window."""
        if not self.event_codes:
            raise SelectError("event_codes must not be empty")
        if self.window_bars < 20:
            raise SelectError("window_bars must be at least 20")
        if not 0 <= self.candidate_zone[0] < self.candidate_zone[1] < self.window_bars:
            raise SelectError("candidate_zone must be strictly inside the window")
        if self.max_windows < 1:
            raise SelectError("max_windows must be positive")
        if self.overlap_bars < 0 or self.overlap_bars >= self.window_bars:
            raise SelectError("overlap_bars must be smaller than window_bars")


@dataclass(frozen=True)
class Window:
    """One chart window around one candidate bar."""

    window_id: str
    start_index: int
    end_index: int
    candidate_index: int
    events: tuple[Event, ...]

    @property
    def candidate_bar(self) -> int:
        """The candidate bar id."""
        return self.events[0].available_bar

    @property
    def start_bar(self) -> int:
        """The first bar id in the window."""
        return self.events[0].available_bar - (self.candidate_index - self.start_index)

    def as_record(self, bundle: Bundle, catalog: dict[int, str]) -> dict[str, Any]:
        """Return the JSON record for ``windows.json``."""
        first = bundle.bars[self.start_index]
        last = bundle.bars[self.end_index - 1]
        return {
            "window_id": self.window_id,
            "start_index": self.start_index,
            "end_index": self.end_index,
            "start_bar": first.bar,
            "end_bar": last.bar,
            "start_time": utc_from_seconds(first.timestamp),
            "end_time": utc_from_seconds(last.timestamp),
            "candidate_index": self.candidate_index,
            "candidate_bar": self.candidate_bar,
            "candidate_time": utc_from_seconds(bundle.bars[self.candidate_index].timestamp),
            "events": [
                {
                    "event_index": event.event_index,
                    "code": event.code,
                    "name": event.label(catalog),
                    "type_code": event.type_code,
                    "subtype_code": event.subtype_code,
                    "source_code": event.source_code,
                    "occurred_bar": event.occurred_bar,
                    "available_bar": event.available_bar,
                    "direction": event.direction,
                    "primary": event.primary,
                    "reference": event.reference,
                }
                for event in self.events
            ],
        }


def _candidate_event_groups(
    bundle: Bundle, spec: WindowSpec
) -> list[tuple[int, tuple[Event, ...]]]:
    """Return candidate list indices and the events available at each bar."""
    wanted = set(spec.event_codes)
    by_bar: dict[int, list[Event]] = {}
    for event in bundle.events:
        if event.code not in wanted:
            continue
        if spec.from_ts is not None and bundle.bars[bundle.index_of(event.available_bar)].timestamp < spec.from_ts:
            continue
        if spec.to_ts is not None and bundle.bars[bundle.index_of(event.available_bar)].timestamp > spec.to_ts:
            continue
        by_bar.setdefault(event.available_bar, []).append(event)
    groups: list[tuple[int, tuple[Event, ...]]] = []
    for bar, events in by_bar.items():
        index = bundle.index_of(bar)
        events.sort(key=lambda event: event.event_index)
        groups.append((index, tuple(events)))
    groups.sort(key=lambda item: (bundle.bars[item[0]].timestamp, item[1][0].event_index))
    return groups


def _window_around(bundle: Bundle, candidate_index: int, spec: WindowSpec) -> tuple[int, int] | None:
    """Fit the window around the candidate and keep the candidate in its zone."""
    start = candidate_index - spec.candidate_zone[0]
    if start < 0:
        start = 0
    end = start + spec.window_bars
    if end > len(bundle.bars):
        end = len(bundle.bars)
        start = end - spec.window_bars
    if start < 0:
        return None
    position = candidate_index - start
    if not spec.candidate_zone[0] <= position < spec.candidate_zone[1]:
        return None
    return start, end


def _stratified_sample(
    candidates: list[tuple[int, tuple[Event, ...]]],
    bundle: Bundle,
    spec: WindowSpec,
) -> list[tuple[int, tuple[Event, ...]]]:
    """Sample candidates by year, then fill, deterministically from the seed."""
    by_year: dict[int, list[tuple[int, tuple[Event, ...]]]] = {}
    for candidate in candidates:
        year = datetime.fromtimestamp(
            bundle.bars[candidate[0]].timestamp, tz=timezone.utc
        ).year
        by_year.setdefault(year, []).append(candidate)
    years = sorted(by_year)
    if not years:
        return []
    per_year = max(1, spec.max_windows // len(years))
    selected: list[tuple[int, tuple[Event, ...]]] = []
    remaining: list[tuple[int, tuple[Event, ...]]] = []
    for year in years:
        pool = list(by_year[year])
        rng = random.Random(spec.seed + year)
        rng.shuffle(pool)
        take = min(per_year, len(pool))
        selected.extend(pool[:take])
        remaining.extend(pool[take:])
    if len(selected) < spec.max_windows:
        remaining.sort(key=lambda item: (bundle.bars[item[0]].timestamp, item[1][0].event_index))
        selected.extend(remaining[: spec.max_windows - len(selected)])
    selected.sort(key=lambda item: (bundle.bars[item[0]].timestamp, item[1][0].event_index))
    return selected


def select_windows(bundle: Bundle, spec: WindowSpec, catalog: dict[int, str] | None = None) -> dict[str, Any]:
    """Select windows and return the JSON-ready record."""
    spec.validate()
    catalog = catalog or {}
    groups = _candidate_event_groups(bundle, spec)
    candidates: list[tuple[int, tuple[Event, ...]]] = []
    last_end = -1
    for candidate_index, events in groups:
        bounds = _window_around(bundle, candidate_index, spec)
        if bounds is None:
            continue
        start, end = bounds
        if start < last_end - spec.overlap_bars:
            continue
        candidates.append((candidate_index, events))
        last_end = end
    selected = _stratified_sample(candidates, bundle, spec)
    windows: list[Window] = []
    for number, (candidate_index, events) in enumerate(selected, 1):
        bounds = _window_around(bundle, candidate_index, spec)
        if bounds is None:
            continue
        start, end = bounds
        windows.append(
            Window(
                window_id=f"w{number:03d}",
                start_index=start,
                end_index=end,
                candidate_index=candidate_index,
                events=events,
            )
        )
    first = bundle.bars[0]
    last = bundle.bars[-1]
    return {
        "schema": "quantrule.simulation-windows/v1",
        "timeframe_secs": bundle.timeframe_secs,
        "window_bars": spec.window_bars,
        "candidate_zone": list(spec.candidate_zone),
        "max_windows": spec.max_windows,
        "min_windows": spec.min_windows,
        "seed": spec.seed,
        "overlap_bars": spec.overlap_bars,
        "event_codes": list(spec.event_codes),
        "event_names": [catalog.get(code, f"event.{code}") for code in spec.event_codes],
        "from": utc_from_seconds(spec.from_ts) if spec.from_ts is not None else utc_from_seconds(first.timestamp),
        "to": utc_from_seconds(spec.to_ts) if spec.to_ts is not None else utc_from_seconds(last.timestamp),
        "candidate_count": len(candidates),
        "selected_count": len(windows),
        "underpowered": len(windows) < spec.min_windows,
        "windows": [window.as_record(bundle, catalog) for window in windows],
    }


def write_windows(path: Path, record: dict[str, Any]) -> None:
    """Write ``windows.json`` atomically with a stable key order."""
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = json.dumps(record, indent=2, sort_keys=True) + "\n"
    temporary = path.with_name(path.name + ".tmp")
    temporary.write_text(payload, encoding="utf-8")
    temporary.replace(path)


def read_windows(path: Path) -> dict[str, Any]:
    """Read ``windows.json``."""
    return json.loads(path.read_text(encoding="utf-8"))
