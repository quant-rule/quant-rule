#!/usr/bin/env python3
"""Read-only EURUSD session spike with a DST-aware London/New York calendar.

The spike uses the v4 model pack's 15m bars and ATR. It tests the London
opening range breakout, the New York opening range breakout, and the New York
close fade against the same session's base rate, with random, back-lag, and
direction-shuffle controls.

The signal rules are deterministic arithmetic on OHLCV. No machine learning,
neural network, fitted model, or statistic enters the event or the barrier.

Usage:
    python3 tools/improvement/eurusd_session_spike.py \
        --matrix config/session-spike/eurusd/matrix.json \
        --config adx25 \
        --out data/improvement/2026-11-04/EURUSD-SESSION-SPIKE \
        --stage all
"""

from __future__ import annotations

import argparse
import csv
import json
import random
import statistics
import sys
from collections import defaultdict
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any
from zoneinfo import ZoneInfo

import path_validation

SCHEMA = "quantrule.eurusd-session-spike/v1"
HORIZON = 20
TARGET_ATR = 2.0
STOP_ATR = 1.0
MIN_EVENTS = 200
MIN_CLUSTERS = 20
MIN_LIFT = 0.05
RANDOM_DRAWS = 200
SHUFFLE_DRAWS = 200
CONTROL_ALPHA = 0.05
LONDON = ZoneInfo("Europe/London")
NEW_YORK = ZoneInfo("America/New_York")


@dataclass
class Bundle:
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


def local_second(timestamp: int, zone: ZoneInfo) -> int:
    local = datetime.fromtimestamp(timestamp, tz=timezone.utc).astimezone(zone)
    return local.hour * 3600 + local.minute * 60 + local.second


def in_london(timestamp: int) -> bool:
    return 8 * 3600 <= local_second(timestamp, LONDON) < 16 * 3600 + 1800


def in_new_york(timestamp: int) -> bool:
    return 9 * 3600 + 1800 <= local_second(timestamp, NEW_YORK) < 16 * 3600


def in_ny_close(timestamp: int) -> bool:
    return 15 * 3600 + 1800 <= local_second(timestamp, NEW_YORK) < 16 * 3600


def local_day(timestamp: int, zone: ZoneInfo) -> int:
    local = datetime.fromtimestamp(timestamp, tz=timezone.utc).astimezone(zone)
    return local.toordinal()


def build_bundle(fold: int, run: Path) -> Bundle | None:
    rows = read_15m(run / "readings.csv")
    if not rows:
        return None
    hits = path_validation.barrier_arrays(rows, HORIZON, TARGET_ATR, STOP_ATR)
    by_timestamp = {row["timestamp"]: index for index, row in enumerate(rows)}
    session_bars: dict[str, list[int]] = {"london": [], "newyork": []}
    close_bars = []
    for index, row in enumerate(rows):
        timestamp = row["timestamp"]
        if in_london(timestamp):
            session_bars["london"].append(index)
        if in_new_york(timestamp):
            session_bars["newyork"].append(index)
        if in_ny_close(timestamp):
            close_bars.append(index)
    return Bundle(
        fold=fold,
        rows=rows,
        hits=hits,
        by_timestamp=by_timestamp,
        session_bars=session_bars,
        close_bars=close_bars,
    )


def grouped_sessions(bundle: Bundle, name: str, zone: ZoneInfo) -> list[list[int]]:
    groups: dict[int, list[int]] = defaultdict(list)
    for index in bundle.session_bars[name]:
        groups[local_day(bundle.rows[index]["timestamp"], zone)].append(index)
    return [groups[key] for key in sorted(groups)]


def next_bar(bundle: Bundle, timestamp: int) -> int | None:
    return bundle.by_timestamp.get(timestamp + 900)


def orb_events(bundle: Bundle, name: str, zone: ZoneInfo) -> list[dict[str, Any]]:
    events = []
    for indices in grouped_sessions(bundle, name, zone):
        if len(indices) < 3:
            continue
        a, b = indices[0], indices[1]
        high = max(bundle.rows[a]["high"], bundle.rows[b]["high"])
        low = min(bundle.rows[a]["low"], bundle.rows[b]["low"])
        for index in indices[2:10]:
            row = bundle.rows[index]
            direction = 1 if row["close"] > high else -1 if row["close"] < low else 0
            if direction == 0:
                continue
            entry = next_bar(bundle, row["timestamp"])
            if entry is None:
                break
            events.append(
                {
                    "event": f"{name}_orb",
                    "session": name,
                    "event_bar": index,
                    "entry": entry,
                    "direction": direction,
                    "base": "session",
                }
            )
            break
    return events


