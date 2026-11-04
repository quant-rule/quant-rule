#!/usr/bin/env python3
"""V5 Phase 2 read-only spike over price-only new information.

The spike screens three candidates on `model_dev`:

- a momentum bar (candle anatomy),
- a rejection wick at a confirmed major level,
- a relative-strength cross against the BTC/ETH/XAU peer basket.

It also audits the pack's volume series. The volume family (VWAP, volume
profile, volume climax) is not tested because the source volume units change by
600x-17,800x within a year on BTC and ETH.

Every rule is deterministic arithmetic on completed OHLCV bars. No machine
learning, neural network, fitted model, or statistic enters an event or the
barrier; statistics is used only to evaluate the events.

Usage:
    python3 tools/improvement/v5_phase2_spike.py \
        --matrix config/model-v4/dev/matrix.json \
        --config adx25 \
        --out data/improvement/2026-11-04/V5-PHASE2-SPIKE/artifacts \
        --seed 0
"""

from __future__ import annotations

import argparse
import csv
import datetime
import json
import math
import random
import statistics
import sys
from collections import Counter, defaultdict
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

import path_validation

SCHEMA = "quantrule.v5-phase2-spike/v1"
TIMEFRAME = 3600
HORIZON = 100
TARGET_ATR = 4.0
STOP_ATR = 2.0
BACK_LAG = 100
BLOCK = 100
MIN_EVENTS = 200
MIN_BLOCKS = 20
MIN_LIFT = 0.05
ALPHA = 0.05
RANDOM_DRAWS = 200
SHUFFLE_DRAWS = 200
PRIMARY_CANDIDATES = 3
LEVEL_LOOKBACK = 500
LEVEL_ATR = 0.25
VOLUME_RATIO_MAX = 10.0


@dataclass
class Bundle:
    instrument: str
    fold: int
    rows: list[dict[str, Any]]
    by_timestamp: dict[int, int]
    levels: list[tuple[int, float]]
    hits: dict[int, list[bool | None]]
    valid: dict[int, list[int]]
    base: dict[int, float | None]


