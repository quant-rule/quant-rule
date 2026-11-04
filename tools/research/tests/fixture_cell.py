#!/usr/bin/env python3
"""Deterministic fixture cell for the research platform dry run.

The fixture writes a summary.json that deliberately fails the gate. It is not a
market claim and never touches real data.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

LIFTS = {"CELL-01": -0.01, "CELL-02": 0.0, "CELL-03": 0.01}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cell", required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument("--verdict", choices=("pass", "fail"), default="fail")
    args = parser.parse_args()
    lift = LIFTS.get(args.cell, 0.0)
    grade = "C"
    if args.verdict == "pass":
        lift = 0.06
        grade = "B"
    out = Path(args.out).expanduser()
    out.mkdir(parents=True, exist_ok=True)
    document = {
        "schema": "quantrule.research-fixture/v1",
        "cell": args.cell,
        "metrics": {
            "n": 100,
            "clusters": 20,
            "hit_rate": 0.30,
            "base_rate": 0.31,
            "lift": lift,
            "lift_lcb": lift - 0.01,
            "grade": grade,
        },
        "verdict": args.verdict,
        "limitations": ["Reference fixture; not a market claim."],
    }
    (out / "summary.json").write_text(
        json.dumps(document, indent=2) + "\n", encoding="utf-8"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
