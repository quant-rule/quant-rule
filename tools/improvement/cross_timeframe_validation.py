#!/usr/bin/env python3
"""Cross-timeframe path validation: 15m event conditioned on the 1h trend state.

The gate asks two questions:

1. Does the 15m event hit its ATR target more often when the last closed 1h bar
   has `ema_stack` in the event direction and `adx_regime == 1`, compared with
   the same 15m event without the 1h filter?
2. Does the 15m event add beyond the 1h state base itself?

The 1h state is read from the last closed 1h bar at the 15m event's availability
bar. It never reads a forming 1h bar.

Statistics is evaluation only. Nothing here enters the model or a strategy.

Usage:
    python3 tools/improvement/cross_timeframe_validation.py \
        --matrix config/model-v4/dev/matrix.json \
        --families config/research/<id>/cells/CELL-01/families.json \
        --out data/research/<id>/dev/CELL-01 \
        --primary-cells 3 \
        --seed 0
"""

from __future__ import annotations

import argparse
import bisect
import csv
import json
import math
import random
import sys
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path
from statistics import NormalDist
from typing import Any

import path_validation

SCHEMA = "quantrule.cross-timeframe-validation/v1"
MIN_EVENTS = 200
MIN_CLUSTERS = 20
MIN_CONDITIONAL_LIFT = 0.05
MIN_INCREMENTAL_LIFT = 0.02
RANDOM_DRAWS = 200
SHUFFLE_DRAWS = 200
CONTROL_ALPHA = 0.05


@dataclass
class Options:
    horizon: int = 20
    target_atr: float = 2.0
    stop_atr: float = 1.0
    random_draws: int = RANDOM_DRAWS
    shuffle_draws: int = SHUFFLE_DRAWS
    back_lag: int = 20
    seed: int = 0


@dataclass
class Cell:
    instrument: str
    fold: int
    partition: str
    config: str
    run: Path


@dataclass
class Bundle:
    instrument: str
    filtered: list[tuple[int, int, bool]]
    prepared: list[tuple[int, int, bool]]
    state_for_bar: list[int | None]
    hits: dict[int, list[bool | None]]
    state_totals: dict[int, tuple[int, int]]
    pools: dict[int, list[int]]


def read_rows(path: Path) -> dict[int, list[dict[str, Any]]]:
    series: dict[int, list[dict[str, Any]]] = defaultdict(list)
    with path.open(newline="", encoding="utf-8") as handle:
        for row in csv.DictReader(handle):
            try:
                timeframe = int(row["timeframe_secs"])
            except (KeyError, TypeError, ValueError):
                continue
            if timeframe not in path_validation.TIMEFRAME_CODES:
                continue
            try:
                bar = int(row["bar"])
                timestamp = int(row["timestamp"])
                high = float(row["high"])
                low = float(row["low"])
                close = float(row["close"])
            except (KeyError, TypeError, ValueError):
                continue
            series[timeframe].append(
                {
                    "bar": bar,
                    "timestamp": timestamp,
                    "high": high,
                    "low": low,
                    "close": close,
                    "atr": path_validation.number(row.get("atr", "")),
                    "ema_stack": path_validation.number(row.get("ema_stack", "")),
                    "adx_regime": path_validation.number(row.get("adx_regime", "")),
                }
            )
    for rows in series.values():
        rows.sort(key=lambda row: row["bar"])
    return series


def last_closed_index(one_hour_timestamps: list[int], t15: int) -> int | None:
    """The last 1h bar whose close is at or before the 15m bar's close."""
    cutoff = t15 - 2_700
    position = bisect.bisect_right(one_hour_timestamps, cutoff) - 1
    return position if position >= 0 else None


