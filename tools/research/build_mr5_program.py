#!/usr/bin/env python3
"""Build an MR research grid from a declarative grid document.

A grid is one model (`M1.0`), a finite list of mechanism hypotheses, and a
finite list of configuration variants per hypothesis. Every cell gets its own
model config, condition, specs, and coverage record. The builder refuses to
overwrite an existing programme directory.

The same layout serves the MR5 and MR6 families. `family` selects the cell
prefix and the spec family; `timeframe` may be overridden per hypothesis so a
family can mix 15m and 5m cells; `extra_instruments` synthesizes run configs for
an instrument that is not present in `runs_source`.

Usage:
    python3 tools/research/build_mr5_program.py \
        --grid config/research/PROGRAM-<date>-<nn>/grid.json \
        --out config/research/PROGRAM-<date>-<nn>
"""

from __future__ import annotations

import argparse
import copy
import json
import re
import shutil
from pathlib import Path
from typing import Any

SCHEMA = "quantrule.research-grid/v1"
PROGRAM_SCHEMA = "quantrule.research-program/v1"
DEFAULT_FAMILY = "MR5"
DEFAULT_TIMEFRAME = 900
INSTRUMENTS = ("XAU", "BTC", "SP500", "EURUSD")
PROGRAM_ID = re.compile(r"PROGRAM-\d{4}-\d{2}-\d{2}-\d{2}")


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
    family = document.get("family", DEFAULT_FAMILY)
    if not isinstance(family, str) or not re.fullmatch(r"MR\d+", family):
        raise SystemExit(f"error: {path} family must look like MR5 or MR6")
    timeframe = document.get("timeframe", DEFAULT_TIMEFRAME)
    if not isinstance(timeframe, int) or isinstance(timeframe, bool) or timeframe <= 0:
        raise SystemExit(f"error: {path} timeframe must be a positive integer")
    instruments = document.get("instruments", list(INSTRUMENTS))
    if (
        not isinstance(instruments, list)
        or not instruments
        or not all(isinstance(value, str) and value for value in instruments)
    ):
        raise SystemExit(f"error: {path} instruments must be a non-empty string list")
    extra = document.get("extra_instruments", [])
    if not isinstance(extra, list):
        raise SystemExit(f"error: {path} extra_instruments must be a list")
    codes = set()
    for entry in extra:
        if not isinstance(entry, dict):
            raise SystemExit(f"error: {path} extra_instruments entries must be objects")
        code = entry.get("code")
        template = entry.get("template")
        instrument = entry.get("instrument")
        source = entry.get("source")
        if not isinstance(code, str) or not code or code in codes:
            raise SystemExit(f"error: {path} extra instrument code is missing or duplicated")
        codes.add(code)
        if not isinstance(template, str) or not template:
            raise SystemExit(f"error: {path} extra instrument {code} needs a template")
        if not isinstance(instrument, str) or not instrument:
            raise SystemExit(f"error: {path} extra instrument {code} needs an instrument id")
        if not isinstance(source, str) or not source:
            raise SystemExit(f"error: {path} extra instrument {code} needs a source file")
        if code not in instruments:
            raise SystemExit(f"error: {path} extra instrument {code} is not in instruments")
    seen: set[str] = set()
    for hypothesis in hypotheses:
        if not isinstance(hypothesis, dict):
            raise SystemExit(f"error: {path} hypothesis must be an object")
        hypothesis_id = hypothesis.get("id")
        if not isinstance(hypothesis_id, str) or not hypothesis_id:
            raise SystemExit(f"error: {path} hypothesis id is missing")
        if not re.fullmatch(rf"{family}\.\d+", hypothesis_id):
            raise SystemExit(f"error: {hypothesis_id} must match {family}.<n>")
        if hypothesis_id in seen:
            raise SystemExit(f"error: duplicate hypothesis {hypothesis_id}")
        seen.add(hypothesis_id)
        hypothesis_timeframe = hypothesis.get("timeframe", timeframe)
        if (
            not isinstance(hypothesis_timeframe, int)
            or isinstance(hypothesis_timeframe, bool)
            or hypothesis_timeframe <= 0
        ):
            raise SystemExit(f"error: {hypothesis_id} timeframe must be a positive integer")
        if not isinstance(hypothesis.get("events"), list) or not hypothesis["events"]:
            raise SystemExit(f"error: {hypothesis_id} events must be a non-empty list")
        if not isinstance(hypothesis.get("variants"), list) or not hypothesis["variants"]:
            raise SystemExit(f"error: {hypothesis_id} variants must be a non-empty list")
    return document


