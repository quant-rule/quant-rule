#!/usr/bin/env python3
"""Build the pre-registered model-v4 validation matrix.

The matrix is one one-year model run per instrument, year, and ADX setting. The
four event hypotheses are evaluated by the path harness against the retained
event families; they are not separate model runs.

Usage:
    python3 tools/improvement/build_v4_matrix.py
"""

from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CONFIG = ROOT / "config" / "model-v4"
SHARED = CONFIG / "shared"
MODEL_GATE = ROOT / "config" / "model-gate" / "shared"

INSTRUMENTS = {
    "BTCUSD": "btc",
    "ETHUSD": "eth",
    "XAUUSD": "xau",
}
PARTITIONS = {
    "dev": [2018, 2019, 2020],
    "validation": [2021, 2022],
}
CONFIGS = {"adx20": "model-adx20.json", "adx25": "model-adx25.json"}
SOURCE = "{instrument}_M1_2018-01-01_2024-12-31.csv"


def write_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")


def data_config(instrument: str, year: int) -> dict[str, object]:
    return {
        "schema": "quantrule.data/v3",
        "source": {
            "kind": "csv",
            "path": f"../../../../data/sources/{SOURCE.format(instrument=instrument)}",
        },
        "data_version": (
            f"dukascopy-bid-{instrument.lower()}-m1-{year}-01-01_{year}-12-31"
        ),
        "from": f"{year}-01-01",
        "to": f"{year}-12-31",
        "source_timeframe": "1m",
        "available_timeframes": ["1m", "5m", "15m", "1h"],
        "max_candles": 600000,
        "instrument": instrument,
    }


def run_config(
    partition: str, instrument: str, slug: str, config: str
) -> dict[str, object]:
    return {
        "schema": "quantrule.run/v2",
        "name": f"model-v4-{slug}-{config}",
        "policy": f"../../../model-gate/shared/policy-{instrument}.json",
        "data": "data.json",
        "model": f"../../shared/{CONFIGS[config]}",
        "strategy": "../../../model-gate/shared/strategy.json",
        "execution": "../../../model-gate/shared/execution.json",
        "portfolio": "../../../model-gate/shared/portfolio.json",
        "analytics": "../../../model-gate/shared/analytics.json",
        "output_root": f"../../../../data/model-v4/{partition}/{slug}-{config}",
    }


def build(partition: str) -> list[dict[str, object]]:
    cells = []
    for instrument, short in INSTRUMENTS.items():
        for year in PARTITIONS[partition]:
            slug = f"{short}-{year}"
            cell_dir = CONFIG / partition / slug
            write_json(cell_dir / "data.json", data_config(instrument, year))
            runs = {}
            for config in CONFIGS:
                write_json(
                    cell_dir / f"run-{config}.json",
                    run_config(partition, instrument, slug, config),
                )
                runs[config] = (
                    f"data/model-v4/{partition}/{slug}-{config}/model"
                )
            cells.append(
                {
                    "instrument": instrument,
                    "short": short,
                    "fold": year,
                    "partition": f"model_{'dev' if partition == 'dev' else 'validation'}",
                    "runs": runs,
                }
            )
    return cells


def main() -> int:
    for partition in PARTITIONS:
        cells = build(partition)
        write_json(
            CONFIG / partition / "matrix.json",
            {
                "schema": "quantrule.model-matrix/v1",
                "partition": partition,
                "configs": list(CONFIGS),
                "primary_config": "adx25",
                "sensitivity_config": "adx20",
                "cells": cells,
            },
        )
    print(f"ok matrix -> {CONFIG}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
