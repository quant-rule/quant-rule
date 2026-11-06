#!/usr/bin/env python3
"""Build the approved M3.x programme config tree.

The builder is deterministic and writes only under
``config/research/PROGRAM-2026-11-05-04``. It copies the ten M2 exploratory area
cells, changes their family and adds a 15m spec, and creates the five primary
state-conditioned cells, their state gates, and the current-schema run configs.

It does not run a model and does not write ``selection.json``; the count-only
preflight writes the selected timeframe.

Usage:
    python3 tools/research/build_m3_program.py
"""

from __future__ import annotations

import argparse
import json
import shutil
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
M2 = ROOT / "config/research/PROGRAM-2026-11-05-03"
M3 = ROOT / "config/research/PROGRAM-2026-11-05-04"
INSTRUMENTS = ["XAU", "BTC", "SP500", "EURUSD"]
TIMEFRAMES = [3600, 900]
STATE_SCHEMA = "quantrule.model-state-gate/v1"
PROGRAM_SCHEMA = "quantrule.research-program/v1"

PRIMARY: dict[str, dict[str, Any]] = {
    "M3.1": {
        "hypothesis": "Trend continuation has positive directional edge in a trend state.",
        "instruments": INSTRUMENTS,
        "event": "signal.trend_continuation",
        "indicator": "adx",
        "readings": {},
        "state": [{"column": "adx_regime", "values": [1]}, {"column": "ema_stack", "values": [-1, 1]}],
    },
    "M3.2": {
        "hypothesis": "VWAP mean reversion has positive directional edge in a coiling state.",
        "instruments": ["XAU", "SP500", "EURUSD"],
        "event": ["signal.vwap_reclaim", "signal.vwap_rejection"],
        "indicator": "vwap",
        "readings": {"extended": {"vwap_window": 24}},
        "state": [{"column": "context_code", "values": [2]}],
    },
    "M3.3": {
        "hypothesis": "A value-area breakout continues in an expanding state.",
        "instruments": ["XAU", "SP500", "EURUSD"],
        "event": "signal.profile_va_breakout",
        "indicator": "volume_profile",
        "readings": {"extended": {"profile_window": 96}},
        "state": [{"column": "context_code", "values": [1]}],
    },
    "M3.4": {
        "hypothesis": "A climax reversal fades the extreme in an exhaustion state.",
        "instruments": ["XAU", "SP500", "EURUSD"],
        "event": "signal.climax_reversal",
        "indicator": "volume_climax",
        "readings": {},
        "state": [{"column": "context_code", "values": [4]}],
    },
    "M3.5": {
        "hypothesis": "An absorption rejection reverses in a settling state.",
        "instruments": ["XAU", "SP500", "EURUSD"],
        "event": "signal.absorption_rejection",
        "indicator": "absorption",
        "readings": {},
        "state": [{"column": "context_code", "values": [3]}],
    },
}

EXPLORATORY = [f"EXPL-{index:02d}" for index in range(1, 11)]