def cell_id(family: str, hypothesis: dict[str, Any], variant: dict[str, Any]) -> str:
    number = str(hypothesis["id"]).split(".", 1)[-1]
    suffix = str(variant["suffix"])
    return f"{family}.{number}-{suffix}"


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
    for key in ("first_events", "second_events"):
        value = condition.get(key)
        if isinstance(value, list):
            for event_id in value:
                if isinstance(event_id, str) and event_id and event_id not in values:
                    values.append(event_id)
    return values


def run_prefix(grid: dict[str, Any]) -> str:
    prefix = grid.get("run_prefix")
    if isinstance(prefix, str) and prefix:
        return prefix
    return str(grid.get("family", DEFAULT_FAMILY)).lower()


def rewrite_prefix(value: str, prefix: str) -> str:
    return re.sub(r"^(?:m\d+|mr\d+)-", f"{prefix}-", value)


def synthesize_instrument_runs(
    runs: Path,
    entry: dict[str, Any],
    prefix: str,
    program_id: str,
) -> None:
    template = runs / str(entry["template"])
    target = runs / str(entry["code"])
    if not template.is_dir():
        raise SystemExit(f"error: run template not found: {template}")
    if target.exists():
        raise SystemExit(f"error: run target already exists: {target}")
    target.mkdir(parents=True)
    policy = json.loads((template / "policy.json").read_text(encoding="utf-8"))
    policy["universe"]["tickets"] = [str(entry["instrument"])]
    write_json(target / "policy.json", policy)
    template_code = str(entry["template"]).lower()
    target_code = str(entry["code"]).lower()
    for year_dir in sorted(path for path in template.iterdir() if path.is_dir()):
        year = year_dir.name
        year_target = target / year
        year_target.mkdir()
        run = json.loads((year_dir / "run.json").read_text(encoding="utf-8"))
        run["name"] = rewrite_prefix(str(run.get("name", "")), prefix).replace(
            f"-{template_code}-", f"-{target_code}-"
        )
        run["output_root"] = PROGRAM_ID.sub(
            program_id, str(run.get("output_root", ""))
        )
        write_json(year_target / "run.json", run)
        data = json.loads((year_dir / "data.json").read_text(encoding="utf-8"))
        data["instrument"] = str(entry["instrument"])
        data["data_version"] = rewrite_prefix(
            str(data.get("data_version", "")), prefix
        ).replace(f"-{template_code}-m1-", f"-{target_code}-m1-")
        data["source"] = dict(data.get("source", {}))
        data["source"]["path"] = f"../../../../../../data/sources/{entry['source']}"
        write_json(year_target / "data.json", data)


def build_program(grid: dict[str, Any], out: Path, repo_root: Path) -> None:
    if out.exists():
        raise SystemExit(f"error: {out} already exists; refusing to overwrite")
    family = str(grid.get("family", DEFAULT_FAMILY))
    prefix = run_prefix(grid)
    default_timeframe = int(grid.get("timeframe", DEFAULT_TIMEFRAME))
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
            document["name"] = rewrite_prefix(
                str(document.get("name", "")), prefix
            )
            document["output_root"] = PROGRAM_ID.sub(
                str(grid["program_id"]), str(document.get("output_root", ""))
            )
            write_json(run_path, document)
        for data_path in sorted((out / "runs").rglob("data.json")):
            document = json.loads(data_path.read_text(encoding="utf-8"))
            document["data_version"] = rewrite_prefix(
                str(document.get("data_version", "")), prefix
            )
            write_json(data_path, document)
        for entry in grid.get("extra_instruments", []):
            synthesize_instrument_runs(
                out / "runs", entry, prefix, str(grid["program_id"])
            )
    path_menu = grid.get("path_menu")
    if path_menu is not None:
        write_json(out / "path-menu.json", path_menu)
    cells = []
    for hypothesis in grid["hypotheses"]:
        timeframe = int(hypothesis.get("timeframe", default_timeframe))
        for variant in hypothesis["variants"]:
            identifier = cell_id(family, hypothesis, variant)
            cell_dir = out / "cells" / identifier
            model = deep_merge(base_model, variant.get("model", {}))
            write_json(cell_dir / "model.json", model)
            condition = deep_merge(
                hypothesis.get("condition", {}), variant.get("condition", {})
            )
            write_json(cell_dir / "condition.json", condition)
            instruments = hypothesis.get(
                "instruments", grid.get("instruments", list(INSTRUMENTS))
            )
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
                    "timeframe": timeframe,
                    "hypothesis": hypothesis.get("hypothesis"),
                },
            )
            for instrument in instruments:
                write_json(
                    cell_dir / f"spec-{instrument}-{timeframe}.json",
                    {
                        "schema": "quantrule.model-spec/v2",
                        "id": identifier,
                        "family": family,
                        "instrument": instrument,
                        "timeframes": [timeframe],
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
                family,
                "--timeframe",
                str(timeframe),
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
        "family": family,
        "run_prefix": prefix,
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
