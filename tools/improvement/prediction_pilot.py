#!/usr/bin/env python3
"""Single-timeframe RSI prediction pilot.

Primary question: after RSI exits oversold (crosses 30 upward) or exits
overbought (crosses 70 downward), is the next 1h return more often in the
reversion direction than the unconditional base rate?

The pilot is prediction-only. It does not model costs, execution, or a strategy.
Development and validation are reported separately; validation is not used for
tuning.

Usage:
    python3 tools/improvement/prediction_pilot.py \
        --dev data/prediction-pilot/eurusd-2018/model \
              data/prediction-pilot/eurusd-2019/model \
              data/prediction-pilot/eurusd-2020/model \
        --validation data/prediction-pilot/eurusd-2021/model \
                    data/prediction-pilot/eurusd-2022/model \
        --out data/prediction-pilot/RSI
"""

from __future__ import annotations

import argparse
import csv
import json
import math
import sys
from collections import defaultdict
from pathlib import Path
from statistics import NormalDist
from typing import Any

TIMEFRAME_SECS = 3600
HORIZONS = [1, 3, 5, 10]
PRIMARY_HORIZON = 1
Z_RAW = 1.959963984540054


def parse_arguments(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dev", nargs="+", required=True, help="development model bundle dirs")
    parser.add_argument("--validation", nargs="+", required=True, help="validation model bundle dirs")
    parser.add_argument("--out", required=True, help="output directory")
    return parser.parse_args(argv)


def read_readings(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    with path.open(newline="", encoding="utf-8") as handle:
        reader = csv.DictReader(handle)
        for row in reader:
            if row.get("timeframe_secs") != str(TIMEFRAME_SECS):
                continue
            try:
                rows.append(
                    {
                        "bar": int(row["bar"]),
                        "close": float(row["close"]),
                        "rsi": optional_float(row.get("rsi")),
                        "rsi_zone": optional_int(row.get("rsi_zone")),
                    }
                )
            except (KeyError, TypeError, ValueError):
                continue
    rows.sort(key=lambda row: row["bar"])
    return rows


def read_events(path: Path) -> list[dict[str, Any]]:
    events: list[dict[str, Any]] = []
    with path.open(newline="", encoding="utf-8") as handle:
        reader = csv.DictReader(handle)
        for row in reader:
            if row.get("timeframe_secs") != str(TIMEFRAME_SECS):
                continue
            if (row.get("event_type_code"), row.get("event_subtype_code"), row.get("source_code")) != ("3", "1", "4"):
                continue
            threshold = optional_float(row.get("reference_value"))
            direction = optional_int(row.get("direction_code"))
            if threshold not in (30.0, 70.0) or direction not in (-1, 1):
                continue
            # Reversion subset only: exit oversold upward, exit overbought
            # downward. The other two crossings are continuation and are not
            # counted as misses in this primary test.
            if not ((threshold == 30.0 and direction == 1) or (threshold == 70.0 and direction == -1)):
                continue
            events.append(
                {
                    "available": int(row["available_bar_index"]),
                    "threshold": threshold,
                    "direction": direction,
                }
            )
    return events


def optional_float(value: Any) -> float | None:
    if value is None or value == "":
        return None
    try:
        return float(value)
    except (TypeError, ValueError):
        return None


def optional_int(value: Any) -> int | None:
    if value is None or value == "":
        return None
    try:
        return int(value)
    except (TypeError, ValueError):
        return None


def sign(value: float) -> int:
    if value > 0.0:
        return 1
    if value < 0.0:
        return -1
    return 0


def forward(rows: list[dict[str, Any]], index: int, horizon: int) -> float | None:
    target = index + horizon
    if index < 0 or target >= len(rows):
        return None
    return rows[target]["close"] / rows[index]["close"] - 1.0


def base_rate(rows: list[dict[str, Any]], horizon: int) -> tuple[int, int]:
    total = 0
    positive = 0
    for index in range(len(rows) - horizon):
        value = forward(rows, index, horizon)
        if value is None:
            continue
        total += 1
        if value > 0.0:
            positive += 1
    return total, positive


def lower_bound(hits: int, n: int) -> float | None:
    if n <= 0:
        return None
    p = hits / n
    se = math.sqrt(max(p * (1.0 - p), 0.0) / n)
    return max(0.0, p - Z_RAW * se)


def event_hit(event: dict[str, Any], value: float) -> bool:
    return sign(value) == event["direction"]


def summarize_bundles(dirs: list[str]) -> dict[str, Any]:
    cells = {horizon: {"periods": 0, "hits": 0, "events": 0, "valid": 0, "base_total": 0, "base_positive": 0} for horizon in HORIZONS}
    zone_cells = {horizon: {"periods": 0, "hits": 0, "events": 0, "valid": 0, "base_total": 0, "base_positive": 0} for horizon in HORIZONS}
    for directory in dirs:
        root = Path(directory).expanduser()
        rows = read_readings(root / "readings.csv")
        events = read_events(root / "events.csv")
        if len(rows) < max(HORIZONS) + 1:
            continue
        bases = {horizon: base_rate(rows, horizon) for horizon in HORIZONS}
        for horizon in HORIZONS:
            base_total, base_positive = bases[horizon]
            cell = cells[horizon]
            cell["base_total"] += base_total
            cell["base_positive"] += base_positive
            for event in events:
                cell["events"] += 1
                value = forward(rows, event["available"], horizon)
                if value is None:
                    continue
                cell["valid"] += 1
                cell["periods"] += 1
                if event_hit(event, value):
                    cell["hits"] += 1
            zone = zone_cells[horizon]
            zone["base_total"] += base_total
            zone["base_positive"] += base_positive
            for index, row in enumerate(rows):
                rsi_zone = row["rsi_zone"]
                if rsi_zone not in (-1, 1):
                    continue
                zone["events"] += 1
                value = forward(rows, index, horizon)
                if value is None:
                    continue
                zone["valid"] += 1
                zone["periods"] += 1
                expected = -rsi_zone
                if sign(value) == expected:
                    zone["hits"] += 1

    def finish(cells: dict[int, dict[str, Any]], label: str) -> dict[str, Any]:
        result = {}
        for horizon, cell in cells.items():
            n = cell["periods"]
            p = cell["hits"] / n if n else None
            base = cell["base_positive"] / cell["base_total"] if cell["base_total"] else None
            result[horizon] = {
                "label": label,
                "n": n,
                "events": cell["events"],
                "coverage": n / cell["events"] if cell["events"] else 0.0,
                "hits": cell["hits"],
                "hit_rate": p,
                "base_rate": base,
                "lift": (p - base) if p is not None and base is not None else None,
                "lcb": lower_bound(cell["hits"], n),
            }
        return result

    return {"rsi_reversion": finish(cells, "RSI 30/70 reversion"), "rsi_zone": finish(zone_cells, "RSI zone")}


def decision(dev: dict[str, Any], validation: dict[str, Any]) -> str:
    primary = dev["rsi_reversion"][PRIMARY_HORIZON]
    check = validation["rsi_reversion"][PRIMARY_HORIZON]
    if primary["lcb"] is None or primary["n"] < 100:
        return "inconclusive"
    if primary["lcb"] > 0.50 and (primary["lift"] or 0.0) >= 0.01:
        if check["lcb"] is not None and check["lcb"] > 0.50 and (check["lift"] or 0.0) >= 0.01:
            return "proceed_to_macd"
        return "development_only"
    return "stop"


def markdown(document: dict[str, Any]) -> str:
    lines = [
        "# RSI Prediction Pilot",
        "",
        f"- Decision: **{document['decision']}**",
        f"- Primary event: RSI 30/70 reversion",
        f"- Primary horizon: {PRIMARY_HORIZON} bar",
        "",
        "| Partition | Event | Horizon | n | Coverage | Hit | Base | Lift | 95% LCB |",
        "|---|---|---:|---:|---:|---:|---:|---:|---:|",
    ]
    for partition in ("dev", "validation"):
        for event in ("rsi_reversion", "rsi_zone"):
            for horizon in HORIZONS:
                cell = document[partition][event][horizon]
                lines.append(
                    "| {p} | {e} | {h} | {n} | {c:.4f} | {hit} | {base} | {lift} | {lcb} |".format(
                        p=partition,
                        e=event,
                        h=horizon,
                        n=cell["n"],
                        c=cell["coverage"],
                        hit=fmt(cell["hit_rate"]),
                        base=fmt(cell["base_rate"]),
                        lift=fmt(cell["lift"]),
                        lcb=fmt(cell["lcb"]),
                    )
                )
    lines.append("")
    lines.append("`proceed_to_macd` requires the dev and validation primary cells to clear 0.50 LCB and 1pp lift. `development_only` means the validation did not confirm. `stop` means the primary event failed development.")
    lines.append("")
    return "\n".join(lines)


def fmt(value: float | None) -> str:
    return "-" if value is None else f"{value:.4f}"


def main(argv: list[str] | None = None) -> int:
    args = parse_arguments(argv)
    dev = summarize_bundles(args.dev)
    validation = summarize_bundles(args.validation)
    result = {
        "schema": "quantrule.prediction-pilot/v1",
        "question": "Does RSI 30/70 reversion predict the next 1h return sign on EURUSD?",
        "asset": "EURUSD",
        "timeframe_secs": TIMEFRAME_SECS,
        "primary_horizon": PRIMARY_HORIZON,
        "dev": dev,
        "validation": validation,
        "decision": decision(dev, validation),
    }
    out = Path(args.out).expanduser()
    out.mkdir(parents=True, exist_ok=True)
    (out / "result.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    (out / "result.md").write_text(markdown(result), encoding="utf-8")
    print(f"ok RSI pilot -> {out} decision={result['decision']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
