#!/usr/bin/env python3
"""Screen a structure-based distance geometry for cost feasibility.

The screen answers one question before any path diagnostic or engine change:
given a published range width `W`, a stop at `stop_fraction * W`, a target at
`target_fraction * W`, and a round-trip cost, can the declared geometry pay
without requiring a win rate above 50%?

It reads a model bundle's `events.csv` and `readings.csv` only. It does not read
a candle path, a fill, a position, a backtest, or a P&L. It is a deterministic
distance screen, not a strategy result and not a path simulation.

The event is the 5m compression-range onset `(type 3, subtype 9, source 5)`. The
range high and low are the event's primary and reference values. The event is
available at the event bar's close; the screen uses the boundary price as the
entry reference. First-touch, gaps, and the ambiguous both-touch rule belong to
the later path diagnostic.

Usage:
    python3 tools/research/scale_screen.py \
        --bundles data/improvement/2026-11-10/SBE-01/artifacts/model/BTCUSD \
        --stop-fraction 0.3 --target-fraction 0.6 \
        --cost-bps 6 --sensitivity-bps 9 \
        --out data/improvement/2026-11-10/SBE-01/scale-screen
"""

from __future__ import annotations

import argparse
import csv
import json
import math
from dataclasses import dataclass, field
from pathlib import Path
from typing import Iterable

SCHEMA = "quantrule.scale-screen/v1"
TIMEFRAME_SECS = 300
EVENT_TYPE = 3
EVENT_SUBTYPE = 9
EVENT_SOURCE = 5
MIN_POOLED_EVENTS = 200
MIN_YEAR_EVENTS = 20
BREAKEVEN_LIMIT = 0.50


class ScaleScreenError(ValueError):
    """An input cannot be screened."""


@dataclass(frozen=True)
class Onset:
    """One compression-range onset event."""

    year: int
    bar: int
    high: float
    low: float
    atr: float


@dataclass(frozen=True)
class Sample:
    """One side of one event at one cost level."""

    year: int
    bar: int
    side: str
    width: float
    width_atr: float
    width_bps: float
    stop_bps: float
    target_bps: float
    stop_price: float
    target_price: float
    cost_price: float
    cost_r: float
    net_target: float
    net_stop: float
    breakeven: float | None
    atr_stop_bps: float
    atr_target_bps: float
    atr_cost_r: float
    atr_net_target: float
    atr_net_stop: float
    atr_breakeven: float | None


@dataclass
class Aggregate:
    """The distribution of one set of samples."""

    count: int = 0
    net_target_positive: int = 0
    pass_count: int = 0
    event_keys: set[tuple[int, int]] = field(default_factory=set)
    width: list[float] | None = None
    width_atr: list[float] | None = None
    width_bps: list[float] | None = None
    stop_bps: list[float] | None = None
    target_bps: list[float] | None = None
    cost_r: list[float] | None = None
    net_target: list[float] | None = None
    breakeven: list[float] | None = None
    atr_cost_r: list[float] | None = None
    atr_breakeven: list[float] | None = None

    def __post_init__(self) -> None:
        self.width = []
        self.width_atr = []
        self.width_bps = []
        self.stop_bps = []
        self.target_bps = []
        self.cost_r = []
        self.net_target = []
        self.breakeven = []
        self.atr_cost_r = []
        self.atr_breakeven = []

    def add(self, sample: Sample) -> None:
        self.count += 1
        self.event_keys.add((sample.year, sample.bar))
        self.net_target_positive += int(sample.net_target > 0.0)
        self.pass_count += int(
            sample.net_target > 0.0
            and sample.breakeven is not None
            and sample.breakeven <= BREAKEVEN_LIMIT
        )
        self.width.append(sample.width)
        self.width_atr.append(sample.width_atr)
        self.width_bps.append(sample.width_bps)
        self.stop_bps.append(sample.stop_bps)
        self.target_bps.append(sample.target_bps)
        self.cost_r.append(sample.cost_r)
        self.net_target.append(sample.net_target)
        if sample.breakeven is not None:
            self.breakeven.append(sample.breakeven)
        self.atr_cost_r.append(sample.atr_cost_r)
        if sample.atr_breakeven is not None:
            self.atr_breakeven.append(sample.atr_breakeven)


