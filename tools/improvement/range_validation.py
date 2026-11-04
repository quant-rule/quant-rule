#!/usr/bin/env python3
"""Compression-range next-bar resolution validation.

The harness reads a model bundle's 1m readings and events, joins each
`signal.compression_range` event at its availability bar, and measures whether
the next bar touches at least one published range boundary. It compares the
event with a matched non-coiling window completion, and runs random-completion
and one-bar back-lag controls.

Statistics is evaluation only. Nothing here enters the model or a strategy.

The protocol is `docs/model/2026-11-04/compression-range.md`.

Usage:
    python3 tools/improvement/range_validation.py \
        --matrix config/compression-range/dev/matrix.json \
        --out data/improvement/2026-11-04/MODEL-RANGE-01/artifacts
"""

from __future__ import annotations

import argparse
import csv
import json
import math
import random
import sys
from collections import defaultdict
from pathlib import Path
from statistics import NormalDist
from typing import Any

from path_validation import cluster_stats, control_p, percentile

EVENT_TYPE = 3
EVENT_SUBTYPE = 8
EVENT_SOURCE = 5
TIMEFRAME_ORDER = {60: "1m", 300: "5m", 900: "15m", 3600: "1h"}
COILING_CODE = 2.0
ATR_BASELINE_BARS = 20
CLUSTER_BARS = 20
HORIZON = 1
MIN_EVENTS = 200
MIN_CLUSTERS = 20
MIN_LIFT = 0.05
MIN_INSTRUMENT_EVENTS = 30
RANDOM_DRAWS = 200
SEED = 0


