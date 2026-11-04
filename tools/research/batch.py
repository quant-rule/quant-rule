#!/usr/bin/env python3
"""Run pre-registered research cells concurrently.

Config mode runs every cell in the current worktree with an isolated output
directory. Worktree mode creates one Git worktree per cell, prepares the cell
brief, and can run the cell inside it. Both modes write a batch summary and
never open a validation phase unless the program allows one-shot access.

Usage:
    python3 tools/research/batch.py \
        --program config/research/<id>/program.json \
        --phase dev \
        --mode config \
        --max-parallel 5
"""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path
from typing import Any

import cell as cell_module
import program as program_module

SCHEMA = "quantrule.research-batch/v1"


def selected_cells(
    program: program_module.Program, raw: str | None
) -> tuple[program_module.Cell, ...]:
    if raw is None:
        return program.cells
    ids = [value.strip() for value in raw.split(",") if value.strip()]
    if not ids:
        raise program_module.ProgramError("--cells selected no cell")
    return tuple(program.cell(cell_id) for cell_id in ids)


def run_config(
    program: program_module.Program,
    phase: str,
    cells: tuple[program_module.Cell, ...],
    max_parallel: int,
    out_root: Path,
) -> dict[str, Any]:
    out_root.mkdir(parents=True, exist_ok=True)
    results: dict[str, dict[str, Any]] = {}
    with ThreadPoolExecutor(max_workers=min(max_parallel, len(cells))) as pool:
        futures = {
            pool.submit(
                cell_module.run_cell,
                program,
                cell,
                phase,
                out_root / cell.cell_id,
            ): cell
            for cell in cells
        }
        for future in as_completed(futures):
            cell = futures[future]
            try:
                record = future.result()
            except Exception as error:  # noqa: BLE001 - recorded as a cell failure
                record = {
                    "cell_id": cell.cell_id,
                    "verdict": "fail",
                    "metrics": {},
                    "limitations": [f"runner error: {error}"],
                }
            results[cell.cell_id] = record
    entries = [
        {
            "cell_id": cell_id,
            "out": str(out_root / cell_id),
            "verdict": record.get("verdict", "fail"),
        }
        for cell_id, record in sorted(results.items())
    ]
    summary = {
        "schema": SCHEMA,
        "program_id": program.program_id,
        "phase": phase,
        "mode": "config",
        "max_parallel": max_parallel,
        "cells": entries,
    }
    (out_root / "batch-summary.json").write_text(
        json.dumps(summary, indent=2) + "\n", encoding="utf-8"
    )
    return summary


def prepare_worktrees(
    program: program_module.Program,
    cells: tuple[program_module.Cell, ...],
    root: Path,
    base: str,
) -> dict[str, Any]:
    root.mkdir(parents=True, exist_ok=True)
    entries = []
    for cell in cells:
        path = root / cell.cell_id
        if path.exists():
            raise program_module.ProgramError(f"worktree path already exists: {path}")
        branch = f"research/{program.program_id}-{cell.cell_id}"
        command = [
            "git",
            "-C",
            str(program.repo_root),
            "worktree",
            "add",
            "-b",
            branch,
            str(path),
            base,
        ]
        completed = subprocess.run(command, capture_output=True, text=True, check=False)
        if completed.returncode != 0:
            raise program_module.ProgramError(
                f"git worktree add failed for {cell.cell_id}: {completed.stderr.strip()}"
            )
        entries.append(
            {
                "cell_id": cell.cell_id,
                "worktree": str(path),
                "branch": branch,
                "status": "prepared",
            }
        )
    return {
        "schema": SCHEMA,
        "program_id": program.program_id,
        "phase": "prepare",
        "mode": "worktree",
        "cells": entries,
    }


