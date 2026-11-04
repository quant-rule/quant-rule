#!/usr/bin/env python3
"""Aggregate a research program and report every cell.

The report reads the cell results, verifies the declared correction, ranks only
the primary cells, and writes program-results.json/md. It never opens
validation and never accepts a baseline.

Usage:
    python3 tools/research/report.py \
        --program config/research/<id>/program.json \
        --results data/research/<id>/dev \
        --out data/research/<id>/program-results
"""

from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path
from statistics import NormalDist
from typing import Any

import program as program_module

SCHEMA = "quantrule.research-program-results/v1"


def read_result(path: Path) -> dict[str, Any] | None:
    if not path.is_file():
        return None
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError:
        return None
    return value if isinstance(value, dict) else None


def metric(record: dict[str, Any] | None, key: str) -> Any:
    if not record:
        return None
    metrics = record.get("metrics")
    if not isinstance(metrics, dict):
        return None
    return metrics.get(key)


def numeric(value: Any) -> float | None:
    if isinstance(value, (int, float)) and not isinstance(value, bool):
        if math.isfinite(float(value)):
            return float(value)
    return None


def rank_value(record: dict[str, Any] | None) -> float:
    for key in ("lift_lcb", "average_instrument_return", "profit_factor"):
        value = numeric(metric(record, key))
        if value is not None:
            return value
    return -math.inf


def correction(program: program_module.Program) -> dict[str, Any]:
    method = str(program.correction.get("method", "bonferroni"))
    cells = int(program.correction.get("primary_cells", 1))
    if method == "bonferroni":
        alpha = 0.05 / cells
        z = NormalDist().inv_cdf(1.0 - alpha / 2.0)
    else:
        alpha = 0.05
        z = NormalDist().inv_cdf(0.975)
    return {
        "method": method,
        "primary_cells": cells,
        "family_alpha": 0.05,
        "cell_alpha": alpha,
        "z": z,
    }


def build(program: program_module.Program, results: Path) -> dict[str, Any]:
    rows = []
    for cell in program.cells:
        record = read_result(results / cell.cell_id / "cell-result.json")
        if record is None:
            verdict = "missing"
            limitations = ["No cell-result.json under the result root."]
        else:
            verdict = str(record.get("verdict", "fail"))
            limitations = [str(item) for item in record.get("limitations", [])]
        rows.append(
            {
                "cell_id": cell.cell_id,
                "hypothesis": cell.hypothesis,
                "primary": cell.primary,
                "verdict": verdict,
                "metrics": record.get("metrics", {}) if record else {},
                "limitations": limitations,
            }
        )
    primary = [row for row in rows if row["primary"]]
    passing = [row for row in primary if row["verdict"] == "pass"]
    passing.sort(key=lambda row: rank_value(row), reverse=True)
    decision = (
        "candidate — human review required; no validation opened"
        if passing
        else "no_edge — no primary cell passed the development gate"
    )
    return {
        "schema": SCHEMA,
        "program_id": program.program_id,
        "premise": program.premise,
        "tier": program.tier,
        "partition_dev": program.partition_dev,
        "partition_validation": program.partition_validation,
        "validation_access": program.validation_access,
        "correction": correction(program),
        "cells": rows,
        "primary": primary,
        "candidate": passing[0]["cell_id"] if passing else None,
        "decision": decision,
    }


def fmt(value: Any) -> str:
    if value is None:
        return "-"
    if isinstance(value, float):
        return f"{value:+.4f}" if abs(value) < 1.0 else f"{value:.4f}"
    return str(value)


def markdown(document: dict[str, Any]) -> str:
    lines = [
        f"# Research Program {document['program_id']}",
        "",
        f"- Premise: {document['premise']}",
        f"- Tier: `{document['tier']}`",
        f"- Development partition: `{document['partition_dev']}`",
        f"- Validation partition: `{document['partition_validation']}`",
        f"- Correction: {document['correction']['method']} over "
        f"{document['correction']['primary_cells']} primary cells",
        f"- Decision: **{document['decision']}**",
        "",
        "| Cell | Primary | Verdict | n | Clusters | Lift | Lift LCB | Grade |",
        "|---|---|---|---:|---:|---:|---:|---|",
    ]
    for row in document["cells"]:
        metrics = row["metrics"]
        lines.append(
            "| {cell} | {primary} | {verdict} | {n} | {clusters} | {lift} | "
            "{lcb} | {grade} |".format(
                cell=row["cell_id"],
                primary="yes" if row["primary"] else "no",
                verdict=row["verdict"],
                n=fmt(metrics.get("n")),
                clusters=fmt(metrics.get("clusters")),
                lift=fmt(metrics.get("lift")),
                lcb=fmt(metrics.get("lift_lcb")),
                grade=fmt(metrics.get("grade")),
            )
        )
    if document["candidate"]:
        lines += [
            "",
            f"Candidate for human review: `{document['candidate']}`. "
            "No validation was opened by this report.",
        ]
    else:
        lines += [
            "",
            "No primary cell passed. The program stops with no edge. "
            "Do not re-register a variant.",
        ]
    return "\n".join(lines) + "\n"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--program", required=True)
    parser.add_argument("--results", required=True)
    parser.add_argument("--out", required=True)
    args = parser.parse_args(argv)
    try:
        loaded = program_module.load_program(args.program)
        document = build(loaded, Path(args.results).expanduser())
    except (program_module.ProgramError, OSError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    out = Path(args.out).expanduser()
    out.mkdir(parents=True, exist_ok=True)
    (out / "program-results.json").write_text(
        json.dumps(document, indent=2) + "\n", encoding="utf-8"
    )
    (out / "program-results.md").write_text(markdown(document), encoding="utf-8")
    print(f"ok report {document['program_id']} -> {out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
