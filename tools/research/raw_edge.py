#!/usr/bin/env python3
"""Measure the raw forward edge of a published entry signal.

The tool reconstructs an entry signal from a model bundle's readings and
events, then measures forward returns, MFE/MAE, and bracket outcomes with no
stop, target, drawdown halt, or position state. It is a diagnostic: it does not
produce a strategy result and it never writes a model or strategy artifact.

The signal is the SR11 conjunction: a Fibonacci test event, an EMA test event,
and the `ema_stack` and `structure_state` readings aligned on the same 15m bar.
Controls are the same events without the states, the states without the events,
and every bar.

The tool is deterministic: inputs are sorted, floating-point sums use
`math.fsum`, and no wall clock or random source enters the output.
"""

from __future__ import annotations

import argparse
import csv
import json
import math
import statistics
from array import array
from dataclasses import dataclass
from pathlib import Path

TIMEFRAME_SECS = 900
LEVEL_TYPE = 1
TEST_SUBTYPE = 1
FIB_SOURCE = 56
EMA_SOURCE = 1
SIDES = ("long", "short")
VARIANTS = ("full", "events", "states", "all")
DEFAULT_HORIZONS = (1, 3, 5, 10, 20)


@dataclass(frozen=True)
class Bar:
    """One 15m model bar with the readings the signal needs."""

    index: int
    open: float
    high: float
    low: float
    close: float
    atr: float
    ema_stack: int
    structure_state: int


@dataclass(frozen=True)
class Bundle:
    """One year of model bars and the entry events."""

    label: str
    bars: tuple[Bar, ...]
    fib: dict[int, frozenset[int]]
    ema: frozenset[int]


class Series:
    """A deterministic collection of per-signal measurements."""

    def __init__(self) -> None:
        self.values = array("d")
        self.count = 0

    def add(self, value: float) -> None:
        self.values.append(value)
        self.count += 1

    def mean(self) -> float | None:
        if not self.values:
            return None
        return math.fsum(self.values) / len(self.values)

    def median(self) -> float | None:
        if not self.values:
            return None
        return statistics.median(self.values)


def _float(value: str) -> float | None:
    try:
        parsed = float(value)
    except (TypeError, ValueError):
        return None
    if not math.isfinite(parsed):
        return None
    return parsed


def _int(value: str) -> int | None:
    try:
        return int(value)
    except (TypeError, ValueError):
        return None


def parse_readings(path: Path) -> tuple[Bar, ...]:
    """Reads the 15m rows of a model bundle's readings CSV."""
    bars: list[Bar] = []
    with path.open(newline="", encoding="utf-8") as handle:
        for row in csv.DictReader(handle):
            if _int(row.get("timeframe_secs", "")) != TIMEFRAME_SECS:
                continue
            index = _int(row.get("bar", ""))
            open_price = _float(row.get("open", ""))
            high = _float(row.get("high", ""))
            low = _float(row.get("low", ""))
            close = _float(row.get("close", ""))
            if index is None or None in (open_price, high, low, close):
                continue
            atr = _float(row.get("atr", "")) or 0.0
            ema_stack = _int(row.get("ema_stack", "")) or 0
            structure_state = _int(row.get("structure_state", "")) or 0
            bars.append(
                Bar(
                    index=index,
                    open=open_price,
                    high=high,
                    low=low,
                    close=close,
                    atr=atr,
                    ema_stack=ema_stack,
                    structure_state=structure_state,
                )
            )
    bars.sort(key=lambda bar: bar.index)
    return tuple(bars)


def parse_events(path: Path) -> tuple[dict[int, frozenset[int]], frozenset[int]]:
    """Reads the Fibonacci and EMA test event bars from a model bundle."""
    fib: dict[int, set[int]] = {1: set(), -1: set()}
    ema: set[int] = set()
    with path.open(newline="", encoding="utf-8") as handle:
        for row in csv.DictReader(handle):
            if _int(row.get("timeframe_secs", "")) != TIMEFRAME_SECS:
                continue
            if _int(row.get("event_type_code", "")) != LEVEL_TYPE:
                continue
            if _int(row.get("event_subtype_code", "")) != TEST_SUBTYPE:
                continue
            source = _int(row.get("source_code", ""))
            bar = _int(row.get("available_bar_index", ""))
            direction = _int(row.get("direction_code", ""))
            if bar is None:
                continue
            if source == FIB_SOURCE and direction in (-1, 1):
                fib[direction].add(bar)
            elif source == EMA_SOURCE:
                ema.add(bar)
    return {key: frozenset(value) for key, value in fib.items()}, frozenset(ema)


