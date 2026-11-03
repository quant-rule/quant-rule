#!/usr/bin/env python3
"""Grade model states and events against their forward claims.

The harness reads the model bundle's `readings.csv` and `events.csv`, joins each
observation at its availability bar, and measures the outcome the claim names.
It reports the hit rate, the unconditional base rate, the lift, a confidence
lower bound, coverage, and replication across instruments.

Statistics is confined to this evaluation layer. No value produced here enters
the model or a strategy.

Usage:
    python3 tools/improvement/claim_validation.py \
        --cells cells.json --out data/improvement/2026-11-03/MODEL-GATE

`cells.json` is a list of objects:

    [{"instrument": "BTCUSD", "partition": "model_dev",
      "run": "data/.../MODEL-.../artifacts/model"}]
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

HORIZONS = [1, 3, 5, 10, 20]
PRIMARY_HORIZON = 5
PRIMARY_PROGRESSION_HORIZON = 10
TIMEFRAME_CODES = {300: "5m", 900: "15m", 3600: "1h"}
STATE_COLUMNS = [
    ("ema_stack", "EMA stack"),
    ("rsi_zone", "RSI zone"),
    ("macd_state", "MACD state"),
    ("macd_histogram_state", "MACD histogram sign"),
    ("di_state", "DI state"),
    ("progression_leg_direction", "Progression leg direction"),
]
REGISTRY = Path(__file__).resolve().parent.parent.parent / "docs/registry/indicators.json"


def parse_arguments(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cells", required=True, help="JSON list of model cells")
    parser.add_argument("--out", required=True, help="output directory")
    parser.add_argument(
        "--timeframes",
        default="5m,15m,1h",
        help="comma-separated timeframe codes (default: 5m,15m,1h)",
    )
    parser.add_argument(
        "--partition",
        choices=("all", "model_dev", "model_validation"),
        default="all",
        help="restrict the cells to one partition (default: all)",
    )
    return parser.parse_args(argv)


def event_names() -> dict[tuple[int, int], str]:
    """Map event codes to registry IDs, preferring a source-qualified Signal key."""
    try:
        registry = json.loads(REGISTRY.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise SystemExit(f"error: cannot read {REGISTRY}: {error}") from None
    names: dict[tuple[int, int], str] = {}
    for event in registry.get("events", []):
        if event.get("status") != "active":
            continue
        event_type = int(event["event_type"])
        subtype = int(event["subtype"])
        sources = event.get("sources", [event.get("source")])
        if event_type == 3 and sources:
            for source in sources:
                names.setdefault((event_type, subtype, int(source)), str(event["id"]))
        names.setdefault((event_type, subtype), str(event["id"]))
    return names


def read_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise SystemExit(f"error: cannot read {path}: {error}") from None


def read_readings(path: Path) -> dict[int, list[dict[str, Any]]]:
    series: dict[int, list[dict[str, Any]]] = defaultdict(list)
    with path.open(newline="", encoding="utf-8") as handle:
        reader = csv.DictReader(handle)
        for row in reader:
            try:
                timeframe = int(row["timeframe_secs"])
                bar = int(row["bar"])
                close = float(row["close"])
            except (KeyError, TypeError, ValueError):
                continue
            if close <= 0.0:
                continue
            parsed = {
                "bar": bar,
                "close": close,
                "ema_stack": integer(row.get("ema_stack")),
                "rsi_zone": integer(row.get("rsi_zone")),
                "macd_state": integer(row.get("macd_state")),
                "macd_histogram_state": integer(row.get("macd_histogram_state")),
                "di_state": integer(row.get("di_state")),
                "progression_leg_direction": integer(row.get("progression_leg_direction")),
            }
            series[timeframe].append(parsed)
    for rows in series.values():
        rows.sort(key=lambda row: row["bar"])
    return series


def integer(value: Any) -> int | None:
    if value is None or value == "":
        return None
    try:
        return int(value)
    except (TypeError, ValueError):
        return None


def read_events(path: Path) -> dict[int, list[dict[str, Any]]]:
    events: dict[int, list[dict[str, Any]]] = defaultdict(list)
    with path.open(newline="", encoding="utf-8") as handle:
        reader = csv.DictReader(handle)
        for row in reader:
            try:
                timeframe = int(row["timeframe_secs"])
                event = {
                    "event_type": int(row["event_type_code"]),
                    "subtype": int(row["event_subtype_code"]),
                    "source": int(row["source_code"]),
                    "available": int(row["available_bar_index"]),
                    "direction": int(row["direction_code"]),
                }
            except (KeyError, TypeError, ValueError):
                continue
            events[timeframe].append(event)
    return events


def forward_return(rows: list[dict[str, Any]], index: int, horizon: int) -> float | None:
    target = index + horizon
    if target >= len(rows):
        return None
    return rows[target]["close"] / rows[index]["close"] - 1.0


def sign(value: float) -> int:
    if value > 0.0:
        return 1
    if value < 0.0:
        return -1
    return 0


def base_rates(rows: list[dict[str, Any]], horizon: int) -> tuple[int, int]:
    total = 0
    positive = 0
    for index in range(len(rows) - horizon):
        value = forward_return(rows, index, horizon)
        if value is None:
            continue
        total += 1
        if value > 0.0:
            positive += 1
    return total, positive


def lower_bound(hits: int, n: int, z: float) -> float | None:
    if n <= 0:
        return None
    p = hits / n
    se = math.sqrt(max(p * (1.0 - p), 0.0) / n)
    return max(0.0, p - z * se)


def family_key(kind: str, name: str, timeframe: str, horizon: int) -> tuple[str, str, str, int]:
    return (kind, name, timeframe, horizon)


def add_cell(
    cells: dict[tuple[str, str, str, int], dict[str, Any]],
    kind: str,
    name: str,
    timeframe: str,
    horizon: int,
    instrument: str,
    hits: int,
    n: int,
    available: int,
    base_total: int,
    base_positive: int,
) -> None:
    key = family_key(kind, name, timeframe, horizon)
    cell = cells.setdefault(
        key,
        {
            "kind": kind,
            "family": name,
            "timeframe": timeframe,
            "horizon": horizon,
            "n": 0,
            "hits": 0,
            "available": 0,
            "base_total": 0,
            "base_positive": 0,
            "instruments": {},
        },
    )
    cell["n"] += n
    cell["hits"] += hits
    cell["available"] += available
    cell["base_total"] += base_total
    cell["base_positive"] += base_positive
    per = cell["instruments"].setdefault(instrument, {"n": 0, "hits": 0})
    per["n"] += n
    per["hits"] += hits


def grade(cell: dict[str, Any], z_corrected: float) -> str:
    n = cell["n"]
    if n == 0:
        return "C"
    coverage = n / cell["available"] if cell["available"] else 0.0
    if coverage < 0.80:
        return "D"
    p = cell["hits"] / n
    base = cell["base_positive"] / cell["base_total"] if cell["base_total"] else None
    lift = (p - base) if base is not None else None
    lcb = lower_bound(cell["hits"], n, z_corrected)
    replication = sum(
        1
        for value in cell["instruments"].values()
        if value["n"] >= 30 and value["hits"] / value["n"] > 0.50
    )
    if (
        coverage >= 0.95
        and lcb is not None
        and lcb > 0.55
        and lift is not None
        and lift >= 0.02
        and replication >= 3
    ):
        return "A"
    if (
        coverage >= 0.90
        and lcb is not None
        and lcb > 0.50
        and lift is not None
        and lift >= 0.01
        and replication >= 2
    ):
        return "B"
    return "C"


def summarize(cells: dict[tuple[str, str, str, int], dict[str, Any]]) -> list[dict[str, Any]]:
    primary = [
        cell
        for key, cell in cells.items()
        if key[3] == (PRIMARY_PROGRESSION_HORIZON if key[0] == "progression" else PRIMARY_HORIZON)
    ]
    m = max(len(primary), 1)
    z_corrected = NormalDist().inv_cdf(1.0 - 0.05 / (2.0 * m))
    z_raw = 1.959963984540054
    result = []
    for key in sorted(cells):
        cell = cells[key]
        n = cell["n"]
        if n == 0:
            continue
        p = cell["hits"] / n
        base = cell["base_positive"] / cell["base_total"] if cell["base_total"] else None
        coverage = n / cell["available"] if cell["available"] else 0.0
        replication = sum(
            1
            for value in cell["instruments"].values()
            if value["n"] >= 30 and value["hits"] / value["n"] > 0.50
        )
        is_primary = key[3] == (
            PRIMARY_PROGRESSION_HORIZON if key[0] == "progression" else PRIMARY_HORIZON
        )
        result.append(
            {
                "kind": cell["kind"],
                "family": cell["family"],
                "timeframe": cell["timeframe"],
                "horizon": cell["horizon"],
                "n": n,
                "hits": cell["hits"],
                "hit_rate": p,
                "base_rate": base,
                "lift": (p - base) if base is not None else None,
                "coverage": coverage,
                "lcb_raw": lower_bound(cell["hits"], n, z_raw),
                "lcb_corrected": lower_bound(cell["hits"], n, z_corrected),
                "replication": replication,
                "primary": is_primary,
                "grade": grade(cell, z_corrected) if is_primary else None,
            }
        )
    return result


def validate(cells_document: list[dict[str, Any]], timeframes: set[int]) -> dict[str, Any]:
    names = event_names()
    cells: dict[tuple[str, str, str, int], dict[str, Any]] = {}
    for entry in cells_document:
        run = Path(entry["run"]).expanduser()
        instrument = str(entry["instrument"])
        partition = str(entry["partition"])
        readings = read_readings(run / "readings.csv")
        events = read_events(run / "events.csv")
        for timeframe in sorted(timeframes):
            rows = readings.get(timeframe, [])
            if len(rows) <= max(HORIZONS):
                continue
            label = TIMEFRAME_CODES.get(timeframe, str(timeframe))
            bases = {horizon: base_rates(rows, horizon) for horizon in HORIZONS}
            for column, name in STATE_COLUMNS:
                for horizon in HORIZONS:
                    base_total, base_positive = bases[horizon]
                    hits = 0
                    n = 0
                    available = 0
                    for index, row in enumerate(rows):
                        state = row[column]
                        if state is None or state == 0:
                            continue
                        value = forward_return(rows, index, horizon)
                        if value is None:
                            continue
                        available += 1
                        if sign(value) == state:
                            hits += 1
                        n += 1
                    add_cell(
                        cells,
                        "progression" if column == "progression_leg_direction" else "state",
                        name,
                        label,
                        horizon,
                        instrument,
                        hits,
                        n,
                        available,
                        base_total,
                        base_positive,
                    )
            for event in events.get(timeframe, []):
                if event["direction"] == 0:
                    continue
                key = (event["event_type"], event["subtype"], event["source"])
                name = names.get(key) or names.get((event["event_type"], event["subtype"]))
                if name is None:
                    continue
                index = event["available"]
                if index >= len(rows):
                    continue
                for horizon in HORIZONS:
                    value = forward_return(rows, index, horizon)
                    if value is None:
                        continue
                    base_total, base_positive = bases[horizon]
                    hit = 1 if sign(value) == event["direction"] else 0
                    add_cell(
                        cells,
                        "event",
                        name,
                        label,
                        horizon,
                        instrument,
                        hit,
                        1,
                        1,
                        base_total,
                        base_positive,
                    )
    summary = summarize(cells)
    primary = [cell for cell in summary if cell["primary"]]
    grades = {cell["family"]: cell["grade"] for cell in primary}
    passing = [
        cell["family"]
        for cell in primary
        if cell["grade"] in ("A", "B")
    ]
    gate = "pass" if passing else ("insufficient" if not primary else "no_edge")
    return {
        "schema": "quantrule.claim-validation/v1",
        "version": "2026-11-03.2",
        "cells": summary,
        "primary": primary,
        "grades": grades,
        "passing_signals": passing,
        "gate": gate,
        "method": {
            "horizons": HORIZONS,
            "primary_horizon": PRIMARY_HORIZON,
            "progression_primary_horizon": PRIMARY_PROGRESSION_HORIZON,
            "confidence": "normal approximation, one-sided lower bound",
            "multiple_testing": "Bonferroni over primary cells",
            "event_universe": "active registry events; Signal events keyed by source",
            "replication": "instrument with n>=30 and hit rate > 0.50",
            "minimum_lift": {"A": 0.02, "B": 0.01},
        },
    }


def format_rate(value: float | None) -> str:
    return "-" if value is None else f"{value:.4f}"


def markdown(document: dict[str, Any]) -> str:
    lines = [
        "# Claim Validation",
        "",
        f"- Gate: **{document['gate']}**",
        f"- Primary cells: {len(document['primary'])}",
        f"- Passing signals: {', '.join(document['passing_signals']) or 'none'}",
        "",
        "| Kind | Family | TF | Horizon | n | Coverage | Hit | Base | Lift | LCB | Replication | Grade |",
        "|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---|",
    ]
    for cell in document["primary"]:
        lines.append(
            "| {kind} | {family} | {tf} | {horizon} | {n} | {coverage} | {hit} | {base} | {lift} | {lcb} | {rep} | {grade} |".format(
                kind=cell["kind"],
                family=cell["family"],
                tf=cell["timeframe"],
                horizon=cell["horizon"],
                n=cell["n"],
                coverage=format_rate(cell["coverage"]),
                hit=format_rate(cell["hit_rate"]),
                base=format_rate(cell["base_rate"]),
                lift=format_rate(cell["lift"]),
                lcb=format_rate(cell["lcb_corrected"]),
                rep=cell["replication"],
                grade=cell["grade"],
            )
        )
    lines.append("")
    lines.append(
        "A/B require a positive corrected lower bound, coverage, and replication; "
        "C is diagnostic only; D blocks. Secondary horizons are in `validation.json`."
    )
    lines.append("")
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    args = parse_arguments(argv)
    document = read_json(Path(args.cells).expanduser())
    if not isinstance(document, list):
        print("error: --cells must be a JSON list", file=sys.stderr)
        return 1
    if args.partition != "all":
        document = [entry for entry in document if entry.get("partition") == args.partition]
        if not document:
            print(f"error: no cells for partition {args.partition}", file=sys.stderr)
            return 1
    timeframes = set()
    for code in args.timeframes.split(","):
        code = code.strip()
        for seconds, label in TIMEFRAME_CODES.items():
            if label == code:
                timeframes.add(seconds)
                break
        else:
            print(f"error: unknown timeframe {code}", file=sys.stderr)
            return 1
    result = validate(document, timeframes)
    out = Path(args.out).expanduser()
    out.mkdir(parents=True, exist_ok=True)
    (out / "validation.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    (out / "validation.md").write_text(markdown(result), encoding="utf-8")
    with (out / "cells.csv").open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(
            handle,
            fieldnames=[
                "kind", "family", "timeframe", "horizon", "n", "hits", "hit_rate",
                "base_rate", "lift", "coverage", "lcb_raw", "lcb_corrected",
                "replication", "primary", "grade",
            ],
        )
        writer.writeheader()
        for cell in result["cells"]:
            writer.writerow(cell)
    print(f"ok claim validation -> {out} gate={result['gate']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
