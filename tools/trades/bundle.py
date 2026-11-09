#!/usr/bin/env python3
"""Read a backtest bundle for trade-context exports.

The module is read-only over the recorded bundle. It never reruns a model,
strategy, or portfolio stage and never reads a live source.
"""

from __future__ import annotations

import csv
import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable

DEFAULT_TIMEFRAME_SECS = 900

STATE_LABELS = {
    "ema_stack": {-1: "bear", 0: "mixed", 1: "bull"},
    "structure_state": {-1: "lower_highs_lows", 0: "mixed", 1: "higher_highs_lows"},
    "vol_regime": {0: "compressed", 1: "normal", 2: "expanded"},
    "rsi_zone": {-1: "oversold", 0: "middle", 1: "overbought"},
    "macd_state": {-1: "below_signal", 0: "equal", 1: "above_signal"},
    "adx_regime": {0: "weak", 1: "strong"},
    "rvol_regime": {-1: "low", 0: "normal", 1: "high"},
}


class BundleError(Exception):
    """The bundle is missing or malformed."""


def read_csv(path: Path) -> list[dict[str, str]]:
    """Read a CSV with a header."""
    with path.open(encoding="utf-8", newline="") as handle:
        return list(csv.DictReader(handle))


def read_ndjson(path: Path) -> list[dict[str, Any]]:
    """Read an NDJSON file, skipping blank lines."""
    records = []
    with path.open(encoding="utf-8") as handle:
        for line in handle:
            if line.strip():
                records.append(json.loads(line))
    return records


def read_json(path: Path) -> dict[str, Any]:
    """Read a JSON object."""
    return json.loads(path.read_text(encoding="utf-8"))


def number_or_none(value: str | None) -> float | None:
    """Parse a numeric field, preserving the missing/null distinction."""
    if value is None or value == "" or value == "null":
        return None
    return float(value)


@dataclass
class Bundle:
    """One recorded backtest bundle."""

    root: Path
    timeframe_secs: int
    readings: list[dict[str, str]]
    events: list[dict[str, str]]
    decisions: dict[int, dict[str, Any]]
    orders: list[dict[str, str]]
    fills: list[dict[str, str]]
    trades: list[dict[str, str]]
    manifest: dict[str, Any]

    def trade(self, trade_id: int) -> dict[str, str]:
        """Return one trade by id."""
        for trade in self.trades:
            if int(trade["trade_id"]) == trade_id:
                return trade
        raise BundleError(f"trade {trade_id} is not in {self.root}")

    def bars_ending_at(self, end_bar: int, limit: int) -> list[dict[str, str]]:
        """Return up to ``limit`` bars with bar <= end_bar."""
        rows = [row for row in self.readings if int(row["bar"]) <= end_bar]
        return rows[-limit:]

    def bars_starting_at(self, start_bar: int, limit: int) -> list[dict[str, str]]:
        """Return up to ``limit`` bars with bar >= start_bar."""
        rows = [row for row in self.readings if int(row["bar"]) >= start_bar]
        return rows[:limit]

    def events_between(self, start_bar: int, end_bar: int) -> list[dict[str, str]]:
        """Return events available in a bar range."""
        return [
            event
            for event in self.events
            if start_bar <= int(event["available_bar_index"]) <= end_bar
        ]

    def events_at(self, bar: int) -> list[dict[str, str]]:
        """Return events available at one bar."""
        return [event for event in self.events if int(event["available_bar_index"]) == bar]

    def decision_at(self, bar: int) -> dict[str, Any] | None:
        """Return the strategy decision at one bar."""
        return self.decisions.get(bar)

    def trade_orders(self, trade: dict[str, str]) -> list[dict[str, str]]:
        """Return orders submitted from before the entry to the exit."""
        entry = int(trade["entry_bar"])
        exit_bar = int(trade["exit_bar"])
        return [
            order
            for order in self.orders
            if entry - 1 <= int(order["submitted_bar"]) <= exit_bar
        ]

    def trade_fills(self, trade: dict[str, str]) -> list[dict[str, str]]:
        """Return fills at the entry or exit bar, or linked by order fill id."""
        entry = int(trade["entry_bar"])
        exit_bar = int(trade["exit_bar"])
        fill_ids = {
            order["fill_id"]
            for order in self.trade_orders(trade)
            if order.get("fill_id") not in (None, "")
        }
        return [
            fill
            for fill in self.fills
            if int(fill["bar"]) in {entry, exit_bar} or fill.get("fill_id") in fill_ids
        ]