def load_bundle(model_dir: Path, label: str | None = None) -> Bundle:
    """Loads one model bundle."""
    readings = model_dir / "readings.csv"
    events = model_dir / "events.csv"
    if not readings.is_file() or not events.is_file():
        raise FileNotFoundError(f"{model_dir} is not a model bundle")
    bars = parse_readings(readings)
    fib, ema = parse_events(events)
    return Bundle(label=label or str(model_dir), bars=bars, fib=fib, ema=ema)


def signal_bar_indices(
    bundle: Bundle, side: str, variant: str
) -> list[int]:
    """Returns the sorted signal bars for one side and variant."""
    if side not in SIDES:
        raise ValueError(f"unknown side: {side}")
    if variant not in VARIANTS:
        raise ValueError(f"unknown variant: {variant}")
    direction = 1 if side == "long" else -1
    by_index = {bar.index: bar for bar in bundle.bars}
    if variant == "all":
        return [bar.index for bar in bundle.bars if bar.atr > 0.0]
    if variant == "events":
        selected = bundle.fib.get(direction, frozenset()) & bundle.ema
    elif variant == "states":
        selected = {
            bar.index
            for bar in bundle.bars
            if bar.atr > 0.0
            and bar.ema_stack == direction
            and bar.structure_state == direction
        }
    else:
        selected = bundle.fib.get(direction, frozenset()) & bundle.ema
        selected = {
            index
            for index in selected
            if by_index.get(index) is not None
            and by_index[index].atr > 0.0
            and by_index[index].ema_stack == direction
            and by_index[index].structure_state == direction
        }
    return sorted(index for index in selected if index in by_index)


def non_overlapping(indices: list[int], gap: int) -> list[int]:
    """Keeps the first signal of each cluster separated by at least `gap` bars."""
    selected: list[int] = []
    last = -gap - 1
    for index in indices:
        if index - last >= gap:
            selected.append(index)
            last = index
    return selected


def forward_metrics(
    bars: tuple[Bar, ...],
    position: int,
    side: str,
    horizons: tuple[int, ...],
    target_atr: float,
    stop_atr: float,
    barrier_bars: int,
    cost_bps: float,
) -> dict[str, object] | None:
    """Measures one signal at `position` in `bars`.

    The signal is known at the close of `bars[position]`; entry is the next
    bar's open. No future bar enters the signal.
    """
    if position + 1 >= len(bars):
        return None
    signal = bars[position]
    entry_bar = bars[position + 1]
    entry = entry_bar.open
    atr = signal.atr
    if entry <= 0.0 or atr <= 0.0:
        return None
    sign = 1.0 if side == "long" else -1.0
    record: dict[str, object] = {
        "bar": signal.index,
        "entry": entry,
        "atr": atr,
    }
    for horizon in horizons:
        end = position + horizon
        if end >= len(bars):
            continue
        window = bars[position + 1 : end + 1]
        highest = max(bar.high for bar in window)
        lowest = min(bar.low for bar in window)
        gross = sign * (bars[end].close - entry) / entry
        if sign > 0:
            mfe = (highest - entry) / atr
            mae = (entry - lowest) / atr
        else:
            mfe = (entry - lowest) / atr
            mae = (highest - entry) / atr
        record[f"h{horizon}"] = {
            "gross_return": gross,
            "net_return": gross - cost_bps / 10_000.0,
            "mfe_atr": mfe,
            "mae_atr": mae,
        }
    target = entry + sign * target_atr * atr
    stop = entry - sign * stop_atr * atr
    outcome = "timeout"
    r_multiple = 0.0
    last = min(barrier_bars, len(bars) - position - 1)
    for offset in range(1, last + 1):
        bar = bars[position + offset]
        if side == "long":
            hit_target = bar.high >= target
            hit_stop = bar.low <= stop
        else:
            hit_target = bar.low <= target
            hit_stop = bar.high >= stop
        if hit_target and hit_stop:
            outcome = "stop"
            r_multiple = -stop_atr
            break
        if hit_target:
            outcome = "target"
            r_multiple = target_atr
            break
        if hit_stop:
            outcome = "stop"
            r_multiple = -stop_atr
            break
    else:
        end = position + last
        if last > 0:
            r_multiple = sign * (bars[end].close - entry) / atr
    record["barrier"] = {
        "outcome": outcome,
        "r_multiple": r_multiple,
        "target": target,
        "stop": stop,
    }
    return record