def write_json(path: Path, document: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(document, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
    )


def events(value: Any) -> list[str]:
    return value if isinstance(value, list) else [value]


def build_cell(cell_id: str) -> dict[str, Any]:
    source = M2 / "cells" / cell_id
    destination = M3 / "cells" / cell_id
    destination.mkdir(parents=True, exist_ok=True)
    coverage = json.loads((source / "coverage.json").read_text(encoding="utf-8"))
    coverage["instruments"] = INSTRUMENTS
    coverage["timeframes"] = TIMEFRAMES
    write_json(destination / "coverage.json", coverage)
    for instrument in INSTRUMENTS:
        spec = json.loads(
            (source / f"spec-{instrument}.json").read_text(encoding="utf-8")
        )
        spec["family"] = "M3"
        for timeframe in TIMEFRAMES:
            spec["timeframes"] = [timeframe]
            write_json(
                destination / f"spec-{instrument}-{timeframe}.json", spec
            )
    return {
        "cell_id": cell_id,
        "hypothesis": f"Unconditional coverage of M2 area cell {cell_id}.",
        "primary": False,
        "config_dir": f"config/research/PROGRAM-2026-11-05-04/cells/{cell_id}",
    }


def build_primary(cell_id: str, definition: dict[str, Any]) -> dict[str, Any]:
    destination = M3 / "cells" / cell_id
    destination.mkdir(parents=True, exist_ok=True)
    event_ids = events(definition["event"])
    write_json(
        destination / "coverage.json",
        {
            "cell_id": cell_id,
            "kind": "primary",
            "areas": [],
            "indicators": [definition["indicator"]],
            "events": event_ids,
            "benchmark_events": [],
            "instruments": definition["instruments"],
            "timeframes": TIMEFRAMES,
        },
    )
    write_json(
        destination / "state.json",
        {"schema": STATE_SCHEMA, "all": definition["state"]},
    )
    for instrument in definition["instruments"]:
        for timeframe in TIMEFRAMES:
            write_json(
                destination / f"spec-{instrument}-{timeframe}.json",
                {
                    "schema": "quantrule.model-spec/v2",
                    "id": cell_id,
                    "family": "M3",
                    "instrument": instrument,
                    "timeframes": [timeframe],
                    "readings": definition["readings"],
                    "active_events": event_ids,
                    "prediction": {
                        "kind": "direction",
                        "events": event_ids,
                        "min_votes": 1,
                    },
                },
            )
    return {
        "cell_id": cell_id,
        "hypothesis": definition["hypothesis"],
        "primary": True,
        "config_dir": f"config/research/PROGRAM-2026-11-05-04/cells/{cell_id}",
    }


def command(cell: dict[str, Any]) -> list[str]:
    values = [
        "python3",
        "tools/research/model_cell.py",
        "--cell-dir",
        "{config}",
        "--phase",
        "{phase}",
        "--out",
        "{out}",
        "--family",
        "M3",
        "--timeframe",
        "auto",
        "--selection",
        "config/research/PROGRAM-2026-11-05-04/selection.json",
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
    ]
    if cell["primary"]:
        values.extend(["--state-file", "{config}/state.json"])
    return values


def build_runs() -> None:
    shared = M2 / "runs" / "shared"
    for name in ("model.json", "execution.json", "portfolio.json"):
        write_json(
            M3 / "runs" / "shared" / name,
            json.loads((shared / name).read_text(encoding="utf-8")),
        )
    for instrument in INSTRUMENTS:
        policy = json.loads(
            (M2 / "runs" / instrument / "policy.json").read_text(encoding="utf-8")
        )
        write_json(M3 / "runs" / instrument / "policy.json", policy)
        for year in range(2018, 2023):
            source = M2 / "runs" / instrument / str(year)
            data = json.loads((source / "data.json").read_text(encoding="utf-8"))
            data["data_version"] = (
                f"m3-{instrument.lower()}-m1-{year}-01-01_{year}-12-31"
            )
            write_json(M3 / "runs" / instrument / str(year) / "data.json", data)
            write_json(
                M3 / "runs" / instrument / str(year) / "run.json",
                {
                    "schema": "quantrule.run/v2",
                    "name": f"m3-{instrument.lower()}-{year}",
                    "model_id": "M1.0",
                    "policy_id": "POL1.0",
                    "execution_id": "E1.0",
                    "portfolio_id": "P1.0",
                    "data": "data.json",
                    "model": "../../shared/model.json",
                    "execution": "../../shared/execution.json",
                    "portfolio": "../../shared/portfolio.json",
                    "policy": "../policy.json",
                    "output_root": "../../../../../../data/research/PROGRAM-2026-11-05-04/tmp",
                },
            )


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--force",
        action="store_true",
        help="rebuild even when selection.json already exists",
    )
    args = parser.parse_args(argv)
    if (M3 / "selection.json").is_file() and not args.force:
        raise SystemExit(
            "error: selection.json exists; the programme is already frozen. "
            "Use --force only for a new programme."
        )
    if M3.exists():
        shutil.rmtree(M3)
    M3.mkdir(parents=True)
    shutil.copy2(M2 / "areas.json", M3 / "areas.json")
    cells = [build_cell(cell_id) for cell_id in EXPLORATORY]
    cells.extend(
        build_primary(cell_id, definition)
        for cell_id, definition in PRIMARY.items()
    )
    for cell in cells:
        cell["command"] = command(cell)
        cell["result_parser"] = "model_prediction"
        cell["result"] = "model_prediction.json"
    write_json(
        M3 / "program.json",
        {
            "schema": PROGRAM_SCHEMA,
            "program_id": "PROGRAM-2026-11-05-04",
            "premise": (
                "Directional event information is conditional on the published "
                "market state; a state-matched base and a state gate produce lift "
                "where the unconditional vote does not."
            ),
            "economic_rationale": (
                "Trend, coiling, expanding, high-range/low-participation, and "
                "settling states are governed by different market mechanisms. "
                "An unconditional vote averages them into zero. The state is a "
                "published causal reading, not a fitted classifier."
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
                "fails, the count switch cannot reach 200 state-matched "
                "predictions and 20 clusters in every primary cell, a shared "
                "prerequisite is missing, or the human stops it. Do not "
                "re-register a variant with a different state, event, timeframe, "
                "horizon, target, stop, filter, cost, or instrument set."
            ),
            "cells": cells,
        },
    )
    build_runs()
    print(
        f"ok {M3.relative_to(ROOT)} cells={len(cells)} "
        f"primary={sum(1 for cell in cells if cell['primary'])}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
