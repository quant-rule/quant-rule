#!/usr/bin/env python3
"""Evaluate a model prediction against forward outcomes.

The evaluator joins a v2 `prediction.csv` at its bar with the forward path in
the same bar's `readings.csv`. It supports a forward ATR-normalized return and a
barrier target/stop, applies a cost hurdle, and compares the prediction with a
direction-matched unconditional base, a random-entry control, a direction
shuffle, and a back-lag control. It reports cluster-robust lift, the lower
confidence bound, per-instrument and per-year results, and a grade.

Statistics is evaluation only. Nothing here enters a model or a strategy.

Usage:
    python3 tools/improvement/model_prediction.py \
        --bundle XAU=data/research/M2/dev/CELL-M2.1 \
        --bundle BTC=data/research/M2/dev/CELL-M2.1-btc \
        --spec-id M2.1 --family M2 \
        --partition model_dev --horizon 20 \
        --target-atr 2.0 --stop-atr 1.0 --cost-bps 6 \
        --out data/research/M2/dev/CELL-M2.1-eval
"""

from __future__ import annotations

import argparse
import csv
import json
import math
import random
import sys
from collections import defaultdict
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from statistics import NormalDist
from typing import Any

from common import PARTITION_TIERS, write_json
from path_validation import cluster_stats, control_p, lower, percentile

SCHEMA = "quantrule.model-prediction/v1"
DEFAULT_HORIZON = 20
DEFAULT_TARGET_ATR = 2.0
DEFAULT_STOP_ATR = 1.0
DEFAULT_COST_BPS = 6.0
DEFAULT_RANDOM_DRAWS = 200
DEFAULT_SHUFFLE_DRAWS = 200
DEFAULT_MIN_EVENTS = 200
DEFAULT_MIN_CLUSTERS = 20
DEFAULT_MIN_LIFT = 0.02
DEFAULT_MIN_INSTRUMENT_EVENTS = 30
DEFAULT_MIN_INSTRUMENTS = 2
DEFAULT_ALPHA = 0.05
DEFAULT_PRIMARY_CELLS = 5


@dataclass(frozen=True)
class BundleInput:
    name: str
    predictions: Path
    readings: Path
    provenance: Path | None


@dataclass(frozen=True)
class Outcome:
    hit: bool
    gross_r: float
    net_r: float
    kind: str
    exit_index: int


@dataclass
class Entry:
    bundle: int
    name: str
    bar: int
    timestamp: int | None
    direction: int
    outcome: Outcome
    base_rate: float | None
    fold_base_rate: float | None
    hit_by_direction: dict[int, bool]


@dataclass
class BundleData:
    name: str
    timeframe_secs: int
    rows: list[dict[str, Any]]
    index_by_bar: dict[int, int]
    entries: list[Entry]
    base: dict[int, dict[str, Any]]
    raw_predictions: int
    purged: int


def parse_arguments(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--bundle",
        action="append",
        default=[],
        metavar="NAME=DIR",
        help="evaluation bundle; may be repeated",
    )
    parser.add_argument("--predictions", default=None, help="single prediction.csv")
    parser.add_argument("--readings", default=None, help="single readings.csv")
    parser.add_argument("--instrument", default="unknown", help="single-bundle instrument")
    parser.add_argument("--spec-id", default=None, help="model spec id for provenance")
    parser.add_argument("--family", default=None, help="model family for provenance")
    parser.add_argument("--partition", default="model_dev", help="declared partition")
    parser.add_argument("--timeframe", type=int, default=None, help="timeframe seconds")
    parser.add_argument("--split-start", default=None, help="first UTC date of the partition")
    parser.add_argument("--split-end", default=None, help="last UTC date of the partition")
    parser.add_argument("--horizon", type=int, default=DEFAULT_HORIZON)
    parser.add_argument(
        "--outcome", choices=("forward", "barrier"), default="barrier"
    )
    parser.add_argument("--target-atr", type=float, default=DEFAULT_TARGET_ATR)
    parser.add_argument("--stop-atr", type=float, default=DEFAULT_STOP_ATR)
    parser.add_argument("--cost-bps", type=float, default=DEFAULT_COST_BPS)
    parser.add_argument("--random-draws", type=int, default=DEFAULT_RANDOM_DRAWS)
    parser.add_argument("--shuffle-draws", type=int, default=DEFAULT_SHUFFLE_DRAWS)
    parser.add_argument("--back-lag", type=int, default=None)
    parser.add_argument("--seed", type=int, default=0)
    parser.add_argument("--primary-cells", type=int, default=DEFAULT_PRIMARY_CELLS)
    parser.add_argument("--alpha", type=float, default=DEFAULT_ALPHA)
    parser.add_argument("--min-events", type=int, default=DEFAULT_MIN_EVENTS)
    parser.add_argument("--min-clusters", type=int, default=DEFAULT_MIN_CLUSTERS)
    parser.add_argument("--min-lift", type=float, default=DEFAULT_MIN_LIFT)
    parser.add_argument(
        "--min-instrument-events", type=int, default=DEFAULT_MIN_INSTRUMENT_EVENTS
    )
    parser.add_argument("--min-instruments", type=int, default=DEFAULT_MIN_INSTRUMENTS)
    parser.add_argument("--out", required=True)
    return parser.parse_args(argv)


