#!/usr/bin/env python3
"""Research program manifest: schema, validation, and CLI.

A program is a premise plus a finite set of pre-registered cells. The manifest
is data, not code. It declares the cells, the partitions, the correction, the
budget, and the stop rule before the first run.

Usage:
    python3 tools/research/program.py --program config/research/<id>/program.json
    python3 tools/research/program.py --program ... --phase validation
"""

from __future__ import annotations

import argparse
import json
import re
from dataclasses import dataclass
from pathlib import Path
from typing import Any

SCHEMA = "quantrule.research-program/v1"
PROGRAM_ID = re.compile(r"^PROGRAM-\d{4}-\d{2}-\d{2}-\d{2}$")
TIERS = {"model", "strategy", "portfolio"}
PARTITIONS = {
    "model_dev",
    "model_validation",
    "strategy_dev",
    "strategy_validation",
    "portfolio_dev",
    "portfolio_holdout",
}
PARSERS = {
    "path_validation",
    "strategy_evaluation",
    "model_prediction",
    "summary_json",
    "none",
}


class ProgramError(ValueError):
    """A program manifest is invalid or a phase is not allowed."""


@dataclass(frozen=True)
class Cell:
    cell_id: str
    hypothesis: str
    primary: bool
    config_dir: str
    command: tuple[str, ...]
    result_parser: str
    result: str


@dataclass(frozen=True)
class Program:
    path: Path
    repo_root: Path
    program_id: str
    premise: str
    economic_rationale: str
    tier: str
    changed: str
    frozen: tuple[str, ...]
    partition_dev: str
    partition_validation: str
    costs: dict[str, Any]
    green_criteria: dict[str, Any]
    correction: dict[str, Any]
    max_parallel: int
    iteration_cap: int
    validation_access: str
    stop_rule: str
    cells: tuple[Cell, ...]

    @property
    def primary_cells(self) -> tuple[Cell, ...]:
        return tuple(cell for cell in self.cells if cell.primary)

    def cell(self, cell_id: str) -> Cell:
        for cell in self.cells:
            if cell.cell_id == cell_id:
                return cell
        raise ProgramError(f"unknown cell: {cell_id}")


def _text(value: Any, path: str) -> str:
    if not isinstance(value, str) or not value.strip():
        raise ProgramError(f"{path} must be a non-empty string")
    return value.strip()


def _mapping(value: Any, path: str) -> dict[str, Any]:
    if not isinstance(value, dict) or not value:
        raise ProgramError(f"{path} must be a non-empty object")
    return value


def _integer(value: Any, path: str, minimum: int, maximum: int) -> int:
    if not isinstance(value, int) or isinstance(value, bool):
        raise ProgramError(f"{path} must be an integer")
    if value < minimum or value > maximum:
        raise ProgramError(f"{path} must be in {minimum}..{maximum}")
    return value