def summarize(
    records: list[dict[str, object]],
    horizons: tuple[int, ...],
) -> dict[str, object]:
    """Aggregates per-signal records into a deterministic summary."""
    summary: dict[str, object] = {"count": len(records)}
    horizon_summary: dict[str, object] = {}
    for horizon in horizons:
        key = f"h{horizon}"
        rows = [record[key] for record in records if key in record]
        if not rows:
            horizon_summary[str(horizon)] = {"count": 0}
            continue
        gross = [row["gross_return"] for row in rows]
        net = [row["net_return"] for row in rows]
        mfe = [row["mfe_atr"] for row in rows]
        mae = [row["mae_atr"] for row in rows]
        mean_net = statistics.fmean(net)
        stderr_net = (
            statistics.stdev(net) / math.sqrt(len(net)) if len(net) > 1 else 0.0
        )
        horizon_summary[str(horizon)] = {
            "count": len(rows),
            "mean_gross_pct": round(statistics.fmean(gross) * 100.0, 5),
            "median_gross_pct": round(statistics.median(gross) * 100.0, 5),
            "mean_net_pct": round(mean_net * 100.0, 5),
            "stderr_net_pct": round(stderr_net * 100.0, 5),
            "ci95_net_pct": [
                round((mean_net - 1.96 * stderr_net) * 100.0, 5),
                round((mean_net + 1.96 * stderr_net) * 100.0, 5),
            ],
            "hit_rate": round(sum(1 for value in gross if value > 0.0) / len(gross), 5),
            "mean_mfe_atr": round(statistics.fmean(mfe), 4),
            "median_mfe_atr": round(statistics.median(mfe), 4),
            "mean_mae_atr": round(statistics.fmean(mae), 4),
            "median_mae_atr": round(statistics.median(mae), 4),
        }
    summary["horizons"] = horizon_summary
    outcomes = [record["barrier"]["outcome"] for record in records]
    rs = [record["barrier"]["r_multiple"] for record in records]
    mean_r = statistics.fmean(rs) if rs else 0.0
    stderr_r = statistics.stdev(rs) / math.sqrt(len(rs)) if len(rs) > 1 else 0.0
    summary["barrier"] = {
        "count": len(outcomes),
        "target": outcomes.count("target"),
        "stop": outcomes.count("stop"),
        "timeout": outcomes.count("timeout"),
        "target_rate": round(outcomes.count("target") / len(outcomes), 5) if outcomes else None,
        "stop_rate": round(outcomes.count("stop") / len(outcomes), 5) if outcomes else None,
        "timeout_rate": round(outcomes.count("timeout") / len(outcomes), 5) if outcomes else None,
        "mean_r": round(mean_r, 4) if rs else None,
        "median_r": round(statistics.median(rs), 4) if rs else None,
        "stderr_r": round(stderr_r, 4) if rs else None,
        "ci95_r": [
            round(mean_r - 1.96 * stderr_r, 4),
            round(mean_r + 1.96 * stderr_r, 4),
        ]
        if rs
        else None,
    }
    return summary


def evaluate_bundle(
    bundle: Bundle,
    side: str,
    variant: str,
    horizons: tuple[int, ...],
    target_atr: float,
    stop_atr: float,
    barrier_bars: int,
    cost_bps: float,
) -> dict[str, object]:
    """Evaluates one bundle, side, and variant."""
    indices = signal_bar_indices(bundle, side, variant)
    position = {bar.index: index for index, bar in enumerate(bundle.bars)}
    records = [
        record
        for index in indices
        if (record := forward_metrics(
            bundle.bars,
            position[index],
            side,
            horizons,
            target_atr,
            stop_atr,
            barrier_bars,
            cost_bps,
        ))
        is not None
    ]
    summary = summarize(records, horizons)
    summary["signals"] = len(indices)
    summary["bars"] = len(bundle.bars)
    if variant == "full":
        selected = non_overlapping(indices, max(horizons))
        non_overlap_records = [
            record
            for index in selected
            if (record := forward_metrics(
                bundle.bars,
                position[index],
                side,
                horizons,
                target_atr,
                stop_atr,
                barrier_bars,
                cost_bps,
            ))
            is not None
        ]
        summary["non_overlapping"] = summarize(non_overlap_records, horizons)
        summary["non_overlapping"]["signals"] = len(selected)
    return summary