def parse_bundles(args: argparse.Namespace) -> list[BundleInput]:
    bundles: list[BundleInput] = []
    for value in args.bundle:
        name, separator, directory = value.partition("=")
        if not separator or not name.strip() or not directory.strip():
            raise SystemExit("error: --bundle must be NAME=DIR")
        root = Path(directory).expanduser()
        prediction = root / "prediction.csv"
        readings = root / "readings.csv"
        for path in (prediction, readings):
            if not path.is_file():
                raise SystemExit(f"error: bundle file not found: {path}")
        provenance = root / "provenance.json"
        bundles.append(
            BundleInput(
                name=name.strip(),
                predictions=prediction,
                readings=readings,
                provenance=provenance if provenance.is_file() else None,
            )
        )
    if bundles:
        if args.predictions or args.readings:
            raise SystemExit("error: use --bundle or --predictions/--readings, not both")
        return bundles
    if not args.predictions or not args.readings:
        raise SystemExit("error: provide --bundle or both --predictions and --readings")
    return [
        BundleInput(
            name=args.instrument,
            predictions=Path(args.predictions).expanduser(),
            readings=Path(args.readings).expanduser(),
            provenance=None,
        )
    ]


def validate_arguments(args: argparse.Namespace) -> None:
    if args.partition not in PARTITION_TIERS:
        raise SystemExit(f"error: unknown partition {args.partition}")
    if args.horizon <= 0:
        raise SystemExit("error: --horizon must be positive")
    if args.outcome == "barrier":
        if args.target_atr <= 0.0 or args.stop_atr <= 0.0:
            raise SystemExit("error: target and stop ATR must be positive")
    if args.cost_bps < 0.0:
        raise SystemExit("error: --cost-bps must be non-negative")
    if args.random_draws < 0 or args.shuffle_draws < 0:
        raise SystemExit("error: draws must be non-negative")
    if args.back_lag is not None and args.back_lag <= 0:
        raise SystemExit("error: --back-lag must be positive")
    if args.primary_cells < 1:
        raise SystemExit("error: --primary-cells must be at least one")
    if not 0.0 < args.alpha < 1.0:
        raise SystemExit("error: --alpha must be in (0, 1)")
    if args.min_events < 0 or args.min_clusters < 0:
        raise SystemExit("error: minimum counts must be non-negative")
    if not 0.0 <= args.min_lift <= 1.0:
        raise SystemExit("error: --min-lift must be in [0, 1]")
    if args.min_instrument_events < 0 or args.min_instruments < 1:
        raise SystemExit("error: instrument thresholds are invalid")
    parse_split_bound(args.split_start, end=False)
    parse_split_bound(args.split_end, end=True)


def parse_split_bound(value: str | None, end: bool) -> int | None:
    if value is None:
        return None
    try:
        day = datetime.strptime(value, "%Y-%m-%d").replace(tzinfo=timezone.utc)
    except ValueError:
        raise SystemExit(f"error: split date {value} is not YYYY-MM-DD") from None
    if end:
        day = day.replace(hour=23, minute=59, second=59)
    return int(day.timestamp())


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


