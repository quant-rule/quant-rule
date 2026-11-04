#!/usr/bin/env python3
"""Read-only session/liquidity spike on model_dev.

The spike uses the v4 model pack's 15m bars and ATR. It tests three fixed UTC
session events — opening-range breakout, opening-range fade, and New York close
fade — against the same session's base rate, with random, back-lag, and
direction-shuffle controls.

The signal rules are deterministic arithmetic on OHLCV. No machine learning,
neural network, fitted model, or statistic enters the event or the barrier.

Usage:
    python3 tools/improvement/session_spike.py \
        --matrix config/model-v4/dev/matrix.json \
        --config adx25 \
        --out data/improvement/2026-11-04/SESSION-LIQUIDITY-SPIKE \
        --stage all
"""

from __future__ import annotations

import argparse
import csv
import json
import math
import random
import statistics
import sys
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import path_validation

SCHEMA = "quantrule.session-spike/v1"
HORIZON = 20
TARGET_ATR = 2.0
STOP_ATR = 1.0
MIN_EVENTS = 200
MIN_CLUSTERS = 20
MIN_LIFT = 0.05
RANDOM_DRAWS = 200
SHUFFLE_DRAWS = 200
CONTROL_ALPHA = 0.05
LONDON_START = 7 * 3600
NEW_YORK_START = 13 * 3600 + 1800
NEW_YORK_CLOSE_START = 20 * 3600 + 1800
SESSION_END = 21 * 3600
INSTRUMENTS = ("XAUUSD", "BTCUSD", "ETHUSD")
PRIMARY = "XAUUSD"


@dataclass
class Cell:
    instrument: str
    fold: int
    run: Path


@dataclass
class Bundle:
    instrument: str
    fold: int
    rows: list[dict[str, Any]]
    hits: dict[int, list[bool | None]]
    by_timestamp: dict[int, int]
    session_bars: dict[str, list[int]]
    close_bars: list[int]


def read_15m(path: Path) -> list[dict[str, Any]]:
    rows = []
    with path.open(newline="", encoding="utf-8") as handle:
        for row in csv.DictReader(handle):
            if row.get("timeframe_secs") != "900":
                continue
            try:
                timestamp = int(row["timestamp"])
                atr = path_validation.number(row.get("atr", ""))
            except (KeyError, TypeError, ValueError):
                continue
            if atr is None or atr <= 0.0:
                continue
            try:
                rows.append(
                    {
                        "bar": int(row["bar"]),
                        "timestamp": timestamp,
                        "open": float(row["open"]),
                        "high": float(row["high"]),
                        "low": float(row["low"]),
                        "close": float(row["close"]),
                        "volume": path_validation.number(row.get("volume", "")),
                        "atr": atr,
                    }
                )
            except (KeyError, TypeError, ValueError):
                continue
    rows.sort(key=lambda value: value["timestamp"])
    return rows


def session_name(timestamp: int) -> str | None:
    second = timestamp % 86_400
    if NEW_YORK_START <= second < SESSION_END:
        return "newyork"
    if LONDON_START <= second < 16 * 3600:
        return "london"
    return None


def session_start(name: str) -> int:
    return LONDON_START if name == "london" else NEW_YORK_START


def build_bundle(cell: Cell) -> Bundle | None:
    rows = read_15m(cell.run / "readings.csv")
    if not rows:
        return None
    hits = path_validation.barrier_arrays(
        rows, HORIZON, TARGET_ATR, STOP_ATR
    )
    by_timestamp = {row["timestamp"]: index for index, row in enumerate(rows)}
    session_bars: dict[str, list[int]] = {"london": [], "newyork": []}
    close_bars = []
    for index, row in enumerate(rows):
        second = row["timestamp"] % 86_400
        if LONDON_START <= second < 16 * 3600:
            session_bars["london"].append(index)
        if NEW_YORK_START <= second < SESSION_END:
            session_bars["newyork"].append(index)
        if NEW_YORK_CLOSE_START <= second < SESSION_END:
            close_bars.append(index)
    return Bundle(
        instrument=cell.instrument,
        fold=cell.fold,
        rows=rows,
        hits=hits,
        by_timestamp=by_timestamp,
        session_bars=session_bars,
        close_bars=close_bars,
    )


