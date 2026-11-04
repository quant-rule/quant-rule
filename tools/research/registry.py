#!/usr/bin/env python3
"""Write a research program record from a program-results.json.

The registry never accepts a baseline. It records the program decision as
no_edge or candidate. An accepted row requires the human to pass
--accepted-by and --accepted-on explicitly.

Usage:
    python3 tools/research/registry.py \
        --program config/research/<id>/program.json \
        --report data/research/<id>/program-results/program-results.json
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

import program as program_module


def read_report(path: Path) -> dict[str, Any]:
    try:
        document = json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError:
        raise program_module.ProgramError(f"report not found: {path}") from None
    except json.JSONDecodeError as error:
        raise program_module.ProgramError(f"invalid report JSON: {error}") from None
    if not isinstance(document, dict):
        raise program_module.ProgramError("report must be an object")
    if document.get("schema") != "quantrule.research-program-results/v1":
        raise program_module.ProgramError("report schema is not recognized")
    return document


def status(report: dict[str, Any], accepted_by: str | None, accepted_on: str | None) -> str:
    if accepted_by or accepted_on:
        if not accepted_by or not accepted_on:
            raise program_module.ProgramError(
                "--accepted-by and --accepted-on must be supplied together"
            )
        return "accepted"
    decision = str(report.get("decision", ""))
    return "candidate" if decision.startswith("candidate") else "no_edge"


def fmt(value: Any) -> str:
    if value is None:
        return "-"
    if isinstance(value, float):
        return f"{value:+.4f}" if abs(value) < 1.0 else f"{value:.4f}"
    return str(value)


def markdown(
    program: program_module.Program,
    report: dict[str, Any],
    state: str,
    accepted_by: str | None,
    accepted_on: str | None,
) -> str:
    lines = [
        f"# Program {program.program_id}",
        "",
        f"- Date: {program.program_id[8:18]} (UTC)",
        f"- Status: **{state}**",
        f"- Tier: `{program.tier}`",
        f"- Premise: {program.premise}",
        f"- Development: `{program.partition_dev}`",
        f"- Validation: `{program.partition_validation}`",
        f"- Correction: {report['correction']['method']} over "
        f"{report['correction']['primary_cells']} primary cells",
        f"- Decision: {report['decision']}",
        f"- Accepted by: {accepted_by or '—'}",
        f"- Accepted on: {accepted_on or '—'}",
        "",
        "| Cell | Primary | Verdict | n | Clusters | Lift | Lift LCB | Grade |",
        "|---|---|---|---:|---:|---:|---:|---|",
    ]
    for row in report.get("cells", []):
        metrics = row.get("metrics", {})
        lines.append(
            "| {cell} | {primary} | {verdict} | {n} | {clusters} | {lift} | "
            "{lcb} | {grade} |".format(
                cell=row.get("cell_id", "?"),
                primary="yes" if row.get("primary") else "no",
                verdict=row.get("verdict", "?"),
                n=fmt(metrics.get("n")),
                clusters=fmt(metrics.get("clusters")),
                lift=fmt(metrics.get("lift")),
                lcb=fmt(metrics.get("lift_lcb")),
                grade=fmt(metrics.get("grade")),
            )
        )
    lines += [
        "",
        "The registry does not accept a baseline. A candidate requires an "
        "independent Quant Validator record and human acceptance.",
    ]
    return "\n".join(lines) + "\n"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--program", required=True)
    parser.add_argument("--report", required=True)
    parser.add_argument("--out")
    parser.add_argument("--accepted-by")
    parser.add_argument("--accepted-on")
    args = parser.parse_args(argv)
    try:
        loaded = program_module.load_program(args.program)
        report = read_report(Path(args.report).expanduser())
        state = status(report, args.accepted_by, args.accepted_on)
        date = loaded.program_id[8:18]
        out = (
            Path(args.out).expanduser()
            if args.out
            else loaded.repo_root
            / "docs"
            / "improvement"
            / date
            / f"{loaded.program_id}.md"
        )
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text(
            markdown(loaded, report, state, args.accepted_by, args.accepted_on),
            encoding="utf-8",
        )
    except (program_module.ProgramError, OSError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    print(f"ok registry {loaded.program_id} {state} -> {out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