def parse_arguments(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--matrix", required=True, help="matrix JSON")
    parser.add_argument("--out", required=True, help="output directory")
    parser.add_argument("--timeframe", choices=("1m", "5m", "15m", "1h"), default="1m")
    parser.add_argument("--coiling-bars", type=int, default=5)
    parser.add_argument("--random-draws", type=int, default=RANDOM_DRAWS)
    parser.add_argument("--seed", type=int, default=SEED)
    return parser.parse_args(argv)


def read_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise SystemExit(f"error: cannot read {path}: {error}") from None


def read_rows_for_timeframe(path: Path, timeframe: int) -> list[dict[str, Any]]:
    """Reads only the selected timeframe, keeping the harness memory bounded."""
    rows: list[dict[str, Any]] = []
    with path.open(newline="", encoding="utf-8") as handle:
        reader = csv.DictReader(handle)
        for row in reader:
            try:
                if int(row["timeframe_secs"]) != timeframe:
                    continue
                rows.append(
                    {
                        "bar": int(row["bar"]),
                        "high": float(row["high"]),
                        "low": float(row["low"]),
                        "atr": float(row["atr"]) if row["atr"] else None,
                        "rvol": float(row["rvol"]) if row["rvol"] else None,
                    }
                )
            except (KeyError, TypeError, ValueError):
                continue
    rows.sort(key=lambda row: row["bar"])
    return rows


def read_range_events(path: Path, timeframe: int) -> list[dict[str, Any]]:
    """Reads only the selected timeframe's compression-range events."""
    events: list[dict[str, Any]] = []
    with path.open(newline="", encoding="utf-8") as handle:
        reader = csv.DictReader(handle)
        for row in reader:
            try:
                if int(row["timeframe_secs"]) != timeframe:
                    continue
                if (
                    int(row["event_type_code"]) != EVENT_TYPE
                    or int(row["event_subtype_code"]) != EVENT_SUBTYPE
                    or int(row["source_code"]) != EVENT_SOURCE
                    or int(row["direction_code"]) != 0
                ):
                    continue
                events.append(
                    {
                        "available": int(row["available_bar_index"]),
                        "occurred": int(row["occurred_bar_index"]),
                        "primary": float(row["primary_value"]) if row["primary_value"] else None,
                        "reference": float(row["reference_value"]) if row["reference_value"] else None,
                    }
                )
            except (KeyError, TypeError, ValueError):
                continue
    events.sort(key=lambda event: event["available"])
    return events


def context_codes(
    rows: list[dict[str, Any]], baseline_bars: int = ATR_BASELINE_BARS
) -> list[float | None]:
    """The contract's four-bucket context code, recomputed from the readings."""
    atr = [row["atr"] for row in rows]
    codes: list[float | None] = []
    for index, row in enumerate(rows):
        if index + 1 < baseline_bars:
            codes.append(None)
            continue
        window = atr[index + 1 - baseline_bars : index + 1]
        if any(value is None for value in window):
            codes.append(None)
            continue
        base = sum(window) / baseline_bars
        if base == 0.0:
            codes.append(None)
            continue
        ratio = atr[index] / base
        rvol = row["rvol"]
        if rvol is None:
            codes.append(None)
        elif ratio >= 1.0 and rvol >= 1.0:
            codes.append(1.0)
        elif ratio < 1.0 and rvol < 1.0:
            codes.append(2.0)
        elif ratio < 1.0:
            codes.append(3.0)
        else:
            codes.append(4.0)
    return codes


def window_hits(
    rows: list[dict[str, Any]], bars: int
) -> tuple[list[bool | None], list[float | None], list[float | None]]:
    """For each bar, whether the next bar touches the window high or low."""
    hits: list[bool | None] = []
    highs: list[float | None] = []
    lows: list[float | None] = []
    for index, _row in enumerate(rows):
        if index + 1 < bars or index + 1 >= len(rows):
            hits.append(None)
            highs.append(None)
            lows.append(None)
            continue
        window = rows[index + 1 - bars : index + 1]
        high = max(item["high"] for item in window)
        low = min(item["low"] for item in window)
        next_bar = rows[index + 1]
        hits.append(next_bar["high"] >= high or next_bar["low"] <= low)
        highs.append(high)
        lows.append(low)
    return hits, highs, lows


def select_events(
    events: list[dict[str, Any]],
    hits: list[bool | None],
    highs: list[float | None],
    lows: list[float | None],
) -> tuple[list[dict[str, Any]], dict[str, int]]:
    counts = {"raw": 0, "duplicate": 0, "mismatch": 0, "invalid": 0}
    by_bar: dict[int, dict[str, Any] | None] = {}
    for event in events:
        counts["raw"] += 1
        bar = event["available"]
        if bar < 0 or bar >= len(hits):
            counts["invalid"] += 1
            continue
        if hits[bar] is None:
            counts["invalid"] += 1
            continue
        primary = event.get("primary")
        reference = event.get("reference")
        if (
            primary is None
            or reference is None
            or highs[bar] is None
            or lows[bar] is None
            or abs(primary - highs[bar]) > 1e-9
            or abs(reference - lows[bar]) > 1e-9
        ):
            counts["mismatch"] += 1
            continue
        if bar in by_bar:
            counts["duplicate"] += 1
            if by_bar[bar] is None:
                continue
            if (
                abs(by_bar[bar]["primary"] - primary) > 1e-9
                or abs(by_bar[bar]["reference"] - reference) > 1e-9
            ):
                by_bar[bar] = None
            continue
        by_bar[bar] = event
    prepared = [event for event in by_bar.values() if event is not None]
    prepared.sort(key=lambda event: event["available"])
    return prepared, counts


class Bundle:
    def __init__(
        self, instrument: str, fold: int, run: Path, bars: int, timeframe: int
    ) -> None:
        self.instrument = instrument
        self.fold = fold
        rows = read_rows_for_timeframe(run / "readings.csv", timeframe)
        events = read_range_events(run / "events.csv", timeframe)
        codes = context_codes(rows)
        hits, highs, lows = window_hits(rows, bars)
        self.hits = hits
        self.coiling = [
            index + 1 >= bars
            and all(codes[index - offset] == COILING_CODE for offset in range(bars))
            for index in range(len(rows))
        ]
        self.baseline = [
            hits[index] is not None and not self.coiling[index]
            for index in range(len(rows))
        ]
        self.events, self.event_counts = select_events(events, hits, highs, lows)
        # The rows, codes, and level arrays are no longer needed; keeping them
        # for nine one-year bundles would multiply the harness memory by four.
        del rows, codes, highs, lows


def rate_at(bundles: list[Bundle], selected: list[tuple[int, dict[str, Any]]]) -> float | None:
    if not selected:
        return None
    hits = 0
    for bundle_index, event in selected:
        hit = bundles[bundle_index].hits[event["available"]]
        if hit:
            hits += 1
    return hits / len(selected)


def baseline_rate(
    bundles: list[Bundle], selected: list[tuple[int, dict[str, Any]]]
) -> float | None:
    by_bundle: dict[int, int] = defaultdict(int)
    for bundle_index, _event in selected:
        by_bundle[bundle_index] += 1
    total = 0.0
    count = 0
    for bundle_index, events in by_bundle.items():
        bundle = bundles[bundle_index]
        valid = [
            index
            for index, hit in enumerate(bundle.hits)
            if hit is not None and bundle.baseline[index]
        ]
        if not valid:
            continue
        hits = sum(1 for index in valid if bundle.hits[index])
        total += (hits / len(valid)) * events
        count += events
    return total / count if count else None


def coiling_rate(
    bundles: list[Bundle], selected: list[tuple[int, dict[str, Any]]]
) -> float | None:
    by_bundle: dict[int, int] = defaultdict(int)
    for bundle_index, _event in selected:
        by_bundle[bundle_index] += 1
    total = 0.0
    count = 0
    for bundle_index, events in by_bundle.items():
        bundle = bundles[bundle_index]
        valid = [
            index
            for index, hit in enumerate(bundle.hits)
            if hit is not None and bundle.coiling[index]
        ]
        if not valid:
            continue
        hits = sum(1 for index in valid if bundle.hits[index])
        total += (hits / len(valid)) * events
        count += events
    return total / count if count else None


def random_control(
    bundles: list[Bundle], selected: list[tuple[int, dict[str, Any]]], draws: int, seed: int
) -> list[float | None]:
    by_bundle: dict[int, int] = defaultdict(int)
    for bundle_index, _event in selected:
        by_bundle[bundle_index] += 1
    rng = random.Random(seed)
    results: list[float | None] = []
    for _ in range(draws):
        hits = 0
        total = 0
        for bundle_index, count in by_bundle.items():
            bundle = bundles[bundle_index]
            pool = [
                index
                for index, hit in enumerate(bundle.hits)
                if hit is not None and bundle.baseline[index]
            ]
            if not pool or count == 0:
                continue
            sample = rng.sample(pool, min(count, len(pool)))
            for index in sample:
                if bundle.hits[index]:
                    hits += 1
                total += 1
        results.append(hits / total if total else None)
    return results


def back_lag_rate(
    bundles: list[Bundle], selected: list[tuple[int, dict[str, Any]]]
) -> tuple[float | None, int]:
    hits = 0
    total = 0
    for bundle_index, event in selected:
        bundle = bundles[bundle_index]
        index = event["available"] - 1
        if index < 0 or index >= len(bundle.hits):
            continue
        hit = bundle.hits[index]
        if hit is None:
            continue
        hits += int(hit)
        total += 1
    return (hits / total if total else None), total


def evaluate(
    bundles: list[Bundle], random_draws: int, seed: int
) -> dict[str, Any]:
    selected = [
        (bundle_index, event)
        for bundle_index, bundle in enumerate(bundles)
        for event in bundle.events
    ]
    stats = cluster_stats(
        [
            (bundle_index, event["available"], bool(bundles[bundle_index].hits[event["available"]]))
            for bundle_index, event in selected
        ],
        CLUSTER_BARS,
    )
    rate = stats["hit_rate"]
    baseline = baseline_rate(bundles, selected)
    coiling = coiling_rate(bundles, selected)
    lift = rate - baseline if rate is not None and baseline is not None else None
    z = NormalDist().inv_cdf(0.975)
    se = stats["se"]
    lift_lcb = lift - z * se if lift is not None and se is not None else None
    random_values = random_control(bundles, selected, random_draws, seed)
    back_rate, back_n = back_lag_rate(bundles, selected)
    back_lift = back_rate - baseline if back_rate is not None and baseline is not None else None
    random_p = control_p(rate, random_values) if rate is not None else None

    instruments: dict[str, dict[str, Any]] = {}
    by_instrument: dict[str, list[tuple[int, dict[str, Any]]]] = defaultdict(list)
    for bundle_index, event in selected:
        by_instrument[bundles[bundle_index].instrument].append((bundle_index, event))
    for instrument in sorted(by_instrument):
        values = by_instrument[instrument]
        hits = sum(
            1 for bundle_index, event in values if bundles[bundle_index].hits[event["available"]]
        )
        base = baseline_rate(bundles, values)
        instruments[instrument] = {
            "n": len(values),
            "hits": hits,
            "hit_rate": hits / len(values) if values else None,
            "matched_base": base,
        }
    replication = sum(
        1
        for value in instruments.values()
        if value["n"] >= MIN_INSTRUMENT_EVENTS
        and value["hit_rate"] is not None
        and value["matched_base"] is not None
        and value["hit_rate"] > value["matched_base"]
    )

    reasons = []
    checks = [
        ("n", stats["n"] >= MIN_EVENTS),
        ("clusters", stats["clusters"] >= MIN_CLUSTERS),
        ("lift", lift is not None and lift >= MIN_LIFT),
        ("lift_lcb", lift_lcb is not None and lift_lcb > 0.0),
        ("random", random_p is not None and random_p <= 0.05),
        (
            "back_lag",
            back_lift is not None
            and (
                back_lift <= 0.01
                or (lift is not None and lift - back_lift >= 0.05)
            ),
        ),
        ("replication", replication >= 2),
    ]
    for name, ok in checks:
        if not ok:
            reasons.append(name)
    saturated = baseline is not None and (1.0 - baseline) < MIN_LIFT
    if saturated:
        decision = "inconclusive"
        reasons.append("saturated")
    elif reasons:
        decision = "no edge" if stats["n"] >= MIN_EVENTS and stats["clusters"] >= MIN_CLUSTERS else "inconclusive"
    else:
        decision = "pass"
    return {
        "schema": "quantrule.compression-range-validation/v1",
        "decision": decision,
        "n": stats["n"],
        "clusters": stats["clusters"],
        "hits": sum(
            1 for bundle_index, event in selected if bundles[bundle_index].hits[event["available"]]
        ),
        "hit_rate": rate,
        "matched_base": baseline,
        "coiling_base": coiling,
        "lift": lift,
        "lift_lcb": lift_lcb,
        "cluster_se": se,
        "random_p": random_p,
        "random_p95": percentile(random_values, 0.95),
        "back_lag_rate": back_rate,
        "back_lag_n": back_n,
        "back_lag_lift": back_lift,
        "replication": replication,
        "instruments": instruments,
        "reasons": reasons,
        "pass": decision == "pass",
        "event_counts": [bundle.event_counts for bundle in bundles],
    }


def fmt(value: float | None) -> str:
    return "-" if value is None else f"{value:.4f}"


def fmt_signed(value: float | None) -> str:
    return "-" if value is None else f"{value:+.4f}"


def markdown(document: dict[str, Any], partition: str) -> str:
    lines = [
        "# Compression-Range Next-Bar Resolution",
        "",
        f"- Partition: `{partition}`",
        f"- Timeframe: {document['timeframe']}",
        f"- Coiling bars: {document['coiling_bars']}",
        f"- Event: `signal.compression_range` (type 3, subtype 8, source 5)",
        f"- Outcome: next-bar touch of either range boundary",
        f"- Decision: **{document['decision']}**",
        "",
        "| n | J | Hit | Matched base | Coiling base | Lift | Lift LCB | Rand p | Back lag | Rep | Pass | Reasons |",
        "|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|---|",
        "| {n} | {j} | {hit} | {base} | {coil} | {lift} | {lcb} | {rand} | {back} | {rep} | {ok} | {reasons} |".format(
            n=document["n"],
            j=document["clusters"],
            hit=fmt(document["hit_rate"]),
            base=fmt(document["matched_base"]),
            coil=fmt(document["coiling_base"]),
            lift=fmt_signed(document["lift"]),
            lcb=fmt(document["lift_lcb"]),
            rand=fmt(document["random_p"]),
            back=fmt_signed(document["back_lag_lift"]),
            rep=document["replication"],
            ok="yes" if document["pass"] else "no",
            reasons=", ".join(document["reasons"]) or "-",
        ),
        "",
        "## Per-instrument",
        "",
        "| Instrument | n | Hit | Matched base | Lift |",
        "|---|---:|---:|---:|---:|",
    ]
    for instrument in sorted(document["instruments"]):
        value = document["instruments"][instrument]
        lines.append(
            "| {name} | {n} | {hit} | {base} | {lift} |".format(
                name=instrument,
                n=value["n"],
                hit=fmt(value["hit_rate"]),
                base=fmt(value["matched_base"]),
                lift=fmt_signed(
                    None
                    if value["hit_rate"] is None or value["matched_base"] is None
                    else value["hit_rate"] - value["matched_base"]
                ),
            )
        )
    lines.append("")
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    args = parse_arguments(argv)
    if args.coiling_bars <= 0:
        print("error: --coiling-bars must be positive", file=sys.stderr)
        return 1
    timeframe = next(
        seconds for seconds, name in TIMEFRAME_ORDER.items() if name == args.timeframe
    )
    matrix = read_json(Path(args.matrix).expanduser())
    if not isinstance(matrix, dict) or not isinstance(matrix.get("cells"), list):
        print("error: matrix must be an object with cells", file=sys.stderr)
        return 1
    bundles = [
        Bundle(
            str(cell["instrument"]),
            int(cell["fold"]),
            Path(cell["run"]).expanduser(),
            args.coiling_bars,
            timeframe,
        )
        for cell in matrix["cells"]
    ]
    document = evaluate(bundles, args.random_draws, args.seed)
    document["partition"] = matrix.get("partition")
    document["timeframe"] = args.timeframe
    document["coiling_bars"] = args.coiling_bars
    document["seed"] = args.seed
    document["random_draws"] = args.random_draws
    out = Path(args.out).expanduser()
    out.mkdir(parents=True, exist_ok=True)
    (out / "range_validation.json").write_text(
        json.dumps(document, indent=2) + "\n", encoding="utf-8"
    )
    (out / "range_validation.md").write_text(
        markdown(document, str(matrix.get("partition"))), encoding="utf-8"
    )
    print(f"ok compression-range validation -> {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
