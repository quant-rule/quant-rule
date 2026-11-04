#!/usr/bin/env python3
"""Range-expansion validation for the compression-breakout event.

The harness reads a model bundle's per-bar readings and events, joins each
compression-breakout event at its availability bar, and measures whether the
next 20 bars' range reaches 2 ATR. It compares the event with three baselines:

- all valid bars;
- coiling bars (context code 2);
- a placebo coiling-to-expanding transition without the breakout.

It also runs random-coiling and back-lag controls and reports a cluster-robust
lower bound. Statistics is evaluation only. Nothing here enters the model or a
strategy.

The protocol is `docs/model/2026-11-04/compression-breakout.md`.

Usage:
    python3 tools/improvement/compression_validation.py \
        --matrix config/compression/dev/matrix.json \
        --out data/improvement/2026-11-04/MODEL-2026-11-04-02/artifacts
"""

from __future__ import annotations

import argparse
import json
import math
import random
import sys
from collections import defaultdict
from pathlib import Path
from statistics import NormalDist
from typing import Any

from path_validation import TIMEFRAME_CODES, cluster_stats, read_events, read_rows

EVENT_TYPE = 3
EVENT_SUBTYPE = 7
EVENT_SOURCE = 5
TIMEFRAME_ORDER = {60: "1m", 300: "5m", 900: "15m", 3600: "1h"}
PRIMARY_CELLS = 2
HORIZON = 20
TARGET_ATR = 2.0
MIN_EVENTS = 200
MIN_CLUSTERS = 20
MIN_LIFT = 0.05
RANDOM_DRAWS = 200
SEED = 0