def load_program(path: str | Path, repo_root: str | Path | None = None) -> Program:
    """Load and validate a program manifest."""
    manifest = Path(path).expanduser()
    if not manifest.is_file():
        raise ProgramError(f"program manifest not found: {manifest}")
    root = Path(repo_root).expanduser() if repo_root is not None else Path.cwd()
    try:
        document = json.loads(manifest.read_text(encoding="utf-8"))
    except json.JSONDecodeError as error:
        raise ProgramError(f"invalid JSON in {manifest}: {error}") from None
    if not isinstance(document, dict):
        raise ProgramError("program manifest must be an object")

    if document.get("schema") != SCHEMA:
        raise ProgramError(f"$.schema must be {SCHEMA}")
    program_id = _text(document.get("program_id"), "$.program_id")
    if not PROGRAM_ID.match(program_id):
        raise ProgramError("$.program_id must match PROGRAM-YYYY-MM-DD-NN")

    premise = _text(document.get("premise"), "$.premise")
    rationale = _text(document.get("economic_rationale"), "$.economic_rationale")
    tier = _text(document.get("tier"), "$.tier")
    if tier not in TIERS:
        raise ProgramError("$.tier must be model, strategy, or portfolio")
    changed = _text(document.get("changed"), "$.changed")
    if changed != tier:
        raise ProgramError("$.changed must equal $.tier")

    frozen = document.get("frozen")
    if not isinstance(frozen, list) or not all(
        isinstance(item, str) and item in TIERS for item in frozen
    ):
        raise ProgramError("$.frozen must be a list of model/strategy/portfolio")
    if changed in frozen or len(set(frozen)) != len(frozen):
        raise ProgramError("$.frozen must not contain $.changed or duplicates")

    partition_dev = _text(document.get("partition_dev"), "$.partition_dev")
    partition_validation = _text(
        document.get("partition_validation"), "$.partition_validation"
    )
    if partition_dev not in PARTITIONS or partition_validation not in PARTITIONS:
        raise ProgramError("$.partition_dev and $.partition_validation must be declared")

    costs = _mapping(document.get("costs"), "$.costs")
    for key in ("commission_bps", "slippage_bps", "funding"):
        value = costs.get(key)
        if not isinstance(value, (int, float)) or isinstance(value, bool) or value < 0:
            raise ProgramError(f"$.costs.{key} must be a non-negative number")
    green = _mapping(document.get("green_criteria"), "$.green_criteria")

    correction = _mapping(document.get("correction"), "$.correction")
    method = correction.get("method")
    if method not in {"bonferroni", "none"}:
        raise ProgramError("$.correction.method must be bonferroni or none")
    primary_cells = correction.get("primary_cells")
    primary_cells = _integer(primary_cells, "$.correction.primary_cells", 1, 64)
    if method == "none" and primary_cells != 1:
        raise ProgramError("$.correction.method none requires one primary cell")

    max_parallel = _integer(document.get("max_parallel"), "$.max_parallel", 1, 64)
    iteration_cap = _integer(document.get("iteration_cap"), "$.iteration_cap", 1, 100)
    validation_access = _text(
        document.get("validation_access"), "$.validation_access"
    )
    if validation_access not in {"none", "one_shot"}:
        raise ProgramError("$.validation_access must be none or one_shot")
    stop_rule = _text(document.get("stop_rule"), "$.stop_rule")

    raw_cells = document.get("cells")
    if not isinstance(raw_cells, list) or not raw_cells:
        raise ProgramError("$.cells must be a non-empty list")
    if len(raw_cells) > 64:
        raise ProgramError("$.cells exceeds 64")
    cells: list[Cell] = []
    seen: set[str] = set()
    for index, raw in enumerate(raw_cells):
        path_prefix = f"$.cells[{index}]"
        if not isinstance(raw, dict):
            raise ProgramError(f"{path_prefix} must be an object")
        cell_id = _text(raw.get("cell_id"), f"{path_prefix}.cell_id")
        if cell_id in seen:
            raise ProgramError(f"duplicate cell_id: {cell_id}")
        seen.add(cell_id)
        hypothesis = _text(raw.get("hypothesis"), f"{path_prefix}.hypothesis")
        primary = raw.get("primary")
        if not isinstance(primary, bool):
            raise ProgramError(f"{path_prefix}.primary must be a boolean")
        config_dir = _text(raw.get("config_dir"), f"{path_prefix}.config_dir")
        test_dir = (root / config_dir).expanduser()
        if not test_dir.is_dir():
            raise ProgramError(f"{path_prefix}.config_dir does not exist: {config_dir}")
        command = raw.get("command")
        if (
            not isinstance(command, list)
            or not command
            or not all(isinstance(item, str) and item for item in command)
        ):
            raise ProgramError(f"{path_prefix}.command must be a non-empty string list")
        if not any("{out}" in item for item in command):
            raise ProgramError(f"{path_prefix}.command must contain {{out}}")
        parser = raw.get("result_parser", "none")
        if parser not in PARSERS:
            raise ProgramError(f"{path_prefix}.result_parser is not registered")
        result = raw.get("result", "")
        if result and not isinstance(result, str):
            raise ProgramError(f"{path_prefix}.result must be a string")
        cells.append(
            Cell(
                cell_id=cell_id,
                hypothesis=hypothesis,
                primary=primary,
                config_dir=config_dir,
                command=tuple(command),
                result_parser=parser,
                result=result,
            )
        )

    primary_count = sum(1 for cell in cells if cell.primary)
    if primary_count == 0:
        raise ProgramError("$.cells must contain at least one primary cell")
    if primary_count != primary_cells:
        raise ProgramError(
            "$.correction.primary_cells must equal the number of primary cells"
        )

    return Program(
        path=manifest,
        repo_root=root,
        program_id=program_id,
        premise=premise,
        economic_rationale=rationale,
        tier=tier,
        changed=changed,
        frozen=tuple(frozen),
        partition_dev=partition_dev,
        partition_validation=partition_validation,
        costs=costs,
        green_criteria=green,
        correction=correction,
        max_parallel=max_parallel,
        iteration_cap=iteration_cap,
        validation_access=validation_access,
        stop_rule=stop_rule,
        cells=tuple(cells),
    )


def require_phase(program: Program, phase: str) -> None:
    """Reject a validation phase when the program has no validation access."""
    if phase not in {"dev", "validation"}:
        raise ProgramError("phase must be dev or validation")
    if phase == "validation" and program.validation_access != "one_shot":
        raise ProgramError("validation_access is not one_shot")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--program", required=True)
    parser.add_argument("--phase", default="dev")
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args(argv)
    try:
        program = load_program(args.program)
        require_phase(program, args.phase)
    except ProgramError as error:
        print(f"error: {error}")
        return 1
    print(
        f"ok {program.program_id} cells={len(program.cells)} "
        f"primary={len(program.primary_cells)} phase={args.phase}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