def _float(value: str | None) -> float | None:
    if value is None:
        return None
    try:
        parsed = float(value)
    except (TypeError, ValueError):
        return None
    return parsed if math.isfinite(parsed) else None


def _int(value: str | None) -> int | None:
    if value is None:
        return None
    try:
        return int(value)
    except (TypeError, ValueError):
        return None


def parse_readings(path: Path, timeframe_secs: int) -> dict[int, float]:
    """Returns the finite positive ATR at each bar of one timeframe."""
    atr: dict[int, float] = {}
    with path.open(newline="", encoding="utf-8") as handle:
        for row in csv.DictReader(handle):
            if _int(row.get("timeframe_secs")) != timeframe_secs:
                continue
            bar = _int(row.get("bar"))
            value = _float(row.get("atr"))
            if bar is None or value is None or value <= 0.0:
                continue
            atr[bar] = value
    return atr


def parse_events(path: Path, timeframe_secs: int, year: int) -> list[Onset]:
    """Returns the 5m onset events with a finite positive range and ATR bar."""
    events: list[Onset] = []
    with path.open(newline="", encoding="utf-8") as handle:
        for row in csv.DictReader(handle):
            if _int(row.get("timeframe_secs")) != timeframe_secs:
                continue
            if _int(row.get("event_type_code")) != EVENT_TYPE:
                continue
            if _int(row.get("event_subtype_code")) != EVENT_SUBTYPE:
                continue
            if _int(row.get("source_code")) != EVENT_SOURCE:
                continue
            if _int(row.get("direction_code")) != 0:
                continue
            bar = _int(row.get("available_bar_index"))
            high = _float(row.get("primary_value"))
            low = _float(row.get("reference_value"))
            if bar is None or high is None or low is None:
                continue
            if high <= low or low <= 0.0:
                continue
            events.append(Onset(year=year, bar=bar, high=high, low=low, atr=0.0))
    events.sort(key=lambda event: event.bar)
    return events


def bundle_files(bundle: Path) -> Path:
    """Returns the directory that holds events.csv and readings.csv.

    A model report writes its bundle under `<out>/model/`; a fixture bundle may
    hold the files directly. Accept both without guessing from the contents.
    """
    model = bundle / "model"
    if (model / "events.csv").is_file() and (model / "readings.csv").is_file():
        return model
    return bundle


def load_year(bundle: Path, year: int, timeframe_secs: int) -> list[Onset]:
    """Loads one year's events and attaches the event-bar ATR."""
    files = bundle_files(bundle)
    events = parse_events(files / "events.csv", timeframe_secs, year)
    atr = parse_readings(files / "readings.csv", timeframe_secs)
    loaded: list[Onset] = []
    for event in events:
        value = atr.get(event.bar)
        if value is None:
            continue
        loaded.append(
            Onset(
                year=event.year,
                bar=event.bar,
                high=event.high,
                low=event.low,
                atr=value,
            )
        )
    return loaded