def evaluate(
    bundles: list[Bundle],
    horizons: tuple[int, ...],
    target_atr: float,
    stop_atr: float,
    barrier_bars: int,
    cost_bps: float,
) -> dict[str, object]:
    """Evaluates every bundle and aggregates by side and variant."""
    result: dict[str, object] = {
        "schema": "quantrule.raw-edge/v1",
        "timeframe": "15m",
        "cost_bps": cost_bps,
        "horizons": list(horizons),
        "barrier": {
            "target_atr": target_atr,
            "stop_atr": stop_atr,
            "bars": barrier_bars,
            "tie_break": "stop_first",
            "entry": "next_bar_open",
        },
        "bundles": [
            {
                "label": bundle.label,
                "bars": len(bundle.bars),
                "fib_long": len(bundle.fib.get(1, frozenset())),
                "fib_short": len(bundle.fib.get(-1, frozenset())),
                "ema": len(bundle.ema),
            }
            for bundle in bundles
        ],
        "sides": {},
    }
    for side in SIDES:
        side_result: dict[str, object] = {}
        for variant in VARIANTS:
            # Aggregate the per-bundle records by concatenating their summaries
            # is not valid for medians, so evaluate the whole side in one pass
            # over the concatenated bars. Year boundaries are preserved by
            # evaluating each bundle separately and re-aggregating raw records.
            records: list[dict[str, object]] = []
            for bundle in bundles:
                indices = signal_bar_indices(bundle, side, variant)
                position = {bar.index: index for index, bar in enumerate(bundle.bars)}
                records.extend(
                    record
                    for index in indices
                    if (record := forward_metrics(
                        bundle.bars,
                        position[index],
                        side,
                        horizons,
                        target_atr,
                        stop_atr,
                        barrier_bars,
                        cost_bps,
                    ))
                    is not None
                )
            summary = summarize(records, horizons)
            summary["signals"] = sum(
                len(signal_bar_indices(bundle, side, variant)) for bundle in bundles
            )
            if variant == "full":
                selected: list[dict[str, object]] = []
                for bundle in bundles:
                    indices = non_overlapping(
                        signal_bar_indices(bundle, side, variant), max(horizons)
                    )
                    position = {bar.index: index for index, bar in enumerate(bundle.bars)}
                    selected.extend(
                        record
                        for index in indices
                        if (record := forward_metrics(
                            bundle.bars,
                            position[index],
                            side,
                            horizons,
                            target_atr,
                            stop_atr,
                            barrier_bars,
                            cost_bps,
                        ))
                        is not None
                    )
                summary["non_overlapping"] = summarize(selected, horizons)
                summary["non_overlapping"]["signals"] = len(selected)
            side_result[variant] = summary
        result["sides"][side] = side_result
    return result


