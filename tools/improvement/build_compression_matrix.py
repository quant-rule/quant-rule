#!/usr/bin/env python3
"""Build the compression-breakout model matrix.

One one-year model run per instrument and year, with the compression event
enabled. Development and validation are separate matrices.

Usage:
    python3 tools/improvement/build_compression_matrix.py
"""

from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CONFIG = ROOT / "config" / "compression"
MODEL = "../../../model-v4/shared/model-compression.json"
SHARED = "../../../model-gate/shared"
INSTRUMENTS = {"BTCUSD": "btc", "ETHUSD": "eth", "XAUUSD": "xau"}
PARTITIONS = {"dev": [2018, 2019, 2020], "validation": [2021, 2022]}


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


def run_config(
    partition: str, instrument: str, slug: str
) -> dict[str, object]:
    return {
        "schema": "quantrule.run/v2",
        "name": f"compression-{slug}",
        "policy": f"{SHARED}/policy-{instrument}.json",
        "data": "data.json",
        "model": MODEL,
        "strategy": f"{SHARED}/strategy.json",
        "execution": f"{SHARED}/execution.json",
        "portfolio": f"{SHARED}/portfolio.json",
        "analytics": f"{SHARED}/analytics.json",
        "output_root": f"../../../../data/model-v4/compression/{partition}/{slug}",
    }


def build(partition: str) -> list[dict[str, object]]:
    cells = []
    for instrument, short in INSTRUMENTS.items():
        for year in PARTITIONS[partition]:
            slug = f"{short}-{year}"
            cell = CONFIG / partition / slug
            write_json(cell / "data.json", data_config(instrument, year))
            write_json(cell / "run.json", run_config(partition, instrument, slug))
            cells.append(
                {
                    "instrument": instrument,
                    "fold": year,
                    "partition": f"model_{'dev' if partition == 'dev' else 'validation'}",
                    "run": f"data/model-v4/compression/{partition}/{slug}/model",
                }
            )
    return cells


def main() -> int:
    for partition in PARTITIONS:
        cells = build(partition)
        write_json(
            CONFIG / partition / "matrix.json",
            {
                "schema": "quantrule.compression-matrix/v1",
                "partition": partition,
                "event": "signal.compression_breakout",
                "timeframes": ["1h", "15m"],
                "horizon_bars": 20,
                "target_atr": 2.0,
                "cells": cells,
            },
        )
    print(f"ok compression matrix -> {CONFIG}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