def load_bundle(cell: Cell, family: dict[str, Any], options: Options) -> Bundle | None:
    rows_by_tf = read_rows(cell.run / "readings.csv")
    events_by_tf = path_validation.read_events(cell.run / "events.csv")
    rows15 = rows_by_tf.get(900, [])
    rows1h = rows_by_tf.get(3600, [])
    if not rows15 or not rows1h:
        return None
    hits = path_validation.barrier_arrays(
        rows15, options.horizon, options.target_atr, options.stop_atr
    )
    one_hour = [int(row["timestamp"]) for row in rows1h]
    state_for_bar: list[int | None] = []
    state_totals: dict[int, list[int]] = {direction: [0, 0] for direction in (-1, 1)}
    for index, row in enumerate(rows15):
        position = last_closed_index(one_hour, int(row["timestamp"]))
        direction = None
        if position is not None:
            state = rows1h[position]
            ema = state["ema_stack"]
            adx = state["adx_regime"]
            if adx == 1.0 and ema in (-1.0, 1.0):
                direction = int(ema)
        state_for_bar.append(direction)
        if direction is not None:
            hit = hits[direction][index]
            if hit is not None:
                state_totals[direction][0] += int(hit)
                state_totals[direction][1] += 1
    prepared, _raw = path_validation.select_events(
        events_by_tf.get(900, []), rows15, family
    )
    prepared_valid: list[tuple[int, int, bool]] = []
    for event in prepared:
        index = int(event["available"])
        direction = int(event["direction"])
        hit = hits[direction][index]
        if hit is not None:
            prepared_valid.append((index, direction, bool(hit)))
    filtered = [
        (index, direction, hit)
        for index, direction, hit in prepared_valid
        if state_for_bar[index] == direction
    ]
    filtered_bars: dict[int, set[int]] = {direction: set() for direction in (-1, 1)}
    for index, direction, _hit in filtered:
        filtered_bars[direction].add(index)
    pools: dict[int, list[int]] = {}
    for direction in (-1, 1):
        pools[direction] = [
            index
            for index in range(len(rows15))
            if state_for_bar[index] == direction
            and hits[direction][index] is not None
            and index not in filtered_bars[direction]
        ]
    return Bundle(
        instrument=cell.instrument,
        filtered=filtered,
        prepared=prepared_valid,
        state_for_bar=state_for_bar,
        hits=hits,
        state_totals={
            direction: (value[0], value[1]) for direction, value in state_totals.items()
        },
        pools=pools,
    )


