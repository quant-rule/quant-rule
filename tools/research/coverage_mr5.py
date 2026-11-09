#!/usr/bin/env python3
"""MR research coverage: mechanism cells on model M1.0 with configuration variants.

The MR families add no indicator or event. The exploratory surface is inherited
from the M2 programme; this checker proves that every cell has a model config, a
spec for each instrument, a registered signal event, and a condition or state
gate, and that the inherited M2 matrix still covers all 46 indicators and 143
events. `family` selects the MR5 or MR6 cell prefix; each cell declares its own
timeframe, so a family may mix 15m and 5m cells.
"""

from __future__ import annotations

import re
from pathlib import Path
from typing import Any

import coverage
import program as program_module

MR_CELL = re.compile(r"^(?P<family>MR\d+)\.(?P<number>\d+)-C(?P<variant>\d{2})$")
KNOWN_INSTRUMENTS = {"XAU", "BTC", "SP500", "EURUSD", "ETH", "COPPER", "TSLA"}
CONDITION_SCHEMA = "quantrule.model-condition/v1"
LATER_EVENTS = {
    "signal.opening_range_sweep",
    "signal.opening_range_sweep_control",
    "signal.breakout_retest_hold",
}
FAMILY_NEW_EVENTS = {
    "MR7": {
        "signal.opening_range_sweep",
        "signal.opening_range_sweep_control",
    }
}