def _metric(
    boundary: float,
    width: float,
    atr: float,
    stop_fraction: float,
    target_fraction: float,
    cost_bps: float,
) -> tuple[float, float, float, float, float, float | None, float, float, float, float | None]:
    """Returns the structural and matched-ATR control metrics for one side."""
    stop_distance = stop_fraction * width
    target_distance = target_fraction * width
    cost_price = cost_bps / 10_000.0 * boundary
    cost_r = cost_price / stop_distance
    net_target = target_distance - cost_price
    net_stop = stop_distance + cost_price
    breakeven = (
        net_stop / (net_stop + net_target)
        if net_target > 0.0 and net_stop + net_target > 0.0
        else None
    )
    atr_stop = atr
    atr_target = 2.0 * atr
    atr_cost_r = cost_price / atr_stop
    atr_net_target = atr_target - cost_price
    atr_net_stop = atr_stop + cost_price
    atr_breakeven = (
        atr_net_stop / (atr_net_stop + atr_net_target)
        if atr_net_target > 0.0 and atr_net_stop + atr_net_target > 0.0
        else None
    )
    return (
        stop_distance,
        target_distance,
        cost_price,
        cost_r,
        net_target,
        breakeven,
        atr_cost_r,
        atr_net_target,
        atr_net_stop,
        atr_breakeven,
    )


def samples_for(
    events: Iterable[Onset],
    stop_fraction: float,
    target_fraction: float,
    cost_bps: float,
) -> list[Sample]:
    """Builds one long and one short sample per event."""
    samples: list[Sample] = []
    for event in events:
        width = event.high - event.low
        if width <= 0.0 or event.atr <= 0.0:
            continue
        for side, boundary in (("long", event.high), ("short", event.low)):
            (
                stop_distance,
                target_distance,
                cost_price,
                cost_r,
                net_target,
                breakeven,
                atr_cost_r,
                atr_net_target,
                atr_net_stop,
                atr_breakeven,
            ) = _metric(
                boundary,
                width,
                event.atr,
                stop_fraction,
                target_fraction,
                cost_bps,
            )
            samples.append(
                Sample(
                    year=event.year,
                    bar=event.bar,
                    side=side,
                    width=width,
                    width_atr=width / event.atr,
                    width_bps=width / boundary * 10_000.0,
                    stop_bps=stop_distance / boundary * 10_000.0,
                    target_bps=target_distance / boundary * 10_000.0,
                    stop_price=boundary - stop_distance
                    if side == "long"
                    else boundary + stop_distance,
                    target_price=boundary + target_distance
                    if side == "long"
                    else boundary - target_distance,
                    cost_price=cost_price,
                    cost_r=cost_r,
                    net_target=net_target,
                    net_stop=stop_distance + cost_price,
                    breakeven=breakeven,
                    atr_stop_bps=event.atr / boundary * 10_000.0,
                    atr_target_bps=2.0 * event.atr / boundary * 10_000.0,
                    atr_cost_r=atr_cost_r,
                    atr_net_target=atr_net_target,
                    atr_net_stop=atr_net_stop,
                    atr_breakeven=atr_breakeven,
                )
            )
    return samples


def percentile(values: list[float], fraction: float) -> float | None:
    """Linear-interpolation percentile; deterministic for a sorted input."""
    if not values:
        return None
    if not 0.0 <= fraction <= 1.0:
        raise ScaleScreenError("fraction must be in 0..1")
    ordered = sorted(values)
    if len(ordered) == 1:
        return ordered[0]
    position = (len(ordered) - 1) * fraction
    lower = math.floor(position)
    upper = math.ceil(position)
    if lower == upper:
        return ordered[lower]
    weight = position - lower
    return ordered[lower] * (1.0 - weight) + ordered[upper] * weight


def _summary(values: list[float]) -> dict[str, float | int | None]:
    if not values:
        return {
            "count": 0,
            "mean": None,
            "p10": None,
            "median": None,
            "p75": None,
            "p90": None,
        }
    return {
        "count": len(values),
        "mean": math.fsum(values) / len(values),
        "p10": percentile(values, 0.10),
        "median": percentile(values, 0.50),
        "p75": percentile(values, 0.75),
        "p90": percentile(values, 0.90),
    }


