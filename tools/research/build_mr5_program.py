#!/usr/bin/env python3
"""Build an MR5 research grid from a declarative grid document.

A grid is one model (`M1.0`), a finite list of mechanism hypotheses, and a
finite list of configuration variants per hypothesis. Every cell gets its own
model config, condition, specs, and coverage record. The builder refuses to
overwrite an existing programme directory.

Usage:
    python3 tools/research/build_mr5_program.py \
        --grid config/research/PROGRAM-<date>-<nn>/grid.json \
        --out config/research/PROGRAM-<date>-<nn>
"""

from __future__ import annotations

import argparse
import copy
import json
import shutil
from pathlib import Path
from typing import Any

SCHEMA = "quantrule.research-grid/v1"
PROGRAM_SCHEMA = "quantrule.research-program/v1"
CELL = "MR5.{hypothesis}-{variant}"
INSTRUMENTS = ("XAU", "BTC", "SP500", "EURUSD")


def read_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise SystemExit(f"error: cannot read {path}: {error}") from None


def write_json(path: Path, document: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(document, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )


def deep_merge(base: dict[str, Any], overrides: dict[str, Any]) -> dict[str, Any]:
    result = copy.deepcopy(base)
    for key, value in overrides.items():
        if (
            key in result
            and isinstance(result[key], dict)
            and isinstance(value, dict)
        ):
            result[key] = deep_merge(result[key], value)
        else:
            result[key] = copy.deepcopy(value)
    return result


def load_grid(path: Path) -> dict[str, Any]:
    document = read_json(path)
    if not isinstance(document, dict):
        raise SystemExit(f"error: {path} must contain an object")
    if document.get("schema") != SCHEMA:
        raise SystemExit(f"error: {path} schema must be {SCHEMA}")
    for field in (
        "program_id",
        "premise",
        "economic_rationale",
        "base_model",
        "hypotheses",
    ):
        if field not in document:
            raise SystemExit(f"error: {path} is missing {field}")
    hypotheses = document["hypotheses"]
    if not isinstance(hypotheses, list) or not hypotheses:
        raise SystemExit(f"error: {path} hypotheses must be a non-empty list")
    seen: set[str] = set()
    for hypothesis in hypotheses:
        if not isinstance(hypothesis, dict):
            raise SystemExit(f"error: {path} hypothesis must be an object")
        hypothesis_id = hypothesis.get("id")
        if not isinstance(hypothesis_id, str) or not hypothesis_id:
            raise SystemExit(f"error: {path} hypothesis id is missing")
        if hypothesis_id in seen:
            raise SystemExit(f"error: duplicate hypothesis {hypothesis_id}")
        seen.add(hypothesis_id)
        if not isinstance(hypothesis.get("events"), list) or not hypothesis["events"]:
            raise SystemExit(f"error: {hypothesis_id} events must be a non-empty list")
        if not isinstance(hypothesis.get("variants"), list) or not hypothesis["variants"]:
            raise SystemExit(f"error: {hypothesis_id} variants must be a non-empty list")
    return document


def cell_id(hypothesis: dict[str, Any], variant: dict[str, Any]) -> str:
    number = str(hypothesis["id"]).split(".", 1)[-1]
    suffix = str(variant["suffix"])
    return f"MR5.{number}-{suffix}"


def relative(path: Path, root: Path) -> str:
    try:
        return str(path.relative_to(root))
    except ValueError:
        return str(path)


def condition_event_ids(condition: dict[str, Any]) -> list[str]:
    values = []
    for key in ("first_event", "second_event"):
        value = condition.get(key)
        if isinstance(value, str) and value and value not in values:
            values.append(value)
    return values


def build_program(grid: dict[str, Any], out: Path, repo_root: Path) -> None:
    if out.exists():
        raise SystemExit(f"error: {out} already exists; refusing to overwrite")
    base_model_path = Path(str(grid["base_model"]))
    if not base_model_path.is_absolute():
        base_model_path = repo_root / base_model_path
    base_model = read_json(base_model_path)
    runs_source = grid.get("runs_source")
    if runs_source:
        source = Path(str(runs_source))
        if not source.is_absolute():
            source = repo_root / source
        if not source.is_dir():
            raise SystemExit(f"error: runs source not found: {source}")
    out.mkdir(parents=True)
    if runs_source:
        shutil.copytree(source, out / "runs")
        for run_path in sorted((out / "runs").rglob("run.json")):
            document = json.loads(run_path.read_text(encoding="utf-8"))
            document["name"] = (
                str(document.get("name", ""))
                .replace("m4-", "mr5-")
                .replace("m3-", "mr5-")
            )
            document["output_root"] = (
                str(document.get("output_root", ""))
                .replace("PROGRAM-2026-11-07-01", str(grid["program_id"]))
                .replace("PROGRAM-2026-11-05-04", str(grid["program_id"]))
            )
            write_json(run_path, document)
        for data_path in sorted((out / "runs").rglob("data.json")):
            document = json.loads(data_path.read_text(encoding="utf-8"))
            document["data_version"] = (
                str(document.get("data_version", ""))
                .replace("m4-", "mr5-")
                .replace("m3-", "mr5-")
            )
            write_json(data_path, document)
    path_menu = grid.get("path_menu")
    if path_menu is not None:
        write_json(out / "path-menu.json", path_menu)
    cells = []
    for hypothesis in grid["hypotheses"]:
        for variant in hypothesis["variants"]:
            identifier = cell_id(hypothesis, variant)
            cell_dir = out / "cells" / identifier
            model = deep_merge(base_model, variant.get("model", {}))
            write_json(cell_dir / "model.json", model)
            condition = deep_merge(
                hypothesis.get("condition", {}), variant.get("condition", {})
            )
            write_json(cell_dir / "condition.json", condition)
            instruments = hypothesis.get("instruments", list(INSTRUMENTS))
            condition_events = condition_event_ids(condition)
            active_events = sorted(
                set(hypothesis["events"]) | set(condition_events)
            )
            write_json(
                cell_dir / "coverage.json",
                {
                    "schema": "quantrule.coverage/v1",
                    "cell_id": identifier,
                    "kind": "primary" if hypothesis.get("primary") and variant["suffix"] == "C01" else "exploratory",
                    "events": hypothesis["events"],
                    "condition_events": condition_events,
                    "instruments": instruments,
                    "timeframe": int(grid.get("timeframe", 900)),
                    "hypothesis": hypothesis.get("hypothesis"),
                },
            )
            for instrument in instruments:
                write_json(
                    cell_dir / f"spec-{instrument}-{int(grid.get('timeframe', 900))}.json",
                    {
                        "schema": "quantrule.model-spec/v2",
                        "id": identifier,
                        "family": "MR5",
                        "instrument": instrument,
                        "timeframes": [int(grid.get("timeframe", 900))],
                        "readings": {},
                        "active_events": active_events,
                        "prediction": {
                            "kind": "direction",
                            "events": hypothesis["events"],
                            "min_votes": 1,
                        },
                    },
                )
            command = [
                "python3",
                "tools/research/model_cell.py",
                "--cell-dir",
                "{config}",
                "--phase",
                "{phase}",
                "--out",
                "{out}",
                "--family",
                "MR5",
                "--timeframe",
                str(int(grid.get("timeframe", 900))),
                "--runs-dir",
                relative(out / "runs", repo_root),
                "--model",
                "{config}/model.json",
                "--condition-file",
                "{config}/condition.json",
                "--horizon",
                "20",
                "--target-atr",
                "2.0",
                "--stop-atr",
                "1.0",
                "--cost-bps",
                "6",
                "--primary-cells",
                str(grid.get("correction", {}).get("primary_cells", 5)),
                "--seed",
                "0",
            ]
            if path_menu is not None:
                command.extend(["--path-menu", relative(out / "path-menu.json", repo_root)])
            cells.append(
                {
                    "cell_id": identifier,
                    "hypothesis": hypothesis.get("hypothesis", ""),
                    "primary": bool(
                        hypothesis.get("primary") and variant["suffix"] == "C01"
                    ),
                    "config_dir": relative(cell_dir, repo_root),
                    "command": command,
                    "result_parser": "model_prediction",
                    "result": "model_prediction.json",
                }
            )
    correction = dict(grid.get("correction", {}))
    primary = [cell for cell in cells if cell["primary"]]
    correction["primary_cells"] = len(primary)
    program = {
        "schema": PROGRAM_SCHEMA,
        "program_id": grid["program_id"],
        "premise": grid["premise"],
        "economic_rationale": grid["economic_rationale"],
        "tier": grid.get("tier", "model"),
        "changed": grid.get("changed", "model"),
        "frozen": grid.get("frozen", ["strategy", "portfolio"]),
        "partition_dev": grid.get("partition_dev", "model_dev"),
        "partition_validation": grid.get("partition_validation", "model_validation"),
        "costs": grid.get("costs", {"commission_bps": 4, "slippage_bps": 2, "funding": 0}),
        "green_criteria": grid.get(
            "green_criteria",
            {
                "min_events": 200,
                "min_clusters": 20,
                "min_lift": 0.02,
                "lift_lcb": "> 0",
            },
        ),
        "correction": correction,
        "max_parallel": int(grid.get("max_parallel", 20)),
        "iteration_cap": int(grid.get("iteration_cap", 1)),
        "validation_access": grid.get("validation_access", "one_shot"),
        "stop_rule": grid.get("stop_rule", "Stop when no primary cell passes."),
        "cells": cells,
    }
    write_json(out / "program.json", program)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--grid", required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument("--repo-root", default=".")
    args = parser.parse_args(argv)
    root = Path(args.repo_root).expanduser().resolve()
    grid_path = Path(args.grid).expanduser()
    if not grid_path.is_absolute():
        grid_path = root / grid_path
    out = Path(args.out).expanduser()
    if not out.is_absolute():
        out = root / out
    grid = load_grid(grid_path)
    build_program(grid, out, root)
    print(f"ok {grid['program_id']} -> {out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