def parse_arguments(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--matrix", required=True, help="matrix JSON")
    parser.add_argument("--out", required=True, help="output directory")
    parser.add_argument("--horizon", type=int, default=HORIZON)
    parser.add_argument("--target-atr", type=float, default=TARGET_ATR)
    parser.add_argument("--random-draws", type=int, default=RANDOM_DRAWS)
    parser.add_argument("--seed", type=int, default=SEED)
    parser.add_argument(
        "--primary-cells",
        type=int,
        default=PRIMARY_CELLS,
        help="Bonferroni cells for the timeframe choice",
    )
    parser.add_argument(
        "--primary-timeframe",
        choices=("1m", "5m", "15m", "1h"),
        default=None,
        help="force the primary timeframe",
    )
    parser.add_argument(
        "--timeframes",
        default="1h,15m",
        help="comma-separated timeframes to evaluate (default: 1h,15m)",
    )
    parser.add_argument(
        "--select",
        action="store_true",
        help="evaluate every timeframe and select the best passing cell",
    )
    return parser.parse_args(argv)


def read_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise SystemExit(f"error: cannot read {path}: {error}") from None


def forward_hits(
    rows: list[dict[str, Any]], horizon: int, target_atr: float
) -> list[bool | None]:
    hits: list[bool | None] = []
    for index, row in enumerate(rows):
        atr = row["atr"]
        if atr is None or atr <= 0.0 or index + horizon >= len(rows):
            hits.append(None)
            continue
        high = max(item["high"] for item in rows[index + 1 : index + horizon + 1])
        low = min(item["low"] for item in rows[index + 1 : index + horizon + 1])
        hits.append((high - low) >= target_atr * atr)
    return hits


def mask_rate(mask: list[bool], hits: list[bool | None]) -> tuple[int, int]:
    valid = [hit for flag, hit in zip(mask, hits) if flag and hit is not None]
    return sum(1 for hit in valid if hit), len(valid)


def select_events(
    events: list[dict[str, Any]], hits: list[bool | None]
) -> tuple[list[dict[str, Any]], int]:
    by_bar: dict[int, dict[str, Any] | None] = {}
    raw = 0
    for event in events:
        if event["type"] != EVENT_TYPE or event["subtype"] != EVENT_SUBTYPE:
            continue
        if event["source"] != EVENT_SOURCE or event["direction"] not in (-1, 1):
            continue
        raw += 1
        bar = event["available"]
        if bar < 0 or bar >= len(hits):
            continue
        if bar not in by_bar:
            by_bar[bar] = event
        elif by_bar[bar] is not None and by_bar[bar]["direction"] != event["direction"]:
            by_bar[bar] = None
    prepared = []
    for event in by_bar.values():
        if event is None:
            continue
        if hits[event["available"]] is None:
            continue
        prepared.append(event)
    prepared.sort(key=lambda event: event["available"])
    return prepared, raw


class Bundle:
    def __init__(
        self,
        instrument: str,
        fold: int,
        run: Path,
        horizon: int,
        target_atr: float,
        timeframes: set[int],
    ) -> None:
        self.instrument = instrument
        self.fold = fold
        rows_by_timeframe = read_rows(run / "readings.csv")
        events_by_timeframe = read_events(run / "events.csv")
        self.hits: dict[int, list[bool | None]] = {}
        self.coiling: dict[int, list[bool]] = {}
        self.placebo: dict[int, list[bool]] = {}
        self.selected: dict[int, list[dict[str, Any]]] = {}
        for timeframe in sorted(timeframes):
            rows = rows_by_timeframe.get(timeframe, [])
            if not rows:
                continue
            hits = forward_hits(rows, horizon, target_atr)
            self.hits[timeframe] = hits
            self.coiling[timeframe] = [
                row["context_code"] == 2.0 for row in rows
            ]
            self.placebo[timeframe] = [
                index > 0
                and rows[index - 1]["context_code"] == 2.0
                and row["context_code"] == 1.0
                for index, row in enumerate(rows)
            ]
            self.selected[timeframe], _ = select_events(
                events_by_timeframe.get(timeframe, []), hits
            )


def event_count(bundles: list[Bundle], timeframe: int) -> int:
    return sum(len(bundle.selected.get(timeframe, [])) for bundle in bundles)


def event_rate(
    bundles: list[Bundle], timeframe: int
) -> tuple[float | None, list[tuple[int, dict[str, Any]]]]:
    selected = [
        (bundle_index, event)
        for bundle_index, bundle in enumerate(bundles)
        for event in bundle.selected.get(timeframe, [])
    ]
    if not selected:
        return None, []
    hits = 0
    for bundle_index, event in selected:
        hit = bundles[bundle_index].hits[timeframe][event["available"]]
        if hit:
            hits += 1
    return hits / len(selected), selected


def baseline_rate(
    bundles: list[Bundle],
    timeframe: int,
    selected: list[tuple[int, dict[str, Any]]],
    kind: str,
) -> float | None:
    by_bundle: dict[int, int] = defaultdict(int)
    for bundle_index, _event in selected:
        by_bundle[bundle_index] += 1
    total = 0.0
    count = 0
    for bundle_index, events in by_bundle.items():
        bundle = bundles[bundle_index]
        mask = bundle.coiling[timeframe] if kind == "coiling" else bundle.placebo[timeframe]
        hits, valid = mask_rate(mask, bundle.hits[timeframe])
        if valid == 0:
            continue
        total += (hits / valid) * events
        count += events
    return total / count if count else None


def random_control(
    bundles: list[Bundle],
    timeframe: int,
    selected: list[tuple[int, dict[str, Any]]],
    draws: int,
    seed: int,
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
                for index, hit in enumerate(bundle.hits[timeframe])
                if hit is not None and bundle.coiling[timeframe][index]
            ]
            if not pool or count == 0:
                continue
            sample = rng.sample(pool, min(count, len(pool)))
            for index in sample:
                hit = bundle.hits[timeframe][index]
                if hit is not None:
                    hits += int(hit)
                    total += 1
        results.append(hits / total if total else None)
    return results


def lagged_rate(
    bundles: list[Bundle],
    timeframe: int,
    selected: list[tuple[int, dict[str, Any]]],
    lag: int,
) -> tuple[float | None, int]:
    hits = 0
    total = 0
    for bundle_index, event in selected:
        bundle = bundles[bundle_index]
        index = event["available"] + lag
        if index < 0 or index >= len(bundle.hits[timeframe]):
            continue
        hit = bundle.hits[timeframe][index]
        if hit is None:
            continue
        hits += int(hit)
        total += 1
    return (hits / total if total else None), total


def select_candidate(candidates: list[dict[str, Any]]) -> dict[str, Any] | None:
    """Selects the passing cell with the highest corrected lift bound."""
    passing = [candidate for candidate in candidates if candidate["pass"]]
    if not passing:
        return None
    return sorted(
        passing,
        key=lambda candidate: (
            -(candidate["lift_lcb"] if candidate["lift_lcb"] is not None else -math.inf),
            0 if candidate["timeframe"] == "5m" else 1,
            candidate["timeframe_secs"],
        ),
    )[0]


def control_p(observed: float, draws: list[float | None]) -> float | None:
    valid = [draw for draw in draws if draw is not None]
    if not valid:
        return None
    return (1 + sum(1 for draw in valid if draw >= observed)) / (len(valid) + 1)


def percentile(draws: list[float | None], fraction: float) -> float | None:
    valid = sorted(draw for draw in draws if draw is not None)
    if not valid:
        return None
    return valid[min(len(valid) - 1, int(fraction * (len(valid) - 1)))]


def evaluate(
    bundles: list[Bundle],
    timeframe: int,
    horizon: int,
    target_atr: float,
    z: float,
    random_draws: int,
    seed: int,
) -> dict[str, Any]:
    rate, selected = event_rate(bundles, timeframe)
    stats = cluster_stats(
        [
            (bundle_index, event["available"], bool(bundles[bundle_index].hits[timeframe][event["available"]]))
            for bundle_index, event in selected
        ],
        horizon,
    )
    coiling = baseline_rate(bundles, timeframe, selected, "coiling")
    placebo = baseline_rate(bundles, timeframe, selected, "placebo")
    all_base = None
    total_hits = 0
    total_valid = 0
    for bundle in bundles:
        hits, valid = mask_rate([True] * len(bundle.hits[timeframe]), bundle.hits[timeframe])
        total_hits += hits
        total_valid += valid
    if total_valid:
        all_base = total_hits / total_valid
    lift = rate - coiling if rate is not None and coiling is not None else None
    saturated = coiling is not None and (1.0 - coiling) < MIN_LIFT
    se = stats["se"]
    lift_lcb = lift - z * se if lift is not None and se is not None else None
    random_values = random_control(bundles, timeframe, selected, random_draws, seed)
    back_rate, back_n = lagged_rate(bundles, timeframe, selected, -horizon)
    forward_rate, forward_n = lagged_rate(bundles, timeframe, selected, horizon)
    back_lift = back_rate - coiling if back_rate is not None and coiling is not None else None
    instruments: dict[str, dict[str, Any]] = {}
    by_instrument: dict[str, list[tuple[int, dict[str, Any]]]] = defaultdict(list)
    for bundle_index, event in selected:
        by_instrument[bundles[bundle_index].instrument].append((bundle_index, event))
    for instrument in sorted(by_instrument):
        values = by_instrument[instrument]
        hits = sum(
            1
            for bundle_index, event in values
            if bundles[bundle_index].hits[timeframe][event["available"]]
        )
        base = baseline_rate(bundles, timeframe, values, "coiling")
        instruments[instrument] = {
            "n": len(values),
            "hits": hits,
            "hit_rate": hits / len(values) if values else None,
            "coiling_base": base,
        }
    replication = sum(
        1
        for value in instruments.values()
        if value["n"] >= 30
        and value["hit_rate"] is not None
        and value["coiling_base"] is not None
        and value["hit_rate"] > value["coiling_base"]
    )
    reasons = []
    checks = [
        ("n", stats["n"] >= MIN_EVENTS),
        ("clusters", stats["clusters"] >= MIN_CLUSTERS),
        ("lift", lift is not None and lift >= MIN_LIFT),
        ("lift_lcb", lift_lcb is not None and lift_lcb > 0.0),
        (
            "placebo",
            rate is not None
            and placebo is not None
            and (
                rate - placebo >= 0.02
                or coiling is None
                or placebo <= coiling
            ),
        ),
        (
            "random",
            rate is not None
            and control_p(rate, random_values) is not None
            and control_p(rate, random_values) <= 0.05,
        ),
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
    return {
        "timeframe": TIMEFRAME_ORDER[timeframe],
        "timeframe_secs": timeframe,
        "n": stats["n"],
        "clusters": stats["clusters"],
        "hits": sum(
            1
            for bundle_index, event in selected
            if bundles[bundle_index].hits[timeframe][event["available"]]
        ),
        "hit_rate": rate,
        "all_base": all_base,
        "coiling_base": coiling,
        "placebo_base": placebo,
        "lift": lift,
        "lift_lcb": lift_lcb,
        "saturated": saturated,
        "cluster_se": se,
        "random_p95": percentile(random_values, 0.95),
        "random_p": control_p(rate, random_values) if rate is not None else None,
        "back_lag_rate": back_rate,
        "back_lag_n": back_n,
        "back_lag_lift": back_lift,
        "forward_lag_rate": forward_rate,
        "forward_lag_n": forward_n,
        "replication": replication,
        "instruments": instruments,
        "pass": not reasons,
        "reasons": reasons,
    }


def markdown(document: dict[str, Any]) -> str:
    lines = [
        "# Compression-Breakout Range Expansion",
        "",
        f"- Horizon: {document['horizon_bars']} bars; range target: {document['target_atr']} ATR",
        f"- Seed: {document['seed']}; random draws: {document['random_draws']}",
        f"- Primary timeframe: {document['primary_timeframe']}",
        f"- Decision: **{document['decision']}**",
        "",
    ]
    if document.get("candidates") is not None:
        lines += [
            "## Candidate cells",
            "",
            "| TF | n | J | Hit | Coiling base | Placebo base | Lift | Lift LCB | Rand p | Back lag | Rep | Pass | Reasons |",
            "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|---|",
        ]
        for candidate in document["candidates"]:
            lines.append(
                "| {tf} | {n} | {j} | {hit} | {coil} | {placebo} | {lift} | {lcb} | {rand} | {back} | {rep} | {ok} | {reasons} |".format(
                    tf=candidate["timeframe"],
                    n=candidate["n"],
                    j=candidate["clusters"],
                    hit=fmt(candidate["hit_rate"]),
                    coil=fmt(candidate["coiling_base"]),
                    placebo=fmt(candidate["placebo_base"]),
                    lift=fmt_signed(candidate["lift"]),
                    lcb=fmt(candidate["lift_lcb"]),
                    rand=fmt(candidate["random_p"]),
                    back=fmt_signed(candidate["back_lag_lift"]),
                    rep=candidate["replication"],
                    ok="yes" if candidate["pass"] else "no",
                    reasons=", ".join(candidate["reasons"]) or "-",
                )
            )
        lines.append("")
    if document.get("primary") is not None:
        row = document["primary"]
        lines += [
            "## Primary",
            "",
            "| TF | n | J | Hit | All base | Coiling base | Placebo base | Lift | Lift LCB | Rand p | Back lag | Rep | Pass | Reasons |",
            "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|---|",
        ]
        lines.append(
            "| {tf} | {n} | {j} | {hit} | {allb} | {coil} | {placebo} | {lift} | {lcb} | {rand} | {back} | {rep} | {ok} | {reasons} |".format(
                tf=row["timeframe"],
                n=row["n"],
                j=row["clusters"],
                hit=fmt(row["hit_rate"]),
                allb=fmt(row["all_base"]),
                coil=fmt(row["coiling_base"]),
                placebo=fmt(row["placebo_base"]),
                lift=fmt_signed(row["lift"]),
                lcb=fmt(row["lift_lcb"]),
                rand=fmt(row["random_p"]),
                back=fmt_signed(row["back_lag_lift"]),
                rep=row["replication"],
                ok="yes" if row["pass"] else "no",
                reasons=", ".join(row["reasons"]) or "-",
            )
        )
    if document.get("secondary") is not None:
        row = document["secondary"]
        lines += [
            "",
            "## Secondary (descriptive)",
            "",
            "| TF | n | J | Hit | Coiling base | Lift | Lift LCB | Rand p | Back lag | Rep |",
            "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|",
        ]
        lines.append(
            "| {tf} | {n} | {j} | {hit} | {coil} | {lift} | {lcb} | {rand} | {back} | {rep} |".format(
                tf=row["timeframe"],
                n=row["n"],
                j=row["clusters"],
                hit=fmt(row["hit_rate"]),
                coil=fmt(row["coiling_base"]),
                lift=fmt_signed(row["lift"]),
                lcb=fmt(row["lift_lcb"]),
                rand=fmt(row["random_p"]),
                back=fmt_signed(row["back_lag_lift"]),
                rep=row["replication"],
            )
        )
    if document.get("primary") is not None:
        lines += [
            "",
            "## Per-instrument (primary)",
            "",
            "| Instrument | n | Hit | Coiling base | Lift |",
            "|---|---:|---:|---:|---:|",
        ]
        for instrument in sorted(document["primary"]["instruments"]):
            value = document["primary"]["instruments"][instrument]
            lines.append(
                "| {name} | {n} | {hit} | {base} | {lift} |".format(
                    name=instrument,
                    n=value["n"],
                    hit=fmt(value["hit_rate"]),
                    base=fmt(value["coiling_base"]),
                    lift=fmt_signed(
                        None
                        if value["hit_rate"] is None or value["coiling_base"] is None
                        else value["hit_rate"] - value["coiling_base"]
                    ),
                )
            )
    lines.append("")
    return "\n".join(lines)


def fmt(value: float | None) -> str:
    return "-" if value is None else f"{value:.4f}"


def fmt_signed(value: float | None) -> str:
    return "-" if value is None else f"{value:+.4f}"


def main(argv: list[str] | None = None) -> int:
    args = parse_arguments(argv)
    matrix = read_json(Path(args.matrix).expanduser())
    if not isinstance(matrix, dict) or not isinstance(matrix.get("cells"), list):
        print("error: matrix must be an object with cells", file=sys.stderr)
        return 1
    timeframes = set()
    for code in args.timeframes.split(","):
        for seconds, name in TIMEFRAME_ORDER.items():
            if code.strip() == name:
                timeframes.add(seconds)
    if not timeframes:
        print("error: no supported timeframes", file=sys.stderr)
        return 1
    bundles = [
        Bundle(
            str(cell["instrument"]),
            int(cell["fold"]),
            Path(cell["run"]).expanduser(),
            args.horizon,
            args.target_atr,
            timeframes,
        )
        for cell in matrix["cells"]
    ]
    counts = {timeframe: event_count(bundles, timeframe) for timeframe in sorted(timeframes)}
    z = NormalDist().inv_cdf(1.0 - 0.05 / (2.0 * args.primary_cells))
    primary_result = None
    secondary_result = None
    candidates = None
    selection = None
    if args.select:
        candidates = [
            evaluate(
                bundles,
                timeframe,
                args.horizon,
                args.target_atr,
                z,
                args.random_draws,
                args.seed + index,
            )
            for index, timeframe in enumerate(sorted(timeframes))
        ]
        selection = select_candidate(candidates)
        if selection is not None:
            decision = f"pass — freeze {selection['timeframe']} and open validation once"
        elif any(
            candidate["n"] < MIN_EVENTS
            or candidate["clusters"] < MIN_CLUSTERS
            or candidate["saturated"]
            for candidate in candidates
        ):
            decision = "inconclusive — a cell is underpowered or saturated"
        else:
            decision = "no edge"
        primary_timeframe = selection["timeframe"] if selection is not None else "none"
    else:
        if args.primary_timeframe is not None:
            primary = next(
                seconds
                for seconds, name in TIMEFRAME_ORDER.items()
                if name == args.primary_timeframe
            )
        else:
            primary = 3600 if counts.get(3600, 0) >= MIN_EVENTS else 900
        secondary = next(
            (timeframe for timeframe in sorted(timeframes) if timeframe != primary),
            None,
        )
        primary_result = evaluate(
            bundles,
            primary,
            args.horizon,
            args.target_atr,
            z,
            args.random_draws,
            args.seed,
        )
        secondary_result = (
            evaluate(
                bundles,
                secondary,
                args.horizon,
                args.target_atr,
                z,
                args.random_draws,
                args.seed + 1,
            )
            if secondary is not None
            else None
        )
        decision = "no edge"
        if primary_result["pass"]:
            decision = "pass — freeze and open validation once"
        elif (
            primary_result["n"] < MIN_EVENTS
            or primary_result["clusters"] < MIN_CLUSTERS
        ):
            decision = "inconclusive — primary sample too small"
        elif primary_result["saturated"]:
            decision = "inconclusive — outcome saturated at the coiling base"
        primary_timeframe = TIMEFRAME_ORDER[primary]
    document = {
        "schema": "quantrule.compression-validation/v1",
        "horizon_bars": args.horizon,
        "target_atr": args.target_atr,
        "seed": args.seed,
        "random_draws": args.random_draws,
        "primary_cells": args.primary_cells,
        "event_counts": {
            TIMEFRAME_ORDER[key]: value for key, value in counts.items()
        },
        "primary_timeframe": primary_timeframe,
        "primary": primary_result,
        "secondary": secondary_result,
        "candidates": candidates,
        "selection": selection,
        "decision": decision,
    }
    out = Path(args.out).expanduser()
    out.mkdir(parents=True, exist_ok=True)
    (out / "compression_validation.json").write_text(
        json.dumps(document, indent=2) + "\n", encoding="utf-8"
    )
    (out / "compression_validation.md").write_text(markdown(document), encoding="utf-8")
    print(f"ok compression validation -> {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