def evaluate(
    bundles: list[Bundle],
    family_id: str,
    options: Options,
    primary_cells: int,
) -> dict[str, Any]:
    n = sum(len(bundle.filtered) for bundle in bundles)
    clusters = 0
    variance = 0.0
    hits_total = 0
    entries: list[tuple[int, int, bool]] = []
    for bundle_index, bundle in enumerate(bundles):
        for index, _direction, hit in bundle.filtered:
            entries.append((bundle_index, index, hit))
            hits_total += int(hit)
    stats = path_validation.cluster_stats(entries, options.horizon)
    p = stats["hit_rate"]
    se = stats["se"]
    z = path_validation.primary_z(primary_cells)

    prepared_hits = sum(int(hit) for bundle in bundles for _, _, hit in bundle.prepared)
    prepared_n = sum(len(bundle.prepared) for bundle in bundles)
    event_rate = prepared_hits / prepared_n if prepared_n else None

    state_rates = []
    for bundle in bundles:
        for _index, direction, _hit in bundle.filtered:
            hits, valid = bundle.state_totals[direction]
            if valid:
                state_rates.append(hits / valid)
    state_base = sum(state_rates) / len(state_rates) if state_rates else None

    conditional_lift = (
        p - event_rate if p is not None and event_rate is not None else None
    )
    incremental = p - state_base if p is not None and state_base is not None else None
    conditional_lcb = path_validation.lower(conditional_lift, se, z)
    incremental_lcb = path_validation.lower(incremental, se, z)

    rng = random.Random(options.seed)
    random_draws = []
    for _ in range(options.random_draws):
        hits = 0
        total = 0
        for bundle in bundles:
            for direction in (-1, 1):
                count = sum(1 for _, event_direction, _ in bundle.filtered if event_direction == direction)
                if count == 0:
                    continue
                pool = bundle.pools[direction]
                sample = rng.sample(pool, min(count, len(pool)))
                for index in sample:
                    hit = bundle.hits[direction][index]
                    if hit is not None:
                        hits += int(hit)
                        total += 1
        random_draws.append(hits / total if total else None)
    random_p = path_validation.control_p(p, random_draws) if p is not None else None

    rng = random.Random(options.seed + 1)
    shuffle_draws = []
    for _ in range(options.shuffle_draws):
        hits = 0
        total = 0
        for bundle in bundles:
            directions = [direction for _, direction, _ in bundle.filtered]
            rng.shuffle(directions)
            for (index, _direction, _hit), direction in zip(bundle.filtered, directions):
                hit = bundle.hits[direction][index]
                if hit is not None:
                    hits += int(hit)
                    total += 1
        shuffle_draws.append(hits / total if total else None)
    shuffle_p = path_validation.control_p(p, shuffle_draws) if p is not None else None

    back_hits = 0
    back_total = 0
    for bundle in bundles:
        for index, direction, _hit in bundle.filtered:
            lagged = index - options.back_lag
            if 0 <= lagged < len(bundle.state_for_bar):
                hit = bundle.hits[direction][lagged]
                if hit is not None:
                    back_hits += int(hit)
                    back_total += 1
    back_rate = back_hits / back_total if back_total else None
    back_lift = (
        back_rate - event_rate
        if back_rate is not None and event_rate is not None
        else None
    )

    instruments: dict[str, dict[str, Any]] = {}
    by_instrument: dict[str, list[Bundle]] = defaultdict(list)
    for bundle in bundles:
        by_instrument[bundle.instrument].append(bundle)
    for instrument, values in sorted(by_instrument.items()):
        instrument_n = sum(len(bundle.filtered) for bundle in values)
        instrument_hits = sum(
            int(hit) for bundle in values for _, _, hit in bundle.filtered
        )
        instrument_state = []
        for bundle in values:
            for _index, direction, _hit in bundle.filtered:
                hits, valid = bundle.state_totals[direction]
                if valid:
                    instrument_state.append(hits / valid)
        base = sum(instrument_state) / len(instrument_state) if instrument_state else None
        instruments[instrument] = {
            "n": instrument_n,
            "hits": instrument_hits,
            "hit_rate": instrument_hits / instrument_n if instrument_n else None,
            "state_base": base,
        }
    replication = sum(
        1
        for value in instruments.values()
        if value["n"] >= 30
        and value["hit_rate"] is not None
        and value["state_base"] is not None
        and value["hit_rate"] > value["state_base"]
    )

    checks = [
        ("n", n >= MIN_EVENTS),
        ("clusters", stats["clusters"] >= MIN_CLUSTERS),
        (
            "conditional_lift",
            conditional_lift is not None and conditional_lift >= MIN_CONDITIONAL_LIFT,
        ),
        ("conditional_lcb", conditional_lcb is not None and conditional_lcb > 0.0),
        (
            "incremental_lift",
            incremental is not None and incremental >= MIN_INCREMENTAL_LIFT,
        ),
        ("incremental_lcb", incremental_lcb is not None and incremental_lcb > 0.0),
        ("random", random_p is not None and random_p <= CONTROL_ALPHA),
        (
            "back_lag",
            back_lift is not None
            and (
                back_lift <= 0.01
                or (
                    conditional_lift is not None
                    and conditional_lift - back_lift >= 0.01
                )
            ),
        ),
        ("shuffle", shuffle_p is not None and shuffle_p <= CONTROL_ALPHA),
        ("replication", replication >= 2),
    ]
    reasons = [name for name, ok in checks if not ok]
    grade = "C" if reasons else ("A" if conditional_lift >= 0.05 and replication >= 3 else "B")
    return {
        "family": family_id,
        "timeframe": "15m",
        "context_timeframe": "1h",
        "n": n,
        "clusters": stats["clusters"],
        "hit_rate": p,
        "base_rate": event_rate,
        "event_rate": event_rate,
        "state_base": state_base,
        "lift": conditional_lift,
        "lift_lcb": conditional_lcb,
        "conditional_lift": conditional_lift,
        "conditional_lift_lcb": conditional_lcb,
        "incremental_lift": incremental,
        "incremental_lift_lcb": incremental_lcb,
        "random_p": random_p,
        "back_lag_lift": back_lift,
        "shuffle_p": shuffle_p,
        "replication": replication,
        "instruments": instruments,
        "grade": grade,
        "reasons": reasons,
    }