def read_predictions(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    with path.open(newline="", encoding="utf-8") as handle:
        reader = csv.DictReader(handle)
        for row in reader:
            try:
                timeframe = int(row["timeframe_secs"])
                bar = int(row["bar"])
                prediction = int(row["prediction"])
            except (KeyError, TypeError, ValueError):
                continue
            if prediction not in (-1, 0, 1):
                raise ValueError(f"{path}: prediction {prediction} is not -1, 0, or 1")
            rows.append(
                {
                    "timeframe_secs": timeframe,
                    "bar": bar,
                    "timestamp": optional_int(row.get("timestamp")),
                    "prediction": prediction,
                    "long_votes": optional_int(row.get("long_votes")),
                    "short_votes": optional_int(row.get("short_votes")),
                }
            )
    rows.sort(key=lambda value: value["bar"])
    for previous, current in zip(rows, rows[1:]):
        if previous["bar"] == current["bar"]:
            raise ValueError(f"{path}: duplicate prediction bar {current['bar']}")
    return rows


def read_readings(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    with path.open(newline="", encoding="utf-8") as handle:
        reader = csv.DictReader(handle)
        for row in reader:
            try:
                timeframe = int(row["timeframe_secs"])
                bar = int(row["bar"])
                close = float(row["close"])
                high = float(row["high"])
                low = float(row["low"])
            except (KeyError, TypeError, ValueError):
                continue
            rows.append(
                {
                    "timeframe_secs": timeframe,
                    "bar": bar,
                    "timestamp": optional_int(row.get("timestamp")),
                    "close": close,
                    "high": high,
                    "low": low,
                    "atr": optional_float(row.get("atr")),
                }
            )
    rows.sort(key=lambda value: value["bar"])
    return rows


def read_provenance(path: Path | None) -> dict[str, Any]:
    if path is None:
        return {}
    try:
        document = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return {}
    spec = document.get("spec")
    return spec if isinstance(spec, dict) else {}


def year_of(timestamp: int | None) -> int | None:
    if timestamp is None:
        return None
    try:
        return datetime.fromtimestamp(timestamp, timezone.utc).year
    except (OverflowError, OSError, ValueError):
        return None


def cost_r(entry: float, atr: float, cost_bps: float) -> float:
    return cost_bps / 10_000.0 * entry / atr


def forward_outcome(
    rows: list[dict[str, Any]],
    index: int,
    direction: int,
    horizon: int,
    cost_bps: float,
) -> Outcome | None:
    if index < 0 or index + horizon >= len(rows):
        return None
    entry_row = rows[index]
    atr = entry_row["atr"]
    if atr is None or not math.isfinite(atr) or atr <= 0.0:
        return None
    entry = entry_row["close"]
    exit_price = rows[index + horizon]["close"]
    gross = direction * (exit_price - entry) / atr
    cost = cost_r(entry, atr, cost_bps)
    net = gross - cost
    return Outcome(net > 0.0, gross, net, "forward", index + horizon)


def barrier_outcome(
    rows: list[dict[str, Any]],
    index: int,
    direction: int,
    horizon: int,
    target_atr: float,
    stop_atr: float,
    cost_bps: float,
) -> Outcome | None:
    if index < 0 or index + horizon >= len(rows):
        return None
    entry_row = rows[index]
    atr = entry_row["atr"]
    if atr is None or not math.isfinite(atr) or atr <= 0.0:
        return None
    entry = entry_row["close"]
    target = entry + direction * target_atr * atr
    stop = entry - direction * stop_atr * atr
    cost = cost_r(entry, atr, cost_bps)
    for offset in range(1, horizon + 1):
        row = rows[index + offset]
        # Stop-first is the conservative tie rule when both are touched.
        if direction > 0:
            if row["low"] <= stop:
                return Outcome(
                    -stop_atr - cost > 0.0,
                    -stop_atr,
                    -stop_atr - cost,
                    "stop",
                    index + offset,
                )
            if row["high"] >= target:
                return Outcome(
                    target_atr - cost > 0.0,
                    target_atr,
                    target_atr - cost,
                    "target",
                    index + offset,
                )
        else:
            if row["high"] >= stop:
                return Outcome(
                    -stop_atr - cost > 0.0,
                    -stop_atr,
                    -stop_atr - cost,
                    "stop",
                    index + offset,
                )
            if row["low"] <= target:
                return Outcome(
                    target_atr - cost > 0.0,
                    target_atr,
                    target_atr - cost,
                    "target",
                    index + offset,
                )
    exit_price = rows[index + horizon]["close"]
    gross = direction * (exit_price - entry) / atr
    net = gross - cost
    return Outcome(net > 0.0, gross, net, "timeout", index + horizon)


def outcome_for(
    rows: list[dict[str, Any]],
    index: int,
    direction: int,
    args: argparse.Namespace,
) -> Outcome | None:
    if args.outcome == "forward":
        return forward_outcome(rows, index, direction, args.horizon, args.cost_bps)
    return barrier_outcome(
        rows,
        index,
        direction,
        args.horizon,
        args.target_atr,
        args.stop_atr,
        args.cost_bps,
    )


def base_statistics(
    rows: list[dict[str, Any]],
    direction: int,
    args: argparse.Namespace,
    split_start: int | None,
    split_end: int | None,
) -> dict[str, Any]:
    hits = 0
    valid = 0
    valid_indices: list[int] = []
    by_year: dict[int, list[int]] = defaultdict(lambda: [0, 0])
    for index, row in enumerate(rows):
        outcome = outcome_for(rows, index, direction, args)
        if outcome is None:
            continue
        timestamp = row["timestamp"]
        if timestamp is not None:
            if split_start is not None and timestamp < split_start:
                continue
            if split_end is not None and timestamp > split_end:
                continue
        exit_timestamp = rows[outcome.exit_index]["timestamp"]
        if exit_timestamp is not None and split_end is not None and exit_timestamp > split_end:
            continue
        valid += 1
        valid_indices.append(index)
        hits += int(outcome.hit)
        year = year_of(row["timestamp"])
        if year is not None:
            by_year[year][0] += int(outcome.hit)
            by_year[year][1] += 1
    return {
        "hits": hits,
        "valid": valid,
        "rate": hits / valid if valid else None,
        "valid_indices": valid_indices,
        "by_year": {
            year: {
                "hits": value[0],
                "valid": value[1],
                "rate": value[0] / value[1] if value[1] else None,
            }
            for year, value in sorted(by_year.items())
        },
    }


def evaluate_bundle(
    bundle: BundleInput, bundle_index: int, args: argparse.Namespace
) -> BundleData:
    predictions = read_predictions(bundle.predictions)
    readings = read_readings(bundle.readings)
    if not predictions:
        raise ValueError(f"{bundle.predictions}: no prediction rows")
    timeframes = {row["timeframe_secs"] for row in predictions}
    if args.timeframe is not None:
        timeframes = {args.timeframe}
    if len(timeframes) != 1:
        raise ValueError(f"{bundle.predictions}: predictions span multiple timeframes")
    timeframe = next(iter(timeframes))
    predictions = [
        row for row in predictions if row["timeframe_secs"] == timeframe
    ]
    if not predictions:
        raise ValueError(f"{bundle.predictions}: no rows for {timeframe}s")
    rows = [row for row in readings if row["timeframe_secs"] == timeframe]
    if not rows:
        raise ValueError(f"{bundle.readings}: no readings for {timeframe}s")
    index_by_bar = {row["bar"]: index for index, row in enumerate(rows)}
    split_start = parse_split_bound(args.split_start, end=False)
    split_end = parse_split_bound(args.split_end, end=True)
    base = {
        direction: base_statistics(rows, direction, args, split_start, split_end)
        for direction in (-1, 1)
    }
    entries: list[Entry] = []
    raw = 0
    purged = 0
    for prediction in predictions:
        direction = prediction["prediction"]
        if direction == 0:
            continue
        raw += 1
        index = index_by_bar.get(prediction["bar"])
        if index is None:
            continue
        outcome = outcome_for(rows, index, direction, args)
        if outcome is None:
            continue
        hit_by_direction = {}
        for side in (-1, 1):
            side_outcome = outcome_for(rows, index, side, args)
            if side_outcome is None:
                hit_by_direction = {}
                break
            hit_by_direction[side] = side_outcome.hit
        if not hit_by_direction:
            continue
        timestamp = prediction["timestamp"] or rows[index]["timestamp"]
        if timestamp is not None:
            if split_start is not None and timestamp < split_start:
                purged += 1
                continue
            if split_end is not None and timestamp > split_end:
                purged += 1
                continue
        exit_timestamp = rows[outcome.exit_index]["timestamp"]
        if exit_timestamp is not None and split_end is not None and exit_timestamp > split_end:
            purged += 1
            continue
        year = year_of(timestamp)
        fold_base = None
        if year is not None:
            fold_base = base[direction]["by_year"].get(year, {}).get("rate")
        entries.append(
            Entry(
                bundle=bundle_index,
                name=bundle.name,
                bar=prediction["bar"],
                timestamp=timestamp,
                direction=direction,
                outcome=outcome,
                base_rate=base[direction]["rate"],
                fold_base_rate=fold_base,
                hit_by_direction=hit_by_direction,
            )
        )
    return BundleData(
        name=bundle.name,
        timeframe_secs=timeframe,
        rows=rows,
        index_by_bar=index_by_bar,
        entries=entries,
        base=base,
        raw_predictions=raw,
        purged=purged,
    )


def weighted_base(entries: list[Entry]) -> float | None:
    values = [entry.base_rate for entry in entries if entry.base_rate is not None]
    return sum(values) / len(values) if values else None


def evaluate_metrics(
    bundles: list[BundleData],
    entries: list[Entry],
    args: argparse.Namespace,
    z: float,
) -> dict[str, Any]:
    stats = cluster_stats(
        [(entry.bundle, entry.bar, entry.outcome.hit) for entry in entries],
        args.horizon,
    )
    n = stats["n"]
    hits = sum(1 for entry in entries if entry.outcome.hit)
    base = weighted_base(entries)
    hit_rate = stats["hit_rate"]
    lift = (
        hit_rate - base
        if hit_rate is not None and base is not None
        else None
    )
    lcb_method = "cluster"
    if stats["se"] is not None:
        lift_lcb = lower(lift, stats["se"], z)
    elif n > 0 and base is not None:
        # Every prediction is its own cluster, so the cluster-robust variance
        # is undefined. The Wilson interval is a conservative fallback.
        wilson = wilson_lower(hits, n, z)
        lift_lcb = wilson - base if wilson is not None else None
        lcb_method = "wilson"
    else:
        lift_lcb = None
    back_lag = args.back_lag if args.back_lag is not None else args.horizon
    back_hits = 0
    back_total = 0
    for entry in entries:
        bundle = bundles[entry.bundle]
        index = bundle.index_by_bar.get(entry.bar - back_lag)
        if index is None:
            continue
        outcome = outcome_for(bundle.rows, index, entry.direction, args)
        if outcome is None:
            continue
        back_total += 1
        back_hits += int(outcome.hit)
    back_rate = back_hits / back_total if back_total else None
    back_lift = (
        back_rate - base
        if back_rate is not None and base is not None
        else None
    )
    random_draws = random_control(bundles, args)
    shuffle_draws = shuffle_control(entries, args)
    return {
        "n": n,
        "predictions": sum(bundle.raw_predictions for bundle in bundles),
        "coverage": n / sum(bundle.raw_predictions for bundle in bundles)
        if sum(bundle.raw_predictions for bundle in bundles)
        else 0.0,
        "purged": sum(bundle.purged for bundle in bundles),
        "clusters": stats["clusters"],
        "hits": hits,
        "hit_rate": hit_rate,
        "base_rate": base,
        "lift": lift,
        "lift_lcb": lift_lcb,
        "lcb_method": lcb_method,
        "cluster_se": stats["se"],
        "random_mean": mean_optional(random_draws),
        "random_p95": percentile(random_draws, 0.95),
        "random_p": control_p(hit_rate, random_draws) if hit_rate is not None else None,
        "shuffle_mean": mean_optional(shuffle_draws),
        "shuffle_p95": percentile(shuffle_draws, 0.95),
        "shuffle_p": control_p(hit_rate, shuffle_draws) if hit_rate is not None else None,
        "back_lag_rate": back_rate,
        "back_lag_n": back_total,
        "back_lag_lift": back_lift,
        "replication": positive_instruments(bundles, args),
        "instruments": instrument_metrics(bundles, args),
        "folds": fold_metrics(entries, args),
    }


def random_control(bundles: list[BundleData], args: argparse.Namespace) -> list[float | None]:
    if args.random_draws == 0:
        return []
    rng = random.Random(args.seed)
    draws: list[float | None] = []
    for _ in range(args.random_draws):
        hits = 0
        total = 0
        for bundle in bundles:
            counts = {direction: 0 for direction in (-1, 1)}
            for entry in bundle.entries:
                counts[entry.direction] += 1
            for direction, count in counts.items():
                pool = bundle.base[direction]["valid_indices"]
                if count == 0 or not pool:
                    continue
                sample = rng.sample(pool, min(count, len(pool)))
                for index in sample:
                    outcome = outcome_for(bundle.rows, index, direction, args)
                    if outcome is not None:
                        hits += int(outcome.hit)
                        total += 1
        draws.append(hits / total if total else None)
    return draws


def shuffle_control(entries: list[Entry], args: argparse.Namespace) -> list[float | None]:
    if args.shuffle_draws == 0:
        return []
    rng = random.Random(args.seed + 1)
    by_bundle: dict[int, list[Entry]] = defaultdict(list)
    for entry in entries:
        by_bundle[entry.bundle].append(entry)
    draws: list[float | None] = []
    for _ in range(args.shuffle_draws):
        hits = 0
        total = 0
        for values in by_bundle.values():
            directions = [entry.direction for entry in values]
            rng.shuffle(directions)
            for entry, direction in zip(values, directions):
                hit = entry.hit_by_direction.get(direction)
                if hit is None:
                    continue
                hits += int(hit)
                total += 1
        draws.append(hits / total if total else None)
    return draws


def wilson_lower(hits: int, n: int, z: float) -> float | None:
    if n <= 0:
        return None
    p = hits / n
    denominator = 1.0 + z * z / n
    center = p + z * z / (2.0 * n)
    margin = z * math.sqrt(p * (1.0 - p) / n + z * z / (4.0 * n * n))
    return max(0.0, (center - margin) / denominator)


def mean_optional(values: list[float | None]) -> float | None:
    valid = [value for value in values if value is not None]
    return sum(valid) / len(valid) if valid else None


def positive_instruments(bundles: list[BundleData], args: argparse.Namespace) -> int:
    return sum(
        1
        for bundle in bundles
        if instrument_metric(bundle, args)["positive"]
    )


def instrument_metric(bundle: BundleData, args: argparse.Namespace) -> dict[str, Any]:
    stats = cluster_stats(
        [(0, entry.bar, entry.outcome.hit) for entry in bundle.entries],
        args.horizon,
    )
    base = weighted_base(bundle.entries)
    hit_rate = stats["hit_rate"]
    lift = hit_rate - base if hit_rate is not None and base is not None else None
    return {
        "n": stats["n"],
        "clusters": stats["clusters"],
        "hit_rate": hit_rate,
        "base_rate": base,
        "lift": lift,
        "positive": (
            stats["n"] >= args.min_instrument_events
            and lift is not None
            and lift > 0.0
        ),
    }


def instrument_metrics(
    bundles: list[BundleData], args: argparse.Namespace
) -> dict[str, dict[str, Any]]:
    return {bundle.name: instrument_metric(bundle, args) for bundle in bundles}


def fold_metrics(entries: list[Entry], args: argparse.Namespace) -> dict[str, dict[str, Any]]:
    grouped: dict[int, list[Entry]] = defaultdict(list)
    for entry in entries:
        year = year_of(entry.timestamp)
        if year is not None:
            grouped[year].append(entry)
    result = {}
    for year, values in sorted(grouped.items()):
        stats = cluster_stats(
            [(entry.bundle, entry.bar, entry.outcome.hit) for entry in values],
            args.horizon,
        )
        base_values = [
            entry.fold_base_rate
            for entry in values
            if entry.fold_base_rate is not None
        ]
        base = sum(base_values) / len(base_values) if base_values else None
        hit_rate = stats["hit_rate"]
        result[str(year)] = {
            "n": stats["n"],
            "clusters": stats["clusters"],
            "hit_rate": hit_rate,
            "base_rate": base,
            "lift": hit_rate - base if hit_rate is not None and base is not None else None,
        }
    return result


def grade(
    metrics: dict[str, Any], args: argparse.Namespace, corrected_alpha: float
) -> tuple[str, bool, dict[str, bool]]:
    n = metrics["n"]
    clusters = metrics["clusters"]
    lift = metrics["lift"]
    lift_lcb = metrics["lift_lcb"]
    random_p = metrics["random_p"]
    shuffle_p = metrics["shuffle_p"]
    back_lift = metrics["back_lag_lift"]
    sample_ok = n >= args.min_events and clusters >= args.min_clusters
    lift_ok = lift is not None and lift >= args.min_lift and lift_lcb is not None and lift_lcb > 0.0
    controls_ok = (
        random_p is not None
        and random_p <= corrected_alpha
        and shuffle_p is not None
        and shuffle_p <= corrected_alpha
    )
    back_ok = (
        back_lift is not None
        and (
            back_lift <= 0.01
            or (lift is not None and lift - back_lift >= 0.01)
        )
    )
    replication_ok = metrics["replication"] >= args.min_instruments
    criteria = {
        "sample": sample_ok,
        "lift": lift_ok,
        "controls": controls_ok,
        "back_lag": back_ok,
        "replication": replication_ok,
    }
    if not sample_ok or lift is None or lift <= 0.0:
        return "D", False, criteria
    if not (lift_ok and controls_ok and back_ok):
        return "C", False, criteria
    if not replication_ok:
        return "B", True, criteria
    return "A", True, criteria


def spec_metadata(bundle: BundleInput) -> dict[str, Any]:
    return read_provenance(bundle.provenance)


def evaluate(args: argparse.Namespace) -> dict[str, Any]:
    validate_arguments(args)
    inputs = parse_bundles(args)
    z = NormalDist().inv_cdf(1.0 - args.alpha / (2.0 * args.primary_cells))
    corrected_alpha = args.alpha / args.primary_cells
    bundles: list[BundleData] = []
    for index, bundle in enumerate(inputs):
        bundles.append(evaluate_bundle(bundle, index, args))
    timeframes = {bundle.timeframe_secs for bundle in bundles}
    if len(timeframes) != 1:
        raise SystemExit("error: bundles use different timeframes")
    timeframe = next(iter(timeframes))
    entries = [entry for bundle in bundles for entry in bundle.entries]
    metrics = evaluate_metrics(bundles, entries, args, z)
    letter, passed, criteria = grade(metrics, args, corrected_alpha)
    spec = spec_metadata(inputs[0])
    spec_id = args.spec_id or spec.get("id")
    family = args.family or spec.get("family")
    instrument = args.instrument if len(inputs) == 1 else "pooled"
    if len(inputs) == 1:
        instrument = inputs[0].name
    limitations = [
        "Prediction evaluation only; no execution, position sizing, or P&L.",
        "Base rate is direction-matched and unconditional within the evaluated partition.",
        "Overlapping outcomes are clustered by entry bar with a horizon gap.",
        "The forward outcome is joined after the model run; it never enters the model.",
    ]
    if len(inputs) == 1:
        limitations.append("A single instrument cannot satisfy the replication criterion.")
    if args.random_draws == 0 or args.shuffle_draws == 0:
        limitations.append("One or both controls were disabled by zero draws.")
    return {
        "schema": SCHEMA,
        "spec_id": spec_id,
        "family": family,
        "instrument": instrument,
        "timeframe_secs": timeframe,
        "partition": args.partition,
        "split_start": args.split_start,
        "split_end": args.split_end,
        "horizon": args.horizon,
        "outcome": args.outcome,
        "target_atr": args.target_atr if args.outcome == "barrier" else None,
        "stop_atr": args.stop_atr if args.outcome == "barrier" else None,
        "cost_bps": args.cost_bps,
        "primary_cells": args.primary_cells,
        "alpha": args.alpha,
        "corrected_alpha": corrected_alpha,
        "z": z,
        "seed": args.seed,
        "random_draws": args.random_draws,
        "shuffle_draws": args.shuffle_draws,
        "back_lag": args.back_lag if args.back_lag is not None else args.horizon,
        "criteria": {
            "min_events": args.min_events,
            "min_clusters": args.min_clusters,
            "min_lift": args.min_lift,
            "min_instrument_events": args.min_instrument_events,
            "min_instruments": args.min_instruments,
            "control_alpha": corrected_alpha,
        },
        "criteria_status": criteria,
        "metrics": metrics,
        "grade": letter,
        "verdict": "pass" if passed else "fail",
        "limitations": limitations,
        "bundles": [
            {
                "name": bundle.name,
                "predictions": str(bundle.predictions),
                "readings": str(bundle.readings),
            }
            for bundle in inputs
        ],
    }


def markdown(document: dict[str, Any]) -> str:
    metrics = document["metrics"]
    lines = [
        "# Model Prediction Evaluation",
        "",
        f"- Schema: `{document['schema']}`",
        f"- Spec: `{document['spec_id'] or '-'}` family `{document['family'] or '-'}`",
        f"- Instrument: `{document['instrument']}`; partition `{document['partition']}`",
        f"- Split: `{document['split_start'] or '-'}` to `{document['split_end'] or '-'}`",
        (
            f"- Outcome: `{document['outcome']}`; horizon {document['horizon']} bars; "
            f"cost {document['cost_bps']} bps"
        ),
        f"- Grade: **{document['grade']}**; verdict **{document['verdict']}**",
        "",
        "| Metric | Value |",
        "|---|---:|",
    ]
    for key in (
        "n",
        "predictions",
        "coverage",
        "purged",
        "clusters",
        "hits",
        "hit_rate",
        "base_rate",
        "lift",
        "lift_lcb",
        "lcb_method",
        "random_p",
        "shuffle_p",
        "back_lag_lift",
        "replication",
    ):
        lines.append(f"| {key} | {format_value(metrics.get(key))} |")
    lines.extend(
        [
            "",
            "## Instruments",
            "",
            "| Instrument | n | Clusters | Hit | Base | Lift |",
            "|---|---:|---:|---:|---:|---:|",
        ]
    )
    for name, value in sorted(metrics["instruments"].items()):
        lines.append(
            f"| {name} | {value['n']} | {value['clusters']} | "
            f"{format_value(value['hit_rate'])} | {format_value(value['base_rate'])} | "
            f"{format_value(value['lift'])} |"
        )
    lines.extend(
        [
            "",
            "## Folds",
            "",
            "| Year | n | Clusters | Hit | Base | Lift |",
            "|---|---:|---:|---:|---:|---:|",
        ]
    )
    for year, value in sorted(metrics["folds"].items()):
        lines.append(
            f"| {year} | {value['n']} | {value['clusters']} | "
            f"{format_value(value['hit_rate'])} | {format_value(value['base_rate'])} | "
            f"{format_value(value['lift'])} |"
        )
    lines.extend(["", "## Limitations", ""])
    lines.extend(f"- {item}" for item in document["limitations"])
    lines.append("")
    return "\n".join(lines)


def format_value(value: Any) -> str:
    if value is None:
        return "-"
    if isinstance(value, float):
        return f"{value:.6f}"
    return str(value)


def main(argv: list[str] | None = None) -> int:
    args = parse_arguments(argv)
    try:
        document = evaluate(args)
    except (ValueError, OSError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    out = Path(args.out).expanduser()
    out.mkdir(parents=True, exist_ok=True)
    write_json(out / "model_prediction.json", document)
    (out / "model_prediction.md").write_text(markdown(document), encoding="utf-8")
    print(
        f"ok {document['spec_id'] or 'model'} {document['instrument']} "
        f"grade={document['grade']} verdict={document['verdict']} -> {out}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
