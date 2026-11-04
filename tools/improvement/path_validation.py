#!/usr/bin/env python3
"""Model v4 path validation against matched baselines and controls.

The harness reads a model bundle's per-bar readings and events, joins each event
at its availability bar, and measures whether price reaches an ATR target before
an ATR stop in the event direction within a fixed bar horizon. It compares the
event hit rate with:

- an unconditional, direction-matched barrier base;
- a state-conditioned barrier base for the state the event claims to mark;
- random-entry, back-lagged, and direction-shuffled controls.

Statistics is evaluation only. Nothing here enters the model or a strategy.

The protocol is `docs/model/2026-11-04/path-validation-v4.md`.

Usage:
    python3 tools/improvement/path_validation.py \
        --matrix config/model-v4/dev/matrix.json \
        --families config/model-v4/families.json \
        --out data/model-v4/evidence/dev
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import math
import random
import sys
from collections import defaultdict
from dataclasses import dataclass, field
from pathlib import Path
from statistics import NormalDist
from typing import Any

TIMEFRAME_CODES = {60: "1m", 300: "5m", 900: "15m", 3600: "1h"}
TIMEFRAME_ORDER = {300: 0, 900: 1, 3600: 2}
EMA_COLUMNS = {1: "ema_fast", 2: "ema_mid", 3: "ema_slow"}
PRIMARY_CELLS = 12
HORIZON = 20
TARGET_ATR = 1.0
STOP_ATR = 1.0
MIN_EVENTS = 200
MIN_CLUSTERS = 20
MIN_LIFT = 0.01
MIN_INCREMENTAL_LIFT = 0.01
RANDOM_DRAWS = 200
SHUFFLE_DRAWS = 200
BACK_LAG = HORIZON
CONTROL_ALPHA = 0.05
SEED = 0


@dataclass
class Options:
    horizon: int = HORIZON
    target_atr: float = TARGET_ATR
    stop_atr: float = STOP_ATR
    random_draws: int = RANDOM_DRAWS
    shuffle_draws: int = SHUFFLE_DRAWS
    back_lag: int = BACK_LAG
    seed: int = SEED


def parse_arguments(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--matrix", required=True, help="matrix JSON or cells JSON list")
    parser.add_argument("--families", required=True, help="event family JSON list")
    parser.add_argument("--out", required=True, help="output directory")
    parser.add_argument("--config", default=None, help="only this config")
    parser.add_argument("--timeframes", default="5m,15m,1h", help="comma-separated codes")
    parser.add_argument("--horizon", type=int, default=HORIZON)
    parser.add_argument("--target-atr", type=float, default=TARGET_ATR)
    parser.add_argument("--stop-atr", type=float, default=STOP_ATR)
    parser.add_argument("--random-draws", type=int, default=RANDOM_DRAWS)
    parser.add_argument("--shuffle-draws", type=int, default=SHUFFLE_DRAWS)
    parser.add_argument("--back-lag", type=int, default=BACK_LAG)
    parser.add_argument("--seed", type=int, default=SEED)
    parser.add_argument(
        "--primary-cells",
        type=int,
        default=PRIMARY_CELLS,
        help="number of primary cells in the Bonferroni correction",
    )
    parser.add_argument(
        "--skip-sensitivity",
        action="store_true",
        help="do not apply the ADX sensitivity condition (one-shot validation)",
    )
    return parser.parse_args(argv)


def read_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise SystemExit(f"error: cannot read {path}: {error}") from None


def number(value: str) -> float | None:
    if not value:
        return None
    try:
        return float(value)
    except ValueError:
        return None


def read_rows(path: Path) -> dict[int, list[dict[str, Any]]]:
    series: dict[int, list[dict[str, Any]]] = defaultdict(list)
    with path.open(newline="", encoding="utf-8") as handle:
        reader = csv.DictReader(handle)
        for row in reader:
            try:
                timeframe = int(row["timeframe_secs"])
            except (KeyError, TypeError, ValueError):
                continue
            if timeframe not in TIMEFRAME_CODES:
                continue
            try:
                bar = int(row["bar"])
                high = float(row["high"])
                low = float(row["low"])
                close = float(row["close"])
            except (KeyError, TypeError, ValueError):
                continue
            series[timeframe].append(
                {
                    "bar": bar,
                    "high": high,
                    "low": low,
                    "close": close,
                    "atr": number(row.get("atr", "")),
                    "rvol": number(row.get("rvol", "")),
                    "atr_ratio": number(row.get("atr_ratio", "")),
                    "rvol_ratio": number(row.get("rvol_ratio", "")),
                    "context_code": number(row.get("context_code", "")),
                    "rsi": number(row.get("rsi", "")),
                    "adx_regime": number(row.get("adx_regime", "")),
                    "adx_slope": number(row.get("adx_slope", "")),
                    "di_state": number(row.get("di_state", "")),
                    "ema_stack": number(row.get("ema_stack", "")),
                    "macd_state": number(row.get("macd_state", "")),
                    "ema_fast": number(row.get("ema_fast", "")),
                    "ema_mid": number(row.get("ema_mid", "")),
                    "ema_slow": number(row.get("ema_slow", "")),
                }
            )
    for rows in series.values():
        rows.sort(key=lambda row: row["bar"])
    return series


def read_events(path: Path) -> dict[int, list[dict[str, Any]]]:
    events: dict[int, list[dict[str, Any]]] = defaultdict(list)
    with path.open(newline="", encoding="utf-8") as handle:
        reader = csv.DictReader(handle)
        for row in reader:
            try:
                timeframe = int(row["timeframe_secs"])
            except (KeyError, TypeError, ValueError):
                continue
            if timeframe not in TIMEFRAME_CODES:
                continue
            try:
                event = {
                    "timeframe": timeframe,
                    "index": int(row["event_index"]),
                    "type": int(row["event_type_code"]),
                    "subtype": int(row["event_subtype_code"]),
                    "source": int(row["source_code"]),
                    "available": int(row["available_bar_index"]),
                    "occurred": int(row["occurred_bar_index"]),
                    "direction": int(row["direction_code"]),
                    "reference": number(row.get("reference_value", "")),
                    "primary": number(row.get("primary_value", "")),
                }
            except (KeyError, TypeError, ValueError):
                continue
            events[timeframe].append(event)
    for values in events.values():
        values.sort(key=lambda event: (event["available"], event["index"]))
    return events


def path_hit(
    rows: list[dict[str, Any]],
    index: int,
    direction: int,
    horizon: int,
    target_atr: float,
    stop_atr: float,
) -> bool | None:
    if index < 0 or index >= len(rows):
        return None
    entry = rows[index]
    atr = entry["atr"]
    if atr is None or atr <= 0.0:
        return None
    if index + horizon >= len(rows):
        return None
    target = entry["close"] + direction * target_atr * atr
    stop = entry["close"] - direction * stop_atr * atr
    for offset in range(1, horizon + 1):
        row = rows[index + offset]
        if direction > 0:
            if row["low"] <= stop:
                return False
            if row["high"] >= target:
                return True
        else:
            if row["high"] >= stop:
                return False
            if row["low"] <= target:
                return True
    return False


def barrier_arrays(
    rows: list[dict[str, Any]], horizon: int, target_atr: float, stop_atr: float
) -> dict[int, list[bool | None]]:
    return {
        direction: [
            path_hit(rows, index, direction, horizon, target_atr, stop_atr)
            for index in range(len(rows))
        ]
        for direction in (-1, 1)
    }


def counts(hits: list[bool | None]) -> tuple[int, int]:
    valid = [hit for hit in hits if hit is not None]
    return sum(1 for hit in valid if hit), len(valid)


def family_matches(family: dict[str, Any], event: dict[str, Any], row: dict[str, Any]) -> bool:
    if event["type"] != family["event_type"]:
        return False
    if event["subtype"] != family["subtype"]:
        return False
    source = family.get("source")
    if source is not None and event["source"] != source:
        return False
    sources = family.get("sources")
    if sources is not None and event["source"] not in sources:
        return False
    reference = family.get("reference_value")
    if reference is not None and (
        event["reference"] is None or abs(event["reference"] - reference) > 1e-9
    ):
        return False
    primary_min = family.get("primary_min")
    if primary_min is not None and (
        event["primary"] is None or event["primary"] < primary_min
    ):
        return False
    atr_ratio_max = family.get("atr_ratio_max")
    if atr_ratio_max is not None:
        ratio = row.get("atr_ratio")
        if ratio is None or ratio > atr_ratio_max:
            return False
    return True


def select_events(
    events: list[dict[str, Any]],
    rows: list[dict[str, Any]],
    family: dict[str, Any],
) -> tuple[list[dict[str, Any]], int]:
    raw = 0
    by_bar: dict[int, dict[str, Any] | None] = {}
    for event in events:
        if event["direction"] not in (-1, 1):
            continue
        index = event["available"]
        if index < 0 or index >= len(rows):
            continue
        if not family_matches(family, event, rows[index]):
            continue
        raw += 1
        bar = event["available"]
        if bar not in by_bar:
            by_bar[bar] = event
        elif by_bar[bar] is not None and by_bar[bar]["direction"] != event["direction"]:
            by_bar[bar] = None
    prepared = [event for event in by_bar.values() if event is not None]
    return prepared, raw


def state_key(
    kind: str, row: dict[str, Any], direction: int, source: int
) -> tuple[str, int, int | None] | None:
    if kind == "trend":
        if (
            row["adx_regime"] == 1
            and row["adx_slope"] == 1
            and row["di_state"] == direction
            and row["ema_stack"] == direction
            and row["macd_state"] == direction
        ):
            return ("trend", direction, None)
        return None
    if kind == "macd_state":
        if row["macd_state"] == direction:
            return ("macd_state", direction, None)
        return None
    if kind == "rsi_side":
        rsi = row["rsi"]
        if rsi is not None and (rsi - 50.0) * direction > 0:
            return ("rsi_side", direction, None)
        return None
    if kind == "ema_side":
        ema = row.get(EMA_COLUMNS.get(source, ""))
        if ema is not None and (row["close"] - ema) * direction > 0:
            return ("ema_side", direction, source)
        return None
    return None


def state_bases(
    rows: list[dict[str, Any]], hits: dict[int, list[bool | None]]
) -> dict[tuple[str, int, int | None], tuple[int, int]]:
    """Counts the barrier outcome on every bar on which each state holds."""
    totals: dict[tuple[str, int, int | None], list[int]] = defaultdict(lambda: [0, 0])
    for index, row in enumerate(rows):
        di = row["di_state"]
        if di in (-1, 1) and (
            row["adx_regime"] == 1
            and row["adx_slope"] == 1
            and row["ema_stack"] == di
            and row["macd_state"] == di
        ):
            hit = hits[int(di)][index]
            if hit is not None:
                totals[("trend", int(di), None)][0] += int(hit)
                totals[("trend", int(di), None)][1] += 1
        macd = row["macd_state"]
        if macd in (-1, 1):
            hit = hits[int(macd)][index]
            if hit is not None:
                totals[("macd_state", int(macd), None)][0] += int(hit)
                totals[("macd_state", int(macd), None)][1] += 1
        rsi = row["rsi"]
        if rsi is not None and rsi != 50.0:
            direction = 1 if rsi > 50.0 else -1
            hit = hits[direction][index]
            if hit is not None:
                totals[("rsi_side", direction, None)][0] += int(hit)
                totals[("rsi_side", direction, None)][1] += 1
        for source, column in EMA_COLUMNS.items():
            ema = row[column]
            if ema is not None and row["close"] != ema:
                direction = 1 if row["close"] > ema else -1
                hit = hits[direction][index]
                if hit is not None:
                    totals[("ema_side", direction, source)][0] += int(hit)
                    totals[("ema_side", direction, source)][1] += 1
    return {key: (value[0], value[1]) for key, value in totals.items()}


@dataclass
class Selected:
    bar: int
    direction: int
    source: int
    index: int
    hit: bool
    state: tuple[str, int, int | None] | None
    state_applicable: bool = True


@dataclass
class Bundle:
    instrument: str
    fold: int
    partition: str
    config: str
    bases: dict[int, dict[int, tuple[int, int]]] = field(default_factory=dict)
    hits: dict[int, dict[int, list[bool | None]]] = field(default_factory=dict)
    valid: dict[int, dict[int, list[int]]] = field(default_factory=dict)
    states: dict[int, dict[tuple[str, int, int | None], tuple[int, int]]] = field(
        default_factory=dict
    )
    selected: dict[tuple[str, int], list[Selected]] = field(default_factory=dict)
    raw: dict[tuple[str, int], int] = field(default_factory=dict)
    prepared: dict[tuple[str, int], int] = field(default_factory=dict)
    signature: dict[tuple[str, int], str] = field(default_factory=dict)


def load_bundle(
    cell: dict[str, Any],
    families: list[dict[str, Any]],
    options: Options,
    timeframes: set[int],
) -> Bundle:
    run = Path(cell["run"]).expanduser()
    bundle = Bundle(
        instrument=str(cell["instrument"]),
        fold=int(cell["fold"]),
        partition=str(cell["partition"]),
        config=str(cell["config"]),
    )
    rows_by_tf = read_rows(run / "readings.csv")
    events_by_tf = read_events(run / "events.csv")
    for timeframe in sorted(timeframes):
        rows = rows_by_tf.get(timeframe, [])
        if not rows:
            continue
        hits = barrier_arrays(rows, options.horizon, options.target_atr, options.stop_atr)
        bundle.hits[timeframe] = hits
        bundle.bases[timeframe] = {
            direction: counts(hits[direction]) for direction in (-1, 1)
        }
        bundle.valid[timeframe] = {
            direction: [index for index, hit in enumerate(hits[direction]) if hit is not None]
            for direction in (-1, 1)
        }
        bundle.states[timeframe] = state_bases(rows, hits)
        for family in families:
            family_id = str(family["id"])
            prepared, raw = select_events(events_by_tf.get(timeframe, []), rows, family)
            selected: list[Selected] = []
            signature: list[tuple[int, int, int]] = []
            for event in prepared:
                direction = int(event["direction"])
                index = int(event["available"])
                hit = hits[direction][index]
                if hit is None:
                    continue
                state_kind = str(family.get("state", ""))
                state_applicable = state_kind != "none"
                state = (
                    state_key(state_kind, rows[index], direction, int(event["source"]))
                    if state_applicable
                    else None
                )
                selected.append(
                    Selected(
                        bar=index,
                        direction=direction,
                        source=int(event["source"]),
                        index=int(event["index"]),
                        hit=bool(hit),
                        state=state,
                        state_applicable=state_applicable,
                    )
                )
                signature.append((index, direction, int(event["source"])))
            bundle.selected[(family_id, timeframe)] = selected
            bundle.raw[(family_id, timeframe)] = raw
            bundle.prepared[(family_id, timeframe)] = len(prepared)
            bundle.signature[(family_id, timeframe)] = hashlib.sha256(
                json.dumps(sorted(signature)).encode("utf-8")
            ).hexdigest()
    return bundle


def cluster_stats(entries: list[tuple[int, int, bool]], horizon: int) -> dict[str, Any]:
    """Cluster-robust statistics with clusters formed inside each bundle.

    `entries` are `(bundle_index, bar, hit)`. Clusters never span bundles.
    """
    n = len(entries)
    if n == 0:
        return {
            "n": 0,
            "clusters": 0,
            "hit_rate": None,
            "se": None,
            "singleton_bundles": 0,
        }
    p = sum(1 for _, _, hit in entries if hit) / n
    grouped: dict[int, list[tuple[int, bool]]] = defaultdict(list)
    for bundle_index, bar, hit in entries:
        grouped[bundle_index].append((bar, hit))
    variance = 0.0
    clusters = 0
    singleton = 0
    for values in grouped.values():
        values.sort()
        groups: list[list[tuple[int, bool]]] = []
        current: list[tuple[int, bool]] = []
        for bar, hit in values:
            if not current or bar - current[-1][0] >= horizon:
                if current:
                    groups.append(current)
                current = [(bar, hit)]
            else:
                current.append((bar, hit))
        if current:
            groups.append(current)
        j = len(groups)
        clusters += j
        if j <= 1:
            singleton += 1
            continue
        residual = sum(
            (sum(1 for _, hit in group if hit) - p * len(group)) ** 2 for group in groups
        )
        variance += (j / (j - 1)) * residual / (n * n)
    return {
        "n": n,
        "clusters": clusters,
        "hit_rate": p,
        "se": math.sqrt(variance) if variance > 0.0 else None,
        "singleton_bundles": singleton,
    }


def control_p(observed: float, draws: list[float | None]) -> float | None:
    valid = [draw for draw in draws if draw is not None]
    if not valid:
        return None
    return (1 + sum(1 for draw in valid if draw >= observed)) / (len(valid) + 1)


def percentile(draws: list[float | None], fraction: float) -> float | None:
    valid = sorted(draw for draw in draws if draw is not None)
    if not valid:
        return None
    index = min(len(valid) - 1, int(fraction * (len(valid) - 1)))
    return valid[index]


def event_base(
    bundles: list[Bundle], timeframe: int, selected: list[tuple[int, Selected]]
) -> float | None:
    if not selected:
        return None
    total = 0.0
    count = 0
    for bundle_index, event in selected:
        hits, valid = bundles[bundle_index].bases[timeframe][event.direction]
        if valid == 0:
            continue
        total += hits / valid
        count += 1
    return total / count if count else None


def state_base(
    bundles: list[Bundle], timeframe: int, selected: list[tuple[int, Selected]]
) -> tuple[float | None, int]:
    total = 0.0
    count = 0
    missing = 0
    for bundle_index, event in selected:
        if not event.state_applicable:
            continue
        if event.state is None:
            missing += 1
            continue
        hits, valid = bundles[bundle_index].states[timeframe].get(event.state, (0, 0))
        if valid == 0:
            missing += 1
            continue
        total += hits / valid
        count += 1
    return (total / count if count else None), missing


def random_draws(
    bundles: list[Bundle],
    timeframe: int,
    selected: list[tuple[int, Selected]],
    options: Options,
) -> list[float | None]:
    by_bundle: dict[int, list[Selected]] = defaultdict(list)
    for bundle_index, event in selected:
        by_bundle[bundle_index].append(event)
    rng = random.Random(options.seed)
    draws: list[float | None] = []
    for _ in range(options.random_draws):
        hits = 0
        total = 0
        for bundle_index, values in by_bundle.items():
            bundle = bundles[bundle_index]
            valid = bundle.valid[timeframe][1]
            if not valid:
                continue
            counts_by_direction: dict[int, int] = defaultdict(int)
            for event in values:
                counts_by_direction[event.direction] += 1
            for direction, count in counts_by_direction.items():
                pool = bundle.valid[timeframe][direction]
                if not pool or count == 0:
                    continue
                sample = rng.sample(pool, min(count, len(pool)))
                for bar in sample:
                    hit = bundle.hits[timeframe][direction][bar]
                    if hit is not None:
                        hits += int(hit)
                        total += 1
        draws.append(hits / total if total else None)
    return draws


def shuffle_draws(
    bundles: list[Bundle],
    timeframe: int,
    selected: list[tuple[int, Selected]],
    options: Options,
) -> list[float | None]:
    by_bundle: dict[int, list[Selected]] = defaultdict(list)
    for bundle_index, event in selected:
        by_bundle[bundle_index].append(event)
    rng = random.Random(options.seed + 1)
    draws: list[float | None] = []
    for _ in range(options.shuffle_draws):
        hits = 0
        total = 0
        for bundle_index, values in by_bundle.items():
            bundle = bundles[bundle_index]
            directions = [event.direction for event in values]
            rng.shuffle(directions)
            for event, direction in zip(values, directions):
                hit = bundle.hits[timeframe][direction][event.bar]
                if hit is not None:
                    hits += int(hit)
                    total += 1
        draws.append(hits / total if total else None)
    return draws


def lagged_rate(
    bundles: list[Bundle],
    timeframe: int,
    selected: list[tuple[int, Selected]],
    lag: int,
) -> tuple[float | None, int]:
    hits = 0
    total = 0
    for bundle_index, event in selected:
        bar = event.bar + lag
        bundle = bundles[bundle_index]
        if bar < 0 or bar >= len(bundle.hits[timeframe][event.direction]):
            continue
        hit = bundle.hits[timeframe][event.direction][bar]
        if hit is None:
            continue
        hits += int(hit)
        total += 1
    return (hits / total if total else None), total


def instrument_stats(
    bundles: list[Bundle], timeframe: int, selected: list[tuple[int, Selected]]
) -> dict[str, dict[str, Any]]:
    by_instrument: dict[str, list[tuple[int, Selected]]] = defaultdict(list)
    for bundle_index, event in selected:
        by_instrument[bundles[bundle_index].instrument].append((bundle_index, event))
    result = {}
    for instrument in sorted(by_instrument):
        values = by_instrument[instrument]
        hits = sum(1 for _, event in values if event.hit)
        base = event_base(bundles, timeframe, values)
        result[instrument] = {
            "n": len(values),
            "hits": hits,
            "hit_rate": hits / len(values) if values else None,
            "base": base,
        }
    return result


def primary_z(cells: int = PRIMARY_CELLS) -> float:
    return NormalDist().inv_cdf(1.0 - CONTROL_ALPHA / (2.0 * cells))


def validation_z() -> float:
    return NormalDist().inv_cdf(1.0 - CONTROL_ALPHA / 2.0)


def lower(value: float | None, se: float | None, z: float) -> float | None:
    if value is None or se is None:
        return None
    return value - z * se


def evaluate(
    bundles: list[Bundle],
    families: list[dict[str, Any]],
    timeframes: set[int],
    options: Options,
    z: float,
) -> list[dict[str, Any]]:
    rows = []
    for family in families:
        family_id = str(family["id"])
        for timeframe in sorted(timeframes):
            selected = [
                (bundle_index, event)
                for bundle_index, bundle in enumerate(bundles)
                for event in bundle.selected.get((family_id, timeframe), [])
            ]
            stats = cluster_stats(
                [(bundle_index, event.bar, event.hit) for bundle_index, event in selected],
                options.horizon,
            )
            n = stats["n"]
            base = event_base(bundles, timeframe, selected)
            state, state_missing = state_base(bundles, timeframe, selected)
            state_applicable = str(family.get("state", "")) != "none"
            p = stats["hit_rate"]
            se = stats["se"]
            lift = p - base if p is not None and base is not None else None
            incremental = p - state if p is not None and state is not None else None
            random_controls = random_draws(bundles, timeframe, selected, options)
            shuffled_controls = shuffle_draws(bundles, timeframe, selected, options)
            back_rate, back_n = lagged_rate(bundles, timeframe, selected, -options.back_lag)
            forward_rate, forward_n = lagged_rate(bundles, timeframe, selected, options.back_lag)
            back_lift = (
                back_rate - base if back_rate is not None and base is not None else None
            )
            instrument = instrument_stats(bundles, timeframe, selected)
            replication = sum(
                1
                for value in instrument.values()
                if value["n"] >= 30
                and value["hit_rate"] is not None
                and value["base"] is not None
                and value["hit_rate"] > value["base"]
            )
            signatures = {
                bundle.signature.get((family_id, timeframe)) for bundle in bundles
            }
            rows.append(
                {
                    "family": family_id,
                    "label": family.get("label", family_id),
                    "timeframe": TIMEFRAME_CODES.get(timeframe, str(timeframe)),
                    "timeframe_secs": timeframe,
                    "config": bundles[0].config if bundles else None,
                    "raw": sum(
                        bundle.raw.get((family_id, timeframe), 0) for bundle in bundles
                    ),
                    "prepared": sum(
                        bundle.prepared.get((family_id, timeframe), 0)
                        for bundle in bundles
                    ),
                    "n": n,
                    "clusters": stats["clusters"],
                    "singleton_bundles": stats["singleton_bundles"],
                    "hits": sum(1 for _, event in selected if event.hit),
                    "hit_rate": p,
                    "base_rate": base,
                    "lift": lift,
                    "lift_lcb": lower(lift, se, z),
                    "state_applicable": state_applicable,
                    "state_rate": state,
                    "state_missing": state_missing,
                    "incremental_lift": incremental,
                    "incremental_lcb": lower(incremental, se, z),
                    "cluster_se": se,
                    "random_mean": (
                        sum(draw for draw in random_controls if draw is not None)
                        / len([draw for draw in random_controls if draw is not None])
                        if any(draw is not None for draw in random_controls)
                        else None
                    ),
                    "random_p95": percentile(random_controls, 0.95),
                    "random_p": (
                        control_p(p, random_controls) if p is not None else None
                    ),
                    "shuffle_mean": (
                        sum(draw for draw in shuffled_controls if draw is not None)
                        / len([draw for draw in shuffled_controls if draw is not None])
                        if any(draw is not None for draw in shuffled_controls)
                        else None
                    ),
                    "shuffle_p95": percentile(shuffled_controls, 0.95),
                    "shuffle_p": (
                        control_p(p, shuffled_controls) if p is not None else None
                    ),
                    "back_lag_rate": back_rate,
                    "back_lag_n": back_n,
                    "back_lag_lift": back_lift,
                    "forward_lag_rate": forward_rate,
                    "forward_lag_n": forward_n,
                    "forward_lag_lift": (
                        forward_rate - base
                        if forward_rate is not None and base is not None
                        else None
                    ),
                    "replication": replication,
                    "instruments": instrument,
                    "event_signature": hashlib.sha256(
                        json.dumps(sorted(signatures, key=str)).encode("utf-8")
                    ).hexdigest(),
                }
            )
    return rows


def grade(row: dict[str, Any], primary: bool) -> tuple[str, list[str]]:
    reasons = []
    if not primary:
        return "sensitivity", reasons
    checks = [
        ("n", row["n"] >= MIN_EVENTS),
        ("clusters", row["clusters"] >= MIN_CLUSTERS),
        ("lift", row["lift"] is not None and row["lift"] >= MIN_LIFT),
        ("lift_lcb", row["lift_lcb"] is not None and row["lift_lcb"] > 0.0),
    ]
    if row["state_applicable"]:
        checks.extend(
            [
                (
                    "incremental_lift",
                    row["incremental_lift"] is not None
                    and row["incremental_lift"] >= MIN_INCREMENTAL_LIFT,
                ),
                (
                    "incremental_lcb",
                    row["incremental_lcb"] is not None
                    and row["incremental_lcb"] > 0.0,
                ),
            ]
        )
    checks.extend(
        [
            (
                "random",
                row["random_p"] is not None and row["random_p"] <= 0.05,
            ),
            (
                "back_lag",
                row["back_lag_lift"] is not None
                and (
                    row["back_lag_lift"] <= 0.01
                    or (
                        row["lift"] is not None
                        and row["lift"] - row["back_lag_lift"] >= 0.01
                    )
                ),
            ),
            (
                "shuffle",
                row["shuffle_p"] is not None and row["shuffle_p"] <= 0.05,
            ),
        ]
    )
    if row["state_applicable"]:
        checks.append(("state", row["state_missing"] == 0))
    checks.append(("replication", row["replication"] >= 2))
    for name, ok in checks:
        if not ok:
            reasons.append(name)
    if reasons:
        return "C", reasons
    if (
        row["lift"] is not None
        and row["lift"] >= 0.02
        and row["replication"] >= 3
    ):
        return "A", reasons
    return "B", reasons


def apply_sensitivity(
    rows: list[dict[str, Any]], primary_config: str, sensitivity_config: str
) -> None:
    index = {
        (row["family"], row["timeframe"], row["config"]): row for row in rows
    }
    for row in rows:
        if row["config"] != primary_config:
            continue
        other = index.get((row["family"], row["timeframe"], sensitivity_config))
        if other is None:
            row["sensitivity_ok"] = False
            row["sensitivity_reason"] = "missing"
            continue
        if row["family"] == "h1_trend_continuation":
            ok = other["lift"] is not None and other["lift"] > 0.0
            reason = "lift" if ok else "non_positive_lift"
        else:
            ok = (
                other["event_signature"] == row["event_signature"]
                and other["n"] == row["n"]
                and other["hits"] == row["hits"]
            )
            reason = "identical" if ok else "event_set_changed"
        row["sensitivity_ok"] = ok
        row["sensitivity_reason"] = reason
        other["sensitivity_ok"] = ok
        other["sensitivity_reason"] = reason


def markdown(document: dict[str, Any]) -> str:
    lines = [
        "# Model v4 Path Validation",
        "",
        f"- Horizon: {document['horizon_bars']} bars",
        f"- Target / stop: {document['target_atr']} / {document['stop_atr']} ATR",
        f"- Seed: {document['seed']}; random draws: {document['random_draws']}; "
        f"shuffle draws: {document['shuffle_draws']}; back lag: {document['back_lag']}",
        f"- Primary config: `{document['primary_config']}`; "
        f"sensitivity config: `{document['sensitivity_config']}`",
        "",
        "## Primary cells",
        "",
        "| Family | TF | n | J | Hit | Base | Lift | Lift LCB | State | Incr. | Incr. LCB | Rand p | Back lag | Shuffle p | Rep | Grade |",
        "|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|",
    ]
    primary = [
        row for row in document["cells"] if row["config"] == document["primary_config"]
    ]
    for row in primary:
        lines.append(
            "| {family} | {tf} | {n} | {clusters} | {hit} | {base} | {lift} | {lcb} | "
            "{state} | {incr} | {incr_lcb} | {rand} | {back} | {shuffle} | {rep} | {grade} |".format(
                family=row["family"],
                tf=row["timeframe"],
                n=row["n"],
                clusters=row["clusters"],
                hit=fmt(row["hit_rate"]),
                base=fmt(row["base_rate"]),
                lift=fmt_signed(row["lift"]),
                lcb=fmt(row["lift_lcb"]),
                state=fmt(row["state_rate"]),
                incr=fmt_signed(row["incremental_lift"]),
                incr_lcb=fmt(row["incremental_lcb"]),
                rand=fmt(row["random_p"]),
                back=fmt_signed(row["back_lag_lift"]),
                shuffle=fmt(row["shuffle_p"]),
                rep=row["replication"],
                grade=row["grade"],
            )
        )
    lines += [
        "",
        "`-` in State/Incr. means the family declares no state (`state: none`); "
        "the state-conditioned incremental test is not applicable.",
    ]
    failed = [row for row in primary if row["reasons"]]
    if failed:
        lines += [
            "",
            "### Failed checks",
            "",
            "| Family | TF | Failed checks |",
            "|---|---|---|",
        ]
        for row in failed:
            lines.append(
                f"| {row['family']} | {row['timeframe']} | {', '.join(row['reasons'])} |"
            )
    lines += [
        "",
        "## Sensitivity cells",
        "",
        "| Family | TF | n | Hit | Base | Lift | Incr. | Rand p | Back lag | Grade | Isolation |",
        "|---|---|---:|---:|---:|---:|---:|---:|---:|---|---|",
    ]
    sensitivity = [
        row
        for row in document["cells"]
        if row["config"] == document["sensitivity_config"]
    ]
    for row in sensitivity:
        lines.append(
            "| {family} | {tf} | {n} | {hit} | {base} | {lift} | {incr} | {rand} | {back} | {grade} | {reason} |".format(
                family=row["family"],
                tf=row["timeframe"],
                n=row["n"],
                hit=fmt(row["hit_rate"]),
                base=fmt(row["base_rate"]),
                lift=fmt_signed(row["lift"]),
                incr=fmt_signed(row["incremental_lift"]),
                rand=fmt(row["random_p"]),
                back=fmt_signed(row["back_lag_lift"]),
                grade=row["grade"],
                reason=row.get("sensitivity_reason", "-"),
            )
        )
    lines += [
        "",
        "## Per-instrument (primary config)",
        "",
        "| Family | TF | Instrument | n | Hit | Base | Lift |",
        "|---|---|---|---:|---:|---:|---:|",
    ]
    for row in primary:
        for instrument in sorted(row["instruments"]):
            value = row["instruments"][instrument]
            lines.append(
                "| {family} | {tf} | {instrument} | {n} | {hit} | {base} | {lift} |".format(
                    family=row["family"],
                    tf=row["timeframe"],
                    instrument=instrument,
                    n=value["n"],
                    hit=fmt(value["hit_rate"]),
                    base=fmt(value["base"]),
                    lift=fmt_signed(
                        None
                        if value["hit_rate"] is None or value["base"] is None
                        else value["hit_rate"] - value["base"]
                    ),
                )
            )
    lines += [
        "",
        "## Decision",
        "",
        document["decision"],
        "",
    ]
    return "\n".join(lines)


def fmt(value: float | None) -> str:
    return "-" if value is None else f"{value:.4f}"


def fmt_signed(value: float | None) -> str:
    return "-" if value is None else f"{value:+.4f}"


def main(argv: list[str] | None = None) -> int:
    args = parse_arguments(argv)
    matrix = read_json(Path(args.matrix).expanduser())
    families = read_json(Path(args.families).expanduser())
    if not isinstance(families, list):
        print("error: families must be a JSON list", file=sys.stderr)
        return 1
    cells = expand_cells(matrix, args.config)
    if not cells:
        print("error: no cells", file=sys.stderr)
        return 1
    timeframes = set()
    for code in args.timeframes.split(","):
        code = code.strip()
        for seconds, name in TIMEFRAME_CODES.items():
            if code == name:
                timeframes.add(seconds)
    if not timeframes:
        print("error: no supported timeframes", file=sys.stderr)
        return 1
    options = Options(
        horizon=args.horizon,
        target_atr=args.target_atr,
        stop_atr=args.stop_atr,
        random_draws=args.random_draws,
        shuffle_draws=args.shuffle_draws,
        back_lag=args.back_lag,
        seed=args.seed,
    )
    bundles = [load_bundle(cell, families, options, timeframes) for cell in cells]
    configs = []
    for cell in cells:
        if cell["config"] not in configs:
            configs.append(cell["config"])
    primary_config = str(matrix.get("primary_config", configs[0])) if isinstance(matrix, dict) else configs[0]
    sensitivity_config = (
        str(matrix.get("sensitivity_config", configs[-1]))
        if isinstance(matrix, dict)
        else configs[-1]
    )
    rows = []
    z = primary_z(args.primary_cells)
    for config in configs:
        subset = [bundle for bundle in bundles if bundle.config == config]
        rows.extend(evaluate(subset, families, timeframes, options, z))
    if not args.skip_sensitivity:
        apply_sensitivity(rows, primary_config, sensitivity_config)
    for row in rows:
        row["grade"], row["reasons"] = grade(row, row["config"] == primary_config)
        if (
            not args.skip_sensitivity
            and row["config"] == primary_config
            and row.get("sensitivity_ok") is not True
        ):
            row["grade"] = "C"
            row["reasons"].append("sensitivity")
    passing = [
        row
        for row in rows
        if row["config"] == primary_config and row["grade"] in ("A", "B")
    ]
    selection = None
    if passing:
        selection = sorted(
            passing,
            key=lambda row: (
                -(row["incremental_lcb"] if row["incremental_lcb"] is not None else -math.inf),
                TIMEFRAME_ORDER.get(row["timeframe_secs"], 99),
                row["family"],
            ),
        )[0]
    decision = "No primary cell passes; the development partition records **no edge**. The validation partition is not opened."
    if selection is not None:
        decision = (
            f"Frozen candidate: `{selection['family']}` on `{selection['timeframe']}` "
            f"under `{primary_config}` (grade {selection['grade']}, "
            f"incremental LCB {fmt(selection['incremental_lcb'])}). "
            "The validation partition may now be opened once."
        )
    document = {
        "schema": "quantrule.path-validation/v2",
        "horizon_bars": options.horizon,
        "target_atr": options.target_atr,
        "stop_atr": options.stop_atr,
        "seed": options.seed,
        "random_draws": options.random_draws,
        "shuffle_draws": options.shuffle_draws,
        "back_lag": options.back_lag,
        "primary_config": primary_config,
        "sensitivity_config": sensitivity_config,
        "primary_cells": args.primary_cells,
        "cells": sorted(
            rows,
            key=lambda row: (
                TIMEFRAME_ORDER.get(row["timeframe_secs"], 99),
                row["family"],
                row["config"],
            ),
        ),
        "selection": selection,
        "decision": decision,
    }
    out = Path(args.out).expanduser()
    out.mkdir(parents=True, exist_ok=True)
    (out / "path_validation.json").write_text(
        json.dumps(document, indent=2) + "\n", encoding="utf-8"
    )
    (out / "path_validation.md").write_text(markdown(document), encoding="utf-8")
    print(f"ok path validation -> {out}")
    return 0


def expand_cells(matrix: Any, config: str | None) -> list[dict[str, Any]]:
    if isinstance(matrix, list):
        cells = matrix
    elif isinstance(matrix, dict) and isinstance(matrix.get("cells"), list):
        cells = []
        for cell in matrix["cells"]:
            runs = cell.get("runs")
            if isinstance(runs, dict):
                for name, run in runs.items():
                    if config is not None and name != config:
                        continue
                    cells.append(
                        {
                            "instrument": cell["instrument"],
                            "fold": cell["fold"],
                            "partition": cell["partition"],
                            "config": name,
                            "run": run,
                        }
                    )
            else:
                cells.append(cell)
    else:
        return []
    return cells


if __name__ == "__main__":
    sys.exit(main())
