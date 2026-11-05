#!/usr/bin/env python3
"""Check a model spec and its data configuration before a run.

The preflight reads the embedded registry contract, the spec document, the run
configuration, and the CSV source metadata. It does not run the model. It
answers three questions:

- Does every event identifier resolve to an active registry event with a role,
  a claim, and a formula?
- Does the spec ladder come from the data configuration's available timeframes?
- Does the source cover the run period and every ladder timeframe?

Usage:
    python3 tools/research/spec_preflight.py \
        --config config/run.json \
        --spec config/spec.json
"""

from __future__ import annotations

import argparse
import csv
import json
import re
import sys
from dataclasses import dataclass
from datetime import date, datetime
from pathlib import Path
from typing import Any

SCHEMA = "quantrule.model-spec/v1"
ROLES = {"signal", "context", "diagnostic"}
STATE_BOUNDS = {
    "ema_stack": (-1, 1),
    "rsi_zone": (-1, 1),
    "macd_state": (-1, 1),
    "macd_histogram_state": (-1, 1),
    "adx_regime": (0, 1),
    "adx_slope": (-1, 1),
    "di_state": (-1, 1),
}
TIMEFRAME_SECONDS = {"1m": 60, "5m": 300, "15m": 900, "1h": 3600, "4h": 14400}
MAX_DEPTH = 16
MAX_NODES = 256
MAX_CONDITIONS = 64
MAX_WITHIN_BARS = 10_000
MAX_LADDER = 8
CSV_COLUMNS = {"date", "open", "high", "low", "close", "volume"}


class PreflightError(ValueError):
    """A spec or its data configuration is not admissible."""


@dataclass(frozen=True)
class Preflight:
    spec_id: str
    registry_version: str
    ladder: tuple[int, ...]
    evaluation_timeframe: int
    condition_count: int
    event_count: int
    source: Path
    bars: int
    data_timeframes: tuple[int, ...]


def read_json(path: Path, label: str) -> dict[str, Any]:
    try:
        document = json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError:
        raise PreflightError(f"{label} not found: {path}") from None
    except json.JSONDecodeError as error:
        raise PreflightError(f"invalid {label} JSON: {error}") from None
    if not isinstance(document, dict):
        raise PreflightError(f"{label} must be an object")
    return document


def number(value: Any, label: str, low: int, high: int | None = None) -> int:
    if not isinstance(value, int) or isinstance(value, bool):
        raise PreflightError(f"{label} must be an integer")
    if value < low or (high is not None and value > high):
        range_label = f"{low}..{high}" if high is not None else f">= {low}"
        raise PreflightError(f"{label} is outside {range_label}")
    return value


def expression(
    value: Any,
    ladder: tuple[int, ...],
    registry: dict[str, dict[str, Any]],
    event_ids: set[str],
) -> tuple[int, int]:
    if not isinstance(value, dict) or len(value) != 1:
        raise PreflightError("an expression must have exactly one key")
    key, operand = next(iter(value.items()))
    if key in ("all", "any"):
        if not isinstance(operand, list) or not operand:
            raise PreflightError(f"{key} must be a non-empty array")
        nodes = 1
        depth = 1
        for child in operand:
            child_nodes, child_depth = expression(child, ladder, registry, event_ids)
            nodes += child_nodes
            depth = max(depth, child_depth + 1)
        if depth > MAX_DEPTH:
            raise PreflightError("the expression is too deep")
        if nodes > MAX_NODES:
            raise PreflightError("the expression is too large")
        return nodes, depth
    if key == "not":
        nodes, depth = expression(operand, ladder, registry, event_ids)
        if depth + 1 > MAX_DEPTH:
            raise PreflightError("the expression is too deep")
        if nodes + 1 > MAX_NODES:
            raise PreflightError("the expression is too large")
        return nodes + 1, depth + 1
    if key == "event":
        event = operand
        if not isinstance(event, dict):
            raise PreflightError("an event expression must be an object")
        event_id = event.get("id")
        if not isinstance(event_id, str) or not event_id:
            raise PreflightError("an event expression needs an id")
        declared = registry.get(event_id)
        if declared is None:
            raise PreflightError(f"the event id {event_id} is not in the registry")
        event_ids.add(event_id)
        if declared.get("status") != "active":
            raise PreflightError(f"the event {event_id} is not active")
        if declared.get("role") not in ROLES:
            raise PreflightError(f"the event {event_id} has an unknown role")
        for field in ("claim", "formula"):
            if not isinstance(declared.get(field), str) or not declared[field]:
                raise PreflightError(f"the event {event_id} has no {field}")
        timeframe = number(event.get("timeframe"), f"{event_id}.timeframe", 1)
        if timeframe not in ladder:
            raise PreflightError(f"the event {event_id} timeframe {timeframe} is not in the ladder")
        if "direction" in event:
            number(event["direction"], f"{event_id}.direction", -1)
            if event["direction"] not in (-1, 0, 1):
                raise PreflightError(f"the event {event_id} direction is outside -1..=1")
        if "within_bars" in event:
            number(event["within_bars"], f"{event_id}.within_bars", 0, MAX_WITHIN_BARS)
        return 1, 1
    if key == "state":
        state = operand
        if not isinstance(state, dict):
            raise PreflightError("a state expression must be an object")
        name = state.get("state")
        if name not in STATE_BOUNDS:
            raise PreflightError(f"the state {name} is not known")
        timeframe = number(state.get("timeframe"), f"{name}.timeframe", 1)
        if timeframe not in ladder:
            raise PreflightError(f"the state {name} timeframe {timeframe} is not in the ladder")
        low, high = STATE_BOUNDS[name]
        number(state.get("equals"), f"{name}.equals", low, high)
        return 1, 1
    raise PreflightError(f"the expression key {key} is not known")