def aggregate(samples: Iterable[Sample]) -> dict[str, object]:
    """Aggregates one set of samples."""
    total = Aggregate()
    by_year: dict[int, Aggregate] = {}
    for sample in samples:
        total.add(sample)
        by_year.setdefault(sample.year, Aggregate()).add(sample)

    def block(aggregate: Aggregate) -> dict[str, object]:
        return {
            "events": len(aggregate.event_keys),
            "samples": aggregate.count,
            "net_target_positive": aggregate.net_target_positive,
            "net_target_positive_rate": (
                aggregate.net_target_positive / aggregate.count
                if aggregate.count
                else None
            ),
            "pass_count": aggregate.pass_count,
            "pass_rate": aggregate.pass_count / aggregate.count if aggregate.count else None,
            "width": _summary(aggregate.width),
            "width_atr": _summary(aggregate.width_atr),
            "width_bps": _summary(aggregate.width_bps),
            "stop_bps": _summary(aggregate.stop_bps),
            "target_bps": _summary(aggregate.target_bps),
            "cost_r": _summary(aggregate.cost_r),
            "net_target": _summary(aggregate.net_target),
            "breakeven": _summary(aggregate.breakeven),
            "atr_control": {
                "cost_r": _summary(aggregate.atr_cost_r),
                "breakeven": _summary(aggregate.atr_breakeven),
            },
        }

    return {
        "pooled": block(total),
        "years": {str(year): block(by_year[year]) for year in sorted(by_year)},
    }


def verdict(base: dict[str, object]) -> tuple[str, list[str]]:
    """Applies the pre-registered go/no-go rule."""
    pooled = base["pooled"]
    years = base["years"]
    reasons: list[str] = []
    assert isinstance(pooled, dict)
    assert isinstance(years, dict)
    count = int(pooled["events"])
    if count < MIN_POOLED_EVENTS:
        reasons.append(f"pooled valid events {count} < {MIN_POOLED_EVENTS}")
    for year in sorted(years):
        year_block = years[year]
        assert isinstance(year_block, dict)
        year_count = int(year_block["events"])
        if year_count < MIN_YEAR_EVENTS:
            reasons.append(f"{year} valid events {year_count} < {MIN_YEAR_EVENTS}")
    net_target = pooled["net_target"]
    breakeven = pooled["breakeven"]
    assert isinstance(net_target, dict)
    assert isinstance(breakeven, dict)
    if net_target["median"] is None or net_target["median"] <= 0.0:
        reasons.append("pooled median net target is not positive")
    if net_target["p75"] is None or net_target["p75"] <= 0.0:
        reasons.append("pooled 75th-percentile net target is not positive")
    if breakeven["median"] is None or breakeven["median"] > BREAKEVEN_LIMIT:
        reasons.append("pooled median breakeven is above 0.50")
    if breakeven["p75"] is None or breakeven["p75"] > BREAKEVEN_LIMIT:
        reasons.append("pooled 75th-percentile breakeven is above 0.50")
    if any("valid events" in reason for reason in reasons):
        return "underpowered", reasons
    return ("feasible" if not reasons else "infeasible"), reasons


def evaluate_bundles(
    bundles: Path,
    stop_fraction: float,
    target_fraction: float,
    cost_bps: float,
    timeframe_secs: int = TIMEFRAME_SECS,
) -> tuple[dict[str, object], list[Sample]]:
    """Loads every year directory under `bundles` and evaluates one cost."""
    years = sorted(
        int(path.name)
        for path in bundles.iterdir()
        if path.is_dir() and path.name.isdigit()
    )
    if not years:
        raise ScaleScreenError(f"{bundles} has no year directories")
    all_samples: list[Sample] = []
    for year in years:
        events = load_year(bundles / str(year), year, timeframe_secs)
        all_samples.extend(
            samples_for(events, stop_fraction, target_fraction, cost_bps)
        )
    return aggregate(all_samples), all_samples


def _format(value: object, digits: int = 4) -> str:
    if value is None:
        return "—"
    if isinstance(value, float):
        return f"{value:.{digits}f}"
    return str(value)