def read_rows(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    with path.open(newline="", encoding="utf-8") as handle:
        for row in csv.DictReader(handle):
            if row.get("timeframe_secs") != str(TIMEFRAME):
                continue
            try:
                timestamp = int(row["timestamp"])
                open_price = float(row["open"])
                high = float(row["high"])
                low = float(row["low"])
                close = float(row["close"])
            except (KeyError, TypeError, ValueError):
                continue
            atr = path_validation.number(row.get("atr", ""))
            if atr is None or atr <= 0.0:
                continue
            rows.append(
                {
                    "timestamp": timestamp,
                    "open": open_price,
                    "high": high,
                    "low": low,
                    "close": close,
                    "volume": path_validation.number(row.get("volume", "")),
                    "atr": atr,
                }
            )
    rows.sort(key=lambda value: value["timestamp"])
    return rows


def read_levels(path: Path) -> list[tuple[int, float]]:
    levels: list[tuple[int, float]] = []
    if not path.exists():
        return levels
    with path.open(newline="", encoding="utf-8") as handle:
        for row in csv.DictReader(handle):
            if row.get("timeframe_secs") != str(TIMEFRAME):
                continue
            if row.get("scale_code") != "2":
                continue
            try:
                available = int(row["available_bar_index"])
                price = float(row["price"])
            except (KeyError, TypeError, ValueError):
                continue
            levels.append((available, price))
    levels.sort()
    return levels


def build_bundle(cell: dict[str, Any], config: str) -> Bundle:
    run = Path(str(cell["run"])).expanduser()
    rows = read_rows(run / "readings.csv")
    hits = path_validation.barrier_arrays(rows, HORIZON, TARGET_ATR, STOP_ATR)
    valid = {
        direction: [index for index, hit in enumerate(hits[direction]) if hit is not None]
        for direction in (-1, 1)
    }
    base = {}
    for direction in (-1, 1):
        values = [hit for hit in hits[direction] if hit is not None]
        base[direction] = (sum(1 for hit in values if hit) / len(values)) if values else None
    return Bundle(
        instrument=str(cell["instrument"]),
        fold=int(cell["fold"]),
        rows=rows,
        by_timestamp={row["timestamp"]: index for index, row in enumerate(rows)},
        levels=read_levels(run / "levels.csv"),
        hits=hits,
        valid=valid,
        base=base,
    )


def month_key(timestamp: int) -> str:
    return datetime.datetime.fromtimestamp(timestamp, tz=datetime.timezone.utc).strftime(
        "%Y-%m"
    )


def volume_audit(bundles: list[Bundle]) -> list[dict[str, Any]]:
    rows = []
    for bundle in bundles:
        monthly: dict[str, list[float]] = defaultdict(list)
        for row in bundle.rows:
            volume = row["volume"]
            if volume is not None and volume > 0.0:
                monthly[month_key(row["timestamp"])].append(volume)
        medians = {
            key: statistics.median(values) for key, values in sorted(monthly.items())
        }
        values = [value for value in medians.values() if value > 0.0]
        ratio = max(values) / min(values) if values else None
        rows.append(
            {
                "instrument": bundle.instrument,
                "fold": bundle.fold,
                "months": len(medians),
                "median_min": min(values) if values else None,
                "median_max": max(values) if values else None,
                "ratio": ratio,
                "admissible": ratio is not None and ratio <= VOLUME_RATIO_MAX,
            }
        )
    return rows


def momentum_events(bundle: Bundle) -> list[tuple[int, int]]:
    events = []
    for index, row in enumerate(bundle.rows):
        span = row["high"] - row["low"]
        if span <= 0.0 or span < 1.5 * row["atr"]:
            continue
        body = abs(row["close"] - row["open"])
        if body / span < 0.7:
            continue
        if row["close"] > row["open"] and (row["close"] - row["low"]) / span >= 0.8:
            events.append((index, 1))
        elif row["close"] < row["open"] and (row["high"] - row["close"]) / span >= 0.8:
            events.append((index, -1))
    return events


def rejection_events(bundle: Bundle) -> list[tuple[int, int]]:
    events = []
    for index, row in enumerate(bundle.rows):
        span = row["high"] - row["low"]
        if span <= 0.0 or span < 0.5 * row["atr"]:
            continue
        upper = row["high"] - max(row["open"], row["close"])
        lower = min(row["open"], row["close"]) - row["low"]
        tolerance = LEVEL_ATR * row["atr"]
        bearish_level = any(
            abs(price - row["high"]) <= tolerance
            and 0 <= index - available <= LEVEL_LOOKBACK
            for available, price in bundle.levels
        )
        bullish_level = any(
            abs(price - row["low"]) <= tolerance
            and 0 <= index - available <= LEVEL_LOOKBACK
            for available, price in bundle.levels
        )
        bearish = (
            upper / span >= 0.6 and row["close"] < row["open"] and bearish_level
        )
        bullish = (
            lower / span >= 0.6 and row["close"] > row["open"] and bullish_level
        )
        if bearish and not bullish:
            events.append((index, -1))
        elif bullish and not bearish:
            events.append((index, 1))
    return events


def gap_events(bundle: Bundle) -> list[tuple[int, int]]:
    events = []
    for index in range(1, len(bundle.rows)):
        gap = bundle.rows[index]["open"] - bundle.rows[index - 1]["close"]
        if abs(gap) >= 0.5 * bundle.rows[index]["atr"]:
            events.append((index, 1 if gap > 0.0 else -1))
    return events


def relative_strength_events(
    bundles: list[Bundle],
) -> dict[tuple[str, int], list[tuple[int, int]]]:
    """Computes relative-strength crosses on the common-timestamp panel.

    Returns a mapping from `(instrument, fold)` to `(bar, direction)` events.
    """
    by_year: dict[int, list[Bundle]] = defaultdict(list)
    for bundle in bundles:
        by_year[bundle.fold].append(bundle)
    result: dict[tuple[str, int], list[tuple[int, int]]] = {}
    for fold, group in sorted(by_year.items()):
        if len(group) != 3:
            continue
        common = sorted(set.intersection(*(set(bundle.by_timestamp) for bundle in group)))
        previous: dict[str, float | None] = {bundle.instrument: None for bundle in group}
        for position in range(24, len(common)):
            now = common[position]
            then = common[position - 24]
            returns: dict[str, float] = {}
            for bundle in group:
                index = bundle.by_timestamp[now]
                index_then = bundle.by_timestamp[then]
                atr = bundle.rows[index]["atr"]
                if atr <= 0.0:
                    break
                returns[bundle.instrument] = (
                    bundle.rows[index]["close"] - bundle.rows[index_then]["close"]
                ) / atr
            else:
                for bundle in group:
                    others = [
                        returns[other]
                        for other in returns
                        if other != bundle.instrument
                    ]
                    relative = returns[bundle.instrument] - sum(others) / len(others)
                    prior = previous[bundle.instrument]
                    if prior is not None:
                        key = (bundle.instrument, fold)
                        result.setdefault(key, [])
                        if prior <= 1.0 < relative:
                            result[key].append((bundle.by_timestamp[now], 1))
                        elif prior >= -1.0 > relative:
                            result[key].append((bundle.by_timestamp[now], -1))
                    previous[bundle.instrument] = relative
                continue
            previous = {bundle.instrument: None for bundle in group}
    return result


def block_stats(
    entries: list[tuple[int, int, int, bool]], block: int = BLOCK
) -> dict[str, Any]:
    """Fixed, non-overlapping block cluster-robust statistics.

    `entries` are `(bundle_index, bar, direction, hit)`. Blocks never span
    bundles, and a block is `bar // block`, so dense events cannot chain into a
    single cluster across a whole bundle.
    """
    n = len(entries)
    if n == 0:
        return {"n": 0, "blocks": 0, "hit_rate": None, "se": None}
    p = sum(1 for *_, hit in entries if hit) / n
    grouped: dict[int, dict[int, list[bool]]] = defaultdict(lambda: defaultdict(list))
    for bundle_index, bar, _, hit in entries:
        grouped[bundle_index][bar // block].append(hit)
    variance = 0.0
    blocks = 0
    for values in grouped.values():
        count = len(values)
        blocks += count
        if count <= 1:
            continue
        residual = sum(
            (sum(1 for hit in hits if hit) - p * len(hits)) ** 2
            for hits in values.values()
        )
        variance += (count / (count - 1)) * residual / (n * n)
    return {"n": n, "blocks": blocks, "hit_rate": p, "se": math.sqrt(variance)}


def event_base(bundles: list[Bundle], entries: list[tuple[int, int, int, bool]]) -> float | None:
    total = 0.0
    count = 0
    for bundle_index, _, direction, _ in entries:
        base = bundles[bundle_index].base[direction]
        if base is None:
            continue
        total += base
        count += 1
    return total / count if count else None


def random_draws(
    bundles: list[Bundle],
    entries: list[tuple[int, int, int, bool]],
    seed: int,
    draws: int = RANDOM_DRAWS,
) -> list[float | None]:
    by_bundle: dict[int, list[int]] = defaultdict(list)
    for bundle_index, _, direction, _ in entries:
        by_bundle[bundle_index].append(direction)
    rng = random.Random(seed)
    result: list[float | None] = []
    for _ in range(draws):
        hits = 0
        total = 0
        for bundle_index, directions in by_bundle.items():
            for direction, count in Counter(directions).items():
                pool = bundles[bundle_index].valid[direction]
                if not pool:
                    continue
                for bar in rng.sample(pool, min(count, len(pool))):
                    hit = bundles[bundle_index].hits[direction][bar]
                    if hit is not None:
                        hits += int(hit)
                        total += 1
        result.append(hits / total if total else None)
    return result


def shuffle_draws(
    bundles: list[Bundle],
    entries: list[tuple[int, int, int, bool]],
    seed: int,
    draws: int = SHUFFLE_DRAWS,
) -> list[float | None]:
    by_bundle: dict[int, list[tuple[int, int]]] = defaultdict(list)
    for bundle_index, bar, direction, _ in entries:
        by_bundle[bundle_index].append((bar, direction))
    rng = random.Random(seed + 1)
    result: list[float | None] = []
    for _ in range(draws):
        hits = 0
        total = 0
        for bundle_index, values in by_bundle.items():
            directions = [direction for _, direction in values]
            rng.shuffle(directions)
            for (bar, _), direction in zip(values, directions):
                hit = bundles[bundle_index].hits[direction][bar]
                if hit is not None:
                    hits += int(hit)
                    total += 1
        result.append(hits / total if total else None)
    return result


def lagged_rate(
    bundles: list[Bundle],
    entries: list[tuple[int, int, int, bool]],
    lag: int,
) -> tuple[float | None, int]:
    hits = 0
    total = 0
    for bundle_index, bar, direction, _ in entries:
        index = bar - lag
        if index < 0 or index >= len(bundles[bundle_index].rows):
            continue
        hit = bundles[bundle_index].hits[direction][index]
        if hit is None:
            continue
        hits += int(hit)
        total += 1
    return (hits / total if total else None), total


def instrument_stats(
    bundles: list[Bundle], entries: list[tuple[int, int, int, bool]]
) -> dict[str, dict[str, Any]]:
    grouped: dict[str, list[tuple[int, int, int, bool]]] = defaultdict(list)
    for entry in entries:
        grouped[bundles[entry[0]].instrument].append(entry)
    result = {}
    for instrument in sorted(grouped):
        values = grouped[instrument]
        hits = sum(1 for *_, hit in values if hit)
        base = event_base(bundles, values)
        result[instrument] = {
            "n": len(values),
            "hits": hits,
            "hit_rate": hits / len(values) if values else None,
            "base": base,
        }
    return result


def evaluate(
    bundles: list[Bundle],
    candidate_id: str,
    label: str,
    kind: str,
    events: list[tuple[int, int, int]],
    z: float,
    seed: int,
) -> dict[str, Any]:
    entries: list[tuple[int, int, int, bool]] = []
    for bundle_index, bar, direction in events:
        hit = bundles[bundle_index].hits[direction][bar]
        if hit is None:
            continue
        entries.append((bundle_index, bar, direction, bool(hit)))

    stats = block_stats(entries)
    p = stats["hit_rate"]
    se = stats["se"]
    base = event_base(bundles, entries)
    lift = p - base if p is not None and base is not None else None
    lcb = path_validation.lower(lift, se, z)
    random_p = (
        path_validation.control_p(p, random_draws(bundles, entries, seed))
        if p is not None
        else None
    )
    shuffle_p = (
        path_validation.control_p(p, shuffle_draws(bundles, entries, seed))
        if p is not None
        else None
    )
    back_rate, back_n = lagged_rate(bundles, entries, BACK_LAG)
    back_lift = back_rate - base if back_rate is not None and base is not None else None
    instruments = instrument_stats(bundles, entries)
    replication = sum(
        1
        for value in instruments.values()
        if value["n"] >= 30
        and value["hit_rate"] is not None
        and value["base"] is not None
        and value["hit_rate"] > value["base"]
    )

    checks = [
        ("n", stats["n"] >= MIN_EVENTS),
        ("blocks", stats["blocks"] >= MIN_BLOCKS),
        ("lift", lift is not None and lift >= MIN_LIFT),
        ("lift_lcb", lcb is not None and lcb > 0.0),
        ("random", random_p is not None and random_p <= ALPHA),
        (
            "back_lag",
            back_lift is not None
            and (back_lift <= 0.01 or (lift is not None and lift - back_lift >= 0.01)),
        ),
        ("shuffle", shuffle_p is not None and shuffle_p <= ALPHA),
        ("replication", replication >= 2),
    ]
    reasons = [name for name, ok in checks if not ok]
    return {
        "id": candidate_id,
        "label": label,
        "kind": kind,
        "n": stats["n"],
        "blocks": stats["blocks"],
        "hit_rate": p,
        "base_rate": base,
        "lift": lift,
        "lift_lcb": lcb,
        "random_p": random_p,
        "back_lag_rate": back_rate,
        "back_lag_n": back_n,
        "back_lag_lift": back_lift,
        "shuffle_p": shuffle_p,
        "replication": replication,
        "instruments": instruments,
        "criteria_met": not reasons,
        "passed": not reasons and kind == "primary",
        "reasons": reasons,
    }


def fmt(value: Any) -> str:
    if value is None:
        return "-"
    if isinstance(value, float):
        return f"{value:.4f}"
    return str(value)


def markdown(document: dict[str, Any]) -> str:
    lines = [
        "# V5 Phase 2 Spike (model_dev 2018–2020)",
        "",
        f"- Horizon: {document['horizon_bars']} bars; target / stop: "
        f"{document['target_atr']} / {document['stop_atr']} ATR",
        f"- Block: {document['block_bars']} bars; seed: {document['seed']}",
        f"- Primary candidates: {document['primary_candidates']}",
        "",
        "## Volume audit",
        "",
        "| Instrument | Fold | Monthly median min | Monthly median max | Ratio | Admissible |",
        "|---|---:|---:|---:|---:|---|",
    ]
    for row in document["volume_audit"]:
        lines.append(
            "| {instrument} | {fold} | {low} | {high} | {ratio} | {ok} |".format(
                instrument=row["instrument"],
                fold=row["fold"],
                low=fmt(row["median_min"]),
                high=fmt(row["median_max"]),
                ratio=fmt(row["ratio"]),
                ok="yes" if row["admissible"] else "no",
            )
        )
    lines += [
        "",
        f"- Volume family: **{document['volume_status']}**.",
        "",
        "## Candidates",
        "",
        "| ID | Candidate | n | Blocks | Hit | Base | Lift | Lift LCB | Rand p | Back lag | Shuffle p | Rep | Verdict |",
        "|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|",
    ]
    for row in document["candidates"]:
        lines.append(
            "| {id} | {label} | {n} | {blocks} | {hit} | {base} | {lift} | {lcb} | {rand} | {back} | {shuffle} | {rep} | {verdict} |".format(
                id=row["id"],
                label=row["label"],
                n=row["n"],
                blocks=row["blocks"],
                hit=fmt(row["hit_rate"]),
                base=fmt(row["base_rate"]),
                lift=fmt(row["lift"]),
                lcb=fmt(row["lift_lcb"]),
                rand=fmt(row["random_p"]),
                back=fmt(row["back_lag_lift"]),
                shuffle=fmt(row["shuffle_p"]),
                rep=row["replication"],
                verdict=row["verdict"],
            )
        )
    lines += [
        "",
        "## Per instrument",
        "",
        "| ID | Instrument | n | Hit | Base | Lift |",
        "|---|---|---:|---:|---:|---:|",
    ]
    for row in document["candidates"]:
        for instrument, value in row["instruments"].items():
            lift = (
                value["hit_rate"] - value["base"]
                if value["hit_rate"] is not None and value["base"] is not None
                else None
            )
            lines.append(
                f"| {row['id']} | {instrument} | {value['n']} | "
                f"{fmt(value['hit_rate'])} | {fmt(value['base'])} | {fmt(lift)} |"
            )
    lines += [
        "",
        "## Decision",
        "",
        document["decision"],
        "",
    ]
    return "\n".join(lines)


def parse_arguments(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--matrix", required=True)
    parser.add_argument("--config", default="adx25")
    parser.add_argument("--out", required=True)
    parser.add_argument("--seed", type=int, default=0)
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_arguments(argv)
    matrix = json.loads(Path(args.matrix).read_text(encoding="utf-8"))
    cells = path_validation.expand_cells(matrix, args.config)
    if not cells:
        print("error: matrix has no cells", file=sys.stderr)
        return 1
    bundles = [build_bundle(cell, args.config) for cell in cells]
    bundles = [bundle for bundle in bundles if bundle.rows]
    if not bundles:
        print("error: no 1h rows", file=sys.stderr)
        return 1

    z = path_validation.primary_z(PRIMARY_CANDIDATES)
    candidates: list[dict[str, Any]] = []

    momentum = [
        (index, bar, direction)
        for index, bundle in enumerate(bundles)
        for bar, direction in momentum_events(bundle)
    ]
    candidates.append(
        evaluate(bundles, "C1", "Momentum bar", "primary", momentum, z, args.seed)
    )

    rejection = [
        (index, bar, direction)
        for index, bundle in enumerate(bundles)
        for bar, direction in rejection_events(bundle)
    ]
    candidates.append(
        evaluate(
            bundles,
            "C2",
            "Rejection wick at major level",
            "primary",
            rejection,
            z,
            args.seed,
        )
    )

    relative = relative_strength_events(bundles)
    relative_events = [
        (index, bar, direction)
        for index, bundle in enumerate(bundles)
        for bar, direction in relative.get((bundle.instrument, bundle.fold), [])
    ]
    candidates.append(
        evaluate(
            bundles,
            "C3",
            "Relative-strength cross",
            "primary",
            relative_events,
            z,
            args.seed,
        )
    )

    gaps = [
        (index, bar, direction)
        for index, bundle in enumerate(bundles)
        if bundle.instrument == "XAUUSD"
        for bar, direction in gap_events(bundle)
    ]
    diagnostic = evaluate(
        bundles, "D1", "Gap continuation (XAU only)", "diagnostic", gaps, z, args.seed
    )

    for row, event_list in zip(
        candidates + [diagnostic],
        [momentum, rejection, relative_events, gaps],
    ):
        total = len(event_list)
        row["raw_events"] = total
        if row["kind"] == "diagnostic":
            row["verdict"] = (
                "inconclusive — underpowered"
                if row["n"] < MIN_EVENTS
                else (
                    "criteria met (single-instrument diagnostic)"
                    if row["criteria_met"]
                    else "no edge"
                )
            )
        else:
            row["verdict"] = "pass" if row["passed"] else "no edge"

    audit = volume_audit(bundles)
    admissible = sorted(
        {
            row["instrument"]
            for row in audit
            if row["admissible"]
        }
    )
    volume_status = (
        "blocked — fewer than two instruments have comparable volume"
        if len(admissible) < 2
        else "admissible"
    )

    primary_passed = [row for row in candidates if row["passed"]]
    if primary_passed:
        decision = (
            "Candidate: "
            + ", ".join(f"`{row['id']}`" for row in primary_passed)
            + ". Write a Phase 2 model charter, implement the event, freeze it, "
            "and run the one-shot confirmatory gate on `model_validation`. "
            "This is exploratory evidence, not an accepted baseline."
        )
    else:
        decision = (
            "No candidate passes; the price-only information surface records "
            "**no edge**. The volume family is untested because the data is not "
            "admissible. Stop Phase 2; do not tune the candidates."
        )

    document = {
        "schema": SCHEMA,
        "partition": "model_dev",
        "config": args.config,
        "horizon_bars": HORIZON,
        "target_atr": TARGET_ATR,
        "stop_atr": STOP_ATR,
        "back_lag": BACK_LAG,
        "block_bars": BLOCK,
        "seed": args.seed,
        "primary_candidates": PRIMARY_CANDIDATES,
        "volume_audit": audit,
        "volume_admissible": admissible,
        "volume_status": volume_status,
        "candidates": candidates,
        "diagnostics": [diagnostic],
        "decision": decision,
    }
    out = Path(args.out).expanduser()
    out.mkdir(parents=True, exist_ok=True)
    (out / "results.json").write_text(
        json.dumps(document, indent=2) + "\n", encoding="utf-8"
    )
    (out / "results.md").write_text(markdown({**document, "candidates": candidates + [diagnostic]}), encoding="utf-8")
    print(f"ok v5 phase 2 spike -> {out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