def ncf_events(bundle: Bundle) -> list[dict[str, Any]]:
    events = []
    for indices in grouped_sessions(bundle, "newyork", NEW_YORK):
        if len(indices) < 2:
            continue
        first = indices[0]
        last = indices[-1]
        move = bundle.rows[last]["close"] - bundle.rows[first]["open"]
        atr = bundle.rows[last]["atr"]
        if abs(move) < 1.0 * atr:
            continue
        entry = next_bar(bundle, bundle.rows[last]["timestamp"])
        if entry is None:
            continue
        events.append(
            {
                "event": "newyork_close_fade",
                "session": "newyork",
                "event_bar": last,
                "entry": entry,
                "direction": -1 if move > 0 else 1,
                "base": "close",
            }
        )
    return events


def bundle_events(bundle: Bundle, event: str) -> list[dict[str, Any]]:
    if event == "london_orb":
        return orb_events(bundle, "london", LONDON)
    if event == "newyork_orb":
        return orb_events(bundle, "newyork", NEW_YORK)
    if event == "newyork_close_fade":
        return ncf_events(bundle)
    raise ValueError(f"unknown event: {event}")


def base_rate(bundle: Bundle, direction: int, base: str, session: str) -> float | None:
    indices = bundle.session_bars[session] if base == "session" else bundle.close_bars
    hits = 0
    valid = 0
    for index in indices:
        hit = bundle.hits[direction][index]
        if hit is not None:
            hits += int(hit)
            valid += 1
    return hits / valid if valid else None


def evaluate_cell(bundles: list[Bundle], event: str) -> dict[str, Any] | None:
    entries = []
    for bundle_index, bundle in enumerate(bundles):
        for record in bundle_events(bundle, event):
            index = record["entry"]
            direction = record["direction"]
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
                    "session": record["session"],
                    "base": record["base"],
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
    rates = []
    for entry in entries:
        rate = base_rate(
            entry["bundle_obj"],
            entry["direction"],
            entry["base"],
            entry["session"],
        )
        if rate is not None:
            rates.append(rate)
    base = sum(rates) / len(rates) if rates else None
    lift = p - base if p is not None and base is not None else None
    lcb = path_validation.lower(lift, se, z)

    rng = random.Random(0)
    random_draws = []
    for _ in range(RANDOM_DRAWS):
        hits = 0
        total = 0
        for entry in entries:
            bundle = entry["bundle_obj"]
            pool = bundle.session_bars[entry["session"]] if entry["base"] == "session" else bundle.close_bars
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
        "instrument": "EURUSD",
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
    }


def profile(bundles: list[Bundle]) -> dict[str, Any]:
    by_hour = {}
    for hour in range(24):
        ranges = []
        atrs = []
        volumes = []
        long_hits = []
        short_hits = []
        for bundle in bundles:
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
            "long_hit_rate": sum(long_hits) / len(long_hits) if long_hits else None,
            "short_hit_rate": sum(short_hits) / len(short_hits) if short_hits else None,
        }
    return {"EURUSD": by_hour}


def fmt(value: Any) -> str:
    if value is None:
        return "-"
    if isinstance(value, float):
        return f"{value:.4f}"
    return str(value)


def profile_markdown(document: dict[str, Any]) -> str:
    lines = [
        "# EURUSD UTC Hour Profile (model_dev 2018–2020)",
        "",
        "| Hour UTC | Bars | Median range | Median ATR | Median volume | Long hit | Short hit |",
        "|---:|---:|---:|---:|---:|---:|---:|",
    ]
    for hour in range(24):
        row = document["EURUSD"][str(hour)]
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
    return "\n".join(lines) + "\n"


def results_markdown(document: dict[str, Any]) -> str:
    lines = [
        "# EURUSD Session Spike Results (model_dev 2018–2020)",
        "",
        "| Event | n | Clusters | Hit | Base | Lift | Lift LCB | Random p | Back lag | Shuffle p | Grade |",
        "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|",
    ]
    for row in document["cells"]:
        lines.append(
            "| {event} | {n} | {clusters} | {hit} | {base} | {lift} | {lcb} | {random} | {back} | {shuffle} | {grade} |".format(
                event=row["event"],
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
        bundle = build_bundle(int(cell["fold"]), Path(str(cell["run"])).expanduser())
        if bundle is not None:
            bundles.append(bundle)
    if not bundles:
        print("error: no EURUSD 15m rows", file=sys.stderr)
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
        for event in ("london_orb", "newyork_orb", "newyork_close_fade"):
            metrics = evaluate_cell(bundles, event)
            if metrics is not None:
                rows.append(metrics)
        passing = [row for row in rows if not row["reasons"]]
        primary_pass = bool(passing)
        underpowered = bool(rows) and all(row["n"] < MIN_EVENTS for row in rows)
        verdict = (
            "candidate" if primary_pass else "inconclusive — underpowered" if underpowered else "no edge"
        )
        document = {
            "schema": SCHEMA,
            "partition": "model_dev",
            "instrument": "EURUSD",
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
        print(f"ok EURUSD session spike {verdict} -> {out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