def check_mr5(
    program_path: Path,
    registry_path: Path,
    inherit_path: Path | None = None,
    repo_root: Path | None = None,
    family: str = "MR5",
) -> dict[str, Any]:
    root = (repo_root or Path.cwd()).resolve()
    loaded = program_module.load_program(program_path, repo_root=root)
    indicators, events = coverage.load_registry(registry_path)
    errors: list[str] = []
    cells: list[dict[str, Any]] = []
    primary = [cell for cell in loaded.cells if cell.primary]
    if not primary:
        errors.append("program has no primary cell")
    if loaded.correction.get("primary_cells") != len(primary):
        errors.append("correction.primary_cells must equal the primary cell count")
    spec_count = 0
    for cell in loaded.cells:
        match = MR_CELL.match(cell.cell_id)
        if not match:
            errors.append(f"{cell.cell_id}: cell id must match {family}.<n>-C<nn>")
        elif match.group("family") != family:
            errors.append(f"{cell.cell_id}: cell id must use the {family} family")
        cell_dir = root / cell.config_dir
        if not cell_dir.is_dir():
            errors.append(f"{cell.cell_id}: config_dir does not exist")
            continue
        model_path = cell_dir / "model.json"
        if not model_path.is_file():
            errors.append(f"{cell.cell_id}: model.json is missing")
        else:
            model = coverage.read_json(model_path)
            if model.get("schema") != "quantrule.model/v4":
                errors.append(f"{cell.cell_id}: model schema is not quantrule.model/v4")
        coverage_path = cell_dir / "coverage.json"
        if not coverage_path.is_file():
            errors.append(f"{cell.cell_id}: coverage.json is missing")
            continue
        record = coverage.read_json(coverage_path)
        if record.get("cell_id") != cell.cell_id:
            errors.append(f"{cell.cell_id}: coverage.cell_id does not match")
        expected_events = record.get("events")
        if (
            not isinstance(expected_events, list)
            or not expected_events
            or not all(isinstance(event, str) for event in expected_events)
        ):
            errors.append(f"{cell.cell_id}: coverage.events must be a non-empty list")
            expected_events = []
        condition_events = record.get("condition_events", [])
        if not isinstance(condition_events, list) or not all(
            isinstance(event, str) for event in condition_events
        ):
            errors.append(f"{cell.cell_id}: coverage.condition_events must be a string list")
            condition_events = []
        for event_id in expected_events:
            event = events.get(event_id)
            if event is None:
                errors.append(f"{cell.cell_id}: event {event_id!r} is not registered")
            elif event.get("role") != "signal":
                errors.append(f"{cell.cell_id}: event {event_id!r} is not a signal event")
        for event_id in condition_events:
            if event_id not in events:
                errors.append(f"{cell.cell_id}: condition event {event_id!r} is not registered")
        instruments = record.get("instruments", list(coverage.INSTRUMENTS))
        if (
            not isinstance(instruments, list)
            or not instruments
            or not all(
                isinstance(item, str) and item in KNOWN_INSTRUMENTS
                for item in instruments
            )
        ):
            errors.append(f"{cell.cell_id}: instruments must be a known list")
            instruments = list(coverage.INSTRUMENTS)
        timeframe = record.get("timeframe")
        if not isinstance(timeframe, int) or isinstance(timeframe, bool) or timeframe <= 0:
            errors.append(f"{cell.cell_id}: timeframe must be a positive integer")
            timeframe = 0
        gate = cell_dir / "condition.json"
        if not gate.is_file():
            gate = cell_dir / "state.json"
        if not gate.is_file():
            errors.append(f"{cell.cell_id}: condition.json or state.json is missing")
        else:
            document = coverage.read_json(gate)
            if gate.name == "condition.json":
                if document.get("schema") != CONDITION_SCHEMA:
                    errors.append(f"{cell.cell_id}: condition schema is not {CONDITION_SCHEMA}")
                if document.get("kind") not in {"sequence", "confluence", "column"}:
                    errors.append(f"{cell.cell_id}: condition kind is not supported")
            elif document.get("schema") != coverage.STATE_SCHEMA:
                errors.append(f"{cell.cell_id}: state schema is not {coverage.STATE_SCHEMA}")
        for instrument in instruments:
            path = cell_dir / f"spec-{instrument}-{timeframe}.json"
            if not path.is_file():
                errors.append(f"{path}: spec is missing")
                continue
            spec_count += 1
            spec = coverage.read_json(path)
            if spec.get("schema") != "quantrule.model-spec/v2":
                errors.append(f"{path}: schema is not quantrule.model-spec/v2")
            if spec.get("family") != family:
                errors.append(f"{path}: family is not {family}")
            if spec.get("id") != cell.cell_id:
                errors.append(f"{path}: id does not match {cell.cell_id}")
            if spec.get("instrument") != instrument:
                errors.append(f"{path}: instrument does not match the file name")
            if spec.get("timeframes") != [timeframe]:
                errors.append(f"{path}: timeframes must be [{timeframe}]")
            spec_events = spec.get("active_events")
            if not isinstance(spec_events, list) or not all(
                isinstance(event, str) for event in spec_events
            ):
                errors.append(f"{path}: active_events must be a string list")
                spec_events = []
            if set(spec_events) != set(expected_events) | set(condition_events):
                errors.append(f"{path}: active_events do not match the prediction and condition events")
            prediction = spec.get("prediction")
            if not isinstance(prediction, dict):
                errors.append(f"{path}: prediction must be an object")
            else:
                if prediction.get("kind") != "direction":
                    errors.append(f"{path}: prediction.kind must be direction")
                if set(prediction.get("events", [])) != set(expected_events):
                    errors.append(f"{path}: prediction.events do not match coverage.events")
                if prediction.get("min_votes") != 1:
                    errors.append(f"{path}: prediction.min_votes must be 1")
        cells.append(
            {
                "cell_id": cell.cell_id,
                "primary": cell.primary,
                "events": expected_events,
                "condition_events": condition_events,
                "instruments": instruments,
                "timeframe": timeframe,
                "gate": gate.name,
            }
        )
    inherited = None
    covered_events: set[str] = set()
    if inherit_path is None:
        errors.append(f"--inherit is required for {family}")
    else:
        inherit = Path(inherit_path).expanduser()
        if not inherit.is_absolute():
            inherit = root / inherit
        inherited_document = coverage.check(inherit, registry_path, root)
        new_events = FAMILY_NEW_EVENTS.get(family, set())
        relevant_errors = [
            error
            for error in inherited_document.get("errors", [])
            if not any(event in error for event in LATER_EVENTS)
        ]
        for error in relevant_errors:
            errors.append(f"inherited: {error}")
        if inherited_document["counts"]["indicators"] != len(indicators):
            errors.append("inherited indicator count does not match the registry")
        if inherited_document["counts"]["events"] != len(events) - len(LATER_EVENTS):
            errors.append("inherited event count does not match the registry")
        covered_events = {
            event_id
            for cell in cells
            for key in ("events", "condition_events")
            for event_id in cell[key]
        }
        for event_id in sorted(new_events - covered_events):
            errors.append(f"{event_id}: new event is not covered by {family}")
        inherited = inherited_document["program_id"]
    document = {
        "schema": coverage.SCHEMA,
        "program_id": loaded.program_id,
        "family": family,
        "ok": not errors,
        "counts": {
            "cells": len(loaded.cells),
            "primary": len(primary),
            "indicators": len(indicators),
            "events": len(events) - len(LATER_EVENTS - covered_events),
            "specs": spec_count,
        },
        "inherited": inherited,
        "cells": cells,
        "errors": errors,
    }
    return document


def markdown_mr5(document: dict[str, Any]) -> str:
    lines = [
        f"# {document['family']} Coverage",
        "",
        f"- Program: `{document['program_id']}`",
        f"- Status: **{'ok' if document['ok'] else 'failed'}**",
        f"- Inherited: `{document.get('inherited') or '-'}`",
        f"- Cells: {document['counts']['cells']} "
        f"({document['counts']['primary']} primary)",
        f"- Indicators: {document['counts']['indicators']}; "
        f"events: {document['counts']['events']}; specs: {document['counts']['specs']}",
        "",
        "| Cell | Primary | Instruments | Timeframe | Events | Gate |",
        "|---|---|---|---:|---|---|",
    ]
    for cell in document["cells"]:
        lines.append(
            f"| {cell['cell_id']} | {'yes' if cell['primary'] else 'no'} | "
            f"{','.join(cell['instruments'])} | {cell['timeframe']} | "
            f"{','.join(cell['events'])} | {cell['gate']} |"
        )
    lines.extend(["", "## Errors", ""])
    lines.extend(f"- {error}" for error in document["errors"])
    lines.append("")
    return "\n".join(lines)