def next_bar(bundle: Bundle, timestamp: int) -> int | None:
    return bundle.by_timestamp.get(timestamp + 900)


def opening_range(
    bundle: Bundle, name: str, day: int
) -> tuple[int, int, float, float] | None:
    start = session_start(name)
    first = day * 86_400 + start
    second = first + 900
    a = bundle.by_timestamp.get(first)
    b = bundle.by_timestamp.get(second)
    if a is None or b is None:
        return None
    return a, b, max(bundle.rows[a]["high"], bundle.rows[b]["high"]), min(
        bundle.rows[a]["low"], bundle.rows[b]["low"]
    )


def orb_events(bundle: Bundle, name: str) -> list[dict[str, Any]]:
    events = []
    start = session_start(name)
    days = sorted({row["timestamp"] // 86_400 for row in bundle.rows})
    for day in days:
        opening = opening_range(bundle, name, day)
        if opening is None:
            continue
        _a, b, high, low = opening
        trigger_start = day * 86_400 + start + 1_800
        trigger_end = trigger_start + 8 * 900
        for index in bundle.session_bars[name]:
            row = bundle.rows[index]
            if row["timestamp"] < trigger_start or row["timestamp"] >= trigger_end:
                continue
            direction = 1 if row["close"] > high else -1 if row["close"] < low else 0
            if direction == 0:
                continue
            entry = next_bar(bundle, row["timestamp"])
            if entry is None:
                break
            events.append(
                {
                    "event": "orb",
                    "session": name,
                    "event_bar": index,
                    "entry": entry,
                    "direction": direction,
                    "base": "session",
                }
            )
            break
    return events


def orf_events(bundle: Bundle, name: str) -> list[dict[str, Any]]:
    events = []
    start = session_start(name)
    days = sorted({row["timestamp"] // 86_400 for row in bundle.rows})
    for day in days:
        opening = opening_range(bundle, name, day)
        if opening is None:
            continue
        a, b, _high, _low = opening
        open_price = bundle.rows[a]["open"]
        close_price = bundle.rows[b]["close"]
        atr = bundle.rows[b]["atr"]
        move = close_price - open_price
        if abs(move) < 1.0 * atr:
            continue
        entry = next_bar(bundle, bundle.rows[b]["timestamp"])
        if entry is None:
            continue
        events.append(
            {
                "event": "orf",
                "session": name,
                "event_bar": b,
                "entry": entry,
                "direction": -1 if move > 0 else 1,
                "base": "session",
            }
        )
    return events


def ncf_events(bundle: Bundle) -> list[dict[str, Any]]:
    events = []
    days = sorted({row["timestamp"] // 86_400 for row in bundle.rows})
    for day in days:
        open_ts = day * 86_400 + NEW_YORK_START
        last_ts = day * 86_400 + NEW_YORK_CLOSE_START + 900
        open_index = bundle.by_timestamp.get(open_ts)
        last_index = bundle.by_timestamp.get(last_ts)
        if open_index is None or last_index is None:
            continue
        move = bundle.rows[last_index]["close"] - bundle.rows[open_index]["open"]
        atr = bundle.rows[last_index]["atr"]
        if abs(move) < 1.0 * atr:
            continue
        entry = next_bar(bundle, bundle.rows[last_index]["timestamp"])
        if entry is None:
            continue
        events.append(
            {
                "event": "ncf",
                "session": "newyork",
                "event_bar": last_index,
                "entry": entry,
                "direction": -1 if move > 0 else 1,
                "base": "close",
            }
        )
    return events


def base_rate(
    bundle: Bundle, direction: int, base: str, session: str
) -> float | None:
    if base == "session":
        indices = bundle.session_bars[session]
    else:
        indices = bundle.close_bars
    hits = 0
    valid = 0
    for index in indices:
        hit = bundle.hits[direction][index]
        if hit is not None:
            hits += int(hit)
            valid += 1
    return hits / valid if valid else None


def evaluate_cell(
    bundles: list[Bundle], event: str, instrument: str
) -> dict[str, Any] | None:
    entries = []
    for bundle_index, bundle in enumerate(bundles):
        for event_record in bundle_events(bundle, event):
            index = event_record["entry"]
            direction = event_record["direction"]
            hit = bundle.hits[direction][index]
            if hit is None:
                continue
            entries.append(
                {
                    "bundle": bundle_index,
                    "bundle_obj": bundle,
                    "entry": index,
                    "direction": direction,
                    "hit": bool(hit),
                    "session": event_record["session"],
                    "base": event_record["base"],
                }
            )
    if not entries:
        return None
    stats = path_validation.cluster_stats(
        [(entry["bundle"], entry["entry"], entry["hit"]) for entry in entries],
        HORIZON,
    )
    p = stats["hit_rate"]
    se = stats["se"]
    z = path_validation.primary_z(3)
    base_rates = []
    for entry in entries:
        rate = base_rate(
            entry["bundle_obj"],
            entry["direction"],
            entry["base"],
            entry["session"],
        )
        if rate is not None:
            base_rates.append(rate)
    base = sum(base_rates) / len(base_rates) if base_rates else None
    lift = p - base if p is not None and base is not None else None
    lcb = path_validation.lower(lift, se, z)

    rng = random.Random(0)
    random_draws = []
    for _ in range(RANDOM_DRAWS):
        hits = 0
        total = 0
        for entry in entries:
            bundle = entry["bundle_obj"]
            if entry["base"] == "session":
                pool = bundle.session_bars[entry["session"]]
            else:
                pool = bundle.close_bars
            pool = [
                index
                for index in pool
                if index != entry["entry"] and bundle.hits[entry["direction"]][index] is not None
            ]
            if not pool:
                continue
            index = rng.choice(pool)
            hit = bundle.hits[entry["direction"]][index]
            if hit is not None:
                hits += int(hit)
                total += 1
        random_draws.append(hits / total if total else None)
    random_p = path_validation.control_p(p, random_draws) if p is not None else None

    rng = random.Random(1)
    shuffle_draws = []
    directions = [entry["direction"] for entry in entries]
    for _ in range(SHUFFLE_DRAWS):
        rng.shuffle(directions)
        hits = 0
        total = 0
        for entry, direction in zip(entries, directions):
            hit = entry["bundle_obj"].hits[direction][entry["entry"]]
            if hit is not None:
                hits += int(hit)
                total += 1
        shuffle_draws.append(hits / total if total else None)
    shuffle_p = path_validation.control_p(p, shuffle_draws) if p is not None else None

    back_hits = 0
    back_total = 0
    for entry in entries:
        lagged = entry["entry"] - HORIZON
        if lagged < 0:
            continue
        hit = entry["bundle_obj"].hits[entry["direction"]][lagged]
        if hit is not None:
            back_hits += int(hit)
            back_total += 1
    back_rate = back_hits / back_total if back_total else None
    back_lift = back_rate - base if back_rate is not None and base is not None else None

    instruments = {}
    by_instrument: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for entry in entries:
        by_instrument[entry["bundle_obj"].instrument].append(entry)
    for name, values in sorted(by_instrument.items()):
        hits = sum(1 for value in values if value["hit"])
        rates = []
        for value in values:
            rate = base_rate(
                value["bundle_obj"], value["direction"], value["base"], value["session"]
            )
            if rate is not None:
                rates.append(rate)
        instrument_base = sum(rates) / len(rates) if rates else None
        instruments[name] = {
            "n": len(values),
            "hits": hits,
            "hit_rate": hits / len(values) if values else None,
            "base": instrument_base,
        }

    checks = [
        ("n", stats["n"] >= MIN_EVENTS),
        ("clusters", stats["clusters"] >= MIN_CLUSTERS),
        ("lift", lift is not None and lift >= MIN_LIFT),
        ("lift_lcb", lcb is not None and lcb > 0.0),
        ("random", random_p is not None and random_p <= CONTROL_ALPHA),
        (
            "back_lag",
            back_lift is not None
            and (
                back_lift <= 0.01
                or (lift is not None and lift - back_lift >= 0.01)
            ),
        ),
        ("shuffle", shuffle_p is not None and shuffle_p <= CONTROL_ALPHA),
    ]
    reasons = [name for name, ok in checks if not ok]
    grade = "C" if reasons else ("A" if lift >= 0.05 else "B")
    return {
        "event": event,
        "instrument": instrument,
        "n": stats["n"],
        "clusters": stats["clusters"],
        "hit_rate": p,
        "base_rate": base,
        "lift": lift,
        "lift_lcb": lcb,
        "random_p": random_p,
        "back_lag_lift": back_lift,
        "shuffle_p": shuffle_p,
        "grade": grade,
        "reasons": reasons,
        "instruments": instruments,
    }


def bundle_events(bundle: Bundle, event: str) -> list[dict[str, Any]]:
    if event == "orb":
        return orb_events(bundle, "london") + orb_events(bundle, "newyork")
    if event == "orf":
        return orf_events(bundle, "london") + orf_events(bundle, "newyork")
    if event == "ncf":
        return ncf_events(bundle)
    raise ValueError(f"unknown event: {event}")


def profile(bundles: list[Bundle]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for instrument in INSTRUMENTS:
        by_hour = {}
        selected = [bundle for bundle in bundles if bundle.instrument == instrument]
        for hour in range(24):
            ranges = []
            atrs = []
            volumes = []
            long_hits = []
            short_hits = []
            for bundle in selected:
                for index, row in enumerate(bundle.rows):
                    if (row["timestamp"] % 86_400) // 3600 != hour:
                        continue
                    ranges.append(row["high"] - row["low"])
                    atrs.append(row["atr"])
                    if row["volume"] is not None:
                        volumes.append(row["volume"])
                    for direction, values in ((1, long_hits), (-1, short_hits)):
                        hit = bundle.hits[direction][index]
                        if hit is not None:
                            values.append(int(hit))
            by_hour[str(hour)] = {
                "bars": len(ranges),
                "range_median": statistics.median(ranges) if ranges else None,
                "atr_median": statistics.median(atrs) if atrs else None,
                "volume_median": statistics.median(volumes) if volumes else None,
                "long_hit_rate": (
                    sum(long_hits) / len(long_hits) if long_hits else None
                ),
                "short_hit_rate": (
                    sum(short_hits) / len(short_hits) if short_hits else None
                ),
            }
        result[instrument] = by_hour
    return result


def profile_markdown(document: dict[str, Any]) -> str:
    lines = ["# Session Profile (model_dev 2018–2020)", ""]
    for instrument in INSTRUMENTS:
        lines += [
            f"## {instrument}",
            "",
            "| Hour UTC | Bars | Median range | Median ATR | Median volume | Long hit | Short hit |",
            "|---:|---:|---:|---:|---:|---:|---:|",
        ]
        for hour in range(24):
            row = document[instrument][str(hour)]
            lines.append(
                "| {hour} | {bars} | {rng} | {atr} | {vol} | {long} | {short} |".format(
                    hour=hour,
                    bars=row["bars"],
                    rng=fmt(row["range_median"]),
                    atr=fmt(row["atr_median"]),
                    vol=fmt(row["volume_median"]),
                    long=fmt(row["long_hit_rate"]),
                    short=fmt(row["short_hit_rate"]),
                )
            )
        lines.append("")
    return "\n".join(lines) + "\n"


def fmt(value: Any) -> str:
    if value is None:
        return "-"
    if isinstance(value, float):
        return f"{value:.4f}"
    return str(value)


def results_markdown(document: dict[str, Any]) -> str:
    lines = [
        "# Session Spike Results (model_dev 2018–2020)",
        "",
        "Primary: XAUUSD. Secondary: BTCUSD, ETHUSD.",
        "",
        "| Event | Instrument | n | Clusters | Hit | Base | Lift | Lift LCB | Random p | Back lag | Shuffle p | Grade |",
        "|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|",
    ]
    for row in document["cells"]:
        lines.append(
            "| {event} | {instrument} | {n} | {clusters} | {hit} | {base} | {lift} | {lcb} | {random} | {back} | {shuffle} | {grade} |".format(
                event=row["event"],
                instrument=row["instrument"],
                n=row["n"],
                clusters=row["clusters"],
                hit=fmt(row["hit_rate"]),
                base=fmt(row["base_rate"]),
                lift=fmt(row["lift"]),
                lcb=fmt(row["lift_lcb"]),
                random=fmt(row["random_p"]),
                back=fmt(row["back_lag_lift"]),
                shuffle=fmt(row["shuffle_p"]),
                grade=row["grade"],
            )
        )
    lines += ["", f"- Primary pass: {document['primary_pass']}", f"- Verdict: {document['verdict']}"]
    return "\n".join(lines) + "\n"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--matrix", required=True)
    parser.add_argument("--config", default="adx25")
    parser.add_argument("--out", required=True)
    parser.add_argument("--stage", choices=("profile", "barrier", "all"), default="all")
    args = parser.parse_args(argv)
    matrix = json.loads(Path(args.matrix).read_text(encoding="utf-8"))
    cells = path_validation.expand_cells(matrix, args.config)
    if not cells:
        print("error: matrix has no cells", file=sys.stderr)
        return 1
    bundles = []
    for cell in cells:
        bundle = build_bundle(
            Cell(
                instrument=str(cell["instrument"]),
                fold=int(cell["fold"]),
                run=Path(str(cell["run"])).expanduser(),
            )
        )
        if bundle is not None:
            bundles.append(bundle)
    if not bundles:
        print("error: no 15m rows", file=sys.stderr)
        return 1
    out = Path(args.out).expanduser()
    out.mkdir(parents=True, exist_ok=True)
    if args.stage in ("profile", "all"):
        profile_document = profile(bundles)
        (out / "profile.json").write_text(
            json.dumps(profile_document, indent=2) + "\n", encoding="utf-8"
        )
        (out / "profile.md").write_text(
            profile_markdown(profile_document), encoding="utf-8"
        )
    if args.stage in ("barrier", "all"):
        rows = []
        for event in ("orb", "orf", "ncf"):
            for instrument in INSTRUMENTS:
                selected = [bundle for bundle in bundles if bundle.instrument == instrument]
                metrics = evaluate_cell(selected, event, instrument)
                if metrics is not None:
                    rows.append(metrics)
        primary = [
            row
            for row in rows
            if row["instrument"] == PRIMARY
        ]
        passing = [row for row in primary if not row["reasons"]]
        primary_pass = bool(passing)
        underpowered = bool(primary) and all(row["n"] < MIN_EVENTS for row in primary)
        verdict = (
            "candidate" if primary_pass else "inconclusive — underpowered" if underpowered else "no edge"
        )
        document = {
            "schema": SCHEMA,
            "partition": "model_dev",
            "primary": PRIMARY,
            "cells": rows,
            "primary_pass": primary_pass,
            "passing_primary_cells": [row["event"] for row in passing],
            "underpowered": underpowered,
            "verdict": verdict,
        }
        (out / "results.json").write_text(
            json.dumps(document, indent=2) + "\n", encoding="utf-8"
        )
        (out / "results.md").write_text(results_markdown(document), encoding="utf-8")
        print(f"ok session spike {verdict} -> {out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