def evaluate_config(
    cells: list[Cell],
    family: dict[str, Any],
    options: Options,
    primary_cells: int,
) -> dict[str, Any] | None:
    bundles = []
    for cell in cells:
        bundle = load_bundle(cell, family, options)
        if bundle is not None:
            bundles.append(bundle)
    if not bundles:
        return None
    return evaluate(bundles, str(family["id"]), options, primary_cells)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--matrix", required=True)
    parser.add_argument("--families", required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument("--config", default=None)
    parser.add_argument("--horizon", type=int, default=20)
    parser.add_argument("--target-atr", type=float, default=2.0)
    parser.add_argument("--stop-atr", type=float, default=1.0)
    parser.add_argument("--position-timeframe", default="15m")
    parser.add_argument("--context-timeframe", default="1h")
    parser.add_argument("--random-draws", type=int, default=RANDOM_DRAWS)
    parser.add_argument("--shuffle-draws", type=int, default=SHUFFLE_DRAWS)
    parser.add_argument("--back-lag", type=int, default=20)
    parser.add_argument("--seed", type=int, default=0)
    parser.add_argument("--primary-cells", type=int, default=3)
    parser.add_argument("--skip-sensitivity", action="store_true")
    args = parser.parse_args(argv)
    if args.position_timeframe != "15m":
        print("error: only 15m is supported as the position timeframe", file=sys.stderr)
        return 1
    if args.context_timeframe != "1h":
        print("error: only 1h is supported as the context timeframe", file=sys.stderr)
        return 1
    try:
        matrix = json.loads(Path(args.matrix).read_text(encoding="utf-8"))
        families = json.loads(Path(args.families).read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    if not isinstance(families, list) or len(families) != 1:
        print("error: families must be a one-element list", file=sys.stderr)
        return 1
    family = families[0]
    context = family.get("context")
    if not isinstance(context, dict):
        print("error: family.context is required", file=sys.stderr)
        return 1
    if context.get("timeframe") != "1h":
        print("error: family.context.timeframe must be 1h", file=sys.stderr)
        return 1
    if context.get("ema_stack") != "agree":
        print("error: family.context.ema_stack must be agree", file=sys.stderr)
        return 1
    if context.get("adx_regime") != 1:
        print("error: family.context.adx_regime must be 1", file=sys.stderr)
        return 1
    cells = path_validation.expand_cells(matrix, None)
    if not cells:
        print("error: matrix has no cells", file=sys.stderr)
        return 1
    configs = sorted({str(cell["config"]) for cell in cells})
    primary_config = str(matrix.get("primary_config", configs[0])) if isinstance(matrix, dict) else configs[0]
    sensitivity_config = str(matrix.get("sensitivity_config", configs[-1])) if isinstance(matrix, dict) else configs[-1]
    options = Options(
        horizon=args.horizon,
        target_atr=args.target_atr,
        stop_atr=args.stop_atr,
        random_draws=args.random_draws,
        shuffle_draws=args.shuffle_draws,
        back_lag=args.back_lag,
        seed=args.seed,
    )
    rows = {}
    for config in configs:
        selected = [
            Cell(
                instrument=str(cell["instrument"]),
                fold=int(cell["fold"]),
                partition=str(cell["partition"]),
                config=config,
                run=Path(str(cell["run"])).expanduser(),
            )
            for cell in cells
            if str(cell["config"]) == config
        ]
        if args.config is not None and config != args.config:
            continue
        metrics = evaluate_config(selected, family, options, args.primary_cells)
        if metrics is not None:
            rows[config] = metrics
    primary = rows.get(primary_config) or next(iter(rows.values()), None)
    sensitivity = rows.get(sensitivity_config) if sensitivity_config != primary_config else None
    primary_pass = bool(primary and not primary["reasons"])
    sensitivity_ok = True
    if sensitivity is not None and not args.skip_sensitivity:
        sensitivity_ok = (
            sensitivity["conditional_lift"] is not None
            and sensitivity["conditional_lift"] > 0.0
            and sensitivity["incremental_lift"] is not None
            and sensitivity["incremental_lift"] > 0.0
        )
    verdict = "pass" if primary_pass and sensitivity_ok else "fail"
    summary = {
        "schema": SCHEMA,
        "family": str(family["id"]),
        "label": family.get("label", str(family["id"])),
        "timeframe": "15m",
        "context_timeframe": "1h",
        "horizon": options.horizon,
        "target_atr": options.target_atr,
        "stop_atr": options.stop_atr,
        "metrics": primary or {},
        "sensitivity": sensitivity,
        "sensitivity_ok": sensitivity_ok,
        "verdict": verdict,
        "reasons": (primary["reasons"] if primary else ["no primary row"]) + ([] if sensitivity_ok else ["sensitivity"]),
        "limitations": [
            "Barrier path on the model pack; no execution costs.",
            "The 1h state is the last closed 1h bar at the 15m event bar.",
            "The 15m event is a timing trigger inside the 1h state, not a second vote.",
        ],
    }
    out = Path(args.out).expanduser()
    out.mkdir(parents=True, exist_ok=True)
    (out / "summary.json").write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    (out / "cross_timeframe_validation.json").write_text(
        json.dumps({"schema": SCHEMA, "family": family, "configs": rows, "primary": primary_config, "sensitivity": sensitivity_config}, indent=2) + "\n",
        encoding="utf-8",
    )
    print(f"ok cross-timeframe {summary['label']} {verdict} -> {out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
