#!/usr/bin/env python3
"""Generate the deterministic EXP-2026-11-01-04 run configurations.

The generator writes one `data.json` and one `run.json` per treatment cell under
`config/`, plus the `matrix-mid.txt` manifest. It uses only the Python standard
library, reads no clock, network, or random source, and produces the same bytes
on every run.

Usage:
    python3 docs/experiments/2026-11-01/generate_mid_configs.py
"""

import json
import sys
from pathlib import Path

EXPERIMENT = Path(__file__).resolve().parent
CONFIG = EXPERIMENT / "config"
SHARED = CONFIG / "shared"

INSTRUMENTS = ["BTCUSD", "ETHUSD", "XAUUSD"]
YEARS = [2021, 2022, 2024]

SOURCE_STEM = {
    "BTCUSD": "BTCUSD_M1_2018-01-01_2024-12-31.csv",
    "ETHUSD": "ETHUSD_M1_2018-01-01_2024-12-31.csv",
    "XAUUSD": "XAUUSD_M1_2018-01-01_2024-12-31.csv",
}

SHARED_FILES = [
    "policy-mid.json",
    "model.json",
    "strategy-mid.json",
    "execution-mid.json",
    "portfolio.json",
    "analytics-mid.json",
]


def window(year):
    """The 13-month causal warm-up window for one evaluation year."""
    return f"{year - 1}-12-01", f"{year}-12-31"


def write_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")


def main():
    missing = [name for name in SHARED_FILES if not (SHARED / name).is_file()]
    if missing:
        print("error: missing shared config: " + ", ".join(missing), file=sys.stderr)
        return 1

    cells = []
    for instrument in INSTRUMENTS:
        for year in YEARS:
            start, end = window(year)
            cell = f"{instrument.lower()}-mid-{year}"
            directory = CONFIG / cell
            data = {
                "schema": "quantrule.data/v3",
                "source": {
                    "kind": "csv",
                    "path": f"../../../../../data/sources/{SOURCE_STEM[instrument]}",
                },
                "instrument": instrument,
                "data_version": f"dukascopy-bid-{instrument.lower()}-m1-{start}_{end}",
                "from": start,
                "to": end,
                "source_timeframe": "1m",
                "available_timeframes": ["1m", "15m", "1h"],
                "max_candles": 600000,
            }
            run = {
                "schema": "quantrule.run/v2",
                "name": cell,
                "policy": "../shared/policy-mid.json",
                "data": "data.json",
                "model": "../shared/model.json",
                "strategy": "../shared/strategy-mid.json",
                "execution": "../shared/execution-mid.json",
                "portfolio": "../shared/portfolio.json",
                "analytics": "../shared/analytics-mid.json",
                "output_root": f"../../../../../target/experiment-EXP-2026-11-01-04/{cell}",
            }
            write_json(directory / "data.json", data)
            write_json(directory / "run.json", run)
            cells.append(cell)

    manifest = EXPERIMENT / "matrix-mid.txt"
    lines = ["# cell config"]
    lines.extend(
        f"{cell} docs/experiments/2026-11-01/config/{cell}/run.json" for cell in cells
    )
    manifest.write_text("\n".join(lines) + "\n", encoding="utf-8")

    expected = len(INSTRUMENTS) * len(YEARS)
    if len(cells) != expected:
        print(f"error: expected {expected} cells, generated {len(cells)}", file=sys.stderr)
        return 1
    print(f"wrote {len(cells)} cells to {CONFIG}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
