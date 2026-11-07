#!/usr/bin/env python3
"""Build the pre-registered M4 location programme.

The builder writes the programme manifest, the five primary cells, their 15m
specs, the ``level_touches`` gate, and the one-year run configs. It refuses to
overwrite an existing programme directory; a rebuild is a deliberate human
action, not an implicit side effect.

Usage:
    python3 tools/research/build_m4_program.py
"""

from __future__ import annotations

import json
import shutil
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
PROGRAM_ID = "PROGRAM-2026-11-07-01"
SOURCE_PROGRAM = "PROGRAM-2026-11-05-04"
PROGRAM_DIR = ROOT / "config" / "research" / PROGRAM_ID
SOURCE_DIR = ROOT / "config" / "research" / SOURCE_PROGRAM
TIMEFRAME = 900
INSTRUMENTS = ("XAU", "BTC", "SP500", "EURUSD")
STATE = {
    "schema": "quantrule.model-state-gate/v1",
    "all": [{"column": "level_touches", "min": 1}],
}
CELLS = (
    (
        "M4.1",
        "signal.rsi_threshold",
        "An RSI threshold crossing at a confirmed major level has a positive "
        "directional path reaction; away from structure it does not.",
    ),
    (
        "M4.2",
        "signal.macd_histogram_slope_flip",
        "A MACD histogram slope flip at a confirmed major level accelerates in "
        "the flip direction; away from structure it does not.",
    ),
    (
        "M4.3",
        "signal.di_cross",
        "A DI cross at a confirmed major level continues in the cross "
        "direction; away from structure it does not.",
    ),
    (
        "M4.4",
        "signal.trend_continuation",
        "A trend continuation signal at a confirmed major level reaches +2 ATR "
        "before -1 ATR; away from structure it does not.",
    ),
    (
        "M4.5",
        "signal.ema_stack_change",
        "An EMA stack state change at a confirmed major level continues; away "
        "from structure it does not.",
    ),
)