def registry(repo_root: Path) -> tuple[dict[str, dict[str, Any]], str]:
    document = read_json(repo_root / "docs/registry/indicators.json", "registry")
    version = document.get("version")
    if not isinstance(version, str) or not version:
        raise PreflightError("the registry has no version")
    events = document.get("events")
    if not isinstance(events, list):
        raise PreflightError("the registry has no events")
    by_id = {event["id"]: event for event in events if isinstance(event, dict) and "id" in event}
    return by_id, version


def model_registry_version(repo_root: Path) -> str:
    source = (repo_root / "crates/quantrule-model/src/lib.rs").read_text(encoding="utf-8")
    match = re.search(r'REGISTRY_VERSION: &str = "([^"]+)"', source)
    if match is None:
        raise PreflightError("cannot find REGISTRY_VERSION in the model crate")
    return match.group(1)


def data_timeframes(data: dict[str, Any]) -> tuple[int, ...]:
    codes = data.get("available_timeframes")
    if not isinstance(codes, list) or not codes:
        raise PreflightError("the data config has no available_timeframes")
    seconds = []
    for code in codes:
        if code not in TIMEFRAME_SECONDS:
            raise PreflightError(f"the data timeframe {code} is not a known code")
        seconds.append(TIMEFRAME_SECONDS[code])
    return tuple(seconds)


