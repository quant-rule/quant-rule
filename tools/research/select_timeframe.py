#!/usr/bin/env python3
"""Select the M3 primary timeframe with a count-only preflight.

The selector runs every primary cell at 1h and 15m, counts the state-matched
predictions and clusters without joining a forward outcome, and chooses 1h when
every primary cell clears the sample gate, otherwise 15m when every cell clears
it, otherwise stops as underpowered.

Usage:
    python3 tools/research/select_timeframe.py \
        --program config/research/PROGRAM-2026-11-05-04/program.json
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path
from typing import Any

import program as program_module

ROOT = Path(__file__).resolve().parents[2]
TIMEFRAMES = (3600, 900)
MIN_PREDICTIONS = 200
MIN_CLUSTERS = 20


def parse_arguments(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--program",
        default="config/research/PROGRAM-2026-11-05-04/program.json",
    )
    parser.add_argument("--qr", default="target/release/quantrule")
    parser.add_argument(
        "--out",
        default="data/research/PROGRAM-2026-11-05-04/preflight",
    )
    parser.add_argument("--max-parallel", type=int, default=5)
    parser.add_argument("--timeout", type=int, default=3_600)
    return parser.parse_args(argv)


def run_count(
    cell: program_module.Cell,
    timeframe: int,
    out_root: Path,
    qr: str,
    runs_dir: Path,
    timeout: int,
) -> tuple[str, int, dict[str, Any]]:
    destination = out_root / str(timeframe) / cell.cell_id
    destination.mkdir(parents=True, exist_ok=True)
    command = [
        sys.executable,
        "tools/research/model_cell.py",
        "--cell-dir",
        cell.config_dir,
        "--phase",
        "dev",
        "--out",
        str(destination),
        "--timeframe",
        str(timeframe),
        "--count-only",
        "--family",
        "M3",
        "--runs-dir",
        str(runs_dir),
        "--qr",
        qr,
        "--repo-root",
        ".",
    ]
    state = Path(cell.config_dir) / "state.json"
    if state.is_file():
        command.extend(["--state-file", str(state)])
    completed = subprocess.run(
        command,
        cwd=ROOT,
        capture_output=True,
        text=True,
        timeout=timeout,
        check=False,
    )
    if completed.returncode != 0:
        message = completed.stderr.strip() or completed.stdout.strip()
        raise SystemExit(
            f"error: count {cell.cell_id} {timeframe}s failed: {message}"
        )
    document = json.loads((destination / "count.json").read_text(encoding="utf-8"))
    return cell.cell_id, timeframe, document


def main(argv: list[str] | None = None) -> int:
    args = parse_arguments(argv)
    program_path = ROOT / args.program
    loaded = program_module.load_program(program_path, repo_root=ROOT)
    primary = [cell for cell in loaded.cells if cell.primary]
    if len(primary) != 5:
        raise SystemExit(f"error: expected 5 primary cells, found {len(primary)}")
    runs_dir = program_path.parent / "runs"
    out_root = ROOT / args.out
    results: dict[str, dict[str, dict[str, Any]]] = {
        cell.cell_id: {} for cell in primary
    }
    for timeframe in TIMEFRAMES:
        with ThreadPoolExecutor(max_workers=args.max_parallel) as executor:
            futures = [
                executor.submit(
                    run_count,
                    cell,
                    timeframe,
                    out_root,
                    args.qr,
                    runs_dir,
                    args.timeout,
                )
                for cell in primary
            ]
            for future in as_completed(futures):
                cell_id, _, document = future.result()
                results[cell_id][str(timeframe)] = {
                    "n": document["pooled"]["n"],
                    "clusters": document["pooled"]["clusters"],
                    "instruments": document["counts"],
                }
                print(
                    f"ok {cell_id} {timeframe}s n={document['pooled']['n']} "
                    f"clusters={document['pooled']['clusters']}"
                )
    selected = None
    for timeframe in TIMEFRAMES:
        if all(
            results[cell.cell_id][str(timeframe)]["n"] >= MIN_PREDICTIONS
            and results[cell.cell_id][str(timeframe)]["clusters"] >= MIN_CLUSTERS
            for cell in primary
        ):
            selected = timeframe
            break
    selection = {
        "schema": "quantrule.m3-selection/v1",
        "program_id": loaded.program_id,
        "partition": "model_dev",
        "rule": (
            "1h if every primary cell has 200 state-matched predictions and 20 "
            "clusters, else 15m if every cell has them, else underpowered"
        ),
        "criteria": {
            "min_predictions": MIN_PREDICTIONS,
            "min_clusters": MIN_CLUSTERS,
        },
        "selected_timeframe_secs": selected,
        "status": "selected" if selected is not None else "underpowered",
        "cells": {
            cell.cell_id: {
                timeframe: results[cell.cell_id][timeframe]
                for timeframe in sorted(results[cell.cell_id])
            }
            for cell in primary
        },
    }
    destination = program_path.parent / "selection.json"
    destination.write_text(
        json.dumps(selection, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )
    print(
        f"{selection['status']}: timeframe={selected} -> {destination}"
    )
    return 0 if selected is not None else 1


if __name__ == "__main__":
    raise SystemExit(main())
