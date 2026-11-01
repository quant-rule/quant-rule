#!/usr/bin/env python3
"""Compare the portfolio report of the v3, v4, v5, and v6 strategy cells.

The command scans ``<run-root>/<cell>/portfolio/report.json`` and emits only the
cells that are actually present. A version with no cell is reported as missing in
the JSON summary; it is never given a fabricated row.
"""

from __future__ import annotations

import argparse
import csv
import json
import re
import sys
from pathlib import Path
from typing import Any

from common import ToolError, portfolio_metrics, read_json

CELL_RE = re.compile(r"^(v[3-6])(?:[-_].*)?$")
VERSIONS = ("v3", "v4", "v5", "v6")
CSV_HEADER = (
    "cell",
    "version",
    "return",
    "max_drawdown",
    "trades",
    "profit_factor",
    "win_rate",
    "halted",
    "final_equity",
    "source",
)


def parse_arguments(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        prog="strategy_compare.py",
        description=(
            "Build a deterministic comparison table from the v3-v6 strategy "
            "cells that exist under a run root. Missing cells are not invented."
        ),
        epilog=(
            "Example: python3 tools/improvement/strategy_compare.py "
            "data/improvement/2026-11-01/STRATEGY-2026-11-01-01"
        ),
    )
    parser.add_argument("run_root", help="directory holding the strategy cells")
    parser.add_argument(
        "--format",
        choices=("json", "csv"),
        default="json",
        help="stdout format (default: json)",
    )
    return parser.parse_args(argv)


def compare(root: Path) -> tuple[list[dict[str, Any]], list[str]]:
    cells: list[dict[str, Any]] = []
    present_versions: set[str] = set()
    for cell_dir in sorted(path for path in root.iterdir() if path.is_dir()):
        match = CELL_RE.fullmatch(cell_dir.name)
        if match is None:
            continue
        report_path = cell_dir / "portfolio" / "report.json"
        if not report_path.is_file():
            continue
        try:
            report = read_json(report_path)
        except ToolError as error:
            raise ToolError(str(error)) from None
        metrics, notes = portfolio_metrics(report)
        version = match.group(1)
        present_versions.add(version)
        cells.append(
            {
                "cell": cell_dir.name,
                "version": version,
                "return": metrics["return"],
                "max_drawdown": metrics["max_drawdown"],
                "trades": metrics["trades"],
                "profit_factor": metrics["profit_factor"],
                "win_rate": metrics["win_rate"],
                "halted": metrics["halted"],
                "final_equity": metrics["final_equity"],
                "source": f"{cell_dir.name}/portfolio/report.json",
                "notes": notes,
            }
        )
    cells.sort(key=lambda cell: (int(cell["version"][1:]), cell["cell"]))
    missing = [version for version in VERSIONS if version not in present_versions]
    return cells, missing


def format_value(value: Any) -> str:
    if value is None:
        return ""
    if isinstance(value, bool):
        return "true" if value else "false"
    return str(value)


def write_csv(cells: list[dict[str, Any]]) -> None:
    writer = csv.writer(sys.stdout, lineterminator="\n")
    writer.writerow(CSV_HEADER)
    for cell in cells:
        writer.writerow(
            [
                cell["cell"],
                cell["version"],
                format_value(cell["return"]),
                format_value(cell["max_drawdown"]),
                format_value(cell["trades"]),
                format_value(cell["profit_factor"]),
                format_value(cell["win_rate"]),
                format_value(cell["halted"]),
                format_value(cell["final_equity"]),
                cell["source"],
            ]
        )


def main(argv: list[str] | None = None) -> int:
    args = parse_arguments(argv)
    root = Path(args.run_root).expanduser()
    if not root.is_dir():
        print(f"error: run root not found: {root}", file=sys.stderr)
        return 1
    try:
        cells, missing = compare(root)
    except ToolError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1

    document = {
        "schema": "quantrule.strategy-compare/v1",
        "run_root": args.run_root,
        "cells": cells,
        "missing": missing,
    }
    if args.format == "csv":
        write_csv(cells)
        return 0

    try:
        text = json.dumps(document, indent=2, ensure_ascii=False, allow_nan=False)
    except (TypeError, ValueError) as error:
        print(f"error: cannot encode comparison: {error}", file=sys.stderr)
        return 1
    sys.stdout.write(text + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
