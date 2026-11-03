#!/usr/bin/env python3
"""COPPERCMD 1h confirmed-pivot Level path pilot.

Primary question: after price tests a confirmed pivot level, does the next
20-bar path reach a 1 ATR target before a 1 ATR stop, in the event direction,
more often than the unconditional barrier rate?

The pilot is prediction-only. It does not model costs, execution, or a strategy.
Development and validation are reported separately; validation is not used for
tuning.

The path protocol is pre-registered in
`docs/model/2026-11-03/path-validation.md`:

- Level test events only, confirmed pivot sources, reference codes >= 1001.
- One fresh test per reference: the first bar of a run of consecutive tests.
- One vote per bar; a bar with conflicting directions is dropped.
- Entry at the event close, ATR at the event bar.
- Target = entry + direction * 1.0 ATR; stop = entry - direction * 1.0 ATR.
- Horizon = 20 future bars; same-bar target and stop ties count as a stop.
- The event bar's own high and low are not used.
- The baseline is the same barrier at every valid bar, per year and direction.
- The event bound is cluster-robust because 20-bar paths overlap.

Usage:
    python3 tools/improvement/path_pilot.py \
        --dev data/path-pilot/copper-2018/model \
              data/path-pilot/copper-2019/model \
              data/path-pilot/copper-2020/model \
        --validation data/path-pilot/copper-2021/model \
                    data/path-pilot/copper-2022/model \
        --out data/path-pilot/PATH
"""

from __future__ import annotations

import argparse
import csv
import json
import math
import sys
from collections import defaultdict
from pathlib import Path
from typing import Any

