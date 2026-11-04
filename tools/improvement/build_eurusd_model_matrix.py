#!/usr/bin/env python3
"""Build the EURUSD model pack configs and matrix for the session spike.

The model pack is the accepted v4 model config on EURUSD 2018-2020. The spike
uses only the 15m bars and ATR; the model events are not used.
"""

from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "config" / "session-spike" / "eurusd"
YEARS = (2018, 2019, 2020)


def write_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")


def data_config(year: int) -> dict[str, object]:
    return {
        "schema": "quantrule.data/v3",
        "source": {
            "kind": "csv",
            "path": "../../../../data/sources/EURUSD_M1_2018-01-01_2024-12-31.csv",
        },
        "data_version": f"dukascopy-bid-eurusd-m1-{year}-01-01_{year}-12-31",
        "from": f"{year}-01-01",
        "to": f"{year}-12-31",
        "source_timeframe": "1m",
        "available_timeframes": ["1m", "5m", "15m", "1h"],
        "max_candles": 600000,
        "instrument": "EURUSD",
    }


def run_config(year: int) -> dict[str, object]:
    return {
        "schema": "quantrule.run/v2",
        "name": f"eurusd-session-spike-{year}",
        "policy": "../shared/policy-EURUSD.json",
        "data": "data.json",
        "model": "../../../model-v4/shared/model-adx25.json",
        "strategy": "../../../model-gate/shared/strategy.json",
        "execution": "../../../model-gate/shared/execution.json",
        "portfolio": "../../../model-gate/shared/portfolio.json",
        "analytics": "../../../model-gate/shared/analytics.json",
        "output_root": f"../../../../data/session-spike/eurusd/{year}",
    }


def main() -> int:
    cells = []
    for year in YEARS:
        cell = BASE / str(year)
        write_json(cell / "data.json", data_config(year))
        write_json(cell / "run.json", run_config(year))
        cells.append(
            {
                "instrument": "EURUSD",
                "short": "eurusd",
                "fold": year,
                "partition": "model_dev",
                "runs": {"adx25": f"data/session-spike/eurusd/{year}/model"},
            }
        )
    write_json(
        BASE / "matrix.json",
        {
            "schema": "quantrule.model-matrix/v1",
            "partition": "model_dev",
            "configs": ["adx25"],
            "primary_config": "adx25",
            "cells": cells,
        },
    )
    print(f"ok EURUSD model matrix -> {BASE}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
