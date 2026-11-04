#!/usr/bin/env python3
"""Build the compression-range model-gate matrix.

Development: 2018-2020. Validation: 2021-2022. Three instruments, 1m.
"""

from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / "config" / "compression-range"
INSTRUMENTS = {"BTCUSD": "btc", "ETHUSD": "eth", "XAUUSD": "xau"}
PARTITIONS = {
    "dev": ([2018, 2019, 2020], "model_dev"),
    "validation": ([2021, 2022], "model_validation"),
}


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
        "name": f"compression-range-{slug}",
        "policy": f"../../../model-gate/shared/policy-{instrument}.json",
        "data": "data.json",
        "model": "../../shared/model-range.json",
        "strategy": "../../../model-gate/shared/strategy.json",
        "execution": "../../../model-gate/shared/execution.json",
        "portfolio": "../../../model-gate/shared/portfolio.json",
        "analytics": "../../../model-gate/shared/analytics.json",
        "output_root": f"../../../../data/compression-range/{partition}/{slug}",
    }


def build(partition: str) -> list[dict[str, object]]:
    years, partition_name = PARTITIONS[partition]
    cells = []
    for instrument, short in INSTRUMENTS.items():
        for year in years:
            slug = f"{short}-{year}"
            cell = BASE / partition / slug
            write_json(cell / "data.json", data_config(instrument, year))
            write_json(cell / "run.json", run_config(partition, instrument, slug))
            cells.append(
                {
                    "instrument": instrument,
                    "fold": year,
                    "partition": partition_name,
                    "run": f"data/compression-range/{partition}/{slug}/model",
                }
            )
    return cells


def main() -> int:
    for partition in PARTITIONS:
        cells = build(partition)
        write_json(
            BASE / partition / "matrix.json",
            {
                "schema": "quantrule.compression-matrix/v1",
                "partition": partition,
                "event": "signal.compression_range",
                "timeframes": ["1m"],
                "horizon_bars": 1,
                "target_atr": None,
                "cells": cells,
            },
        )
    print(f"ok compression-range matrix -> {BASE}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