def load_bundle(root: Path, timeframe_secs: int = DEFAULT_TIMEFRAME_SECS) -> Bundle:
    """Load and validate one backtest bundle."""
    relevant = {
        "readings": root / "model" / "readings.csv",
        "events": root / "model" / "events.csv",
        "decisions": root / "strategy" / "decisions.ndjson",
        "orders": root / "execution" / "orders.csv",
        "fills": root / "execution" / "fills.csv",
        "trades": root / "portfolio" / "trades.csv",
    }
    missing = [name for name, path in relevant.items() if not path.is_file()]
    if missing:
        raise BundleError(f"{root} is missing {', '.join(missing)}")
    readings = [
        row
        for row in read_csv(relevant["readings"])
        if int(row["timeframe_secs"]) == timeframe_secs
    ]
    readings.sort(key=lambda row: int(row["bar"]))
    if not readings:
        raise BundleError(f"{root} has no {timeframe_secs}-second readings")
    events = [
        row
        for row in read_csv(relevant["events"])
        if int(row["timeframe_secs"]) == timeframe_secs
    ]
    decisions = {
        int(record["bar_index"]): record for record in read_ndjson(relevant["decisions"])
    }
    manifest_path = root / "manifest.json"
    manifest = read_json(manifest_path) if manifest_path.is_file() else {}
    return Bundle(
        root=root,
        timeframe_secs=timeframe_secs,
        readings=readings,
        events=events,
        decisions=decisions,
        orders=read_csv(relevant["orders"]),
        fills=read_csv(relevant["fills"]),
        trades=read_csv(relevant["trades"]),
        manifest=manifest,
    )


def event_catalog(path: Path) -> dict[int, dict[str, Any]]:
    """Load the event-code registry, when present."""
    if not path.is_file():
        return {}
    registry = read_json(path)
    return {int(event["code"]): event for event in registry.get("events", [])}


def event_label(code: int, catalog: dict[int, dict[str, Any]]) -> str:
    """Return the registry id for an event code, or the code itself."""
    event = catalog.get(code)
    return event["id"] if event else str(code)


def state_labels(row: dict[str, str]) -> dict[str, str | None]:
    """Return the readable label for every published discrete state."""
    labels: dict[str, str | None] = {}
    for field, mapping in STATE_LABELS.items():
        value = row.get(field)
        if value in (None, ""):
            labels[field] = None
        else:
            labels[field] = mapping.get(int(float(value)), "unknown")
    return labels


def reading_summary(row: dict[str, str]) -> dict[str, Any]:
    """Convert one readings row into a JSON-safe summary."""
    summary: dict[str, Any] = {}
    for field, value in row.items():
        if field in {"timeframe_secs", "bar", "timestamp"}:
            summary[field] = int(value) if value != "" else None
        else:
            try:
                summary[field] = number_or_none(value)
            except ValueError:
                summary[field] = value
    summary["state_labels"] = state_labels(row)
    return summary


def write_csv(path: Path, rows: Iterable[dict[str, str]]) -> None:
    """Write rows with a deterministic header order."""
    rows = list(rows)
    path.parent.mkdir(parents=True, exist_ok=True)
    if not rows:
        path.write_text("", encoding="utf-8")
        return
    fields = list(rows[0].keys())
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=fields)
        writer.writeheader()
        writer.writerows(rows)


def write_json(path: Path, value: Any) -> None:
    """Write deterministic, pretty JSON."""
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def timestamp_utc(seconds: int) -> str:
    """Format UTC seconds in the source timestamp shape."""
    from datetime import datetime, timezone

    return datetime.fromtimestamp(seconds, tz=timezone.utc).strftime(
        "%Y-%m-%d %H:%M:%S+00:00"
    )
