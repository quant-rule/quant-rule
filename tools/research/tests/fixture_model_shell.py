#!/usr/bin/env python3
"""Deterministic fake `quantrule report model` for the model-cell dry run.

It writes a synthetic prediction and readings bundle under `<out>/model/`. The
synthetic series has a known directional edge so the real evaluator can be
exercised without market data.
"""

from __future__ import annotations

import argparse
import csv
from pathlib import Path

BARS = 2000


def predictions() -> dict[int, int]:
    values = {}
    for index in range(0, BARS, 62):
        values[index] = 1
        if index + 31 < BARS:
            values[index + 31] = -1
    return values


def main() -> int:
    parser = argparse.ArgumentParser(add_help=False)
    parser.add_argument("--config")
    parser.add_argument("--spec")
    parser.add_argument("--out", required=True)
    args, _ = parser.parse_known_args()
    out = Path(args.out).expanduser()
    model = out / "model"
    model.mkdir(parents=True, exist_ok=True)
    directions = predictions()
    close = [100.0]
    for index in range(1, BARS + 1):
        previous = index - 1
        if previous in directions:
            close.append(close[-1] + directions[previous] * 2.0)
        else:
            close.append(close[-1] + (1.0 if previous % 2 == 0 else -1.0))
    with (model / "prediction.csv").open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle)
        writer.writerow(
            [
                "timeframe_secs",
                "bar",
                "timestamp",
                "prediction",
                "long_votes",
                "short_votes",
            ]
        )
        for index in range(BARS):
            writer.writerow(
                [
                    3600,
                    index,
                    1_515_000_000 + index * 3600,
                    directions.get(index, 0),
                    1 if directions.get(index) == 1 else 0,
                    1 if directions.get(index) == -1 else 0,
                ]
            )
    with (model / "readings.csv").open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle)
        writer.writerow(
            [
                "timeframe_secs",
                "bar",
                "timestamp",
                "open",
                "high",
                "low",
                "close",
                "volume",
                "atr",
            ]
        )
        for index in range(BARS + 1):
            writer.writerow(
                [
                    3600,
                    index,
                    1_515_000_000 + index * 3600,
                    close[index],
                    close[index] + 0.5,
                    close[index] - 0.5,
                    close[index],
                    1.0,
                    1.0,
                ]
            )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