def render_markdown(result: dict[str, object]) -> str:
    """Renders a compact human-readable report."""
    lines = [
        "# Raw Edge Diagnostic",
        "",
        "Entry: Fibonacci test + EMA test, `ema_stack` and `structure_state` aligned.",
        "Entry price: next 15m bar open. No stop, target, halt, or position state.",
        f"Costs: {result['cost_bps']} bps round trip. Barrier: {result['barrier']['target_atr']}:{result['barrier']['stop_atr']} ATR, {result['barrier']['bars']} bars, stop first.",
        "",
        "## Bundles",
        "",
        "| Bundle | Bars | Fib long | Fib short | EMA |",
        "|---|---:|---:|---:|---:|",
    ]
    for bundle in result["bundles"]:
        lines.append(
            f"| {bundle['label']} | {bundle['bars']} | {bundle['fib_long']} | {bundle['fib_short']} | {bundle['ema']} |"
        )
    for side in SIDES:
        lines += ["", f"## {side.capitalize()}", ""]
        lines += [
            "| Variant | Signals | 5-bar net | 10-bar net | 20-bar net | 20-bar hit | MFE 20 | MAE 20 | Target | Stop | Timeout | Mean R |",
            "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|",
        ]
        for variant in VARIANTS:
            summary = result["sides"][side][variant]
            h5 = summary["horizons"]["5"]
            h10 = summary["horizons"]["10"]
            h20 = summary["horizons"]["20"]
            barrier = summary["barrier"]
            lines.append(
                f"| {variant} | {summary['signals']} | "
                f"{_fmt(h5.get('mean_net_pct'))} | {_fmt(h10.get('mean_net_pct'))} | {_fmt(h20.get('mean_net_pct'))} | "
                f"{_pct(h20.get('hit_rate'))} | {_fmt(h20.get('mean_mfe_atr'))} | {_fmt(h20.get('mean_mae_atr'))} | "
                f"{_pct(barrier.get('target_rate'))} | {_pct(barrier.get('stop_rate'))} | {_pct(barrier.get('timeout_rate'))} | "
                f"{_fmt(barrier.get('mean_r'))} |"
            )
        full = result["sides"][side]["full"]
        non = full["non_overlapping"]
        twenty_non = non["horizons"]["20"]
        barrier_non = non["barrier"]
        lines += [
            "",
            "Non-overlapping full signals: "
            f"{non['signals']} signals; 20-bar net {_fmt(twenty_non.get('mean_net_pct'))}% "
            f"(95% CI {_ci(twenty_non.get('ci95_net_pct'))}); "
            f"mean R {_fmt(barrier_non.get('mean_r'))} "
            f"(95% CI {_ci(barrier_non.get('ci95_r'))}); "
            f"target rate {_pct(barrier_non.get('target_rate'))}.",
        ]
        lines += [
            "",
            "Full-signal horizons:",
            "",
            "| Bars | Count | Mean gross | Mean net | Median net | Hit rate | MFE | MAE |",
            "|---:|---:|---:|---:|---:|---:|---:|---:|",
        ]
        for horizon in result["horizons"]:
            row = full["horizons"][str(horizon)]
            lines.append(
                f"| {horizon} | {row['count']} | {_fmt(row.get('mean_gross_pct'))}% | "
                f"{_fmt(row.get('mean_net_pct'))}% | {_fmt(row.get('median_gross_pct'))}% | "
                f"{_pct(row.get('hit_rate'))} | {_fmt(row.get('mean_mfe_atr'))} | {_fmt(row.get('mean_mae_atr'))} |"
            )
    return "\n".join(lines) + "\n"


def _fmt(value: object) -> str:
    if value is None:
        return "—"
    return f"{float(value):.4f}"


def _pct(value: object) -> str:
    if value is None:
        return "—"
    return f"{float(value) * 100.0:.1f}%"


def _ci(value: object) -> str:
    if not isinstance(value, list) or len(value) != 2:
        return "—"
    return f"{float(value[0]):.4f}, {float(value[1]):.4f}"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--model", action="append", required=True, help="model bundle directory")
    parser.add_argument("--out", required=True, help="output directory")
    parser.add_argument("--horizons", default="1,3,5,10,20")
    parser.add_argument("--target-atr", type=float, default=3.0)
    parser.add_argument("--stop-atr", type=float, default=1.0)
    parser.add_argument("--barrier-bars", type=int, default=20)
    parser.add_argument("--cost-bps", type=float, default=6.0)
    args = parser.parse_args()
    horizons = tuple(sorted({int(value) for value in args.horizons.split(",")}))
    if not horizons or min(horizons) <= 0:
        raise ValueError("horizons must be positive")
    bundles = [load_bundle(Path(path)) for path in args.model]
    bundles.sort(key=lambda bundle: bundle.label)
    result = evaluate(
        bundles,
        horizons,
        args.target_atr,
        args.stop_atr,
        args.barrier_bars,
        args.cost_bps,
    )
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    (out / "raw_edge.json").write_text(
        json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    (out / "raw_edge.md").write_text(render_markdown(result), encoding="utf-8")
    for side in SIDES:
        full = result["sides"][side]["full"]
        twenty = full["horizons"]["20"]
        barrier = full["barrier"]
        print(
            f"{side}: signals {full['signals']} | 20-bar net {twenty.get('mean_net_pct')}% | "
            f"hit {twenty.get('hit_rate')} | target {barrier.get('target_rate')} | "
            f"stop {barrier.get('stop_rate')} | mean R {barrier.get('mean_r')}"
        )
    print(f"report: {out / 'raw_edge.md'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