def run_worktrees(
    program: program_module.Program,
    phase: str,
    cells: tuple[program_module.Cell, ...],
    root: Path,
    base: str,
    max_parallel: int,
    out_root: Path,
) -> dict[str, Any]:
    prepared = prepare_worktrees(program, cells, root, base)
    relative_program = program.path
    try:
        relative_program = program.path.relative_to(program.repo_root)
    except ValueError:
        pass
    out_root.mkdir(parents=True, exist_ok=True)

    def run_one(entry: dict[str, Any]) -> dict[str, Any]:
        worktree = Path(entry["worktree"])
        cell = program.cell(entry["cell_id"])
        cell_out = worktree / "data" / "research" / program.program_id / phase / cell.cell_id
        command = [
            sys.executable,
            "tools/research/cell.py",
            "--program",
            str(relative_program),
            "--cell",
            cell.cell_id,
            "--phase",
            phase,
            "--out",
            str(cell_out),
        ]
        completed = subprocess.run(
            command, cwd=worktree, capture_output=True, text=True, check=False
        )
        copied = out_root / cell.cell_id / "cell-result.json"
        copied.parent.mkdir(parents=True, exist_ok=True)
        source = cell_out / "cell-result.json"
        if source.is_file():
            shutil.copy2(source, copied)
        return {
            "cell_id": cell.cell_id,
            "worktree": str(worktree),
            "branch": entry["branch"],
            "out": str(cell_out),
            "exit_code": completed.returncode,
            "stderr": completed.stderr.strip(),
            "status": "ran",
        }

    entries = []
    with ThreadPoolExecutor(max_workers=min(max_parallel, len(cells))) as pool:
        futures = {pool.submit(run_one, entry): entry for entry in prepared["cells"]}
        for future in as_completed(futures):
            entries.append(future.result())
    summary = {
        "schema": SCHEMA,
        "program_id": program.program_id,
        "phase": phase,
        "mode": "worktree",
        "max_parallel": max_parallel,
        "cells": sorted(entries, key=lambda value: value["cell_id"]),
    }
    (out_root / "batch-summary.json").write_text(
        json.dumps(summary, indent=2) + "\n", encoding="utf-8"
    )
    return summary


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--program", required=True)
    parser.add_argument("--phase", default="dev")
    parser.add_argument("--mode", choices=("config", "worktree"), default="config")
    parser.add_argument("--max-parallel", type=int)
    parser.add_argument("--cells")
    parser.add_argument("--worktree-root")
    parser.add_argument("--base", default="HEAD")
    parser.add_argument("--prepare-only", action="store_true")
    parser.add_argument("--out")
    args = parser.parse_args(argv)
    try:
        loaded = program_module.load_program(args.program)
        program_module.require_phase(loaded, args.phase)
        if args.phase == "validation" and args.cells is None:
            raise program_module.ProgramError("validation requires --cells promotion")
        cells = selected_cells(loaded, args.cells)
        max_parallel = args.max_parallel or loaded.max_parallel
        if max_parallel < 1 or max_parallel > 64:
            raise program_module.ProgramError("--max-parallel must be in 1..64")
        out_root = (
            Path(args.out).expanduser()
            if args.out
            else loaded.repo_root
            / "data"
            / "research"
            / loaded.program_id
            / args.phase
        )
        if args.mode == "config":
            summary = run_config(loaded, args.phase, cells, max_parallel, out_root)
        else:
            if args.worktree_root is None:
                raise program_module.ProgramError("--worktree-root is required")
            root = Path(args.worktree_root).expanduser()
            if args.prepare_only:
                summary = prepare_worktrees(loaded, cells, root, args.base)
                (out_root).mkdir(parents=True, exist_ok=True)
                (out_root / "batch-summary.json").write_text(
                    json.dumps(summary, indent=2) + "\n", encoding="utf-8"
                )
            else:
                summary = run_worktrees(
                    loaded,
                    args.phase,
                    cells,
                    root,
                    args.base,
                    max_parallel,
                    out_root,
                )
    except (program_module.ProgramError, OSError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    print(f"ok batch {summary['program_id']} {summary['phase']} cells={len(summary['cells'])}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
