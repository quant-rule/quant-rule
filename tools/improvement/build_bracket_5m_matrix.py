#!/usr/bin/env python3
"""Build the 5m v10 compression-bracket strategy matrix.

Development: 2018-2022, one-year folds. Validation: 2024. Three instruments.
"""

from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "config" / "compression-bracket-5m"
INSTRUMENTS = {"BTCUSD": "btc", "ETHUSD": "eth", "XAUUSD": "xau"}
PARTITIONS = {"dev": [2018, 2019, 2020, 2021, 2022], "validation": [2024]}


def write_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")


def data_config(instrument: str, year: int) -> dict[str, object]:
    return {
        "schema": "quantrule.data/v3",
        "source": {
            "kind": "csv",
            "path": f"../../../../data/sources/{instrument}_M1_2018-01-01_2024-12-31.csv",
        },
        "data_version": f"dukascopy-bid-{instrument.lower()}-m1-{year}-01-01_{year}-12-31",
        "from": f"{year}-01-01",
        "to": f"{year}-12-31",
        "source_timeframe": "1m",
        "available_timeframes": ["1m", "5m", "15m", "1h"],
        "max_candles": 600000,
        "instrument": instrument,
    }


def run_config(partition: str, instrument: str, slug: str) -> dict[str, object]:
    return {
        "schema": "quantrule.run/v2",
        "name": f"compression-bracket-5m-{slug}",
        "policy": f"../../../model-gate/shared/policy-{instrument}.json",
        "data": "data.json",
        "model": "../../../compression-range/shared/model-range.json",
        "strategy": "../../shared/strategy-v10-5m.json",
        "execution": "../../shared/execution-5m.json",
        "portfolio": "../../shared/portfolio-5m.json",
        "analytics": "../../../model-gate/shared/analytics.json",
        "output_root": f"../../../../data/compression-bracket-5m/{partition}/{slug}",
    }


def build(partition: str) -> list[dict[str, object]]:
    cells = []
    for instrument, short in INSTRUMENTS.items():
        for year in PARTITIONS[partition]:
            slug = f"{short}-{year}"
            cell = BASE / partition / slug
            write_json(cell / "data.json", data_config(instrument, year))
            write_json(cell / "run.json", run_config(partition, instrument, slug))
            cells.append(
                {
                    "instrument": instrument,
                    "fold": year,
                    "partition": f"strategy_{'dev' if partition == 'dev' else 'validation'}",
                    "run": f"data/compression-bracket-5m/{partition}/{slug}",
                }
            )
    return cells


def main() -> int:
    for partition in PARTITIONS:
        cells = build(partition)
        write_json(
            BASE / partition / "matrix.json",
            {
                "schema": "quantrule.strategy-matrix/v1",
                "partition": partition,
                "rule": "compression-bracket-5m",
                "cells": cells,
            },
        )
    print(f"ok compression-bracket-5m matrix -> {BASE}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