def write_json(path: Path, document: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(document, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )


def cell_command() -> list[str]:
    return [
        "python3",
        "tools/research/model_cell.py",
        "--cell-dir",
        "{config}",
        "--phase",
        "{phase}",
        "--out",
        "{out}",
        "--family",
        "M4",
        "--timeframe",
        str(TIMEFRAME),
        "--runs-dir",
        f"config/research/{PROGRAM_ID}/runs",
        "--horizon",
        "20",
        "--target-atr",
        "2.0",
        "--stop-atr",
        "1.0",
        "--cost-bps",
        "6",
        "--primary-cells",
        "5",
        "--seed",
        "0",
        "--state-file",
        "{config}/state.json",
    ]


def build_program() -> dict[str, Any]:
    cells = []
    for cell_id, event, hypothesis in CELLS:
        cells.append(
            {
                "cell_id": cell_id,
                "hypothesis": hypothesis,
                "primary": True,
                "config_dir": f"config/research/{PROGRAM_ID}/cells/{cell_id}",
                "command": cell_command(),
                "result_parser": "model_prediction",
                "result": "model_prediction.json",
            }
        )
    return {
        "schema": "quantrule.research-program/v1",
        "program_id": PROGRAM_ID,
        "premise": (
            "Directional event information is conditional on location relative "
            "to a confirmed major level: the same event at a confirmed major "
            "level has positive forward edge, while away from structure it does "
            "not."
        ),
        "economic_rationale": (
            "Confirmed major pivots concentrate resting liquidity, stop orders, "
            "and market-maker inventory. A directional event at a level is a "
            "liquidity interaction with a counterparty flow; the same event "
            "mid-range has no such mechanism. M2 averaged locations in an "
            "unconditional vote, M3 conditioned on state but still averaged "
            "locations, and the 15m family programme tested the families "
            "unconditionally."
        ),
        "tier": "model",
        "changed": "model",
        "frozen": ["strategy", "portfolio"],
        "partition_dev": "model_dev",
        "partition_validation": "model_validation",
        "costs": {
            "commission_bps": 4,
            "slippage_bps": 2,
            "funding": 0,
        },
        "green_criteria": {
            "min_events": 200,
            "min_clusters": 20,
            "min_lift": 0.02,
            "lift_lcb": "> 0",
            "random_p": "<= 0.01",
            "shuffle_p": "<= 0.01",
            "back_lag": "pass",
            "replication": ">= 2 instruments",
            "state_increment": ">= 0.01",
            "promotion": "grade A",
        },
        "correction": {"method": "bonferroni", "primary_cells": 5},
        "max_parallel": 5,
        "iteration_cap": 1,
        "validation_access": "one_shot",
        "stop_rule": (
            "Stop when no primary cell reaches grade A, a validation cell "
            "fails, the count gate cannot reach 200 condition-matched "
            "predictions and 20 clusters in every primary cell, a shared "
            "prerequisite is missing, or the human stops it. Do not re-register "
            "a variant with a different event, timeframe, horizon, target, "
            "stop, filter, cost, instrument set, gate column, or gate "
            "threshold."
        ),
        "cells": cells,
    }


def build_cells() -> None:
    for cell_id, event, _hypothesis in CELLS:
        cell_dir = PROGRAM_DIR / "cells" / cell_id
        write_json(
            cell_dir / "coverage.json",
            {
                "schema": "quantrule.coverage/v1",
                "cell_id": cell_id,
                "kind": "primary",
                "events": [event],
                "instruments": list(INSTRUMENTS),
                "timeframe": TIMEFRAME,
                "gate": {"column": "level_touches", "min": 1},
            },
        )
        write_json(cell_dir / "state.json", STATE)
        for instrument in INSTRUMENTS:
            write_json(
                cell_dir / f"spec-{instrument}-{TIMEFRAME}.json",
                {
                    "schema": "quantrule.model-spec/v2",
                    "id": cell_id,
                    "family": "M4",
                    "instrument": instrument,
                    "timeframes": [TIMEFRAME],
                    "readings": {},
                    "active_events": [event],
                    "prediction": {
                        "kind": "direction",
                        "events": [event],
                        "min_votes": 1,
                    },
                },
            )


def copy_runs() -> None:
    target = PROGRAM_DIR / "runs"
    shutil.copytree(SOURCE_DIR / "runs", target)
    for run_path in sorted(target.rglob("run.json")):
        document = json.loads(run_path.read_text(encoding="utf-8"))
        document["name"] = str(document.get("name", "")).replace("m3-", "m4-")
        document["output_root"] = str(document.get("output_root", "")).replace(
            SOURCE_PROGRAM, PROGRAM_ID
        )
        write_json(run_path, document)
    for data_path in sorted(target.rglob("data.json")):
        document = json.loads(data_path.read_text(encoding="utf-8"))
        document["data_version"] = str(document.get("data_version", "")).replace(
            "m3-", "m4-"
        )
        write_json(data_path, document)
    _migrate_shared_configs(target)


def _migrate_shared_configs(target: Path) -> None:
    """Bring the copied M3 sibling configs onto the current S3 contract."""
    portfolio_path = target / "shared" / "portfolio.json"
    if portfolio_path.is_file():
        portfolio = json.loads(portfolio_path.read_text(encoding="utf-8"))
        portfolio["schema"] = "quantrule.portfolio/v2"
        portfolio.setdefault("id", "P1.0")
        portfolio.pop("allow_short", None)
        write_json(portfolio_path, portfolio)
    execution_path = target / "shared" / "execution.json"
    if execution_path.is_file():
        execution = json.loads(execution_path.read_text(encoding="utf-8"))
        execution["schema"] = "quantrule.execution/v2"
        execution.setdefault("id", "E1.0")
        write_json(execution_path, execution)
    for policy_path in sorted(target.glob("*/policy.json")):
        policy = json.loads(policy_path.read_text(encoding="utf-8"))
        policy.setdefault("id", "POL1.0")
        write_json(policy_path, policy)


def main() -> int:
    if PROGRAM_DIR.exists():
        print(f"error: {PROGRAM_DIR} already exists; refusing to overwrite")
        return 1
    if not (SOURCE_DIR / "runs").is_dir():
        print(f"error: source run configs not found: {SOURCE_DIR / 'runs'}")
        return 1
    PROGRAM_DIR.mkdir(parents=True)
    write_json(PROGRAM_DIR / "program.json", build_program())
    build_cells()
    copy_runs()
    print(f"ok {PROGRAM_ID} cells={len(CELLS)} timeframe={TIMEFRAME}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
