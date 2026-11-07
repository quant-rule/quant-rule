#!/usr/bin/env python3
"""Prepare a per-bar condition from published model artifacts.

The research runner evaluates predictions with the existing condition gate. This
module does not change the model and does not compute an indicator from raw
candles. It reads the model's published readings, events, and levels, applies a
pre-registered rule, and writes a conditioned readings bundle plus a gate:

- `sequence`: event A, then event B within N bars, optionally at the same level,
  with an optional direction and close-side rule;
- `confluence`: at least K independent published references within X ATR;
- `column`: a published reading bounded by values or a min/max range, including
  `level_age` and `level_touches` freshness conditions.

The output is fed to `model_prediction.py` as a state gate on the `condition`
column. The condition is evaluation-only; the Rust model never reads it.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
from pathlib import Path
from typing import Any

SCHEMA = "quantrule.model-condition/v1"
GATE_SCHEMA = "quantrule.model-state-gate/v1"
CONDITION_COLUMN = "condition"
REGISTRY_EVENTS = "events"

SEQUENCE_FIELDS = {
    "kind",
    "description",
    "timeframe_secs",
    "first_event",
    "second_event",
    "within_bars",
    "same_level",
    "level_tolerance_atr",
    "direction",
    "close_side",
}
CONFLUENCE_FIELDS = {
    "kind",
    "description",
    "timeframe_secs",
    "references",
    "min_count",
    "within_atr",
}
COLUMN_FIELDS = {
    "kind",
    "description",
    "timeframe_secs",
    "column",
    "values",
    "min",
    "max",
    "level_age_max",
    "level_touches_min",
}
DIRECTIONS = {"any", "same", "opposite"}
CLOSE_SIDES = {"any", "breakout", "original"}
PIVOT_ALIASES = {
    "pivot_high": "last_pivot_high",
    "pivot_low": "last_pivot_low",
}


def parse_arguments(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bundle", required=True, help="merged model bundle")
    parser.add_argument("--condition", required=True, help="condition JSON")
    parser.add_argument("--registry", default="docs/registry/indicators.json")
    parser.add_argument("--out", required=True)
    parser.add_argument("--events", default=None)
    parser.add_argument("--levels", default=None)
    return parser.parse_args(argv)


def read_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise SystemExit(f"error: cannot read {path}: {error}") from None


def write_json(path: Path, document: Any) -> None:
    path.write_text(
        json.dumps(document, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )


def file_digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read_csv(path: Path) -> tuple[list[str], list[list[str]]]:
    try:
        with path.open(newline="", encoding="utf-8") as handle:
            reader = csv.reader(handle)
            header = next(reader, None)
            rows = list(reader)
    except OSError as error:
        raise SystemExit(f"error: cannot read {path}: {error}") from None
    if header is None:
        raise SystemExit(f"error: {path} has no header")
    return header, rows


def write_csv(path: Path, header: list[str], rows: list[list[str]]) -> None:
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle)
        writer.writerow(header)
        writer.writerows(rows)


def load_event_codes(registry_path: Path) -> dict[str, int]:
    registry = read_json(registry_path)
    events = registry.get(REGISTRY_EVENTS)
    if not isinstance(events, list):
        raise SystemExit(f"error: {registry_path} has no events array")
    codes: dict[str, int] = {}
    for event in events:
        if not isinstance(event, dict):
            continue
        event_id = event.get("id")
        code = event.get("code")
        if isinstance(event_id, str) and isinstance(code, int) and not isinstance(code, bool):
            codes[event_id] = code
    return codes


def optional_number(value: Any) -> float | None:
    try:
        number = float(value)
    except (TypeError, ValueError):
        return None
    return number if number == number else None


def optional_int(value: Any) -> int | None:
    number = optional_number(value)
    if number is None or not number.is_integer():
        return None
    return int(number)


def load_condition(path: Path) -> dict[str, Any]:
    document = read_json(path)
    if not isinstance(document, dict):
        raise SystemExit(f"error: {path} must contain an object")
    if document.get("schema") != SCHEMA:
        raise SystemExit(f"error: {path} schema must be {SCHEMA}")
    kind = document.get("kind")
    common = {"schema", "kind", "description"}
    if kind == "sequence":
        allowed = common | SEQUENCE_FIELDS
    elif kind == "confluence":
        allowed = common | CONFLUENCE_FIELDS
    elif kind == "column":
        allowed = common | COLUMN_FIELDS
    else:
        raise SystemExit(f"error: {path} has unknown kind {kind!r}")
    unknown = sorted(set(document) - allowed)
    if unknown:
        raise SystemExit(f"error: {path} has unknown fields: {unknown}")
    if kind == "sequence":
        for field in ("first_event", "second_event"):
            value = document.get(field)
            if not isinstance(value, str) or not value:
                raise SystemExit(f"error: {path} {field} must be a non-empty event id")
        within = document.get("within_bars")
        if not isinstance(within, int) or isinstance(within, bool) or within < 0:
            raise SystemExit(f"error: {path} within_bars must be a non-negative integer")
        direction = document.get("direction", "any")
        if direction not in DIRECTIONS:
            raise SystemExit(f"error: {path} direction must be one of {sorted(DIRECTIONS)}")
        close_side = document.get("close_side", "any")
        if close_side not in CLOSE_SIDES:
            raise SystemExit(f"error: {path} close_side must be one of {sorted(CLOSE_SIDES)}")
    elif kind == "confluence":
        references = document.get("references")
        if (
            not isinstance(references, list)
            or not references
            or not all(isinstance(value, str) and value for value in references)
        ):
            raise SystemExit(f"error: {path} references must be a non-empty string list")
        min_count = document.get("min_count")
        if not isinstance(min_count, int) or isinstance(min_count, bool) or min_count < 1:
            raise SystemExit(f"error: {path} min_count must be a positive integer")
        within_atr = optional_number(document.get("within_atr"))
        if within_atr is None or within_atr <= 0:
            raise SystemExit(f"error: {path} within_atr must be positive")
    else:
        column = document.get("column")
        if not isinstance(column, str) or not column:
            raise SystemExit(f"error: {path} column must be a non-empty string")
        if not any(
            document.get(key) is not None
            for key in ("values", "min", "max", "level_age_max", "level_touches_min")
        ):
            raise SystemExit(f"error: {path} needs values, min, max, level_age_max, or level_touches_min")
    return document


def event_rows(
    path: Path,
    code_by_id: dict[str, int],
    event_id: str,
    timeframe: int,
) -> dict[int, list[dict[str, Any]]]:
    code = code_by_id.get(event_id)
    if code is None:
        raise SystemExit(f"error: event {event_id!r} is not registered")
    header, rows = read_csv(path)
    try:
        code_index = header.index("event_code")
        available_index = header.index("available_bar_index")
        occurred_index = header.index("occurred_bar_index")
        direction_index = header.index("direction_code")
        reference_code_index = header.index("reference_code")
        reference_value_index = header.index("reference_value")
        timeframe_index = header.index("timeframe_secs")
    except ValueError as error:
        raise SystemExit(f"error: {path} is missing a sequence column: {error}") from None
    by_bar: dict[int, list[dict[str, Any]]] = {}
    for row in rows:
        try:
            if int(row[timeframe_index]) != timeframe:
                continue
            if int(row[code_index]) != code:
                continue
            available = int(row[available_index])
            occurred = int(row[occurred_index])
            direction = int(row[direction_index])
        except (IndexError, TypeError, ValueError):
            continue
        by_bar.setdefault(available, []).append(
            {
                "bar": available,
                "occurred": occurred,
                "direction": direction,
                "reference_code": row[reference_code_index].strip(),
                "reference_value": optional_number(row[reference_value_index]),
            }
        )
    for values in by_bar.values():
        values.sort(
            key=lambda value: (
                value["occurred"],
                value["reference_code"],
                value["reference_value"] if value["reference_value"] is not None else 0.0,
            )
        )
    return by_bar


def same_level(
    first: dict[str, Any],
    second: dict[str, Any],
    atr: float | None,
    tolerance_atr: float,
) -> bool:
    first_code = first.get("reference_code")
    second_code = second.get("reference_code")
    if first_code and second_code:
        return first_code == second_code
    first_value = first.get("reference_value")
    second_value = second.get("reference_value")
    if first_value is None or second_value is None:
        return False
    tolerance = max(0.0, tolerance_atr) * (atr if atr is not None and atr > 0 else 0.0)
    return abs(first_value - second_value) <= tolerance


def close_side_matches(
    close: float,
    level: float | None,
    first_direction: int,
    close_side: str,
) -> bool:
    if close_side == "any":
        return True
    if level is None:
        return False
    if close_side == "breakout":
        return close > level if first_direction > 0 else close < level
    return close < level if first_direction > 0 else close > level


def sequence_condition(
    spec: dict[str, Any],
    events_path: Path,
    readings: dict[int, dict[str, str]],
    code_by_id: dict[str, int],
    timeframe: int,
) -> tuple[dict[int, int], int]:
    first = event_rows(events_path, code_by_id, str(spec["first_event"]), timeframe)
    second = event_rows(events_path, code_by_id, str(spec["second_event"]), timeframe)
    within = int(spec.get("within_bars", 0))
    same = bool(spec.get("same_level", False))
    tolerance = float(spec.get("level_tolerance_atr", 0.0))
    direction = str(spec.get("direction", "any"))
    close_side = str(spec.get("close_side", "any"))
    condition: dict[int, int] = {}
    matches = 0
    for bar in sorted(second):
        row = readings.get(bar)
        if row is None:
            continue
        close = optional_number(row.get("close"))
        atr = optional_number(row.get("atr"))
        for second_event in second[bar]:
            matched = False
            for first_bar in range(max(0, bar - within), bar + 1):
                for first_event in first.get(first_bar, []):
                    if direction == "same" and second_event["direction"] != first_event["direction"]:
                        continue
                    if direction == "opposite" and second_event["direction"] == first_event["direction"]:
                        continue
                    if same and not same_level(first_event, second_event, atr, tolerance):
                        continue
                    if close is None:
                        continue
                    if not close_side_matches(
                        close,
                        first_event.get("reference_value"),
                        int(first_event["direction"]),
                        close_side,
                    ):
                        continue
                    matched = True
                    break
                if matched:
                    break
            if matched:
                condition[bar] = 1
                matches += 1
                break
    return condition, matches


def confluence_condition(
    spec: dict[str, Any],
    readings: dict[int, dict[str, str]],
) -> tuple[dict[int, int], int]:
    references = [str(value) for value in spec["references"]]
    min_count = int(spec["min_count"])
    within_atr = float(spec["within_atr"])
    condition: dict[int, int] = {}
    matches = 0
    for bar, row in readings.items():
        close = optional_number(row.get("close"))
        atr = optional_number(row.get("atr"))
        if close is None or atr is None or atr <= 0:
            continue
        count = 0
        for reference in references:
            column = PIVOT_ALIASES.get(reference, reference)
            value = optional_number(row.get(column))
            if value is None:
                continue
            if abs(close - value) <= within_atr * atr:
                count += 1
        if count >= min_count:
            condition[bar] = 1
            matches += 1
    return condition, matches


def column_condition(
    spec: dict[str, Any],
    readings: dict[int, dict[str, str]],
) -> tuple[dict[int, int], int]:
    column = str(spec["column"])
    values = spec.get("values")
    if values is not None and (
        not isinstance(values, list)
        or not values
        or not all(isinstance(value, int) and not isinstance(value, bool) for value in values)
    ):
        raise SystemExit("error: column values must be a non-empty integer list")
    minimum = spec.get("min")
    maximum = spec.get("max")
    level_age_max = spec.get("level_age_max")
    level_touches_min = spec.get("level_touches_min")
    condition: dict[int, int] = {}
    matches = 0
    for bar, row in readings.items():
        value = optional_int(row.get(column))
        if value is None:
            continue
        if values is not None and value not in values:
            continue
        if minimum is not None and value < int(minimum):
            continue
        if maximum is not None and value > int(maximum):
            continue
        if level_age_max is not None:
            age = optional_int(row.get("level_age"))
            if age is None or age > int(level_age_max):
                continue
        if level_touches_min is not None:
            touches = optional_int(row.get("level_touches"))
            if touches is None or touches < int(level_touches_min):
                continue
        condition[bar] = 1
        matches += 1
    return condition, matches


def main(argv: list[str] | None = None) -> int:
    args = parse_arguments(argv)
    bundle = Path(args.bundle).expanduser()
    out = Path(args.out).expanduser()
    out.mkdir(parents=True, exist_ok=True)
    spec = load_condition(Path(args.condition).expanduser())
    readings_path = bundle / "readings.csv"
    events_path = Path(args.events).expanduser() if args.events else bundle / "events.csv"
    header, rows = read_csv(readings_path)
    if CONDITION_COLUMN in header:
        raise SystemExit(f"error: {readings_path} already has a {CONDITION_COLUMN} column")
    try:
        bar_index = header.index("bar")
        timeframe_index = header.index("timeframe_secs")
    except ValueError as error:
        raise SystemExit(f"error: {readings_path} is missing a bar/timeframe column: {error}") from None
    timeframes = {int(row[timeframe_index]) for row in rows if row[timeframe_index]}
    if len(timeframes) != 1:
        raise SystemExit(f"error: {readings_path} spans multiple timeframes")
    timeframe = next(iter(timeframes))
    declared = spec.get("timeframe_secs")
    if declared is not None and declared != timeframe:
        raise SystemExit(
            f"error: condition timeframe {declared} does not match bundle {timeframe}"
        )
    readings = {
        int(row[bar_index]): {name: row[index] for index, name in enumerate(header)}
        for row in rows
    }
    kind = spec["kind"]
    if kind == "sequence":
        if not events_path.is_file():
            raise SystemExit(f"error: events file not found: {events_path}")
        code_by_id = load_event_codes(Path(args.registry).expanduser())
        condition, matches = sequence_condition(
            spec, events_path, readings, code_by_id, timeframe
        )
    elif kind == "confluence":
        condition, matches = confluence_condition(spec, readings)
    else:
        condition, matches = column_condition(spec, readings)
    header = header + [CONDITION_COLUMN]
    conditioned = []
    for row in rows:
        bar = int(row[bar_index])
        conditioned.append(list(row) + [str(condition.get(bar, 0))])
    write_csv(out / "readings.csv", header, conditioned)
    for name in ("prediction.csv", "events.csv", "levels.csv", "panel.csv"):
        source = bundle / name
        if source.is_file():
            (out / name).write_bytes(source.read_bytes())
    gate = {
        "schema": GATE_SCHEMA,
        "all": [{"column": CONDITION_COLUMN, "values": [1]}],
    }
    write_json(out / "state.json", gate)
    document = {
        "schema": SCHEMA,
        "kind": kind,
        "spec": spec,
        "timeframe_secs": timeframe,
        "bars": len(rows),
        "condition_bars": len(condition),
        "matches": matches,
        "inputs": {
            "readings_sha256": file_digest(readings_path),
            "events_sha256": file_digest(events_path) if events_path.is_file() else None,
            "condition_sha256": file_digest(Path(args.condition).expanduser()),
        },
    }
    write_json(out / "condition.json", document)
    print(
        f"ok condition kind={kind} bars={len(rows)} matched={len(condition)} -> {out}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
