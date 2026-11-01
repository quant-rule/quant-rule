#!/usr/bin/env python3
"""Write the tier-specific evidence summary for one improvement run.

The command reads the run's published report and extracts only numbers the
report contains. A missing report leaves every field null with a note; it never
fills a value with zero, a guess, or a confidence number the model did not
publish.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path
from typing import Any

from common import (
    EVIDENCE_SCHEMA,
    TIERS,
    ToolError,
    model_metrics,
    portfolio_metrics,
    read_json,
    write_json,
)


def parse_arguments(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        prog="evidence.py",
        description=(
            "Read a run folder's artifacts and write artifacts/evidence.json. "
            "Strategy and portfolio tiers use artifacts/portfolio/report.json; "
            "the model tier uses artifacts/model/report.json."
        ),
        epilog=(
            "Example: python3 tools/improvement/evidence.py "
            "data/improvement/2026-11-01/MODEL-2026-11-01-01"
        ),
    )
    parser.add_argument("run_dir", help="improvement run directory")
    return parser.parse_args(argv)


def empty_portfolio_summary() -> dict[str, Any]:
    return {
        "return": None,
        "max_drawdown": None,
        "trades": None,
        "profit_factor": None,
        "win_rate": None,
        "halted": None,
        "final_equity": None,
    }


def empty_model_summary() -> dict[str, Any]:
    return {
        "model_version": None,
        "view_contract": None,
        "data_version": None,
        "requested_period": None,
        "ladder_seconds": None,
        "timeframes": [],
    }


def main(argv: list[str] | None = None) -> int:
    args = parse_arguments(argv)
    run_dir = Path(args.run_dir).expanduser()
    if not run_dir.is_dir():
        print(f"error: run directory not found: {run_dir}", file=sys.stderr)
        return 1

    manifest_path = run_dir / "manifest.json"
    if not manifest_path.is_file():
        print(f"error: manifest not found: {manifest_path}", file=sys.stderr)
        return 1
    try:
        manifest = read_json(manifest_path)
    except ToolError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    if not isinstance(manifest, dict):
        print(f"error: {manifest_path}: manifest must be a JSON object", file=sys.stderr)
        return 1
    tier = manifest.get("tier")
    if tier not in TIERS:
        print(f"error: {manifest_path}: tier must be one of {', '.join(TIERS)}", file=sys.stderr)
        return 1
    run_id = manifest.get("run_id")
    if not isinstance(run_id, str) or not run_id:
        print(f"error: {manifest_path}: run_id is missing", file=sys.stderr)
        return 1

    artifacts = run_dir / "artifacts"
    try:
        artifacts.mkdir(parents=True, exist_ok=True)
    except OSError as error:
        print(f"error: cannot create {artifacts}: {error}", file=sys.stderr)
        return 1

    notes: list[str] = []
    if tier == "model":
        source = artifacts / "model" / "report.json"
        source_name = "artifacts/model/report.json"
        summary = empty_model_summary()
    else:
        source = artifacts / "portfolio" / "report.json"
        source_name = "artifacts/portfolio/report.json"
        summary = empty_portfolio_summary()

    if not source.is_file():
        notes.append(f"{source_name} is missing; summary fields are null")
    else:
        try:
            report = read_json(source)
        except ToolError as error:
            print(f"error: {error}", file=sys.stderr)
            return 1
        if tier == "model":
            summary, extraction_notes = model_metrics(report)
        else:
            summary, extraction_notes = portfolio_metrics(report)
        notes.extend(extraction_notes)

    document = {
        "schema": EVIDENCE_SCHEMA,
        "run_id": run_id,
        "tier": tier,
        "source": source_name if source.is_file() else None,
        "summary": summary,
        "notes": notes,
    }
    destination = artifacts / "evidence.json"
    try:
        write_json(destination, document)
    except ToolError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1

    print(f"ok {run_id} evidence -> {destination}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
