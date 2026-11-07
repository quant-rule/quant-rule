#!/usr/bin/env python3
"""Descriptive path and expectancy evaluation for a model cell.

This is an evaluation artifact, not a signal. It reads the model's published
prediction and forward OHLCV readings, optionally filters by the condition
column, and reports MFE/MAE, time-to-target, time-to-stop, and expected R under
a pre-registered exit menu. The primary model gate remains the hit-rate
evaluator; this module answers whether the path is asymmetric enough to matter
after costs.
"""

from __future__ import annotations

import argparse
import csv
import json
import math
from pathlib import Path
from statistics import mean, median
from typing import Any

SCHEMA = "quantrule.model-path/v1"
DEFAULT_MENU = (
    {"target_atr": 2.0, "stop_atr": 1.0, "horizon": 20},
    {"target_atr": 1.0, "stop_atr": 1.0, "horizon": 20},
    {"target_atr": 3.0, "stop_atr": 1.0, "horizon": 40},
)


def parse_arguments(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bundle", action="append", required=True, help="instrument=path")
    parser.add_argument("--spec-id", default=None)
    parser.add_argument("--family", default=None)
    parser.add_argument("--partition", default=None)
    parser.add_argument("--condition-column", default=None)
    parser.add_argument("--menu", default=None, help="JSON exit menu")
    parser.add_argument("--cost-bps", type=float, default=6.0)
    parser.add_argument("--out", required=True)
    return parser.parse_args(argv)


def read_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise SystemExit(f"error: cannot read {path}: {error}") from None


def read_csv(path: Path) -> tuple[list[str], list[dict[str, str]]]:
    try:
        with path.open(newline="", encoding="utf-8") as handle:
            reader = csv.DictReader(handle)
            rows = list(reader)
    except OSError as error:
        raise SystemExit(f"error: cannot read {path}: {error}") from None
    return list(reader.fieldnames or []), rows


def optional_float(value: Any) -> float | None:
    try:
        number = float(value)
    except (TypeError, ValueError):
        return None
    return number if math.isfinite(number) else None


def optional_int(value: Any) -> int | None:
    number = optional_float(value)
    if number is None or not number.is_integer():
        return None
    return int(number)


def quantile(values: list[float], fraction: float) -> float | None:
    if not values:
        return None
    ordered = sorted(values)
    index = min(len(ordered) - 1, int(fraction * (len(ordered) - 1)))
    return ordered[index]


def parse_menu(args: argparse.Namespace) -> list[dict[str, float]]:
    if args.menu:
        document = read_json(Path(args.menu).expanduser())
        if not isinstance(document, list) or not document:
            raise SystemExit("error: menu must be a non-empty JSON list")
        menu = document
    else:
        menu = [dict(item) for item in DEFAULT_MENU]
    result = []
    for item in menu:
        if not isinstance(item, dict):
            raise SystemExit("error: menu item must be an object")
        target = optional_float(item.get("target_atr"))
        stop = optional_float(item.get("stop_atr"))
        horizon = optional_int(item.get("horizon"))
        if target is None or stop is None or horizon is None:
            raise SystemExit("error: menu item needs target_atr, stop_atr, horizon")
        if target <= 0 or stop <= 0 or horizon <= 0:
            raise SystemExit("error: menu values must be positive")
        result.append(
            {"target_atr": target, "stop_atr": stop, "horizon": horizon}
        )
    return result


def bundle_paths(values: list[str]) -> list[tuple[str, Path]]:
    result = []
    for value in values:
        if "=" not in value:
            raise SystemExit(f"error: --bundle must be name=path, got {value!r}")
        name, path = value.split("=", 1)
        result.append((name, Path(path).expanduser()))
    return result


def path_for(
    rows: list[dict[str, Any]],
    index: int,
    direction: int,
    target_atr: float,
    stop_atr: float,
    horizon: int,
    cost_bps: float,
) -> dict[str, Any] | None:
    if index + horizon >= len(rows):
        return None
    entry_row = rows[index]
    atr = entry_row["atr"]
    entry = entry_row["close"]
    if atr is None or atr <= 0:
        return None
    target = entry + direction * target_atr * atr
    stop = entry - direction * stop_atr * atr
    mfe = 0.0
    mae = 0.0
    target_time = None
    stop_time = None
    exit_r = None
    exit_kind = "timeout"
    for offset in range(1, horizon + 1):
        row = rows[index + offset]
        if direction > 0:
            favorable = (row["high"] - entry) / atr
            adverse = (row["low"] - entry) / atr
        else:
            favorable = (entry - row["low"]) / atr
            adverse = (entry - row["high"]) / atr
        mfe = max(mfe, favorable)
        mae = min(mae, adverse)
        if direction > 0:
            hit_stop = row["low"] <= stop
            hit_target = row["high"] >= target
        else:
            hit_stop = row["high"] >= stop
            hit_target = row["low"] <= target
        if hit_stop:
            stop_time = offset
            exit_r = -stop_atr
            exit_kind = "stop"
            break
        if hit_target:
            target_time = offset
            exit_r = target_atr
            exit_kind = "target"
            break
    if exit_r is None:
        exit_r = direction * (rows[index + horizon]["close"] - entry) / atr
    cost = cost_bps / 10_000.0 * entry / atr
    return {
        "direction": direction,
        "mfe_atr": mfe,
        "mae_atr": mae,
        "exit_r": exit_r - cost,
        "gross_r": exit_r,
        "exit": exit_kind,
        "time_to_target": target_time,
        "time_to_stop": stop_time,
        "bars_held": (
            target_time
            if target_time is not None
            else stop_time
            if stop_time is not None
            else horizon
        ),
    }


def summarize(rows: list[dict[str, Any]]) -> dict[str, Any]:
    if not rows:
        return {"n": 0}
    net = [row["exit_r"] for row in rows]
    mfe = [row["mfe_atr"] for row in rows]
    mae = [row["mae_atr"] for row in rows]
    targets = [row["time_to_target"] for row in rows if row["time_to_target"] is not None]
    stops = [row["time_to_stop"] for row in rows if row["time_to_stop"] is not None]
    return {
        "n": len(rows),
        "mean_net_r": mean(net),
        "median_net_r": median(net),
        "win_rate": sum(1 for value in net if value > 0) / len(net),
        "mean_mfe_atr": mean(mfe),
        "median_mfe_atr": median(mfe),
        "p10_mfe_atr": quantile(mfe, 0.10),
        "p90_mfe_atr": quantile(mfe, 0.90),
        "mean_mae_atr": mean(mae),
        "median_mae_atr": median(mae),
        "p10_mae_atr": quantile(mae, 0.10),
        "p90_mae_atr": quantile(mae, 0.90),
        "target_count": len(targets),
        "stop_count": len(stops),
        "timeout_count": len(rows) - len(targets) - len(stops),
        "mean_time_to_target": mean(targets) if targets else None,
        "mean_time_to_stop": mean(stops) if stops else None,
    }


def main(argv: list[str] | None = None) -> int:
    args = parse_arguments(argv)
    bundles = bundle_paths(args.bundle)
    menu = parse_menu(args)
    out = Path(args.out).expanduser()
    out.mkdir(parents=True, exist_ok=True)
    instruments: dict[str, list[dict[str, Any]]] = {}
    all_rows: list[dict[str, Any]] = []
    for name, bundle in bundles:
        _, predictions = read_csv(bundle / "prediction.csv")
        _, readings = read_csv(bundle / "readings.csv")
        readings_by_bar = {int(row["bar"]): row for row in readings}
        series = [
            {
                "close": optional_float(row["close"]),
                "high": optional_float(row["high"]),
                "low": optional_float(row["low"]),
                "atr": optional_float(row["atr"]),
            }
            for row in readings
        ]
        rows = []
        for prediction in predictions:
            try:
                bar = int(prediction["bar"])
                direction = int(prediction["prediction"])
            except (KeyError, TypeError, ValueError):
                continue
            if direction not in (-1, 1):
                continue
            reading = readings_by_bar.get(bar)
            if reading is None:
                continue
            if args.condition_column is not None:
                condition = optional_int(reading.get(args.condition_column))
                if condition != 1:
                    continue
            try:
                close = optional_float(reading["close"])
                atr = optional_float(reading["atr"])
            except KeyError:
                continue
            if close is None or atr is None:
                continue
            path_rows = []
            for item in menu:
                path = path_for(
                    series,
                    bar,
                    direction,
                    item["target_atr"],
                    item["stop_atr"],
                    item["horizon"],
                    args.cost_bps,
                )
                if path is not None:
                    path_rows.append(
                        {
                            "menu": item,
                            "path": path,
                        }
                    )
            if path_rows:
                rows.append({"bar": bar, "direction": direction, "paths": path_rows})
        instruments[name] = rows
        all_rows.extend(rows)
    summaries = []
    for item in menu:
        key = f"{item['target_atr']}/{item['stop_atr']}x{item['horizon']}"
        selected = [row["paths"][index]["path"] for row in all_rows for index in range(len(row["paths"])) if row["paths"][index]["menu"] == item]
        summaries.append({"menu": item, "key": key, "summary": summarize(selected)})
    document = {
        "schema": SCHEMA,
        "spec_id": args.spec_id,
        "family": args.family,
        "partition": args.partition,
        "cost_bps": args.cost_bps,
        "condition_column": args.condition_column,
        "instruments": {
            name: {
                "n": len(rows),
                "bars": [row["bar"] for row in rows],
            }
            for name, rows in instruments.items()
        },
        "menu": menu,
        "summaries": summaries,
    }
    (out / "path.json").write_text(
        json.dumps(document, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )
    lines = [
        "# Model Path and Expectancy",
        "",
        f"- Spec: `{args.spec_id}`",
        f"- Partition: `{args.partition}`",
        f"- Condition column: `{args.condition_column}`",
        f"- Cost: {args.cost_bps} bps",
        "",
        "| Exit | n | Mean net R | Median net R | Win rate | Mean MFE | Mean MAE | Targets | Stops | Timeouts |",
        "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|",
    ]
    for row in summaries:
        summary = row["summary"]
        lines.append(
            f"| {row['key']} | {summary.get('n', 0)} | "
            f"{summary.get('mean_net_r', '—')} | {summary.get('median_net_r', '—')} | "
            f"{summary.get('win_rate', '—')} | {summary.get('mean_mfe_atr', '—')} | "
            f"{summary.get('mean_mae_atr', '—')} | {summary.get('target_count', '—')} | "
            f"{summary.get('stop_count', '—')} | {summary.get('timeout_count', '—')} |"
        )
    lines.append("")
    (out / "path.md").write_text("\n".join(lines), encoding="utf-8")
    print(f"ok path spec={args.spec_id} menu={len(menu)} -> {out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
