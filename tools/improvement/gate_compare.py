#!/usr/bin/env python3
"""Compare the development and validation claim-validation results.

A family passes the model gate only when the same family and timeframe reaches
A or B on both partitions. A validation-only positive cell is not a baseline; it
is the expected false-positive surface of a one-shot sample.

Usage:
    python3 tools/improvement/gate_compare.py \
        --dev data/.../MODEL-2026-11-03-01/artifacts/validation.json \
        --validation data/.../MODEL-2026-11-03-02/artifacts/validation.json \
        --out data/improvement/2026-11-03/MODEL-GATE
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any


def parse_arguments(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dev", required=True, help="development validation.json")
    parser.add_argument("--validation", required=True, help="validation validation.json")
    parser.add_argument("--out", required=True, help="output directory")
    return parser.parse_args(argv)


def load(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise SystemExit(f"error: cannot read {path}: {error}") from None
    if not isinstance(value, dict):
        raise SystemExit(f"error: {path} is not a JSON object")
    return value


def primary_map(document: dict[str, Any]) -> dict[tuple[str, str], dict[str, Any]]:
    cells: dict[tuple[str, str], dict[str, Any]] = {}
    for cell in document.get("primary", []):
        if not isinstance(cell, dict):
            continue
        key = (str(cell.get("family")), str(cell.get("timeframe")))
        cells[key] = cell
    return cells


def compare(dev: dict[str, Any], validation: dict[str, Any]) -> dict[str, Any]:
    dev_cells = primary_map(dev)
    val_cells = primary_map(validation)
    keys = sorted(set(dev_cells) | set(val_cells))
    rows = []
    passing = []
    for key in keys:
        dev_cell = dev_cells.get(key, {})
        val_cell = val_cells.get(key, {})
        dev_grade = dev_cell.get("grade")
        val_grade = val_cell.get("grade")
        both = dev_grade in ("A", "B") and val_grade in ("A", "B")
        if both:
            passing.append(f"{key[0]} {key[1]}")
        rows.append(
            {
                "family": key[0],
                "timeframe": key[1],
                "dev_grade": dev_grade,
                "validation_grade": val_grade,
                "dev_hit": dev_cell.get("hit_rate"),
                "validation_hit": val_cell.get("hit_rate"),
                "dev_lcb": dev_cell.get("lcb_corrected"),
                "validation_lcb": val_cell.get("lcb_corrected"),
                "dev_lift": dev_cell.get("lift"),
                "validation_lift": val_cell.get("lift"),
                "passes": both,
            }
        )
    validation_only = [
        f"{row['family']} {row['timeframe']}"
        for row in rows
        if row["validation_grade"] in ("A", "B") and row["dev_grade"] not in ("A", "B")
    ]
    dev_only = [
        f"{row['family']} {row['timeframe']}"
        for row in rows
        if row["dev_grade"] in ("A", "B") and row["validation_grade"] not in ("A", "B")
    ]
    return {
        "schema": "quantrule.model-gate/v1",
        "dev_gate": dev.get("gate"),
        "validation_gate": validation.get("gate"),
        "gate": "pass" if passing else "no_edge",
        "passing": passing,
        "validation_only": validation_only,
        "dev_only": dev_only,
        "rows": rows,
    }


def markdown(result: dict[str, Any]) -> str:
    lines = [
        "# Model Gate",
        "",
        f"- Development partition: **{result['dev_gate']}**",
        f"- Validation partition: **{result['validation_gate']}**",
        f"- Combined gate: **{result['gate']}**",
        f"- Passing families: {', '.join(result['passing']) or 'none'}",
        f"- Validation-only cells: {', '.join(result['validation_only']) or 'none'}",
        "",
        "| Family | TF | Dev grade | Val grade | Dev hit | Val hit | Dev LCB | Val LCB | Passes |",
        "|---|---|---|---|---:|---:|---:|---:|---|",
    ]
    for row in result["rows"]:
        lines.append(
            "| {family} | {tf} | {dev} | {val} | {dh} | {vh} | {dl} | {vl} | {passes} |".format(
                family=row["family"],
                tf=row["timeframe"],
                dev=row["dev_grade"],
                val=row["validation_grade"],
                dh="-" if row["dev_hit"] is None else f"{row['dev_hit']:.4f}",
                vh="-" if row["validation_hit"] is None else f"{row['validation_hit']:.4f}",
                dl="-" if row["dev_lcb"] is None else f"{row['dev_lcb']:.4f}",
                vl="-" if row["validation_lcb"] is None else f"{row['validation_lcb']:.4f}",
                passes="yes" if row["passes"] else "no",
            )
        )
    lines.append("")
    lines.append(
        "A family passes only when the same family and timeframe is A or B on both partitions. "
        "A validation-only cell is not a model baseline."
    )
    lines.append("")
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    args = parse_arguments(argv)
    result = compare(load(Path(args.dev)), load(Path(args.validation)))
    out = Path(args.out).expanduser()
    out.mkdir(parents=True, exist_ok=True)
    (out / "gate.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    (out / "gate.md").write_text(markdown(result), encoding="utf-8")
    print(f"ok model gate -> {out} gate={result['gate']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
