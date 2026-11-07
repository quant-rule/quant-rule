#!/usr/bin/env python3
"""Check the research coverage matrix and its frozen cell specs.

The checker reads the program manifest, the committed area map, the registry,
and every cell's `coverage.json` and specs. It supports the M2 full matrix, the
M3 state matrix, and the M4 primary matrix whose exploratory coverage is
inherited from M2.

It is read-only. It does not run the model and does not open validation.

Usage:
    python3 tools/research/coverage.py \
        --program config/research/PROGRAM-2026-11-05-03/program.json
    python3 tools/research/coverage.py \
        --program config/research/PROGRAM-2026-11-07-01/program.json \
        --family M4 \
        --inherit config/research/PROGRAM-2026-11-05-03/program.json

The file is slightly over the 1,000-line guideline after the M4 inherited
coverage path. The M2, M3, and M4 checks share the registry, area, spec, and
report helpers; splitting only the M4 path would duplicate or invert those
dependencies, so the split is deferred to a dedicated refactor of the shared
coverage kernel.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

import program as program_module

INSTRUMENTS = ["XAU", "BTC", "SP500", "EURUSD"]
DATA_FILES = {
    "XAU": "XAUUSD_M1_2018-01-01_2024-12-31.csv",
    "BTC": "BTCUSD_M1_2018-01-01_2024-12-31.csv",
    "SP500": "ESANDP500_M1_2018-01-01_2024-12-31.csv",
    "EURUSD": "EURUSD_M1_2018-01-01_2024-12-31.csv",
}
PRIMARY_IDS = ["M2.1", "M2.2", "M2.3", "M2.4", "M2.5"]
PRIMARY_IDS_M3 = ["M3.1", "M3.2", "M3.3", "M3.4", "M3.5"]
PRIMARY_IDS_M4 = ["M4.1", "M4.2", "M4.3", "M4.4", "M4.5"]
M3_TIMEFRAMES = (3600, 900)
M4_TIMEFRAME = 900
STATE_SCHEMA = "quantrule.model-state-gate/v1"
STATE_COLUMNS = {
    "context_code",
    "adx_regime",
    "adx_slope",
    "ema_stack",
    "di_state",
    "rsi_zone",
    "macd_state",
}
SCHEMA = "quantrule.coverage/v1"


def parse_arguments(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--program", required=True)
    parser.add_argument(
        "--registry", default="docs/registry/indicators.json", help="registry JSON"
    )
    parser.add_argument(
        "--family",
        choices=("M2", "M3", "M4", "MR5"),
        default="M2",
        help="programme family layout",
    )
    parser.add_argument(
        "--inherit",
        default=None,
        help="programme whose coverage is inherited (M4)",
    )
    parser.add_argument("--out", default=None, help="write coverage.json and .md here")
    return parser.parse_args(argv)


def read_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise SystemExit(f"error: cannot read {path}: {error}") from None


def load_registry(path: Path) -> tuple[dict[str, dict], dict[str, dict]]:
    document = read_json(path)
    indicators = {}
    for indicator in document.get("indicators", []):
        if isinstance(indicator, dict) and isinstance(indicator.get("id"), str):
            indicators[indicator["id"]] = indicator
    events = {}
    for event in document.get("events", []):
        if isinstance(event, dict) and isinstance(event.get("id"), str):
            events[event["id"]] = event
    return indicators, events


def load_areas(path: Path) -> dict[int, list[str]]:
    document = read_json(path)
    raw = document.get("areas") if isinstance(document, dict) else None
    if not isinstance(raw, dict) or not raw:
        raise SystemExit(f"error: {path}: areas must be a non-empty object")
    areas = {}
    for key, value in raw.items():
        try:
            area = int(key)
        except (TypeError, ValueError):
            raise SystemExit(f"error: {path}: area {key!r} is not an integer") from None
        if not isinstance(value, list) or not all(isinstance(item, str) for item in value):
            raise SystemExit(f"error: {path}: area {key} must be a string list")
        areas[area] = value
    return areas


def area_indicators(areas: list[int], mapping: dict[int, list[str]]) -> list[str]:
    values = []
    for area in areas:
        for indicator in mapping.get(area, []):
            if indicator not in values:
                values.append(indicator)
    return values


def area_events(
    indicators: list[str], events: dict[str, dict]
) -> set[str]:
    owners = set(indicators)
    return {
        event_id
        for event_id, event in events.items()
        if event.get("indicator") in owners
    }


def condition_events(value: Any) -> list[str]:
    if not isinstance(value, dict):
        return []
    if isinstance(value.get("event"), dict):
        identifier = value["event"].get("id")
        return [identifier] if isinstance(identifier, str) else []
    values = []
    for key in ("all", "any"):
        children = value.get(key)
        if isinstance(children, list):
            for child in children:
                values.extend(condition_events(child))
    if isinstance(value.get("not"), dict):
        values.extend(condition_events(value["not"]))
    return values


def expected_panel(instrument: str) -> dict[str, str]:
    return {
        other: f"../../../../../data/sources/{DATA_FILES[other]}"
        for other in INSTRUMENTS
        if other != instrument
    }


def check(
    program_path: Path, registry_path: Path, repo_root: Path | None = None
) -> dict[str, Any]:
    root = (repo_root or Path.cwd()).resolve()
    loaded = program_module.load_program(program_path, repo_root=root)
    indicators, events = load_registry(registry_path)
    areas_path = program_path.parent / "areas.json"
    areas = load_areas(areas_path)
    errors: list[str] = []
    cells: list[dict[str, Any]] = []
    seen_indicators: set[str] = set()
    seen_events: set[str] = set()
    exploratory = [cell for cell in loaded.cells if not cell.primary]
    primary = [cell for cell in loaded.cells if cell.primary]
    if len(exploratory) != 10:
        errors.append(f"program has {len(exploratory)} exploratory cells; expected 10")
    if len(primary) != 5:
        errors.append(f"program has {len(primary)} primary cells; expected 5")
    if sorted(cell.cell_id for cell in primary) != PRIMARY_IDS:
        errors.append(
            "primary cell ids must be " + ", ".join(PRIMARY_IDS)
        )
    for cell in loaded.cells:
        cell_dir = root / cell.config_dir
        coverage_path = cell_dir / "coverage.json"
        if not coverage_path.is_file():
            errors.append(f"{cell.cell_id}: coverage.json is missing")
            continue
        coverage = read_json(coverage_path)
        cell_areas = coverage.get("areas")
        if not isinstance(cell_areas, list) or not all(
            isinstance(area, int) for area in cell_areas
        ):
            errors.append(f"{cell.cell_id}: coverage.areas must be an integer list")
            continue
        declared = coverage.get("indicators")
        expected_indicators = area_indicators(cell_areas, areas)
        if declared != expected_indicators:
            errors.append(
                f"{cell.cell_id}: indicators {declared!r} do not match areas "
                f"{cell_areas!r} -> {expected_indicators!r}"
            )
        if coverage.get("cell_id") != cell.cell_id:
            errors.append(f"{cell.cell_id}: coverage.cell_id does not match")
        if coverage.get("kind") != ("primary" if cell.primary else "exploratory"):
            errors.append(f"{cell.cell_id}: coverage.kind does not match primary flag")
        benchmark = coverage.get("benchmark_events", [])
        if not isinstance(benchmark, list) or not all(
            isinstance(event, str) for event in benchmark
        ):
            errors.append(f"{cell.cell_id}: benchmark_events must be a string list")
            benchmark = []
        expected_active = area_events(expected_indicators, events) | set(benchmark)
        if cell.primary:
            if benchmark:
                errors.append(f"{cell.cell_id}: a primary cell must not carry a benchmark")
            expected_active = area_events(expected_indicators, events)
        spec_paths = sorted(cell_dir.glob("spec-*.json"))
        expected_paths = [
            cell_dir / f"spec-{instrument}.json" for instrument in INSTRUMENTS
        ]
        if set(spec_paths) != set(expected_paths):
            errors.append(
                f"{cell.cell_id}: specs must be exactly "
                + ", ".join(path.name for path in expected_paths)
            )
        specs = [read_json(path) for path in spec_paths]
        if not specs:
            continue
        base = specs[0]
        for path, spec in zip(spec_paths, specs):
            instrument = path.stem.removeprefix("spec-")
            if spec.get("schema") != "quantrule.model-spec/v2":
                errors.append(f"{path}: schema is not quantrule.model-spec/v2")
            if spec.get("family") != "M2":
                errors.append(f"{path}: family is not M2")
            if spec.get("id") != cell.cell_id:
                errors.append(f"{path}: id does not match {cell.cell_id}")
            if spec.get("instrument") != instrument:
                errors.append(f"{path}: instrument does not match the file name")
            if spec.get("timeframes") != [3600]:
                errors.append(f"{path}: timeframes must be [3600]")
            active = spec.get("active_events")
            if not isinstance(active, list) or not all(
                isinstance(event, str) for event in active
            ):
                errors.append(f"{path}: active_events must be a string list")
                active = []
            if len(active) != len(set(active)):
                errors.append(f"{path}: active_events has duplicates")
            unknown = sorted(set(active) - set(events))
            if unknown:
                errors.append(f"{path}: unknown active events {unknown}")
            if set(active) != expected_active:
                missing = sorted(expected_active - set(active))
                extra = sorted(set(active) - expected_active)
                errors.append(
                    f"{path}: event set mismatch; missing {missing}, extra {extra}"
                )
            if active != base.get("active_events"):
                errors.append(f"{path}: active_events differ between instruments")
            prediction = spec.get("prediction")
            if not isinstance(prediction, dict):
                errors.append(f"{path}: prediction must be an object")
                prediction = {}
            kind = prediction.get("kind")
            if kind == "direction":
                prediction_events = prediction.get("events", [])
            elif kind == "vote":
                prediction_events = list(prediction.get("long", [])) + list(
                    prediction.get("short", [])
                )
            else:
                errors.append(f"{path}: prediction.kind must be direction or vote")
                prediction_events = []
            if not isinstance(prediction_events, list) or not all(
                isinstance(event, str) for event in prediction_events
            ):
                errors.append(f"{path}: prediction events must be a string list")
                prediction_events = []
            if len(prediction_events) != len(set(prediction_events)):
                errors.append(f"{path}: prediction events has duplicates")
            for event_id in prediction_events:
                if event_id not in events:
                    errors.append(f"{path}: prediction event {event_id} is unknown")
                elif events[event_id].get("role") != "signal":
                    errors.append(
                        f"{path}: prediction event {event_id} is not a signal"
                    )
                if event_id not in active:
                    errors.append(f"{path}: prediction event {event_id} is not active")
            if prediction != base.get("prediction"):
                errors.append(f"{path}: prediction differs between instruments")
            if spec.get("conditions") != base.get("conditions"):
                errors.append(f"{path}: conditions differ between instruments")
            if spec.get("readings") != base.get("readings"):
                errors.append(f"{path}: readings differ between instruments")
            for condition in spec.get("conditions", []):
                if not isinstance(condition, dict):
                    continue
                for event_id in condition_events(condition.get("when")):
                    if event_id not in events:
                        errors.append(f"{path}: condition event {event_id} is unknown")
                    if event_id not in active:
                        errors.append(f"{path}: condition event {event_id} is not active")
            needs_panel = 9 in cell_areas
            needs_session = 10 in cell_areas
            if needs_panel:
                panel = spec.get("panel")
                if not isinstance(panel, dict):
                    errors.append(f"{path}: panel is required for this area")
                else:
                    if panel.get("leader") not in INSTRUMENTS:
                        errors.append(f"{path}: panel leader is not an instrument")
                    sources = panel.get("sources")
                    if not isinstance(sources, dict):
                        errors.append(f"{path}: panel.sources must be an object")
                    else:
                        if instrument in sources:
                            errors.append(
                                f"{path}: panel sources repeat the evaluated instrument"
                            )
                        if set(sources) != set(INSTRUMENTS) - {instrument}:
                            errors.append(
                                f"{path}: panel sources must be the other three instruments"
                            )
                        for other, source in sources.items():
                            expected = expected_panel(instrument).get(other)
                            if source != expected:
                                errors.append(
                                    f"{path}: panel source for {other} is not {expected}"
                                )
            elif "panel" in spec:
                errors.append(f"{path}: panel is not used by this area")
            if needs_session:
                session = spec.get("session")
                if not isinstance(session, dict):
                    errors.append(f"{path}: session is required for this area")
                elif (
                    session.get("zone") != "london"
                    or session.get("start_hour") != 8
                    or session.get("end_hour") != 16
                    or session.get("opening_range_bars") != 2
                ):
                    errors.append(f"{path}: session is not the declared London window")
            elif "session" in spec:
                errors.append(f"{path}: session is not used by this area")
        cell_active = set(base.get("active_events", []))
        seen_indicators |= set(expected_indicators)
        seen_events |= cell_active
        cells.append(
            {
                "cell_id": cell.cell_id,
                "kind": "primary" if cell.primary else "exploratory",
                "areas": cell_areas,
                "indicators": len(expected_indicators),
                "active_events": len(cell_active),
                "prediction_events": len(
                    base.get("prediction", {}).get(
                        "events",
                        list(base.get("prediction", {}).get("long", []))
                        + list(base.get("prediction", {}).get("short", [])),
                    )
                ),
                "panel": "panel" in base,
                "session": "session" in base,
            }
        )
    missing_indicators = sorted(set(indicators) - seen_indicators)
    missing_events = sorted(set(events) - seen_events)
    if missing_indicators:
        errors.append(f"uncovered indicators: {missing_indicators}")
    if missing_events:
        errors.append(f"uncovered events: {missing_events}")
    document = {
        "schema": SCHEMA,
        "program_id": loaded.program_id,
        "ok": not errors,
        "counts": {
            "cells": len(loaded.cells),
            "exploratory": len(exploratory),
            "primary": len(primary),
            "indicators": len(indicators),
            "events": len(events),
            "specs": len(loaded.cells) * len(INSTRUMENTS),
        },
        "cells": cells,
        "missing_indicators": missing_indicators,
        "missing_events": missing_events,
        "errors": errors,
    }
    return document


def check_m3(
    program_path: Path, registry_path: Path, repo_root: Path | None = None
) -> dict[str, Any]:
    """Checks the M3 programme: 10 exploratory + 5 state-conditioned cells."""
    root = (repo_root or Path.cwd()).resolve()
    loaded = program_module.load_program(program_path, repo_root=root)
    indicators, events = load_registry(registry_path)
    areas_path = program_path.parent / "areas.json"
    areas = load_areas(areas_path)
    errors: list[str] = []
    cells: list[dict[str, Any]] = []
    seen_indicators: set[str] = set()
    seen_events: set[str] = set()
    exploratory = [cell for cell in loaded.cells if not cell.primary]
    primary = [cell for cell in loaded.cells if cell.primary]
    if len(exploratory) != 10:
        errors.append(f"program has {len(exploratory)} exploratory cells; expected 10")
    if len(primary) != 5:
        errors.append(f"program has {len(primary)} primary cells; expected 5")
    if sorted(cell.cell_id for cell in primary) != PRIMARY_IDS_M3:
        errors.append("primary cell ids must be " + ", ".join(PRIMARY_IDS_M3))
    spec_count = 0
    for cell in loaded.cells:
        cell_dir = root / cell.config_dir
        coverage_path = cell_dir / "coverage.json"
        if not coverage_path.is_file():
            errors.append(f"{cell.cell_id}: coverage.json is missing")
            continue
        coverage = read_json(coverage_path)
        if coverage.get("cell_id") != cell.cell_id:
            errors.append(f"{cell.cell_id}: coverage.cell_id does not match")
        expected_kind = "primary" if cell.primary else "exploratory"
        if coverage.get("kind") != expected_kind:
            errors.append(f"{cell.cell_id}: coverage.kind does not match primary flag")
        cell_areas = coverage.get("areas", [])
        if not isinstance(cell_areas, list) or not all(
            isinstance(area, int) for area in cell_areas
        ):
            errors.append(f"{cell.cell_id}: coverage.areas must be an integer list")
            cell_areas = []
        declared = coverage.get("indicators")
        if cell.primary:
            expected_events = coverage.get("events")
            if not isinstance(expected_events, list) or not expected_events:
                errors.append(f"{cell.cell_id}: primary events must be a non-empty list")
                expected_events = []
            expected_indicators = sorted(
                {
                    events[event_id]["indicator"]
                    for event_id in expected_events
                    if event_id in events
                    and isinstance(events[event_id].get("indicator"), str)
                }
            )
            if declared != expected_indicators:
                errors.append(
                    f"{cell.cell_id}: indicators {declared!r} do not match "
                    f"events {expected_events!r} -> {expected_indicators!r}"
                )
            expected_active = set(expected_events)
        else:
            expected_indicators = area_indicators(cell_areas, areas)
            if declared != expected_indicators:
                errors.append(
                    f"{cell.cell_id}: indicators {declared!r} do not match areas "
                    f"{cell_areas!r} -> {expected_indicators!r}"
                )
            benchmark = coverage.get("benchmark_events", [])
            if not isinstance(benchmark, list) or not all(
                isinstance(event, str) for event in benchmark
            ):
                errors.append(f"{cell.cell_id}: benchmark_events must be a string list")
                benchmark = []
            expected_active = area_events(expected_indicators, events) | set(benchmark)
        instruments = coverage.get("instruments", INSTRUMENTS)
        if (
            not isinstance(instruments, list)
            or not instruments
            or not all(isinstance(item, str) and item in INSTRUMENTS for item in instruments)
        ):
            errors.append(f"{cell.cell_id}: instruments must be a non-empty known list")
            instruments = list(INSTRUMENTS)
        timeframes = coverage.get("timeframes", list(M3_TIMEFRAMES))
        if (
            not isinstance(timeframes, list)
            or not timeframes
            or not all(isinstance(item, int) and item in M3_TIMEFRAMES for item in timeframes)
        ):
            errors.append(f"{cell.cell_id}: timeframes must be in {list(M3_TIMEFRAMES)}")
            timeframes = list(M3_TIMEFRAMES)
        base = None
        cell_active: set[str] = set()
        for instrument in instruments:
            for timeframe in timeframes:
                path = cell_dir / f"spec-{instrument}-{timeframe}.json"
                if not path.is_file():
                    errors.append(f"{path}: spec is missing")
                    continue
                spec_count += 1
                spec = read_json(path)
                if spec.get("schema") != "quantrule.model-spec/v2":
                    errors.append(f"{path}: schema is not quantrule.model-spec/v2")
                if spec.get("family") != "M3":
                    errors.append(f"{path}: family is not M3")
                if spec.get("id") != cell.cell_id:
                    errors.append(f"{path}: id does not match {cell.cell_id}")
                if spec.get("instrument") != instrument:
                    errors.append(f"{path}: instrument does not match the file name")
                if spec.get("timeframes") != [timeframe]:
                    errors.append(f"{path}: timeframes must be [{timeframe}]")
                active = spec.get("active_events")
                if not isinstance(active, list) or not all(
                    isinstance(event, str) for event in active
                ):
                    errors.append(f"{path}: active_events must be a string list")
                    active = []
                if len(active) != len(set(active)):
                    errors.append(f"{path}: active_events has duplicates")
                unknown = sorted(set(active) - set(events))
                if unknown:
                    errors.append(f"{path}: unknown active events {unknown}")
                if set(active) != expected_active:
                    missing = sorted(expected_active - set(active))
                    extra = sorted(set(active) - expected_active)
                    errors.append(
                        f"{path}: event set mismatch; missing {missing}, extra {extra}"
                    )
                prediction = spec.get("prediction")
                if not isinstance(prediction, dict):
                    errors.append(f"{path}: prediction must be an object")
                    prediction = {}
                kind = prediction.get("kind")
                if kind == "direction":
                    prediction_events = prediction.get("events", [])
                elif kind == "vote":
                    prediction_events = list(prediction.get("long", [])) + list(
                        prediction.get("short", [])
                    )
                else:
                    errors.append(f"{path}: prediction.kind must be direction or vote")
                    prediction_events = []
                if not isinstance(prediction_events, list) or not all(
                    isinstance(event, str) for event in prediction_events
                ):
                    errors.append(f"{path}: prediction events must be a string list")
                    prediction_events = []
                for event_id in prediction_events:
                    if event_id not in events:
                        errors.append(f"{path}: prediction event {event_id} is unknown")
                    elif events[event_id].get("role") != "signal":
                        errors.append(
                            f"{path}: prediction event {event_id} is not a signal"
                        )
                    if event_id not in active:
                        errors.append(
                            f"{path}: prediction event {event_id} is not active"
                        )
                if base is None:
                    base = spec
                    cell_active = set(active)
                else:
                    if prediction != base.get("prediction"):
                        errors.append(f"{path}: prediction differs between specs")
                    if spec.get("conditions") != base.get("conditions"):
                        errors.append(f"{path}: conditions differ between specs")
                    if spec.get("readings") != base.get("readings"):
                        errors.append(f"{path}: readings differ between specs")
                for condition in spec.get("conditions", []):
                    if not isinstance(condition, dict):
                        continue
                    for event_id in condition_events(condition.get("when")):
                        if event_id not in events:
                            errors.append(
                                f"{path}: condition event {event_id} is unknown"
                            )
                        if event_id not in active:
                            errors.append(
                                f"{path}: condition event {event_id} is not active"
                            )
                needs_panel = 9 in cell_areas
                needs_session = 10 in cell_areas
                if needs_panel:
                    panel = spec.get("panel")
                    if not isinstance(panel, dict):
                        errors.append(f"{path}: panel is required for this area")
                    else:
                        if panel.get("leader") not in INSTRUMENTS:
                            errors.append(f"{path}: panel leader is not an instrument")
                        sources = panel.get("sources")
                        if not isinstance(sources, dict):
                            errors.append(f"{path}: panel.sources must be an object")
                        else:
                            if instrument in sources:
                                errors.append(
                                    f"{path}: panel sources repeat the evaluated instrument"
                                )
                            if set(sources) != set(INSTRUMENTS) - {instrument}:
                                errors.append(
                                    f"{path}: panel sources must be the other three instruments"
                                )
                            for other, source in sources.items():
                                expected = expected_panel(instrument).get(other)
                                if source != expected:
                                    errors.append(
                                        f"{path}: panel source for {other} is not {expected}"
                                    )
                elif "panel" in spec:
                    errors.append(f"{path}: panel is not used by this area")
                if needs_session:
                    session = spec.get("session")
                    if not isinstance(session, dict):
                        errors.append(f"{path}: session is required for this area")
                    elif (
                        session.get("zone") != "london"
                        or session.get("start_hour") != 8
                        or session.get("end_hour") != 16
                        or session.get("opening_range_bars") != 2
                    ):
                        errors.append(f"{path}: session is not the declared London window")
                elif "session" in spec:
                    errors.append(f"{path}: session is not used by this area")
        if cell.primary:
            state_path = cell_dir / "state.json"
            state_document = None
            if not state_path.is_file():
                errors.append(f"{state_path}: state.json is missing")
            else:
                state_document = read_json(state_path)
                if not isinstance(state_document, dict) or state_document.get(
                    "schema"
                ) != STATE_SCHEMA:
                    errors.append(f"{state_path}: schema is not {STATE_SCHEMA}")
                clauses = state_document.get("all") if isinstance(state_document, dict) else None
                if not isinstance(clauses, list) or not clauses:
                    errors.append(f"{state_path}: all must be a non-empty list")
                else:
                    seen_columns = set()
                    for index, clause in enumerate(clauses):
                        if not isinstance(clause, dict):
                            errors.append(f"{state_path}: all[{index}] must be an object")
                            continue
                        column = clause.get("column")
                        if column not in STATE_COLUMNS:
                            errors.append(
                                f"{state_path}: all[{index}].column is not a state column"
                            )
                        if column in seen_columns:
                            errors.append(f"{state_path}: column {column!r} is repeated")
                        seen_columns.add(column)
                        values = clause.get("values")
                        if (
                            not isinstance(values, list)
                            or not values
                            or not all(
                                isinstance(value, int) and not isinstance(value, bool)
                                for value in values
                            )
                        ):
                            errors.append(
                                f"{state_path}: all[{index}].values must be an integer list"
                            )
            if len(instruments) < 2:
                errors.append(f"{cell.cell_id}: a primary cell needs at least two instruments")
        seen_indicators |= set(expected_indicators)
        seen_events |= cell_active
        cells.append(
            {
                "cell_id": cell.cell_id,
                "kind": expected_kind,
                "areas": cell_areas,
                "instruments": instruments,
                "timeframes": timeframes,
                "indicators": len(expected_indicators),
                "active_events": len(cell_active),
                "prediction_events": len(
                    base.get("prediction", {}).get(
                        "events",
                        list(base.get("prediction", {}).get("long", []))
                        + list(base.get("prediction", {}).get("short", [])),
                    )
                )
                if base is not None
                else 0,
                "state_gate": (
                    state_document.get("all")
                    if cell.primary and isinstance(state_document, dict)
                    else None
                ),
            }
        )
    missing_indicators = sorted(set(indicators) - seen_indicators)
    missing_events = sorted(set(events) - seen_events)
    if missing_indicators:
        errors.append(f"uncovered indicators: {missing_indicators}")
    if missing_events:
        errors.append(f"uncovered events: {missing_events}")
    document = {
        "schema": SCHEMA,
        "program_id": loaded.program_id,
        "family": "M3",
        "ok": not errors,
        "counts": {
            "cells": len(loaded.cells),
            "exploratory": len(exploratory),
            "primary": len(primary),
            "indicators": len(indicators),
            "events": len(events),
            "specs": spec_count,
        },
        "cells": cells,
        "missing_indicators": missing_indicators,
        "missing_events": missing_events,
        "errors": errors,
    }
    return document


def check_m4(
    program_path: Path,
    registry_path: Path,
    inherit_path: Path | None = None,
    repo_root: Path | None = None,
) -> dict[str, Any]:
    """Checks the M4 programme: five location-conditioned cells.

    The M4 surface adds no indicator or event, so the exploratory coverage is
    inherited from the M2 programme rather than re-run. This checker proves the
    five primary cells, their single signal event, their 15m specs, and their
    ``level_touches`` gate, then checks the inherited M2 matrix.
    """
    root = (repo_root or Path.cwd()).resolve()
    loaded = program_module.load_program(program_path, repo_root=root)
    indicators, events = load_registry(registry_path)
    published_readings = {
        output
        for indicator in indicators.values()
        for output in indicator.get("outputs", [])
        if isinstance(output, str)
    }
    errors: list[str] = []
    cells: list[dict[str, Any]] = []
    if "level_touches" not in published_readings:
        errors.append("level_touches is not a published reading")
    primary = [cell for cell in loaded.cells if cell.primary]
    if len(loaded.cells) != 5:
        errors.append(f"program has {len(loaded.cells)} cells; expected 5")
    if sorted(cell.cell_id for cell in primary) != PRIMARY_IDS_M4:
        errors.append("primary cell ids must be " + ", ".join(PRIMARY_IDS_M4))
    spec_count = 0
    for cell in primary:
        cell_dir = root / cell.config_dir
        coverage_path = cell_dir / "coverage.json"
        if not coverage_path.is_file():
            errors.append(f"{cell.cell_id}: coverage.json is missing")
            continue
        coverage = read_json(coverage_path)
        if coverage.get("cell_id") != cell.cell_id:
            errors.append(f"{cell.cell_id}: coverage.cell_id does not match")
        if coverage.get("kind") != "primary":
            errors.append(f"{cell.cell_id}: coverage.kind must be primary")
        expected_events = coverage.get("events")
        if (
            not isinstance(expected_events, list)
            or len(expected_events) != 1
            or not all(isinstance(event, str) for event in expected_events)
        ):
            errors.append(f"{cell.cell_id}: coverage.events must be one event id")
            expected_events = []
        event_id = expected_events[0] if expected_events else None
        event = events.get(event_id) if event_id else None
        if event is None:
            errors.append(f"{cell.cell_id}: event {event_id!r} is not registered")
        elif event.get("role") != "signal":
            errors.append(f"{cell.cell_id}: event {event_id!r} is not a signal event")
        gate_path = cell_dir / "state.json"
        if not gate_path.is_file():
            errors.append(f"{cell.cell_id}: state.json is missing")
        else:
            gate = read_json(gate_path)
            if gate.get("schema") != STATE_SCHEMA:
                errors.append(f"{cell.cell_id}: state.json schema is not {STATE_SCHEMA}")
            clauses = gate.get("all")
            if not isinstance(clauses, list) or len(clauses) != 1:
                errors.append(f"{cell.cell_id}: state.json must have one clause")
            else:
                clause = clauses[0]
                if not isinstance(clause, dict):
                    errors.append(f"{cell.cell_id}: state.json clause is not an object")
                else:
                    if clause.get("column") != "level_touches":
                        errors.append(
                            f"{cell.cell_id}: gate column must be level_touches"
                        )
                    if clause.get("min") != 1:
                        errors.append(f"{cell.cell_id}: gate min must be 1")
                    if "values" in clause:
                        errors.append(
                            f"{cell.cell_id}: gate must not combine values with min"
                        )
        instruments = coverage.get("instruments", INSTRUMENTS)
        if (
            not isinstance(instruments, list)
            or not instruments
            or not all(
                isinstance(item, str) and item in INSTRUMENTS for item in instruments
            )
        ):
            errors.append(f"{cell.cell_id}: instruments must be a known list")
            instruments = list(INSTRUMENTS)
        if coverage.get("timeframe") != M4_TIMEFRAME:
            errors.append(f"{cell.cell_id}: timeframe must be {M4_TIMEFRAME}")
        cell_active: set[str] = set()
        for instrument in instruments:
            path = cell_dir / f"spec-{instrument}-{M4_TIMEFRAME}.json"
            if not path.is_file():
                errors.append(f"{path}: spec is missing")
                continue
            spec_count += 1
            spec = read_json(path)
            if spec.get("schema") != "quantrule.model-spec/v2":
                errors.append(f"{path}: schema is not quantrule.model-spec/v2")
            if spec.get("family") != "M4":
                errors.append(f"{path}: family is not M4")
            if spec.get("id") != cell.cell_id:
                errors.append(f"{path}: id does not match {cell.cell_id}")
            if spec.get("instrument") != instrument:
                errors.append(f"{path}: instrument does not match the file name")
            if spec.get("timeframes") != [M4_TIMEFRAME]:
                errors.append(f"{path}: timeframes must be [{M4_TIMEFRAME}]")
            active = spec.get("active_events")
            if not isinstance(active, list) or not all(
                isinstance(item, str) for item in active
            ):
                errors.append(f"{path}: active_events must be a string list")
                active = []
            if len(active) != len(set(active)):
                errors.append(f"{path}: active_events has duplicates")
            if event_id is not None and set(active) != {event_id}:
                errors.append(f"{path}: active_events must be exactly [{event_id}]")
            cell_active.update(active)
            prediction = spec.get("prediction")
            if not isinstance(prediction, dict):
                errors.append(f"{path}: prediction must be an object")
                prediction = {}
            if prediction.get("kind") != "direction":
                errors.append(f"{path}: prediction.kind must be direction")
            if prediction.get("events") != [event_id]:
                errors.append(f"{path}: prediction.events must be [{event_id}]")
            if prediction.get("min_votes") != 1:
                errors.append(f"{path}: prediction.min_votes must be 1")
        cells.append(
            {
                "cell_id": cell.cell_id,
                "kind": "primary",
                "events": expected_events,
                "active_events": sorted(cell_active),
                "instruments": instruments,
                "timeframe": M4_TIMEFRAME,
                "gate": {"column": "level_touches", "min": 1},
            }
        )
    inherited = None
    if inherit_path is None:
        errors.append("--inherit is required for M4")
    else:
        inherit = Path(inherit_path).expanduser()
        if not inherit.is_absolute():
            inherit = root / inherit
        inherited_document = check(inherit, registry_path, root)
        if not inherited_document["ok"]:
            errors.append("inherited coverage is not ok")
        if inherited_document["counts"]["indicators"] != len(indicators):
            errors.append("inherited indicator count does not match the registry")
        if inherited_document["counts"]["events"] != len(events):
            errors.append("inherited event count does not match the registry")
        inherited = inherited_document["program_id"]
    document = {
        "schema": SCHEMA,
        "program_id": loaded.program_id,
        "family": "M4",
        "ok": not errors,
        "counts": {
            "cells": len(loaded.cells),
            "primary": len(primary),
            "indicators": len(indicators),
            "events": len(events),
            "specs": spec_count,
        },
        "inherited": inherited,
        "cells": cells,
        "errors": errors,
    }
    return document


def markdown_m4(document: dict[str, Any]) -> str:
    lines = [
        "# M4.x Coverage",
        "",
        f"- Program: `{document['program_id']}`",
        f"- Status: **{'ok' if document['ok'] else 'failed'}**",
        f"- Inherited: `{document.get('inherited') or '-'}`",
        f"- Primary cells: {document['counts']['primary']}",
        f"- Indicators: {document['counts']['indicators']}; events: {document['counts']['events']}; specs: {document['counts']['specs']}",
        "",
        "| Cell | Kind | Instruments | Timeframe | Events | Gate |",
        "|---|---|---|---:|---|---|",
    ]
    for cell in document["cells"]:
        gate = cell["gate"]
        lines.append(
            f"| {cell['cell_id']} | {cell['kind']} | "
            f"{','.join(cell['instruments'])} | {cell['timeframe']} | "
            f"{','.join(cell['events'])} | "
            f"{gate['column']} >= {gate['min']} |"
        )
    lines.extend(["", "## Errors", ""])
    if document["errors"]:
        lines.extend(f"- {error}" for error in document["errors"])
    else:
        lines.append("- None.")
    lines.append("")
    return "\n".join(lines)


def markdown_m3(document: dict[str, Any]) -> str:
    lines = [
        "# M3.x Coverage",
        "",
        f"- Program: `{document['program_id']}`",
        f"- Status: **{'ok' if document['ok'] else 'failed'}**",
        f"- Cells: {document['counts']['cells']} ({document['counts']['exploratory']} exploratory, {document['counts']['primary']} primary)",
        f"- Indicators: {document['counts']['indicators']}; events: {document['counts']['events']}; specs: {document['counts']['specs']}",
        "",
        "| Cell | Kind | Areas | Instruments | Timeframes | Indicators | Active events | Prediction events | State gate |",
        "|---|---|---|---|---|---:|---:|---:|---|",
    ]
    for cell in document["cells"]:
        state = cell["state_gate"]
        state_text = "-"
        if state:
            state_text = "; ".join(
                f"{clause['column']} in {clause['values']}" for clause in state
            )
        lines.append(
            f"| {cell['cell_id']} | {cell['kind']} | "
            f"{','.join(str(area) for area in cell['areas']) or '-'} | "
            f"{','.join(cell['instruments'])} | "
            f"{','.join(str(tf) for tf in cell['timeframes'])} | "
            f"{cell['indicators']} | {cell['active_events']} | "
            f"{cell['prediction_events']} | {state_text} |"
        )
    lines.extend(["", "## Errors", ""])
    if document["errors"]:
        lines.extend(f"- {error}" for error in document["errors"])
    else:
        lines.append("- None.")
    lines.append("")
    return "\n".join(lines)

def markdown(document: dict[str, Any]) -> str:
    lines = [
        "# M2.x Coverage",
        "",
        f"- Program: `{document['program_id']}`",
        f"- Status: **{'ok' if document['ok'] else 'failed'}**",
        f"- Cells: {document['counts']['cells']} ({document['counts']['exploratory']} exploratory, {document['counts']['primary']} primary)",
        f"- Indicators: {document['counts']['indicators']}; events: {document['counts']['events']}; specs: {document['counts']['specs']}",
        "",
        "| Cell | Kind | Areas | Indicators | Active events | Prediction events | Panel | Session |",
        "|---|---|---|---:|---:|---:|---|---|",
    ]
    for cell in document["cells"]:
        lines.append(
            f"| {cell['cell_id']} | {cell['kind']} | {','.join(str(area) for area in cell['areas'])} | "
            f"{cell['indicators']} | {cell['active_events']} | {cell['prediction_events']} | "
            f"{'yes' if cell['panel'] else '-'} | {'yes' if cell['session'] else '-'} |"
        )
    lines.extend(["", "## Errors", ""])
    if document["errors"]:
        lines.extend(f"- {error}" for error in document["errors"])
    else:
        lines.append("- None.")
    lines.append("")
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    args = parse_arguments(argv)
    if args.family == "MR5":
        from coverage_mr5 import check_mr5, markdown_mr5

        document = check_mr5(
            Path(args.program).expanduser(),
            Path(args.registry).expanduser(),
            Path(args.inherit).expanduser() if args.inherit else None,
        )
        render = markdown_mr5
    elif args.family == "M4":
        document = check_m4(
            Path(args.program).expanduser(),
            Path(args.registry).expanduser(),
            Path(args.inherit).expanduser() if args.inherit else None,
        )
        render = markdown_m4
    elif args.family == "M3":
        document = check_m3(
            Path(args.program).expanduser(), Path(args.registry).expanduser()
        )
        render = markdown_m3
    else:
        document = check(
            Path(args.program).expanduser(), Path(args.registry).expanduser()
        )
        render = markdown
    if args.out:
        out = Path(args.out).expanduser()
        out.mkdir(parents=True, exist_ok=True)
        (out / "coverage.json").write_text(
            json.dumps(document, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
        )
        (out / "coverage.md").write_text(render(document), encoding="utf-8")
    if document["ok"]:
        message = (
            f"ok {document['program_id']} cells={document['counts']['cells']} "
            f"indicators={document['counts']['indicators']} "
            f"events={document['counts']['events']}"
        )
        if document.get("inherited"):
            message += f" inherited={document['inherited']}"
        print(message)
        return 0
    for error in document["errors"]:
        print(f"error: {error}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