TIMEFRAME_SECS = 3600
TARGET_ATR = 1.0
STOP_ATR = 1.0
HORIZON = 20
Z_95 = 1.959963984540054
MIN_EVENTS = 200
MIN_CLUSTERS = 20
MIN_LIFT = 0.02
PIVOT_REFERENCE_MIN = 1001
PIVOT_SOURCES = ("7", "8")
SUBTYPES = {
    "test": "1",
    "cross": "2",
    "break": "3",
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
                rows.append(
                    {
                        "bar": int(row["bar"]),
                        "timestamp": int(row["timestamp"]),
                        "high": float(row["high"]),
                        "low": float(row["low"]),
                        "close": float(row["close"]),
                        "atr": optional_float(row.get("atr")),
                    }
                )
            except (KeyError, TypeError, ValueError):
                continue
    rows.sort(key=lambda row: row["bar"])
    return rows


def read_events(path: Path) -> list[dict[str, Any]]:
    events: list[dict[str, Any]] = []
    with path.open(newline="", encoding="utf-8") as handle:
        for row in csv.DictReader(handle):
            if row.get("timeframe_secs") != str(TIMEFRAME_SECS):
                continue
            if row.get("event_type_code") != "1":
                continue
            subtype = row.get("event_subtype_code")
            if subtype not in SUBTYPES.values():
                continue
            if row.get("source_code") not in PIVOT_SOURCES:
                continue
            reference = optional_int(row.get("reference_code"))
            if reference is None or reference < PIVOT_REFERENCE_MIN:
                continue
            direction = optional_int(row.get("direction_code"))
            if direction not in (-1, 1):
                continue
            completeness = optional_float(row.get("data_completeness"))
            if completeness is None or completeness < 1.0:
                continue
            bar = optional_int(row.get("available_bar_index"))
            if bar is None or bar < 0:
                continue
            events.append(
                {
                    "bar": bar,
                    "subtype": subtype,
                    "source": row["source_code"],
                    "reference": reference,
                    "direction": direction,
                    "event_index": optional_int(row.get("event_index")) or 0,
                }
            )
    events.sort(key=lambda event: (event["bar"], event["event_index"], event["reference"]))
    return events


def prepare_subtype(
    events: list[dict[str, Any]], subtype: str
) -> tuple[list[dict[str, Any]], list[dict[str, Any]], int]:
    subset = sorted(
        (event for event in events if event["subtype"] == subtype),
        key=lambda event: (event["bar"], event["event_index"], event["reference"]),
    )
    fresh: list[dict[str, Any]] = []
    last_raw_bar: dict[int, int] = {}
    for event in subset:
        previous = last_raw_bar.get(event["reference"])
        if previous is None or event["bar"] > previous + 1:
            fresh.append(event)
        last_raw_bar[event["reference"]] = event["bar"]

    by_bar: dict[int, list[dict[str, Any]]] = defaultdict(list)
    for event in fresh:
        by_bar[event["bar"]].append(event)
    prepared: list[dict[str, Any]] = []
    conflicts = 0
    for bar in sorted(by_bar):
        directions = {event["direction"] for event in by_bar[bar]}
        if len(directions) != 1:
            conflicts += 1
            continue
        prepared.append(
            min(
                by_bar[bar],
                key=lambda event: (event["event_index"], event["reference"]),
            )
        )
    return fresh, prepared, conflicts


def optional_int(value: Any) -> int | None:
    if value is None or value == "":
        return None
    try:
        return int(value)
    except (TypeError, ValueError):
        return None


def optional_float(value: Any) -> float | None:
    if value is None or value == "":
        return None
    try:
        return float(value)
    except (TypeError, ValueError):
        return None


def path_outcome(
    rows: list[dict[str, Any]],
    bar: int,
    direction: int,
    target_atr: float = TARGET_ATR,
    stop_atr: float = STOP_ATR,
    horizon: int = HORIZON,
) -> dict[str, Any]:
    if direction not in (-1, 1):
        return {"status": "invalid_direction"}
    if bar < 0 or bar >= len(rows):
        return {"status": "invalid_bar"}
    entry = rows[bar]["close"]
    atr = rows[bar]["atr"]
    if atr is None or atr <= 0.0:
        return {"status": "invalid_atr"}
    if bar + horizon >= len(rows):
        return {"status": "short_history"}

    target = entry + direction * target_atr * atr
    stop = entry - direction * stop_atr * atr
    for offset in range(1, horizon + 1):
        candle = rows[bar + offset]
        high = candle["high"]
        low = candle["low"]
        if direction == 1:
            hit_target = high >= target
            hit_stop = low <= stop
        else:
            hit_target = low <= target
            hit_stop = high >= stop
        if hit_target and hit_stop:
            return {"status": "stop", "bars": offset}
        if hit_stop:
            return {"status": "stop", "bars": offset}
        if hit_target:
            return {"status": "target", "bars": offset}
    return {"status": "timeout", "bars": horizon}


def base_rates(rows: list[dict[str, Any]]) -> dict[str, Any]:
    total = 0
    long_hits = 0
    short_hits = 0
    for bar in range(len(rows) - HORIZON):
        if rows[bar]["atr"] is None or rows[bar]["atr"] <= 0.0:
            continue
        long_outcome = path_outcome(rows, bar, 1)
        short_outcome = path_outcome(rows, bar, -1)
        if long_outcome["status"] in ("invalid_atr", "short_history"):
            continue
        total += 1
        if long_outcome["status"] == "target":
            long_hits += 1
        if short_outcome["status"] == "target":
            short_hits += 1
    if total == 0:
        return {"n": 0, "long": 0.0, "short": 0.0}
    return {
        "n": total,
        "long": long_hits / total,
        "short": short_hits / total,
    }


def clusters_from_events(events: list[tuple[int, int]]) -> list[tuple[int, int]]:
    clusters: list[tuple[int, int]] = []
    current_n = 0
    current_hits = 0
    previous_bar: int | None = None
    for bar, hit in sorted(events):
        # A path covers bar+1..bar+HORIZON. An event at exactly bar+HORIZON
        # starts after the previous path ends, so it is a new cluster.
        if previous_bar is not None and bar - previous_bar >= HORIZON:
            if current_n:
                clusters.append((current_n, current_hits))
            current_n = 0
            current_hits = 0
        current_n += 1
        current_hits += hit
        previous_bar = bar
    if current_n:
        clusters.append((current_n, current_hits))
    return clusters


def cluster_statistics(clusters: list[tuple[int, int]]) -> dict[str, Any]:
    n = sum(cluster_n for cluster_n, _ in clusters)
    hits = sum(cluster_hits for _, cluster_hits in clusters)
    clusters_count = len(clusters)
    if n == 0 or clusters_count == 0:
        return {
            "n": 0,
            "clusters": 0,
            "hits": 0,
            "hit_rate": None,
            "naive_se": None,
            "cluster_se": None,
        }
    p = hits / n
    naive_se = math.sqrt(max(p * (1.0 - p) / n, 0.0))
    if clusters_count <= 1:
        cluster_se = None
    else:
        total = 0.0
        for cluster_n, cluster_hits in clusters:
            total += (cluster_hits - p * cluster_n) ** 2
        variance = (clusters_count / (clusters_count - 1)) * total / (n * n)
        cluster_se = math.sqrt(max(variance, 0.0))
    return {
        "n": n,
        "clusters": clusters_count,
        "hits": hits,
        "hit_rate": p,
        "naive_se": naive_se,
        "cluster_se": cluster_se,
    }


def summarize_bundles(dirs: list[str]) -> dict[str, Any]:
    cells = {
        key: {
            "raw": 0,
            "fresh": 0,
            "prepared": 0,
            "conflicts": 0,
            "dropped_atr": 0,
            "dropped_history": 0,
            "long_n": 0,
            "short_n": 0,
            "base_sum": 0.0,
            "cluster_list": [],
        }
        for key in SUBTYPES
    }
    breakdown: dict[str, dict[str, Any]] = {
        f"{source}/{'long' if direction == 1 else 'short'}": {
            "n": 0,
            "hits": 0,
            "base_sum": 0.0,
        }
        for source in PIVOT_SOURCES
        for direction in (-1, 1)
    }
    years: dict[str, Any] = {}
    for directory in dirs:
        root = Path(directory).expanduser()
        rows = read_readings(root / "readings.csv")
        events = read_events(root / "events.csv")
        bases = base_rates(rows)
        if bases["n"] == 0:
            continue
        years[str(root)] = {
            "bars": bases["n"],
            "base_long": bases["long"],
            "base_short": bases["short"],
        }
        for key, subtype in SUBTYPES.items():
            cell = cells[key]
            cell["raw"] += sum(1 for event in events if event["subtype"] == subtype)
            fresh, prepared, conflicts = prepare_subtype(events, subtype)
            cell["fresh"] += len(fresh)
            cell["prepared"] += len(prepared)
            cell["conflicts"] += conflicts
            bundle_events: list[tuple[int, int]] = []
            for event in prepared:
                outcome = path_outcome(rows, event["bar"], event["direction"])
                if outcome["status"] == "invalid_atr":
                    cell["dropped_atr"] += 1
                    continue
                if outcome["status"] == "short_history":
                    cell["dropped_history"] += 1
                    continue
                if outcome["status"] not in ("target", "stop", "timeout"):
                    cell["dropped_history"] += 1
                    continue
                hit = 1 if outcome["status"] == "target" else 0
                expected = bases["long"] if event["direction"] == 1 else bases["short"]
                cell["base_sum"] += expected
                bundle_events.append((event["bar"], hit))
                if event["direction"] == 1:
                    cell["long_n"] += 1
                else:
                    cell["short_n"] += 1
                if key == "test":
                    label = f"{event['source']}/{'long' if event['direction'] == 1 else 'short'}"
                    bucket = breakdown[label]
                    bucket["n"] += 1
                    bucket["hits"] += hit
                    bucket["base_sum"] += expected
            cell["cluster_list"].extend(clusters_from_events(bundle_events))

    result = {}
    for key, cell in cells.items():
        stats = cluster_statistics(cell["cluster_list"])
        n = stats["n"]
        if n:
            base = cell["base_sum"] / n
            lift = stats["hit_rate"] - base
            if stats["cluster_se"] is None:
                hit_lcb = None
                lift_lcb = None
            else:
                hit_lcb = stats["hit_rate"] - Z_95 * stats["cluster_se"]
                lift_lcb = lift - Z_95 * stats["cluster_se"]
        else:
            base = None
            lift = None
            hit_lcb = None
            lift_lcb = None
        result[key] = {
            "raw": cell["raw"],
            "fresh": cell["fresh"],
            "prepared": cell["prepared"],
            "conflicts": cell["conflicts"],
            "dropped_atr": cell["dropped_atr"],
            "dropped_history": cell["dropped_history"],
            "n": n,
            "clusters": stats["clusters"],
            "hits": stats["hits"],
            "hit_rate": stats["hit_rate"],
            "base_rate": base,
            "lift": lift,
            "naive_se": stats["naive_se"],
            "cluster_se": stats["cluster_se"],
            "hit_lcb": hit_lcb,
            "lift_lcb": lift_lcb,
            "long_n": cell["long_n"],
            "short_n": cell["short_n"],
            "coverage": (n / cell["raw"]) if cell["raw"] else 0.0,
        }

    for label, bucket in breakdown.items():
        n = bucket["n"]
        bucket["hit_rate"] = bucket["hits"] / n if n else None
        bucket["base_rate"] = bucket["base_sum"] / n if n else None
        bucket["lift"] = (
            bucket["hit_rate"] - bucket["base_rate"]
            if n and bucket["hit_rate"] is not None and bucket["base_rate"] is not None
            else None
        )
    return {"cells": result, "breakdown": breakdown, "years": years}


def passes(cell: dict[str, Any]) -> bool:
    return (
        cell["n"] >= MIN_EVENTS
        and cell["clusters"] >= MIN_CLUSTERS
        and cell["lift"] is not None
        and cell["lift"] >= MIN_LIFT
        and cell["lift_lcb"] is not None
        and cell["lift_lcb"] > 0.0
    )


def decision(dev: dict[str, Any], validation: dict[str, Any]) -> str:
    primary = dev["cells"]["test"]
    check = validation["cells"]["test"]
    if primary["n"] < MIN_EVENTS or primary["clusters"] < MIN_CLUSTERS:
        return "inconclusive"
    if not passes(primary):
        return "stop"
    if check["n"] < MIN_EVENTS or check["clusters"] < MIN_CLUSTERS:
        return "validation_inconclusive"
    if passes(check):
        return "proceed_to_replication"
    return "development_only"


def fmt(value: float | None) -> str:
    return "-" if value is None else f"{value:.4f}"


def markdown(document: dict[str, Any]) -> str:
    lines = [
        "# COPPERCMD Pivot-Level Path Pilot",
        "",
        f"- Decision: **{document['decision']}**",
        "- Primary event: confirmed pivot Level test/retest",
        "- Path: 1.0 ATR target before 1.0 ATR stop within 20 bars",
        "- Entry: event-bar close; stop-first tie-break",
        "",
        "## Primary test",
        "",
        "| Partition | Raw | Fresh | Prepared | Valid n | Clusters | Coverage | Hit | Base | Lift | Naive SE | Cluster SE | Hit LCB | Lift LCB |",
        "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|",
    ]
    for partition in ("dev", "validation"):
        cell = document[partition]["cells"]["test"]
        lines.append(
            "| {p} | {raw} | {fresh} | {prepared} | {n} | {j} | {c:.4f} | {hit} | {base} | {lift} | {ns} | {cs} | {hl} | {ll} |".format(
                p=partition,
                raw=cell["raw"],
                fresh=cell["fresh"],
                prepared=cell["prepared"],
                n=cell["n"],
                j=cell["clusters"],
                c=cell["coverage"],
                hit=fmt(cell["hit_rate"]),
                base=fmt(cell["base_rate"]),
                lift=fmt(cell["lift"]),
                ns=fmt(cell["naive_se"]),
                cs=fmt(cell["cluster_se"]),
                hl=fmt(cell["hit_lcb"]),
                ll=fmt(cell["lift_lcb"]),
            )
        )

    lines.extend(
        [
            "",
            "## Secondary events",
            "",
            "| Partition | Event | Valid n | Clusters | Hit | Base | Lift | Cluster SE | Lift LCB |",
            "|---|---|---:|---:|---:|---:|---:|---:|---:|",
        ]
    )
    for partition in ("dev", "validation"):
        for key in ("cross", "break"):
            cell = document[partition]["cells"][key]
            lines.append(
                "| {p} | {e} | {n} | {j} | {hit} | {base} | {lift} | {cs} | {ll} |".format(
                    p=partition,
                    e=key,
                    n=cell["n"],
                    j=cell["clusters"],
                    hit=fmt(cell["hit_rate"]),
                    base=fmt(cell["base_rate"]),
                    lift=fmt(cell["lift"]),
                    cs=fmt(cell["cluster_se"]),
                    ll=fmt(cell["lift_lcb"]),
                )
            )

    lines.extend(
        [
            "",
            "## Per-year barrier base",
            "",
            "| Bundle | Valid bars | Long base | Short base |",
            "|---|---:|---:|---:|",
        ]
    )
    for partition in ("dev", "validation"):
        for bundle, values in sorted(document[partition]["years"].items()):
            lines.append(
                "| {b} | {n} | {long} | {short} |".format(
                    b=Path(bundle).parent.name,
                    n=values["bars"],
                    long=fmt(values["base_long"]),
                    short=fmt(values["base_short"]),
                )
            )

    lines.extend(
        [
            "",
            "## Descriptive breakdown of the primary test",
            "",
            "| Partition | Source/direction | n | Hit | Base | Lift |",
            "|---|---|---:|---:|---:|---:|",
        ]
    )
    for partition in ("dev", "validation"):
        for label in sorted(document[partition]["breakdown"]):
            bucket = document[partition]["breakdown"][label]
            lines.append(
                "| {p} | {label} | {n} | {hit} | {base} | {lift} |".format(
                    p=partition,
                    label=label,
                    n=bucket["n"],
                    hit=fmt(bucket["hit_rate"]),
                    base=fmt(bucket["base_rate"]),
                    lift=fmt(bucket["lift"]),
                )
            )

    lines.extend(
        [
            "",
            "## Rules",
            "",
            "A hit is the 1.0 ATR target reached before the 1.0 ATR stop within 20 future bars. A same-bar target and stop counts as a stop. A timeout is a miss. The event bar's own high and low are not used.",
            "",
            "The base rate is the same long or short barrier at every valid bar in the same year. The event set uses the event-weighted per-year base.",
            "",
            "The primary pass needs `n >= 200`, `J >= 20`, point lift `>= +0.02`, and lift LCB `> 0.00` on both partitions. The LCB uses the cluster-robust standard error; the naive error is reported for comparison.",
            "",
            "`proceed_to_replication` is not a strategy go. A pass must replicate on a second fresh asset or timeframe.",
            "",
            "The event is fresh when it is the first test for its reference after at least one bar without a test. A bar with conflicting fresh directions is dropped. The 20-bar path includes gaps.",
            "",
            "`raw` is the filtered event count, `fresh` is after the consecutive-bar rule, `prepared` is after the one-vote-per-bar rule, and `valid n` is after the ATR and 20-bar history checks.",
            "",
        ]
    )
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    args = parse_arguments(argv)
    dev = summarize_bundles(args.dev)
    validation = summarize_bundles(args.validation)
    result = {
        "schema": "quantrule.path-pilot/v1",
        "question": (
            "Does a confirmed pivot Level test reach a 1 ATR target before a "
            "1 ATR stop within 20 bars more often than the unconditional "
            "barrier rate on COPPERCMD 1h?"
        ),
        "asset": "COPPERCMDUSD",
        "timeframe_secs": TIMEFRAME_SECS,
        "primary_event": "level.test",
        "path": {
            "target_atr": TARGET_ATR,
            "stop_atr": STOP_ATR,
            "horizon_bars": HORIZON,
            "entry": "event_close",
            "tie_break": "stop_first",
        },
        "rules": {
            "min_events": MIN_EVENTS,
            "min_clusters": MIN_CLUSTERS,
            "min_lift": MIN_LIFT,
            "pivot_reference_min": PIVOT_REFERENCE_MIN,
            "pivot_sources": list(PIVOT_SOURCES),
        },
        "dev": dev,
        "validation": validation,
        "decision": decision(dev, validation),
    }
    out = Path(args.out).expanduser()
    out.mkdir(parents=True, exist_ok=True)
    (out / "result.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    (out / "result.md").write_text(markdown(result), encoding="utf-8")
    print(f"ok path pilot -> {out} decision={result['decision']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