def render_markdown(document: dict[str, object]) -> str:
    """Renders the deterministic report."""
    parameters = document["parameters"]
    base = document["base"]
    sensitivity = document["sensitivity"]
    assert isinstance(parameters, dict)
    assert isinstance(base, dict)
    assert isinstance(sensitivity, dict)
    base_pooled = base["pooled"]
    assert isinstance(base_pooled, dict)

    lines = [
        "# SBE-01 Scale Screen",
        "",
        "- Claim type: scale and cost-feasibility screen; no edge claim",
        f"- Entry: 5m compression-range onset `(3,9,5)`",
        f"- Geometry: stop `{parameters['stop_fraction']:.2f} W`, target `{parameters['target_fraction']:.2f} W`",
        f"- Base cost: {parameters['base_cost_bps']} bps round trip; sensitivity: {parameters['sensitivity_cost_bps']} bps",
        f"- Verdict: **{base['verdict']}**",
        "",
        "## Pooled",
        "",
        "| Metric | Median | p75 | p90 |",
        "|---|---:|---:|---:|",
    ]
    for key, label in (
        ("width_atr", "W / ATR"),
        ("stop_bps", "Stop (bps)"),
        ("target_bps", "Target (bps)"),
        ("cost_r", "Cost / R"),
        ("net_target", "Net target (price)"),
        ("breakeven", "Breakeven win rate"),
    ):
        block = base_pooled[key]
        assert isinstance(block, dict)
        lines.append(
            f"| {label} | {_format(block['median'])} | {_format(block['p75'])} | {_format(block['p90'])} |"
        )
    lines += [
        "",
        f"- Valid events: {base_pooled['events']} ({base_pooled['samples']} side samples)",
        f"- Net target positive: {base_pooled['net_target_positive']} samples ({_format(base_pooled['net_target_positive_rate'])})",
        f"- Breakeven at or below 0.50: {base_pooled['pass_count']} samples ({_format(base_pooled['pass_rate'])})",
        f"- Reasons: {'; '.join(base['reasons']) if base['reasons'] else 'none'}",
        "",
        "## Per year",
        "",
        "| Year | Events | Samples | W/ATR median | Stop bps | Cost/R median | Breakeven median | p75 | Pass rate |",
        "|---:|---:|---:|---:|---:|---:|---:|---:|---:|",
    ]
    years = base["years"]
    assert isinstance(years, dict)
    for year in sorted(years):
        block = years[year]
        assert isinstance(block, dict)
        width_atr = block["width_atr"]
        stop_bps = block["stop_bps"]
        cost_r = block["cost_r"]
        breakeven = block["breakeven"]
        assert isinstance(width_atr, dict)
        assert isinstance(stop_bps, dict)
        assert isinstance(cost_r, dict)
        assert isinstance(breakeven, dict)
        lines.append(
            f"| {year} | {block['events']} | {block['samples']} | {_format(width_atr['median'])} | "
            f"{_format(stop_bps['median'])} | {_format(cost_r['median'])} | "
            f"{_format(breakeven['median'])} | {_format(breakeven['p75'])} | "
            f"{_format(block['pass_rate'])} |"
        )
    lines += [
        "",
        "## Matched ATR control",
        "",
        "| Cost bps | Cost/R median | Breakeven median | Breakeven p75 |",
        "|---:|---:|---:|---:|",
    ]
    for label, block in (("base", base), ("sensitivity", sensitivity)):
        pooled = block["pooled"]
        assert isinstance(pooled, dict)
        atr = pooled["atr_control"]
        assert isinstance(atr, dict)
        cost_r = atr["cost_r"]
        breakeven = atr["breakeven"]
        assert isinstance(cost_r, dict)
        assert isinstance(breakeven, dict)
        cost = parameters["base_cost_bps"] if label == "base" else parameters["sensitivity_cost_bps"]
        lines.append(
            f"| {cost} | {_format(cost_r['median'])} | {_format(breakeven['median'])} | {_format(breakeven['p75'])} |"
        )
    lines += [
        "",
        "## Limitations",
        "",
        "- The screen uses the boundary price as the entry reference. It does not",
        "  simulate the next-bar touch, gaps, slippage, or the ambiguous both-touch",
        "  rule; those belong to the path diagnostic.",
        "- The screen cannot prove an edge. It only shows whether the declared",
        "  geometry is cost-feasible before more compute is spent.",
        "- The geometry is pre-registered. A failed screen is not a reason to search",
        "  fractions.",
        "",
    ]
    return "\n".join(lines)


