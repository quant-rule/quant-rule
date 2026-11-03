#!/usr/bin/env python3
"""XAGUSD 1h MACD-only prediction pilot.

Primary: MACD line/signal cross, next-1h return sign. Secondary: MACD zero cross
and histogram slope flip. The pilot is prediction-only; it does not model costs,
execution, or a strategy.

Usage:
    python3 tools/improvement/macd_pilot.py \
        --dev data/macd-pilot/xagusd-2018/model \
              data/macd-pilot/xagusd-2019/model \
              data/macd-pilot/xagusd-2020/model \
        --validation data/macd-pilot/xagusd-2021/model \
                    data/macd-pilot/xagusd-2022/model \
        --out data/macd-pilot/MACD
"""

from __future__ import annotations

import argparse
import csv
import json
import math
import sys
from pathlib import Path
from statistics import NormalDist
from typing import Any

TIMEFRAME_SECS = 3600
PRIMARY_HORIZON = 1
Z_RAW = 1.959963984540054

EVENTS = {
    "macd_cross": ("2", "13", "MACD line/signal cross"),
    "macd_zero": ("5", "13", "MACD zero cross"),
    "macd_histogram": ("4", "15", "MACD histogram slope flip"),
}


def parse_arguments(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dev", nargs="+", required=True, help="development model bundle dirs")
    parser.add_argument("--validation", nargs="+", required=True, help="validation model bundle dirs")
    parser.add_argument("--out", required=True, help="output directory")
    return parser.parse_args(argv)


def read_readings(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    with path.open(newline="", encoding="utf-8") as handle:
        for row in csv.DictReader(handle):
            if row.get("timeframe_secs") != str(TIMEFRAME_SECS):
                continue
            try:
                rows.append({"bar": int(row["bar"]), "close": float(row["close"])})
            except (KeyError, TypeError, ValueError):
                continue
    rows.sort(key=lambda row: row["bar"])
    return rows


def read_events(path: Path) -> dict[str, list[dict[str, Any]]]:
    events: dict[str, list[dict[str, Any]]] = {key: [] for key in EVENTS}
    with path.open(newline="", encoding="utf-8") as handle:
        for row in csv.DictReader(handle):
            if row.get("timeframe_secs") != str(TIMEFRAME_SECS):
                continue
            if row.get("event_type_code") != "3":
                continue
            direction = optional_int(row.get("direction_code"))
            if direction not in (-1, 1):
                continue
            for key, (subtype, source, _) in EVENTS.items():
                if row.get("event_subtype_code") == subtype and row.get("source_code") == source:
                    events[key].append(
                        {
                            "available": int(row["available_bar_index"]),
                            "direction": direction,
                        }
                    )
                    break
    return events


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


def forward(rows: list[dict[str, Any]], index: int) -> float | None:
    if index < 0 or index + PRIMARY_HORIZON >= len(rows):
        return None
    return rows[index + PRIMARY_HORIZON]["close"] / rows[index]["close"] - 1.0


def base_rate(rows: list[dict[str, Any]]) -> tuple[int, int]:
    total = 0
    positive = 0
    for index in range(len(rows) - PRIMARY_HORIZON):
        value = forward(rows, index)
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


def summarize(dirs: list[str]) -> dict[str, Any]:
    cells = {
        key: {"events": 0, "valid": 0, "hits": 0, "base_total": 0, "base_positive": 0}
        for key in EVENTS
    }
    for directory in dirs:
        root = Path(directory).expanduser()
        rows = read_readings(root / "readings.csv")
        events = read_events(root / "events.csv")
        if len(rows) <= PRIMARY_HORIZON:
            continue
        base_total, base_positive = base_rate(rows)
        for key in EVENTS:
            cell = cells[key]
            cell["base_total"] += base_total
            cell["base_positive"] += base_positive
            for event in events[key]:
                cell["events"] += 1
                value = forward(rows, event["available"])
                if value is None:
                    continue
                cell["valid"] += 1
                if sign(value) == event["direction"]:
                    cell["hits"] += 1
    result = {}
    for key, cell in cells.items():
        n = cell["valid"]
        p = cell["hits"] / n if n else None
        base = cell["base_positive"] / cell["base_total"] if cell["base_total"] else None
        result[key] = {
            "label": EVENTS[key][2],
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


def decision(dev: dict[str, Any], validation: dict[str, Any]) -> str:
    primary = dev["macd_cross"]
    check = validation["macd_cross"]
    if primary["n"] < 100:
        return "inconclusive"
    if primary["lcb"] is not None and primary["lcb"] > 0.50 and (primary["lift"] or 0.0) >= 0.01:
        if (
            check["lcb"] is not None
            and check["lcb"] > 0.50
            and (check["lift"] or 0.0) >= 0.01
        ):
            return "proceed_to_conjunction"
        return "development_only"
    return "stop"


def fmt(value: float | None) -> str:
    return "-" if value is None else f"{value:.4f}"


def markdown(document: dict[str, Any]) -> str:
    lines = [
        "# XAGUSD MACD Prediction Pilot",
        "",
        f"- Decision: **{document['decision']}**",
        "- Primary event: MACD line/signal cross",
        f"- Primary horizon: {PRIMARY_HORIZON} bar",
        "",
        "| Partition | Event | n | Coverage | Hit | Base | Lift | 95% LCB |",
        "|---|---|---:|---:|---:|---:|---:|---:|",
    ]
    for partition in ("dev", "validation"):
        for key in EVENTS:
            cell = document[partition][key]
            lines.append(
                "| {p} | {e} | {n} | {c:.4f} | {hit} | {base} | {lift} | {lcb} |".format(
                    p=partition,
                    e=key,
                    n=cell["n"],
                    c=cell["coverage"],
                    hit=fmt(cell["hit_rate"]),
                    base=fmt(cell["base_rate"]),
                    lift=fmt(cell["lift"]),
                    lcb=fmt(cell["lcb"]),
                )
            )
    lines.append("")
    lines.append(
        "`proceed_to_conjunction` requires the primary line/signal cross to clear "
        "0.50 LCB and 1pp lift on both partitions. `stop` means it failed development."
    )
    lines.append("")
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    args = parse_arguments(argv)
    dev = summarize(args.dev)
    validation = summarize(args.validation)
    result = {
        "schema": "quantrule.macd-pilot/v1",
        "question": "Does the MACD line/signal cross predict the next 1h return sign on XAGUSD?",
        "asset": "XAGUSD",
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
    print(f"ok MACD pilot -> {out} decision={result['decision']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