def source_metadata(
    source: Path, period_from: date | None, period_to: date | None, ladder: tuple[int, ...]
) -> tuple[int, tuple[int, ...]]:
    try:
        handle = source.open("r", encoding="utf-8", newline="")
    except OSError as error:
        raise PreflightError(f"cannot read the source {source}: {error}") from None
    with handle:
        reader = csv.DictReader(handle)
        if reader.fieldnames is None:
            raise PreflightError("the source has no header")
        columns = {name.strip().lower() for name in reader.fieldnames}
        if not CSV_COLUMNS <= columns:
            raise PreflightError("the source header is not Date,Open,High,Low,Close,Volume")
        bars = 0
        buckets: dict[int, set[int]] = {timeframe: set() for timeframe in ladder}
        previous: datetime | None = None
        for row in reader:
            raw = row.get("Date") or row.get("date")
            if raw is None:
                raise PreflightError("the source row has no Date")
            try:
                timestamp = datetime.fromisoformat(raw)
            except ValueError:
                raise PreflightError(f"the source date {raw!r} is not ISO 8601") from None
            if previous is not None and timestamp <= previous:
                raise PreflightError("the source timestamps are not strictly ascending")
            previous = timestamp
            if period_from is not None and timestamp.date() < period_from:
                continue
            if period_to is not None and timestamp.date() > period_to:
                continue
            bars += 1
            epoch = int(timestamp.timestamp())
            for timeframe in ladder:
                buckets[timeframe].add(epoch // timeframe)
    empty = [timeframe for timeframe, values in buckets.items() if not values]
    if empty:
        raise PreflightError(f"the source covers no bar for timeframe(s) {empty}")
    return bars, tuple(len(buckets[timeframe]) for timeframe in ladder)


def preflight(config_path: Path, spec_path: Path, repo_root: Path | None = None) -> Preflight:
    root = repo_root or Path(__file__).resolve().parents[2]
    run = read_json(config_path, "run config")
    data_ref = run.get("data")
    if not isinstance(data_ref, str) or not data_ref:
        raise PreflightError("the run config has no data path")
    data_path = (config_path.parent / data_ref).resolve()
    data = read_json(data_path, "data config")
    schema = data.get("schema")
    if not isinstance(schema, str) or not schema.startswith("quantrule.data/"):
        raise PreflightError("the data schema is not recognized")
    if not isinstance(data.get("data_version"), str) or not data["data_version"]:
        raise PreflightError("the data version is empty")
    number(data.get("max_candles", 1), "max_candles", 1)

    configured = data_timeframes(data)
    registry_events, registry_version = registry(root)
    model_version = model_registry_version(root)
    if model_version != registry_version:
        raise PreflightError(
            f"the model registry version {model_version} does not match the registry {registry_version}"
        )
    spec = read_json(spec_path, "spec")
    if spec.get("schema") != SCHEMA:
        raise PreflightError(f"the spec schema is not {SCHEMA}")
    spec_id = spec.get("id")
    if not isinstance(spec_id, str) or not spec_id:
        raise PreflightError("the spec id is empty")
    raw_ladder = spec.get("ladder")
    if not isinstance(raw_ladder, list) or not raw_ladder or len(raw_ladder) > MAX_LADDER:
        raise PreflightError(f"the ladder must have 1..{MAX_LADDER} entries")
    ladder_values = [number(value, "a ladder entry", 1) for value in raw_ladder]
    if any(later <= earlier for earlier, later in zip(ladder_values, ladder_values[1:])):
        raise PreflightError("the ladder is not strictly ascending")
    ladder = tuple(ladder_values)
    evaluation = number(
        spec.get("evaluation_timeframe", ladder[0]), "evaluation_timeframe", 1
    )
    if evaluation not in ladder:
        raise PreflightError("the evaluation timeframe is not in the ladder")

    conditions = spec.get("conditions")
    if not isinstance(conditions, list) or not conditions:
        raise PreflightError("the spec has no conditions")
    if len(conditions) > MAX_CONDITIONS:
        raise PreflightError(f"the spec has more than {MAX_CONDITIONS} conditions")
    identifiers = set()
    event_ids = set()
    for condition in conditions:
        if not isinstance(condition, dict):
            raise PreflightError("a condition must be an object")
        identifier = condition.get("id")
        if not isinstance(identifier, str) or not identifier:
            raise PreflightError("a condition id is empty")
        if identifier in identifiers:
            raise PreflightError(f"the condition id {identifier} is duplicated")
        identifiers.add(identifier)
        nodes, depth = expression(
            condition.get("when"), ladder, registry_events, event_ids
        )
        if nodes > MAX_NODES or depth > MAX_DEPTH:
            raise PreflightError("a condition expression is too large or too deep")

    missing = [timeframe for timeframe in ladder if timeframe not in configured]
    if missing:
        raise PreflightError(f"the spec timeframes {missing} are not in the data configuration")

    source_ref = data.get("source", {}).get("path")
    if not isinstance(source_ref, str) or not source_ref:
        raise PreflightError("the data config has no source path")
    source = (data_path.parent / source_ref).resolve()
    period_from = None
    period_to = None
    if isinstance(data.get("from"), str):
        try:
            period_from = date.fromisoformat(data["from"])
        except ValueError:
            raise PreflightError("the data from date is not ISO 8601") from None
    if isinstance(data.get("to"), str):
        try:
            period_to = date.fromisoformat(data["to"])
        except ValueError:
            raise PreflightError("the data to date is not ISO 8601") from None
    if period_from is not None and period_to is not None and period_from > period_to:
        raise PreflightError("the data from date is after the to date")
    bars, _ = source_metadata(source, period_from, period_to, ladder)
    if bars == 0:
        raise PreflightError("the source has no bar in the requested period")

    return Preflight(
        spec_id=spec_id,
        registry_version=registry_version,
        ladder=ladder,
        evaluation_timeframe=evaluation,
        condition_count=len(conditions),
        event_count=len(event_ids),
        source=source,
        bars=bars,
        data_timeframes=configured,
    )


def render(result: Preflight) -> str:
    return "\n".join(
        [
            "spec_preflight: ok",
            f"spec: {result.spec_id}",
            f"schema: {SCHEMA}",
            f"registry_version: {result.registry_version}",
            f"ladder: {','.join(str(value) for value in result.ladder)}",
            f"evaluation_timeframe: {result.evaluation_timeframe}",
            f"conditions: {result.condition_count}",
            f"events: {result.event_count}",
            f"source: {result.source}",
            f"bars: {result.bars}",
            f"data_timeframes: {','.join(str(value) for value in result.data_timeframes)}",
        ]
    )


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--config", required=True, type=Path)
    parser.add_argument("--spec", required=True, type=Path)
    args = parser.parse_args(argv)
    try:
        result = preflight(args.config.resolve(), args.spec.resolve())
    except PreflightError as error:
        print(f"spec_preflight: error: {error}", file=sys.stderr)
        return 1
    print(render(result))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