def write_report(document: dict[str, object], out: Path) -> None:
    out.mkdir(parents=True, exist_ok=True)
    (out / "scale_screen.json").write_text(
        json.dumps(document, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    (out / "scale_screen.md").write_text(render_markdown(document), encoding="utf-8")


def run(
    bundles: Path,
    stop_fraction: float,
    target_fraction: float,
    cost_bps: float,
    sensitivity_bps: float,
    out: Path,
    timeframe_secs: int = TIMEFRAME_SECS,
) -> dict[str, object]:
    """Evaluates the base and sensitivity costs and writes the report."""
    if stop_fraction <= 0.0 or target_fraction <= 0.0:
        raise ScaleScreenError("stop and target fractions must be positive")
    if cost_bps < 0.0 or sensitivity_bps < 0.0:
        raise ScaleScreenError("costs must be non-negative")
    base_aggregate, _ = evaluate_bundles(
        bundles, stop_fraction, target_fraction, cost_bps, timeframe_secs
    )
    sensitivity_aggregate, _ = evaluate_bundles(
        bundles, stop_fraction, target_fraction, sensitivity_bps, timeframe_secs
    )
    base_verdict, base_reasons = verdict(base_aggregate)
    sensitivity_verdict, sensitivity_reasons = verdict(sensitivity_aggregate)
    document: dict[str, object] = {
        "schema": SCHEMA,
        "parameters": {
            "stop_fraction": stop_fraction,
            "target_fraction": target_fraction,
            "base_cost_bps": cost_bps,
            "sensitivity_cost_bps": sensitivity_bps,
            "timeframe_secs": timeframe_secs,
            "geometry": "range_fraction",
            "entry": "compression_range_onset",
        },
        "base": {
            **base_aggregate,
            "verdict": base_verdict,
            "reasons": base_reasons,
        },
        "sensitivity": {
            **sensitivity_aggregate,
            "verdict": sensitivity_verdict,
            "reasons": sensitivity_reasons,
        },
    }
    write_report(document, out)
    return document


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bundles", type=Path, required=True)
    parser.add_argument("--stop-fraction", type=float, default=0.3)
    parser.add_argument("--target-fraction", type=float, default=0.6)
    parser.add_argument("--cost-bps", type=float, default=6.0)
    parser.add_argument("--sensitivity-bps", type=float, default=9.0)
    parser.add_argument("--timeframe-secs", type=int, default=TIMEFRAME_SECS)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    try:
        document = run(
            bundles=args.bundles,
            stop_fraction=args.stop_fraction,
            target_fraction=args.target_fraction,
            cost_bps=args.cost_bps,
            sensitivity_bps=args.sensitivity_bps,
            out=args.out,
            timeframe_secs=args.timeframe_secs,
        )
    except (OSError, ScaleScreenError) as error:
        print(f"scale screen failed: {error}")
        return 1
    base = document["base"]
    assert isinstance(base, dict)
    pooled = base["pooled"]
    assert isinstance(pooled, dict)
    print(
        f"ok scale screen -> {args.out} | verdict {base['verdict']} | "
        f"events {pooled['events']} | "
        f"breakeven median {_format(pooled['breakeven']['median'])}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
